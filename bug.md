# รายงาน Bug: Action = "Wait" แต่มีการยิงออเดอร์ (PUT) บน Deriv จริง

**วันที่บันทึก:** 29 สิงหาคม 2026  
**สถานะ:** รอการแก้ไข (Pending Fix)  
**ไฟล์ที่เกี่ยวข้อง:**  
- [src/deriv.rs](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/deriv.rs) (บรรทัด ~1094-1098, ~1489-1494, ~1524-1598)
- [src/get_action.rs](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/get_action.rs) (บรรทัด ~351-358)
- [src/getActionByPKTrend.rs](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/getActionByPKTrend.rs)

---

## 1. อาการของปัญหา (Symptom)
บนหน้าเว็บ `index_short_term.html` ในตาราง **Recent Trade History**:
- คอลัมน์ **Action** แสดงเป็น `Wait`
- แต่มีการส่งคำสั่งเทรดจริงบน Deriv พร้อมยอดเงิน Stake (เช่น $17.00, $70.00, $300.00) และมีกำไร/ขาดทุนบันทึกจริง
- มักเกิดขึ้นเมื่ออยู่ในรอบ Martingale ไม้ลึก ๆ เช่น ไม้ที่ 5, 7, 9 (`subTradeno` = 5, 7, 9) และ Strategy Code เป็นกลุ่ม `V2-PKT-SPK-BEARTRAP`, `V2-PKT-SPK-NODIRECTION` เป็นต้น

---

## 2. สาเหตุของปัญหา (Root Cause)

### 2.1 สัญญาณถูกประเมินเป็น `Wait`
เมื่อบอทอยู่ในโหมด Martingale (`state.in_martingale == true` หรือ `loss_con >= 3`):
1. กลยุทธ์ `V2` ขอยืมสัญญาณจาก `getActionByPKTrend` ([src/get_action.rs](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/get_action.rs#L353))
2. สัญญาณ PKTrend ตรวจพบว่ากราฟไม่มีทิศทางชัดเจนหรือเป็นโซนดัก (Trap) จึงคืนค่า `suggest_color = "idle"`
3. ระบบใน [src/deriv.rs](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/deriv.rs#L1094) แปลงเป็น `TradeAction::Wait`:
   ```rust
   let trade_action = match suggest.as_str() {
       "green" => TradeAction::Call,
       "red" => TradeAction::Put,
       _ => TradeAction::Wait,
   };
   ```

### 2.2 โค้ดยิงออเดอร์ไม่มีการเช็ค `Wait` (Default เข้า `PUT`)
ใน [src/deriv.rs](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/deriv.rs#L1489) โค้ดตัดสินใจเลือกประเภทสัญญาด้วยเงื่อนไข binary `if/else`:
```rust
let contract_type = if trade_action == TradeAction::Call {
    "CALL"
} else {
    "PUT" // <-- เมื่อ trade_action == TradeAction::Wait จะหลุดมาเข้าเคสนี้ทันที
};
```
ทำให้บอทนำสัญญา `"PUT"` ไปส่ง Proposal / Direct Buy ไปยัง Deriv ทันที ทั้งๆ ที่สัญญาณควรจะ `Wait` (รอ)

### 2.3 การบันทึก Log แสดง `Wait`
โค้ดใน [src/deriv.rs](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/deriv.rs#L1499) บันทึกค่า `thisAction` จาก enum `trade_action`:
```rust
"thisAction": format!("{:?}", trade_action), // ได้ "Wait"
```
จึงทำให้ข้อมูลที่บันทึกลง `trades.json` และแสดงบนหน้าเว็บมีค่าเป็น `Wait` สวนทางกับสัญญา `PUT` ที่ถูกยิงออกไปจริง

---

## 3. แนวทางการแก้ไขในอนาคต (Proposed Solution)

1. **ดักกรณี `TradeAction::Wait` ก่อนส่งคำสั่งซื้อ:**
   - เช็คว่าถ้า `trade_action == TradeAction::Wait` ให้ **ไม่ทำการเปิดออเดอร์** (ข้ามการส่ง Buy / Proposal)
   - ตัวอย่างรูปแบบ:
     ```rust
     if trade_action == TradeAction::Wait {
         // Log ข้ามการเทรด หรือบันทึกสถานะ Wait/Skipped ลง trades.json
         // กำหนดพฤติกรรม Martingale: คงค่า loss_con ไว้รอแท่งถัดไปที่มีสัญญาณชัดเจน
         return Ok(()); // หรือข้ามบล็อกการยิงออเดอร์
     }
     ```

2. **กำหนดพฤติกรรมของ Martingale เมื่อเจอ `Wait`:**
   - ตัดสินใจว่าในกรณีที่กำลังทบไม้ (Martingale) แล้วสัญญาณแนะนำให้ `Wait`:
     - **ทางเลือกที่ 1:** หยุดรอแท่งถัดไป โดยยังคงระดับไม้เดิมไว้ (Freeze Martingale Step) จนกว่าจะได้สัญญาณ `CALL` หรือ `PUT` ชัดเจน
     - **ทางเลือกที่ 2:** ถ้าจำเป็นต้องออกทุกแท่งใน Martingale จะต้องมี Default Rule ที่ชัดเจนกว่าการหลุดเข้า `PUT` โดยไม่ได้ตั้งใจ

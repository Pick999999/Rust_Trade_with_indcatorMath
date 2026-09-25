# แผนการพัฒนาระบบ Long Term Trade (Plan 1)

## 1. แนวทางการเพิ่มฟีเจอร์ (Architecture Decision)
**ตัดสินใจ:** สร้างไฟล์ HTML และ JS ใหม่แยกต่างหาก (เช่น `long_term_trade.html`, `long_term_trade.js`) 

**เหตุผล:**
1. **ไฟล์เดิมใหญ่และซับซ้อนมาก:** `index.html` เดิมมีขนาดใหญ่กว่า 2,600 บรรทัด การแทรก Logic ใหม่ที่มีความซับซ้อนและเงื่อนไขต่างกัน เสี่ยงทำให้ระบบ Multiplex เดิมเกิด Bug
2. **แยก Logic ชัดเจน (Separation of Concerns):** Long Term Trade มี Timeframe (15M - 1Day) เงื่อนไข EMA 3 เส้น และเงื่อนไขการขายแบบเฉพาะตัว การแยกไฟล์จะทำให้โค้ดเป็นระเบียบและจัดการง่าย
3. **ลดความซ้ำซ้อนและตีกันของตัวแปร:** โฟกัสการพัฒนาได้ตรงจุด ไม่ต้องพะวงกับตัวแปรเดิม
4. **UX/UI:** เพิ่มปุ่มนำทาง (Navigation) ใน `index.html` เพื่อลิงก์มายังหน้า `long_term_trade.html` ทำให้ผู้ใช้ยังคงรู้สึกว่าอยู่ในแอปพลิเคชันเดียวกัน

## 2. การออกแบบส่วนหน้าตาผู้ใช้งาน (UI)
หน้าตาจะยึดตามดีไซน์และธีมของระบบเดิม แต่มีเมนูตั้งค่าเฉพาะดังนี้:
* **Timeframe Selector:** Dropdown เลือก Timeframe (15M, 30M, 1H, 2H, 1Day)
* **Indicator Settings:** ช่องกรอกค่า Period สำหรับ EMA 3 เส้น (Short, Medium, Long)
* **Exit Strategy Settings:** 
  * ช่องกรอก `Target Profit` (เช่น 1 USD) เพื่อกำหนดเป้าหมายกำไรในออเดอร์นั้น
  * Checkbox "ขายเมื่อเกิดจุดตัดครั้งต่อไป" (กรณีไม่ต้องการกำหนด Target Profit)
* **Schedule Settings:** กำหนดเวลาเปิด-ปิดรับออเดอร์ (`startTimeTrade` และ `stopTimeTrade`)
* **Dashboard / Tracking Table:** ตารางแสดงสถานะ Order ที่เปิดอยู่ (Open Contracts) และแสดง Profit/Loss แบบ Real-time 

## 3. ลอจิกการทำงาน (Logic & Flow)
1. **Data Fetching:** เชื่อมต่อ WebSocket ของ Deriv API เพื่อ Subscribe ข้อมูลแท่งเทียน (OHLC) ตาม Timeframe ที่เลือก
2. **EMA Calculation:** นำข้อมูลแท่งเทียนมาคำนวณค่า EMA ทั้ง 3 เส้นแบบ Real-time ทุกครั้งที่ได้รับข้อมูลราคาอัปเดต
3. **Entry Condition (เงื่อนไขการเข้าเทรด):**
   * ตรวจสอบจุดตัดของเส้น EMA:
     1. EMA Short ตัดกับ EMA Medium
     2. EMA Long ตัดกับ EMA Short และ EMA Medium
   * ตรวจสอบเวลา: ตรวจสอบว่าเวลาปัจจุบันอยู่ในช่วง `startTimeTrade` ถึง `stopTimeTrade` หรือไม่
   * หากเงื่อนไขทั้งหมดครบถ้วน ระบบจะส่งคำสั่ง **Buy**
4. **Exit Condition (เงื่อนไขการขายออเดอร์):**
   * ระบบจะ Subscribe เพื่อตรวจสอบสถานะของ Open Contract (ออเดอร์ที่ถืออยู่) เพื่อดู Profit/Loss อย่างต่อเนื่อง
   * **กรณีที่ 1 (Target Profit):** ถ้ายอด Profit ของออเดอร์นั้น มากกว่าหรือเท่ากับ Target ที่ตั้งไว้ -> ส่งคำสั่ง **Sell** ทันที
   * **กรณีที่ 2 (Sell on Next Cross):** หากไม่ตั้ง Target ระบบจะคำนวณ EMA ต่อไปเรื่อยๆ หากเกิดจุดตัดของ EMA ในทิศทางตรงกันข้ามกับขาที่เข้าเทรด -> ส่งคำสั่ง **Sell** ทันที

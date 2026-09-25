# สรุป Algorithm การหา Action ใน `get_action.rs`

ไฟล์นี้สรุปเงื่อนไขและ Algorithm ของแต่ละ Strategy ที่ใช้ในการตัดสินใจเปิดออเดอร์ (Call/Put) รวมถึงการส่งผ่านตัวแปร `full_analysis`

## การตรวจสอบ Parameter `full_analysis`
✅ **ตรวจสอบแล้ว:** ทุกฟังก์ชันใน `get_action.rs` ได้รับ parameter `full_analysis: Option<&FullAnalysisResult>` (หรือ `_full_analysis` สำหรับตัวที่ยังไม่ได้ใช้งานจริง) เรียบร้อยแล้ว พร้อมสำหรับการพัฒนา Algorithm ต่อไป

---

## 1. V1 — Default Strategy (Backward Compatible)
**ฟังก์ชัน:** `get_suggest_color(this_color: &str, loss_con: u32, _full_analysis: Option<&FullAnalysisResult>) -> String`

- **หลักการทำงาน:**
  - กำหนดค่าเริ่มต้นแนะนำเป็น `"green"` (Call) เสมอ
  - หากเสียต่อเนื่อง (Loss Streak) ต่ำกว่า 2 (`loss_con < 2`) จะยังคงแนะนำ `"green"` ต่อไป
  - **Martingale Override:** ถ้ายอดเสียต่อเนื่องตั้งแต่ 2 ขึ้นไป (`loss_con >= 2`) ระบบจะ**เปลี่ยนไปเล่นตามสีของแท่งเทียนปัจจุบัน** 
    - ถ้าแท่งปัจจุบันสีแดง (`this_color == "red"`) แนะนำ `"red"` (Put)
    - ถ้าไม่ใช่ แนะนำ `"green"` (Call)

## 2. V2 — Early React Strategy
**ฟังก์ชัน:** `get_suggest_color_v2(this_color: &str, loss_con: u32, _full_analysis: Option<&FullAnalysisResult>) -> String`

- **หลักการทำงาน:** ทำงานคล้าย V1 แต่จะตอบสนองไวกว่า (ลด Threshold)
  - กำหนดค่าเริ่มต้นแนะนำเป็น `"green"` เสมอเมื่อ `loss_con == 0`
  - ถ้ายอดเสียต่อเนื่องตั้งแต่ 1 ขึ้นไป (`loss_con >= 1`) ระบบจะเริ่มเล่นตามสีของแท่งเทียนปัจจุบันทันที
    - สีแดง แนะนำ `"red"`
    - สีเขียว แนะนำ `"green"`

## 3. V3A — EMA Direction Consensus
**ฟังก์ชัน:** `get_suggest_color_v3a(analysis: &AnalysisObject, loss_con: u32, _full_analysis: Option<&FullAnalysisResult>) -> String`

- **หลักการทำงาน:** ใช้การโหวตทิศทางจาก EMA 3 เส้น (Short, Medium, Long) เพื่อกรองสัญญาณหลอก (Noise) 
  - ฟังก์ชันนี้จะทำงานและเริ่มโหวตเมื่อ `loss_con >= 2` เท่านั้น (ต่ำกว่า 2 แนะนำ `"green"`)
  - **การนับคะแนนโหวต:** ถือว่าทิศทาง `Up` มีค่า 1 คะแนน
  - ถ้าคะแนนเสียงส่วนใหญ่โหวต `Up` (ตั้งแต่ 2 เส้นขึ้นไป) ระบบจะแนะนำ `"green"`
  - แต่ถ้าเสียงส่วนใหญ่มองลง (คะแนน `Up` น้อยกว่า 2) ระบบจะแนะนำ `"red"`

## 4. V3B — EMA Short Direction + CutType Hybrid
**ฟังก์ชัน:** `get_suggest_color_v3b(analysis: &AnalysisObject, loss_con: u32, _full_analysis: Option<&FullAnalysisResult>) -> String`

- **หลักการทำงาน:** อ้างอิงทิศทางหลักจาก EMA Short เป็นหลัก แต่จะถูก Override ได้ด้วยสัญญาณตัดเส้น (Crossover) จับ Reversal ได้ไวขึ้น
  - จะเข้าเงื่อนไขเมื่อ `loss_con >= 2`
  - **ขั้นที่ 1:** ตรวจสอบทิศทางของ EMA Short ถ้าเป็น `Up` ให้ `"green"` ถ้าเป็น `Down` ให้ `"red"`
  - **ขั้นที่ 2 (Override):** หากพบสัญญาณตัดเส้นที่รุนแรงกว่า จะยึดตามสัญญาณนั้นแทนทิศทางเดิม
    - ถ้าพบ `CutUp` (EMA Short ตัดขึ้น) ยืนยันแนะนำ `"green"` เสมอ
    - ถ้าพบ `CutDown` (EMA Short ตัดลง) ยืนยันแนะนำ `"red"` เสมอ

---

## Dispatcher & Action Builder

**`get_suggest_color_by_strategy` (Dispatcher)**
- เป็นตัวกลางรับชื่อ Strategy แบบ Enum (`SuggestStrategy::V1`, `V2`, `V3A`, `V3B`) จากนั้นจะทำการ Route ไปยังฟังก์ชันที่ตรงกัน
- ส่งผ่าน `full_analysis` ไปยังฟังก์ชันย่อยอย่างถูกต้อง

**`get_trade_action` (Main Engine)**
- เป็นจุดเริ่มต้นที่รับค่า Analysis ในแต่ละวินาที
- ตรวจสอบเบื้องต้นว่าแท่งเทียนปัจจุบันมี Spike หรือไม่ (`analysis.is_atr`) ถ้าไม่มีจะตอบ `TradeAction::Wait` ทันที
- ถ้าเป็น ATR Spike จะเรียกฟังก์ชัน `get_suggest_color` เพื่อหาสีที่เหมาะสม
- แล้วแปลงข้อความสี `"green"` หรือ `"red"` ให้กลายเป็น Action ที่ระบบเทรดเข้าใจคือ `TradeAction::Call` หรือ `TradeAction::Put`

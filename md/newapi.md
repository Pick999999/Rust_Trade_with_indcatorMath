# สรุปการปรับปรุงระบบใหม่ (New Deriv API & OTP WebSockets)

เอกสารนี้สรุปการปรับปรุงโค้ดฝั่งระบบเทรด Rust และ UI เพื่อรองรับการอัปเกรดระบบความปลอดภัยของ Deriv API ซึ่งต้องการการพิสูจน์ตัวตนผ่าน **OTP WebSocket** ในการขอข้อมูลดัชนีจำลอง (Synthetic Indices) และการแก้ไขจุดบกพร่องต่าง ๆ ที่เกิดขึ้น

---

## 1. การเปลี่ยนไปใช้ OTP WebSocket แทน Public WebSocket

เนื่องจาก Deriv API เวอร์ชันใหม่จำกัดการดึงข้อมูล Synthetic Indices (เช่น `1HZ10V`, `1HZ50V`, `1HZ100V`) โดย **Public WebSocket ที่ไม่ผ่านการล็อกอินจะไม่คืนค่าข้อมูลใด ๆ และจะฟ้องข้อผิดพลาด `InvalidSymbol`** ขณะเดียวกันโทเคนประเภท PAT (`pat_...`) ก็ไม่สามารถส่งคำสั่ง `authorize` ตรง ๆ ผ่าน WebSocket ได้

**ทางแก้ไข:** เปลี่ยนระบบให้ขอ OTP ผ่าน HTTPS REST API (`/otp`) เพื่อนำ URL เชื่อมต่อที่เป็นแบบ Authenticated มาใช้สำหรับดึงข้อมูลแท่งเทียน (Candle history / Ticks history)

### ฟังก์ชันที่ได้รับการแก้ไข

#### A. ในไฟล์ [long_term.rs](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/long_term.rs)
1. **`start_longterm_bot`**
   * **เดิม:** เชื่อมต่อไปยัง Public WS `wss://ws.derivws.com/websockets/v3?app_id=...` โดยไม่มีการส่ง Authorization
   * **ใหม่:** ดึง Credentials ที่ Active อยู่ด้วย `crate::get_active_credentials(None)` จากนั้นทำ POST request ไปที่ REST API `/otp` เพื่อนำ URL ที่ได้มาสร้างการเชื่อมต่อ WebSocket ดึงข้อมูลแท่งเทียน
2. **`fetch_lt_chart_candles`**
   * **เดิม:** ดึงประวัติกราฟย้อนหลังผ่าน Public WS
   * **ใหม่:** เปลี่ยนไปดึงข้อมูลผ่าน OTP WebSocket โดยใช้วิธีเรียก OTP REST API เช่นเดียวกับฟังก์ชันข้างต้นก่อนการเชื่อมต่อ

#### B. ในไฟล์ [deriv.rs](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/deriv.rs)
1. **`start_deriv_bot_multiplexed`**
   * **เดิม:** สร้าง 1 Public WS สำหรับรับข้อมูลแท่งเทียน และ 1 OTP Private WS สำหรับการเทรด
   * **ใหม่:** เปลี่ยนไปใช้ **2x OTP WebSockets** (เชื่อมต่อผ่าน OTP ทั้งสองฝั่ง ฝั่งดึงแท่งเทียน และ ฝั่งรับข้อมูลส่วนตัว/ส่งคำสั่งซื้อขาย) เพื่อหลีกเลี่ยงข้อผิดพลาดสัญลักษณ์ไม่ถูกต้อง
2. **`reconnect_public_ws`**
   * **เดิม:** สร้างการเชื่อมต่อ Public WS ใหม่ขึ้นมาเปล่า ๆ
   * **ใหม่:** อัปเกรดให้รับพารามิเตอร์ credentials (`api_token`, `app_id`, `account_id`) เพื่อทำการขอ OTP URL ใหม่ในการต่ออายุเชื่อมต่อดึงแท่งเทียน

---

## 2. การแก้ไขการส่งคำสั่งซื้อขาย (Manual & Auto Trade)

### ฟังก์ชันที่ได้รับการแก้ไข
* **`lt_manual_trade` command loop** ในไฟล์ [long_term.rs](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/long_term.rs)
  * **เดิม:** นำค่าตัวแปร `asset` ที่รับมาจาก frontend (ซึ่งส่งมาเป็นรูปแบบ `"vol10"`, `"vol50"`) ไประบุในฟิลด์ `"underlying_symbol"` ตรง ๆ ส่งผลให้ API ปฏิเสธคำสั่งเทรดเนื่องจากสัญลักษณ์ผิดรูปแบบ
  * **ใหม่:** เพิ่มคำสั่งสร้างตัวแปร `deriv_symbol = map_asset_to_symbol(asset);` ก่อนส่ง JSON Request ไปยังเซิร์ฟเวอร์ เพื่อเปลี่ยนรูปแบบเป็นสัญลักษณ์ของ Deriv เช่น `"1HZ10V"`, `"1HZ50V"`

---

## 3. การแก้ไขข้อบกพร่องฝั่ง Frontend UI (Refresh/Load)

### ฟังก์ชันที่ได้รับการแก้ไขในไฟล์ [long_term_trade.html](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/public/long_term_trade.html)

1. **`ltLoadSettings()`**
   * **เดิม:** มีการใช้ตัวแปร `assetMapping` เพื่อแปลงรหัสสินทรัพย์ขากลับ (เช่น แปลง `"vol10"` กลับเป็น `"R_10"`) ส่งผลให้ checkbox หน้าหลักไม่ติ๊กเลือกข้อมูลเนื่องจาก HTML ปลายทางได้เปลี่ยนเป็นรูปแบบ `vol*` หมดแล้ว
   * **ใหม่:** นำ `assetMapping` ออก และเช็ค checkbox ด้วย `settings.assets.includes(cb.value)` ตรง ๆ
2. **`ltChartAssetSelect` (Change Event Listener)**
   * **เดิม:** ดึงข้อมูลกราฟจากตัวแปร cache `ltCandlesData` เท่านั้น หากกด F5 รีเฟรชหน้าเว็บ ข้อมูลนี้จะว่างเปล่าทำให้กราฟมืด
   * **ใหม่:** ปรับให้ทำการเรียกดึงข้อมูลจาก API `/api/longterm/candles` ทันทีที่มีการเปลี่ยนหรือเริ่มต้นเลือกสินทรัพย์ เพื่ออัปเดตกราฟให้เป็นปัจจุบันที่สุดเสมอ
3. **การตั้งค่าเริ่มต้น (Auto-select fallback):**
   * ในฟังก์ชัน `loadSettings()` หากค่า `settings.chartAsset` ว่างเปล่า ระบบจะเลือกสินทรัพย์ตัวแรกที่กำลังเปิดใช้งานอยู่ในฝั่งซ้ายของหน้าจอขึ้นมาทำกราฟทันทีเพื่อป้องกันกราฟดำ
4. **ความคลีนของ Console Log:**
   * ทำการปิด (Comment out) คำสั่ง `console.log` ที่มีความถี่สูง เช่น `[lt_candle_update]`, `[lt_orders_update]` และ `order entry markers` เพื่อลดภาระการแสดงผลบน browser console

# สรุปการเปลี่ยนแปลง Deriv API (Version เก่า vs Version ใหม่)

จากการศึกษาเอกสารอ้างอิงและการทดสอบนำมาใช้จริง (Implement) ในการเปิดออเดอร์ Rise/Fall สามารถสรุปความแตกต่างและสิ่งที่เปลี่ยนแปลงไปจาก Deriv API เวอร์ชันเก่า ได้ดังนี้ครับ

## 1. สถาปัตยกรรมการเชื่อมต่อ (Authentication & Connection)
จุดเปลี่ยนที่ใหญ่ที่สุดคือวิธีการยืนยันตัวตน (Authentication) เพื่อเปิดคำสั่งเทรด

*   **Version เก่า (Legacy):**
    *   **การเชื่อมต่อ:** ใช้ WebSocket เส้นเดียวสำหรับทุกอย่าง (ทั้งดึงราคาและส่งคำสั่งเทรด) เช่น ต่อไปที่ `wss://ws.derivws.com/websockets/v3?app_id=1089`
    *   **การยืนยันตัวตน:** หลังจากเชื่อมต่อ WebSocket แล้ว จะต้องส่งคำสั่ง `{"authorize": "API_TOKEN"}` เข้าไปใน WebSocket เพื่อให้เซสชันนั้นมีสิทธิ์ในการเทรด

*   **Version ใหม่ (REST + OTP WebSocket):**
    *   **การยืนยันตัวตน:** เปลี่ยนไปใช้มาตรฐาน **OAuth 2.0 (Bearer Token)** แทน API Token แบบเดิม 
    *   **การเชื่อมต่อเพื่อเทรด (Authenticated):** ไม่มีการส่งคำสั่ง `authorize` ใน WebSocket อีกต่อไป แต่ต้องทำผ่าน 2 ขั้นตอนคือ:
        1.  ยิง HTTP POST (REST API) ไปที่ `https://api.derivws.com/trading/v1/options/accounts/{account_id}/otp` พร้อมแนบ `App ID` และ `Bearer Token`
        2.  ระบบจะคืนค่ากลับมาเป็น WebSocket URL พิเศษที่แนบ OTP ไว้ด้วย (เช่น `wss://api.derivws.com/.../ws/demo?otp=abc123xyz`)
        3.  เมื่อนำ URL นี้นำไปเชื่อมต่อ WebSocket **จะถือว่ายืนยันตัวตนเสร็จสมบูรณ์ทันที** และพร้อมส่งคำสั่งเทรด (`buy`, `sell`, `proposal`) ได้เลย
    *   **ข้อมูลสาธารณะ (Public Data):** หากต้องการแค่ดูราคา (Ticks) หรือข้อมูลตลาด ให้ต่อผ่าน `wss://api.derivws.com/trading/v1/options/ws/public` ได้เลยโดยไม่ต้องใช้ Token

## 2. โครงสร้างพารามิเตอร์การเทรดที่เข้มงวดขึ้น (Payload Validation)
ถึงแม้ว่าคำสั่งหลักอย่าง `proposal`, `buy` หรือ `sell` จะยังคงโครงสร้างคล้ายเดิม แต่ตัว API ใหม่มีความเข้มงวดในการตรวจสอบ (Validation) มากขึ้น:

*   **ฟิลด์ `symbol` เปลี่ยนเป็น `underlying_symbol`:**
    *   **เก่า:** ในคำสั่ง `proposal` หรือ `parameters` ในคำสั่ง `buy` สามารถใช้คีย์ `symbol: "R_100"` ได้ตรงๆ
    *   **ใหม่:** ต้องเปลี่ยนไปใช้คีย์ `underlying_symbol: "R_100"` เท่านั้น หากใช้แบบเก่าระบบจะแจ้ง Error: `Input validation failed: parameters`
*   **ยกเลิกฟิลด์ที่ไม่ได้ใช้แล้ว (เช่น `product_type`):**
    *   **เก่า:** คำสั่งเทรดบางประเภทต้องการพารามิเตอร์ `product_type: "basic"` เพื่อกำหนดชนิดสินค้า
    *   **ใหม่:** หากมีการส่ง `product_type` เข้าไปใน `parameters` ของคำสั่ง `buy` ระบบจะมองว่าเป็นฟิลด์ขยะและตีกลับ (Reject) ทันที (Input validation failed) ต้องตัดออกเหลือเฉพาะฟิลด์ที่จำเป็นเท่านั้น

## 3. เพิ่มขีดความสามารถฝั่ง REST API 
นอกจาก WebSocket สำหรับการเทรดแล้ว Version ใหม่ยังเพิ่ม Endpoint แบบ HTTP เข้ามาช่วยจัดการบัญชี ซึ่งเวอร์ชันเก่าไม่มีหรือทำได้ยาก เช่น:
*   `GET /trading/v1/options/accounts` ดึงลิสต์บัญชีทั้งหมด
*   `POST /trading/v1/options/accounts` สร้างบัญชี Options ใหม่
*   `POST /trading/v1/options/accounts/{account_id}/reset-demo-balance` รีเซ็ตยอดเงินบัญชี Demo ให้กลับมาเต็มได้ง่ายๆ ผ่าน HTTP POST

## 4. ข้อจำกัดและ Rate Limit
เพื่อให้ระบบมีความเสถียรขึ้น API ใหม่จึงได้ระบุข้อจำกัดที่ชัดเจน:
*   **REST API:** ถูกจำกัดที่ 60 requests / นาที / 1 Token
*   **WebSocket:** ส่งข้อมูลได้สูงสุด 100 requests / วินาที / 1 Connection และห้ามเปิด Connection ซ้อนกันเกิน 5 เส้นต่อผู้ใช้ 1 คน

---

### บทสรุปสำหรับนักพัฒนา
หากต้องอัปเกรดระบบจาก Legacy สู่ New API สิ่งที่ต้องปรับแก้หลักๆ คือ **"การแยก WebSocket เป็น 2 เส้น"** (เส้นหนึ่งดูราคาไม่ต้อง Auth, อีกเส้นสำหรับเทรดต้องขอ OTP ก่อนเปิด) และ **"การเก็บกวาด Payload คำสั่งเทรด"** ให้เหลือเฉพาะพารามิเตอร์ที่ตรงตามสเปกเป๊ะๆ เท่านั้นครับ

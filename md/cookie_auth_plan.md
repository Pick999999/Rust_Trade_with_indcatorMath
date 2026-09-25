# 🍪 แผนการยกระดับความปลอดภัยด้วย Fingerprint + HTTP-Only Cookies (Session Authentication)

เอกสารนี้อธิบายแผนการผสานระบบ Fingerprint เดิมเข้ากับระบบ **HTTP-Only Cookies** เพื่อเพิ่มความปลอดภัยและรองรับการจัดการ Session (API Key) แบบ Dynamic สำหรับผู้ใช้แต่ละคน

---

## 🎯 แนวคิดหลัก (Core Concept)

1. **ใช้ Fingerprint เป็นตัวระบุเครื่อง**: ยังคงใช้เบราว์เซอร์ Fingerprint เป็นตัวตรวจสอบสิทธิ์แรกเข้า
2. **Dynamic API Key**: เมื่อตรวจพบ Fingerprint ที่ได้รับอนุญาต ระบบจะสร้าง API Key ชุดใหม่ให้ทุกครั้งที่เปิดหน้าเว็บ (Rotating Key)
3. **HTTP-Only Cookie**: API Key ที่สร้างขึ้นใหม่นี้ จะไม่ถูกส่งกลับไปให้ JavaScript ฝั่ง Frontend อ่านตรงๆ แต่จะถูกเซ็ตลงใน **HTTP-Only Cookie** แทน เพื่อป้องกัน XSS Attack

---

## 🛠️ แผนการปรับปรุงโค้ด (Implementation Steps)

### 1. ปรับปรุงโครงสร้างไฟล์ `allowed_fingerprints.json`
เปลี่ยนจากเดิมที่เป็นแค่ Array ของ String ให้เป็น Object ที่เก็บข้อมูล API Key ของแต่ละคน:
```json
{
  "b2c9eb33c97d4b60477564be6a6696c2": {
    "apikey": "...",
    "updated_at": "2026-06-14T12:00:00Z"
  },
  "d23ba6650bd89ef94c008801086ac778": {
    "apikey": "...",
    "updated_at": "2026-06-14T12:00:00Z"
  }
}
```
*(ถ้าในไฟล์มีแค่รายชื่อ Fingerprint เปล่าๆ ระบบจะแปลงโครงสร้างให้อัตโนมัติตอนเริ่มรันเซิร์ฟเวอร์เพื่อให้เข้ากับระบบใหม่)*

### 2. ฝั่ง Backend (Rust / Axum)
*   **เพิ่ม Dependency `axum-extra`**: ใน `Cargo.toml` เปิดใช้ฟีเจอร์ `cookie` และ `cookie-private` (ถ้าต้องการเข้ารหัส Cookie ขั้นสุด) หรือใช้แค่ฟีเจอร์พื้นฐาน
*   **สร้าง Endpoint `POST /api/auth/fingerprint`**:
    *   รับค่า Fingerprint จากหน้าเว็บ
    *   ตรวจสอบว่ามี Fingerprint นี้ใน `allowed_fingerprints.json` หรือไม่
    *   ถ้า **มี**: สร้าง API Key สุ่ม (เช่น UUID) อัปเดตลงไฟล์ JSON (และในหน่วยความจำที่ Shared State)
    *   เซ็ต HTTP Response Header `Set-Cookie: auth_token=<API_KEY_ที่สร้างใหม่>; HttpOnly; Path=/; SameSite=Strict` กลับไปให้เบราว์เซอร์
*   **แก้ไข `auth_middleware`**:
    *   เลิกอ่านค่าจาก Header `X-API-Key`
    *   เปลี่ยนมาดึงค่าจาก `auth_token` Cookie
    *   นำค่า API Key จาก Cookie ไปตรวจสอบใน Memory State ว่าเป็น API Key ที่ Active อยู่จริงหรือไม่

### 3. ฝั่ง Frontend (HTML / JavaScript)
*   **ปรับปรุงตอนโหลดหน้าเว็บ**:
    *   เมื่อหน้าเว็บเปิดขึ้นมา จะสร้าง Fingerprint
    *   ยิง Request นำ Fingerprint ไปยัง `POST /api/auth/fingerprint` 
    *   เมื่อสำเร็จ (HTTP 200 OK) เบราว์เซอร์จะเก็บ Cookie ให้อัตโนมัติ จากนั้นหน้าเว็บถึงจะเริ่มดึงข้อมูล (Dashboard/Charts) และเชื่อมต่อ WebSocket
*   **ลบ Header X-API-Key**:
    *   ไม่ต้องใช้ Interceptor แนบ Header `X-API-Key` ใน Fetch อีกต่อไป
*   **WebSocket**:
    *   ลบ `?token=...` ออกจาก URL ของ WebSocket ขาเชื่อมต่อ เพราะเบราว์เซอร์จะจัดการส่ง Cookie ให้เองตอนทำ Handshake ของ WebSocket อัตโนมัติ

---

## 🔍 สรุปข้อดีของระบบใหม่
1. **ปลอดภัยจาก XSS**: API Key ถูกเก็บใน HTTP-Only Cookie ทำให้ JavaScript ของ Hacker ขโมยไม่ได้
2. **Rotating Session**: API Key เปลี่ยนใหม่ทุกครั้งที่โหลดหน้าเว็บ (Refresh/New Tab) หาก Cookie หลุดไปก็จะหมดอายุทันทีเมื่อผู้ใช้ตัวจริงเข้าเว็บใหม่
3. **ไร้รอยต่อ (Seamless)**: ผู้ใช้ยังคงได้รับประสบการณ์เดิมคือเข้าเว็บได้เลย (ใช้ Fingerprint) ไม่ต้องพิมพ์รหัสผ่าน แต่หลังบ้านปลอดภัยขึ้นมาก

---

> **การดำเนินการถัดไป:**
> หากแผนนี้ตรงตามความต้องการ (เก็บ Fingerprint ไว้เหมือนเดิม + เพิ่ม API Key ลงใน JSON และจับคู่ทำ HTTP-Only Cookie) แจ้งให้ผมเริ่มเขียนโค้ดได้เลยครับ!

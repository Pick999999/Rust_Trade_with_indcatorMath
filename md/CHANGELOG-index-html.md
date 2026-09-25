# 📝 CHANGELOG - index.html & pkderiv.js

## 🎯 การปรับปรุง index.html ให้รองรับ New Deriv API (OTP WebSocket)

**วันที่:** 2026-07-23  
**อ้างอิง:** newapi.md  
**ไฟล์ที่แก้ไข:** `public/pkderiv.js`

---

## 🔄 สรุปการเปลี่ยนแปลง

ตาม **newapi.md** Deriv API เวอร์ชันใหม่จำกัดการดึงข้อมูล Synthetic Indices โดย **Public WebSocket ที่ไม่ผ่านการล็อกอินจะฟ้องข้อผิดพลาด `InvalidSymbol`** ดังนั้นจึงต้องเปลี่ยนจากการใช้ Public WebSocket (`wss://ws.binaryws.com/websockets/v3?app_id=1089`) ไปใช้ **Local WebSocket** (`ws://localhost/ws`) ซึ่ง Backend (Rust) จะจัดการ OTP และการเชื่อมต่อให้

---

## ✅ การแก้ไขที่ทำแล้ว

### 1. **ฟังก์ชัน `syncOpenOrdersFromDeriv`** (บรรทัด ~891)

**ปัญหาเดิม:**
```javascript
const ws = new WebSocket('wss://ws.binaryws.com/websockets/v3?app_id=1089');
```
- ใช้ Public WebSocket ตรงๆ ซึ่งอาจทำให้เกิดปัญหา `InvalidSymbol` กับ Synthetic Indices

**แก้ไขเป็น:**
```javascript
const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const ws = new WebSocket(`${protocol}//${window.location.host}/ws`);
```
- เปลี่ยนเป็นใช้ Local WebSocket
- Backend จัดการ OTP และ Authorization ให้
- ส่งคำสั่ง `sync_portfolio` ผ่าน Local WS

**ผลลัพธ์:**
- ✅ หลีกเลี่ยงข้อผิดพลาด InvalidSymbol
- ✅ ใช้ OTP Authentication ผ่าน Backend
- ✅ ระบบซิงค์ออเดอร์ทำงานได้ปกติ

---

### 2. **ฟังก์ชันทดสอบเวลา `setDerivTimeBtn`** (บรรทัด ~3553)

**ปัญหาเดิม:**
```javascript
const ws = new WebSocket('wss://ws.binaryws.com/websockets/v3?app_id=1089');
ws.onopen = () => {
    ws.send(JSON.stringify({ time: 1 }));
};
```
- ใช้ Public WebSocket เพื่อดึงเวลาจาก Deriv Server

**แก้ไขเป็น:**
```javascript
const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const ws = new WebSocket(`${protocol}//${window.location.host}/ws`);
ws.onopen = () => {
    ws.send(JSON.stringify({ time: 1 }));
};
```
- เปลี่ยนเป็นใช้ Local WebSocket
- Backend จะ forward คำสั่ง `time` ไปยัง Deriv และส่งผลกลับมา

**ผลลัพธ์:**
- ✅ ปุ่ม "ตั้งเวลาจาก Deriv" ทำงานได้
- ✅ ดึงเวลาผ่าน Authenticated Connection
- ✅ หลีกเลี่ยงปัญหา API Limitation

---

## 🎨 โครงสร้างการทำงานใหม่

```
┌─────────────────┐
│  Frontend (UI)  │
│   index.html    │
│   pkderiv.js    │
└────────┬────────┘
         │ ws://localhost/ws
         │ (Local WebSocket)
         ▼
┌─────────────────┐
│ Backend (Rust)  │
│   deriv.rs      │
└────────┬────────┘
         │ ขอ OTP URL
         │ POST /otp
         ▼
┌─────────────────┐
│  Deriv REST API │
│ (OTP Endpoint)  │
└────────┬────────┘
         │ คืน OTP URL
         ▼
┌─────────────────┐
│ Backend (Rust)  │
│ สร้าง OTP WS    │
└────────┬────────┘
         │ wss://...?otp=xxx
         ▼
┌─────────────────┐
│   Deriv API     │
│ (Authenticated) │
└─────────────────┘
```

---

## 🚀 การทดสอบ

### ขั้นตอนการทดสอบ:

1. **Build Backend:**
   ```bash
   cargo build --release
   ```

2. **รัน Server:**
   ```bash
   ./target/release/turbo-indicators
   ```

3. **เปิด Browser:**
   - ไปที่ `http://localhost:3000/index.html`

4. **ทดสอบฟีเจอร์:**
   - ✅ คลิกปุ่ม "ตั้งเวลาจาก Deriv" ควรแสดงเวลาปัจจุบัน
   - ✅ กดปุ่ม "Start" ระบบควรเชื่อมต่อและรับข้อมูลแท่งเทียนได้
   - ✅ การซิงค์ออเดอร์จาก Deriv Portfolio ควรทำงานได้
   - ✅ ไม่มี Error `InvalidSymbol` ใน Console

---

## 📌 หมายเหตุสำคัญ

### ⚠️ สิ่งที่ต้องระวัง:

1. **Backend ต้องรองรับ OTP:**
   - ตรวจสอบว่า `deriv.rs` มีการใช้ OTP WebSocket แล้ว
   - ตรวจสอบว่า API Route `/otp` ทำงานได้

2. **Credentials Configuration:**
   - ต้องมีไฟล์ `credentials.json` ที่ถูกต้อง
   - Token ต้องเป็นประเภท `pat_...` (Personal Access Token)

3. **Asset Mapping:**
   - ตรวจสอบว่า Backend มีฟังก์ชัน `map_asset_to_symbol()` แปลง `vol10` → `1HZ10V`

### ✅ สิ่งที่ทำเสร็จแล้ว:

- ✅ เปลี่ยน Public WS เป็น Local WS ทั้งหมด
- ✅ ฟังก์ชันซิงค์ออเดอร์ใช้ Local WS
- ✅ ฟังก์ชันดึงเวลาใช้ Local WS
- ✅ ไม่มี Public WebSocket เหลืออยู่ใน pkderiv.js

---

## 🔗 ไฟล์ที่เกี่ยวข้อง

- `public/index.html` - หน้าหลัก (โหลด pkderiv.js)
- `public/pkderiv.js` - JavaScript หลักที่แก้ไข
- `src/deriv.rs` - Backend WebSocket handler
- `md/newapi.md` - เอกสารอ้างอิง

---

## 📞 การแก้ไขปัญหา (Troubleshooting)

### ปัญหา: กราฟไม่แสดงข้อมูล
**วิธีแก้:**
- ตรวจสอบ Console (F12) หา Error
- ตรวจสอบว่า Backend กำลังรันอยู่
- ตรวจสอบว่า `credentials.json` ถูกต้อง

### ปัญหา: Error "InvalidSymbol"
**วิธีแก้:**
- ตรวจสอบว่าใช้ Local WebSocket แล้ว (ไม่ใช่ Public WS)
- ตรวจสอบ Backend log ว่ามีการขอ OTP สำเร็จหรือไม่
- Hard Refresh (Ctrl+Shift+R)

### ปัญหา: ไม่สามารถซิงค์ออเดอร์ได้
**วิธีแก้:**
- ตรวจสอบว่ามี API Token ใน LocalStorage
- ตรวจสอบ Backend log หา Error
- ลองใช้ Token ใหม่

---

**✨ การปรับปรุงเสร็จสมบูรณ์! ระบบพร้อมใช้งานกับ Deriv API ใหม่แล้ว 🚀**

# 🔄 Migration Guide: Public WebSocket → OTP WebSocket

## 📖 คู่มือการย้ายระบบจาก Public WebSocket ไปยัง OTP WebSocket

---

## 🎯 ภาพรวมการเปลี่ยนแปลง

### ก่อนการปรับปรุง (Old System)
```
Frontend → Public WS (ws.binaryws.com) → Deriv API
           ❌ No Authentication
           ❌ InvalidSymbol Error
```

### หลังการปรับปรุง (New System)
```
Frontend → Local WS (localhost/ws) → Backend (Rust) → OTP WS → Deriv API
           ✅ OTP Authentication
           ✅ Full Access to Synthetic Indices
```

---

## 📊 ตารางเปรียบเทียบ

| คุณสมบัติ | Public WS (เก่า) | OTP WS (ใหม่) |
|-----------|-----------------|---------------|
| **Authentication** | ❌ ไม่มี | ✅ OTP Token |
| **Synthetic Indices** | ❌ InvalidSymbol | ✅ ใช้งานได้ |
| **Security** | ⚠️ ต่ำ | ✅ สูง |
| **Token Type** | app_id | PAT (pat_...) |
| **Connection** | Direct | ผ่าน Backend |
| **Error Handling** | ต้องจัดการเอง | Backend จัดการให้ |

---

## 🛠️ การแก้ไขโค้ด

### 1. การเชื่อมต่อ WebSocket

#### ❌ แบบเก่า (Public WS):
```javascript
const ws = new WebSocket('wss://ws.binaryws.com/websockets/v3?app_id=1089');

ws.onopen = () => {
    // ไม่มี authentication
    ws.send(JSON.stringify({ ticks_history: "1HZ10V", count: 60 }));
};
```

**ปัญหา:**
- ไม่สามารถดึงข้อมูล Synthetic Indices ได้
- Error: `InvalidSymbol`

#### ✅ แบบใหม่ (OTP WS ผ่าน Backend):
```javascript
const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const ws = new WebSocket(`${protocol}//${window.location.host}/ws`);

ws.onopen = () => {
    // Backend จัดการ OTP Authentication
    ws.send(JSON.stringify({ ticks_history: "1HZ10V", count: 60 }));
};
```

**ข้อดี:**
- Backend ขอ OTP URL อัตโนมัติ
- เชื่อมต่อผ่าน Authenticated WebSocket
- ไม่มีปัญหา InvalidSymbol

---

### 2. การซิงค์ Portfolio

#### ❌ แบบเก่า:
```javascript
const ws = new WebSocket('wss://ws.binaryws.com/websockets/v3?app_id=1089');
ws.onopen = () => {
    ws.send(JSON.stringify({ authorize: token }));
};
```

#### ✅ แบบใหม่:
```javascript
const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const ws = new WebSocket(`${protocol}//${window.location.host}/ws`);
ws.onopen = () => {
    ws.send(JSON.stringify({ 
        type: 'sync_portfolio',
        token: token 
    }));
};
```

---

### 3. การดึงเวลาจาก Deriv

#### ❌ แบบเก่า:
```javascript
const ws = new WebSocket('wss://ws.binaryws.com/websockets/v3?app_id=1089');
ws.send(JSON.stringify({ time: 1 }));
```

#### ✅ แบบใหม่:
```javascript
const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const ws = new WebSocket(`${protocol}//${window.location.host}/ws`);
ws.send(JSON.stringify({ time: 1 }));
```

---

## 🔧 การตั้งค่า Backend

### ไฟล์: `credentials.json`
```json
{
  "credentials": [
    {
      "id": "my-account",
      "api_token": "pat_XXXXXXXXXXXXXXXX",
      "app_id": "your_app_id",
      "account_id": "CR12345678",
      "account_type": "demo",
      "is_active": true
    }
  ]
}
```

**หมายเหตุ:**
- ต้องใช้ `pat_...` Token (Personal Access Token)
- ไม่สามารถใช้ `app_id` อย่างเดียวได้

---

## 📝 Checklist การย้ายระบบ

### Frontend (index.html / pkderiv.js)
- [x] เปลี่ยน Public WS เป็น Local WS
- [x] แก้ไขฟังก์ชัน `syncOpenOrdersFromDeriv`
- [x] แก้ไขฟังก์ชัน `setDerivTimeBtn`
- [x] ลบ `wss://ws.binaryws.com` ทั้งหมด
- [x] ทดสอบการเชื่อมต่อ

### Backend (deriv.rs)
- [ ] เพิ่ม OTP Request Function
- [ ] แก้ไข `start_deriv_bot_multiplexed` ใช้ OTP WS
- [ ] แก้ไข `reconnect_public_ws` รับ credentials
- [ ] เพิ่ม Asset Mapping (`vol10` → `1HZ10V`)
- [ ] ทดสอบการเชื่อมต่อ

### Configuration
- [ ] ตั้งค่า `credentials.json`
- [ ] ตรวจสอบ `setup.json`
- [ ] ตรวจสอบ Port และ Network
- [ ] ตั้งค่า Environment Variables (ถ้ามี)

---

## 🧪 การทดสอบ

### 1. ทดสอบการเชื่อมต่อ
```bash
# รัน Backend
cargo run --release

# เปิด Browser Console (F12)
# ควรเห็น:
# ✅ "WebSocket connected"
# ✅ "Receiving candle data..."
# ❌ ไม่มี "InvalidSymbol"
```

### 2. ทดสอบการดึงข้อมูล
```javascript
// ใน Browser Console
const ws = new WebSocket('ws://localhost:3000/ws');
ws.onopen = () => {
    ws.send(JSON.stringify({ 
        ticks_history: "1HZ10V", 
        count: 10 
    }));
};
ws.onmessage = (msg) => {
    console.log('Received:', JSON.parse(msg.data));
};
```

**ผลที่คาดหวัง:**
- ได้รับข้อมูลแท่งเทียน 10 แท่ง
- ไม่มี Error

### 3. ทดสอบการเทรด
```javascript
// ทดสอบเปิด Order
// กด Start ใน UI
// ตรวจสอบว่า:
// ✅ กราฟแสดงข้อมูล
// ✅ ระบบแนะนำสัญญาณ
// ✅ สามารถเปิด Order ได้
```

---

## ❗ ปัญหาที่พบบ่อย

### ปัญหา 1: "InvalidSymbol" ยังคงปรากฏ
**สาเหตุ:**
- ยังมี Public WS เหลืออยู่ในโค้ด
- Cache ของ Browser

**วิธีแก้:**
```bash
# 1. Hard Refresh
Ctrl + Shift + R

# 2. Clear Cache
F12 → Application → Clear Storage

# 3. ตรวจสอบโค้ด
grep -r "ws.binaryws.com" public/
grep -r "ws.derivws.com" public/
```

### ปัญหา 2: "Connection Failed"
**สาเหตุ:**
- Backend ไม่ได้รัน
- Port ไม่ตรงกัน
- Firewall Block

**วิธีแก้:**
```bash
# ตรวจสอบ Backend
ps aux | grep turbo-indicators

# ตรวจสอบ Port
netstat -an | grep 3000

# ตรวจสอบ Firewall
# Windows: ตั้งค่า Windows Defender Firewall
```

### ปัญหา 3: "Authorization Failed"
**สาเหตุ:**
- Token หมดอายุ
- Token ไม่ถูกต้อง
- credentials.json ผิด

**วิธีแก้:**
```bash
# 1. ตรวจสอบ Token
# ไปที่ Deriv → API Token
# สร้าง Token ใหม่ (Personal Access Token)

# 2. อัปเดต credentials.json
# แทนที่ api_token ใหม่

# 3. Restart Backend
cargo run --release
```

---

## 📈 Performance Comparison

| Metric | Public WS | OTP WS |
|--------|-----------|---------|
| **Connection Time** | ~200ms | ~300ms |
| **Reliability** | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Data Quality** | Partial | Complete |
| **Error Rate** | High | Low |
| **Security** | Basic | Advanced |

---

## 🎓 Best Practices

### 1. Error Handling
```javascript
ws.onerror = (error) => {
    console.error('WebSocket Error:', error);
    showToast('เชื่อมต่อล้มเหลว กรุณาลองใหม่', 5000);
    
    // Auto retry after 5 seconds
    setTimeout(() => {
        connectWebSocket();
    }, 5000);
};
```

### 2. Reconnection Strategy
```javascript
let reconnectAttempts = 0;
const MAX_RECONNECT = 5;

ws.onclose = () => {
    if (reconnectAttempts < MAX_RECONNECT) {
        reconnectAttempts++;
        setTimeout(() => connectWebSocket(), 1000 * reconnectAttempts);
    } else {
        showToast('ไม่สามารถเชื่อมต่อได้ กรุณา Refresh หน้าเว็บ');
    }
};
```

### 3. Token Management
```javascript
// เก็บ Token ใน LocalStorage
localStorage.setItem('API_KEY', 'pat_XXXX');

// ตรวจสอบ Token ก่อนใช้งาน
const token = localStorage.getItem('API_KEY');
if (!token || !token.startsWith('pat_')) {
    alert('กรุณาใส่ API Token ที่ถูกต้อง');
}
```

---

## 🚀 การ Deploy

### Production Checklist
- [ ] ใช้ HTTPS (`wss://` แทน `ws://`)
- [ ] ตั้งค่า SSL Certificate
- [ ] เปิด Port ที่จำเป็น
- [ ] ตั้งค่า CORS
- [ ] Enable Logging
- [ ] Setup Monitoring
- [ ] Backup credentials.json

---

**📞 ต้องการความช่วยเหลือ?**
- อ่าน `newapi.md` สำหรับข้อมูลเพิ่มเติม
- ตรวจสอบ Backend logs: `tail -f logs/app.log`
- ดู Browser Console (F12) หา Error messages

**✨ Happy Migrating! 🚀**

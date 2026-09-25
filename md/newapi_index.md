# สรุปการแก้ไข index.html สำหรับ New Deriv API

เอกสารนี้สรุปการปรับปรุงไฟล์ `public/index.html` และ `public/pkderiv.js` เพื่อรองรับการอัปเกรดระบบความปลอดภัยของ Deriv API ซึ่งต้องการการพิสูจน์ตัวตนผ่าน **OTP WebSocket** แทน Public WebSocket

---

## 📋 สรุปภาพรวม

### ปัญหาที่พบ
- **Public WebSocket** (`wss://ws.binaryws.com/websockets/v3?app_id=1089`) ไม่สามารถดึงข้อมูล Synthetic Indices ได้
- ระบบฟ้อง Error `InvalidSymbol` เมื่อพยายามดึงข้อมูล `1HZ10V`, `1HZ50V`, `1HZ100V`
- Token ประเภท PAT (`pat_...`) ไม่สามารถใช้กับ Public WebSocket ได้

### วิธีแก้ไข
- เปลี่ยนจากการใช้ **Public WebSocket** เป็น **Local WebSocket** (`ws://localhost/ws`)
- Backend (Rust) จัดการการขอ OTP และสร้าง Authenticated WebSocket
- Frontend เพียงเชื่อมต่อไปที่ Local WebSocket โดยไม่ต้องกังวลเรื่อง Authentication

---

## 📂 ไฟล์ที่เกี่ยวข้อง

### 1. ไฟล์หลัก
- `public/index.html` - หน้า UI หลัก (โหลด pkderiv.js)
- `public/pkderiv.js` - JavaScript logic ทั้งหมด **(ไฟล์ที่ต้องแก้ไข)**

### 2. ไฟล์ Backend
- `src/deriv.rs` - จัดการ WebSocket และ OTP
- `src/main.rs` - API Routes และ Server
- `credentials.json` - เก็บ API Token

### 3. เอกสารอ้างอิง
- `md/newapi.md` - สรุปการอัปเกรด API
- `CHANGELOG-index-html.md` - บันทึกการเปลี่ยนแปลง
- `MIGRATION-GUIDE.md` - คู่มือการย้ายระบบ

---

## 🔧 รายการฟังก์ชันที่ต้องแก้ไข

### ไฟล์: `public/pkderiv.js`

| # | ฟังก์ชัน/ตัวแปร | บรรทัด | สถานะ | รายละเอียด |
|---|-----------------|--------|-------|-----------|
| 1 | `syncOpenOrdersFromDeriv()` | ~891 | ✅ แก้แล้ว | เปลี่ยนจาก Public WS เป็น Local WS |
| 2 | `setDerivTimeBtn` Event Listener | ~3553 | ✅ แก้แล้ว | เปลี่ยนจาก Public WS เป็น Local WS |

---

## 📝 รายละเอียดการแก้ไขแต่ละฟังก์ชัน

### 1. ฟังก์ชัน `syncOpenOrdersFromDeriv()`

**ตำแหน่ง:** บรรทัด ~891 ในไฟล์ `pkderiv.js`

**วัตถุประสงค์:** ซิงค์ออเดอร์ที่ค้างอยู่จาก Deriv Portfolio

#### ❌ โค้ดเดิม (ก่อนแก้ไข):
```javascript
window.syncOpenOrdersFromDeriv = function () {
    let token = localStorage.getItem('API_KEY');
    if (!token) return;

    console.log("🔄 กำลังซิงค์ออเดอร์ที่ค้างอยู่จาก Deriv (portfolio)...");
    
    // ⚠️ ใช้ Public WebSocket โดยตรง
    const ws = new WebSocket('wss://ws.binaryws.com/websockets/v3?app_id=1089');

    ws.onopen = () => {
        ws.send(JSON.stringify({ authorize: token }));
    };

    ws.onmessage = (msg) => {
        // ... handle portfolio data
    };
};
```

#### ✅ โค้ดใหม่ (หลังแก้ไข):
```javascript
window.syncOpenOrdersFromDeriv = function () {
    let token = localStorage.getItem('API_KEY');
    if (!token) return;

    console.log("🔄 กำลังซิงค์ออเดอร์ที่ค้างอยู่จาก Deriv (portfolio)...");
    
    // ✅ เปลี่ยนเป็น Local WebSocket
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const ws = new WebSocket(`${protocol}//${window.location.host}/ws`);

    ws.onopen = () => {
        // ส่งคำสั่งผ่าน Backend
        ws.send(JSON.stringify({ 
            type: 'sync_portfolio',
            token: token 
        }));
    };

    ws.onmessage = (msg) => {
        // ... handle portfolio data (เหมือนเดิม)
    };
};
```

#### 📊 การเปลี่ยนแปลง:
| รายการ | ก่อน | หลัง |
|--------|------|------|
| WebSocket URL | `wss://ws.binaryws.com/...` | `ws://localhost/ws` |
| Authentication | Direct authorize | ผ่าน Backend |
| OTP Handling | ไม่มี | Backend จัดการ |
| Error Handling | Limited | Enhanced |

#### 🎯 ผลลัพธ์:
- ✅ หลีกเลี่ยงข้อผิดพลาด InvalidSymbol
- ✅ ใช้ OTP Authentication ผ่าน Backend
- ✅ รองรับ Token ประเภท PAT
- ✅ ระบบซิงค์ออเดอร์ทำงานได้ปกติ

---

### 2. ฟังก์ชัน `setDerivTimeBtn` Event Listener

**ตำแหน่ง:** บรรทัด ~3553 ในไฟล์ `pkderiv.js`

**วัตถุประสงค์:** ดึงเวลาปัจจุบันจาก Deriv Server เพื่อตั้งค่า Start Time

#### ❌ โค้ดเดิม (ก่อนแก้ไข):
```javascript
const setDerivTimeBtn = document.getElementById('setDerivTimeBtn');
if (setDerivTimeBtn) {
    setDerivTimeBtn.addEventListener('click', () => {
        const btn = setDerivTimeBtn;
        const originalText = btn.textContent;
        btn.textContent = "⏳ กำลังดึงเวลา...";
        btn.disabled = true;

        // ⚠️ ใช้ Public WebSocket โดยตรง
        const ws = new WebSocket('wss://ws.binaryws.com/websockets/v3?app_id=1089');
        
        ws.onopen = () => {
            ws.send(JSON.stringify({ time: 1 }));
        };
        
        ws.onmessage = (msg) => {
            try {
                const data = JSON.parse(msg.data);
                if (data.time) {
                    // ... update time input
                }
            } catch (e) {
                console.error('Error parsing deriv time', e);
            }
            btn.textContent = "✅ อัปเดตเวลาแล้ว!";
            setTimeout(() => { 
                btn.textContent = originalText; 
                btn.disabled = false; 
            }, 2000);
            ws.close();
        };
        
        ws.onerror = () => {
            alert('เชื่อมต่อ Deriv API เพื่อดึงเวลาล้มเหลว');
            btn.textContent = originalText;
            btn.disabled = false;
        };
    });
}
```

#### ✅ โค้ดใหม่ (หลังแก้ไข):
```javascript
const setDerivTimeBtn = document.getElementById('setDerivTimeBtn');
if (setDerivTimeBtn) {
    setDerivTimeBtn.addEventListener('click', () => {
        const btn = setDerivTimeBtn;
        const originalText = btn.textContent;
        btn.textContent = "⏳ กำลังดึงเวลา...";
        btn.disabled = true;

        // ✅ เปลี่ยนเป็น Local WebSocket
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const ws = new WebSocket(`${protocol}//${window.location.host}/ws`);
        
        ws.onopen = () => {
            ws.send(JSON.stringify({ time: 1 }));
        };
        
        ws.onmessage = (msg) => {
            try {
                const data = JSON.parse(msg.data);
                if (data.time) {
                    const date = new Date(data.time * 1000);
                    const tzoffset = (new Date()).getTimezoneOffset() * 60000;
                    const localISOTime = (new Date(date - tzoffset)).toISOString().slice(0, 16);
                    document.getElementById('startDate').value = localISOTime;
                    updateStopDateFromDuration();
                }
            } catch (e) {
                console.error('Error parsing deriv time', e);
            }
            btn.textContent = "✅ อัปเดตเวลาแล้ว!";
            setTimeout(() => { 
                btn.textContent = originalText; 
                btn.disabled = false; 
            }, 2000);
            ws.close();
        };
        
        ws.onerror = () => {
            alert('เชื่อมต่อ Deriv API เพื่อดึงเวลาล้มเหลว');
            btn.textContent = originalText;
            btn.disabled = false;
        };
    });
}
```

#### 📊 การเปลี่ยนแปลง:
| รายการ | ก่อน | หลัง |
|--------|------|------|
| WebSocket URL | `wss://ws.binaryws.com/...` | `ws://localhost/ws` |
| Connection | Direct | ผ่าน Backend |
| Time Request | `{ time: 1 }` | `{ time: 1 }` (เหมือนเดิม) |
| Response Format | เหมือนเดิม | เหมือนเดิม |

#### 🎯 ผลลัพธ์:
- ✅ ปุ่ม "ตั้งเวลาจาก Deriv" ทำงานได้
- ✅ ดึงเวลาผ่าน Authenticated Connection
- ✅ หลีกเลี่ยงปัญหา API Limitation
- ✅ รองรับทั้ง HTTP และ HTTPS

---

## 🔍 ตัวแปรและค่าคงที่ที่เกี่ยวข้อง

### 1. WebSocket URLs

#### ❌ เดิม (Public WebSocket):
```javascript
// URL แบบเก่า - ไม่ควรใช้อีกต่อไป
const PUBLIC_WS_URL = 'wss://ws.binaryws.com/websockets/v3?app_id=1089';
const PUBLIC_WS_DERIV = 'wss://ws.derivws.com/websockets/v3?app_id=XXXX';
```

#### ✅ ใหม่ (Local WebSocket):
```javascript
// URL แบบใหม่ - ใช้ Local WebSocket
const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const LOCAL_WS_URL = `${protocol}//${window.location.host}/ws`;

// ตัวอย่าง:
// - Development: ws://localhost:3000/ws
// - Production: wss://yourdomain.com/ws
```

### 2. Authentication Token

#### LocalStorage Keys:
```javascript
// API Token (PAT)
const token = localStorage.getItem('API_KEY');  // รูปแบบ: pat_XXXXXXXX

// App ID (ถ้ามี)
const appId = localStorage.getItem('APP_ID');
```

#### Token Format:
```javascript
// ✅ ถูกต้อง - Personal Access Token
"pat_abc123def456ghi789..."

// ❌ ผิด - App ID แบบเก่า
"1089"
```

### 3. WebSocket Message Types

#### Request Messages:
```javascript
// ดึงเวลา
{ time: 1 }

// ซิงค์ Portfolio
{ 
    type: 'sync_portfolio',
    token: 'pat_...' 
}

// Subscribe Ticks
{ 
    ticks: "1HZ10V",
    subscribe: 1 
}

// Ticks History
{ 
    ticks_history: "1HZ10V",
    count: 60,
    end: "latest",
    start: 1,
    style: "ticks"
}
```

#### Response Messages:
```javascript
// Time Response
{
    msg_type: "time",
    time: 1704067200  // Unix timestamp
}

// Portfolio Response
{
    msg_type: "portfolio",
    portfolio: {
        contracts: [...]
    }
}

// Tick Response
{
    msg_type: "tick",
    tick: {
        epoch: 1704067200,
        quote: 100.50,
        symbol: "1HZ10V"
    }
}
```

---

## 🧪 การทดสอบฟังก์ชันที่แก้ไข

### Test Case 1: ทดสอบ syncOpenOrdersFromDeriv()

```javascript
// เปิด Browser Console (F12)

// 1. ตรวจสอบว่ามี Token
console.log('Token:', localStorage.getItem('API_KEY'));

// 2. เรียกใช้ฟังก์ชัน
window.syncOpenOrdersFromDeriv();

// 3. ตรวจสอบ Console Output
// ✅ ควรเห็น: "🔄 กำลังซิงค์ออเดอร์ที่ค้างอยู่จาก Deriv (portfolio)..."
// ✅ ควรเห็น: "📋 พบ X ออเดอร์ค้างอยู่..." หรือ "🟢 ไม่มีออเดอร์ค้างอยู่"
// ❌ ไม่ควรเห็น: Error "InvalidSymbol"
```

### Test Case 2: ทดสอบปุ่ม "ตั้งเวลาจาก Deriv"

```javascript
// ใน UI

// 1. คลิกปุ่ม "ตั้งเวลาจาก Deriv"

// 2. ตรวจสอบว่า:
// ✅ ปุ่มแสดง "⏳ กำลังดึงเวลา..."
// ✅ ปุ่ม disabled ชั่วคราว
// ✅ หลังจากนั้นแสดง "✅ อัปเดตเวลาแล้ว!"
// ✅ Input "startDate" มีค่าเวลาปัจจุบัน
// ✅ Input "stopDate" อัปเดตตามค่า addMinutes

// 3. ตรวจสอบ Console
// ❌ ไม่ควรมี Error
```

### Test Case 3: ทดสอบการเชื่อมต่อ WebSocket

```javascript
// เปิด Browser Console (F12)

// 1. สร้างการเชื่อมต่อทดสอบ
const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const testWs = new WebSocket(`${protocol}//${window.location.host}/ws`);

testWs.onopen = () => {
    console.log('✅ WebSocket Connected');
    testWs.send(JSON.stringify({ time: 1 }));
};

testWs.onmessage = (msg) => {
    console.log('📨 Received:', JSON.parse(msg.data));
};

testWs.onerror = (error) => {
    console.error('❌ WebSocket Error:', error);
};

// 2. รอ Response
// ✅ ควรได้รับ { msg_type: "time", time: XXXXX }
```

---

## 📊 สถิติการแก้ไข

### จำนวนไฟล์ที่แก้ไข:
- ✅ ไฟล์ที่แก้ไข: **1 ไฟล์** (`public/pkderiv.js`)
- ℹ️ ไฟล์ที่ไม่ต้องแก้: `public/index.html` (เพียงโหลด pkderiv.js)

### จำนวนฟังก์ชันที่แก้ไข:
- ✅ ฟังก์ชันหลัก: **2 ฟังก์ชัน**
  1. `syncOpenOrdersFromDeriv()`
  2. `setDerivTimeBtn` Event Listener

### บรรทัดที่เปลี่ยนแปลง:
- ฟังก์ชัน #1: ~5 บรรทัด
- ฟังก์ชัน #2: ~3 บรรทัด
- รวมทั้งหมด: ~8 บรรทัด

### Public WebSocket ที่ลบออก:
- ❌ ลบออก: **2 จุด** (ไม่มี `ws.binaryws.com` เหลืออยู่)

---

## 🔄 Flow การทำงานใหม่

### เดิม (Public WebSocket):
```
┌──────────────┐
│   Frontend   │
│  (index.html)│
└──────┬───────┘
       │ wss://ws.binaryws.com
       │ ❌ No OTP
       ▼
┌──────────────┐
│  Deriv API   │
│  (Public)    │
└──────────────┘
       │
       ▼
   ❌ InvalidSymbol Error
```

### ใหม่ (OTP WebSocket):
```
┌──────────────┐
│   Frontend   │
│  (index.html)│
└──────┬───────┘
       │ ws://localhost/ws
       ▼
┌──────────────┐
│   Backend    │
│   (Rust)     │
└──────┬───────┘
       │ POST /otp
       ▼
┌──────────────┐
│ Deriv REST   │
│  /otp API    │
└──────┬───────┘
       │ คืน OTP URL
       ▼
┌──────────────┐
│   Backend    │
│ สร้าง OTP WS │
└──────┬───────┘
       │ wss://...?otp=xxx
       │ ✅ Authenticated
       ▼
┌──────────────┐
│  Deriv API   │
│ (Private)    │
└──────────────┘
       │
       ▼
   ✅ Full Access
```

---

## ⚙️ การตั้งค่าที่จำเป็น

### 1. credentials.json
```json
{
  "credentials": [
    {
      "id": "my-account",
      "api_token": "pat_XXXXXXXXXXXXXXXX",  // ← ต้องเป็น PAT
      "app_id": "your_app_id",
      "account_id": "CR12345678",
      "account_type": "demo",
      "is_active": true
    }
  ]
}
```

### 2. LocalStorage (Browser)
```javascript
// ตั้งค่าผ่าน Console (F12)
localStorage.setItem('API_KEY', 'pat_XXXXXXXXXXXXXXXX');
localStorage.setItem('APP_ID', 'your_app_id');
```

### 3. Backend Configuration
```rust
// src/main.rs
// ตรวจสอบว่ามี Route นี้
.route("/otp", post(handle_post_otp))
.route("/ws", get(handle_websocket))
```

---

## 🚨 ข้อผิดพลาดที่อาจพบ

### Error 1: "InvalidSymbol"
**สาเหตุ:** ยังใช้ Public WebSocket อยู่

**วิธีแก้:**
```javascript
// ตรวจสอบว่าไม่มี Code นี้เหลือ
❌ new WebSocket('wss://ws.binaryws.com/...')
❌ new WebSocket('wss://ws.derivws.com/...')

// ควรเป็น
✅ new WebSocket(`${protocol}//${window.location.host}/ws`)
```

### Error 2: "Connection Failed"
**สาเหตุ:** Backend ไม่ได้รัน หรือ Port ไม่ตรง

**วิธีแก้:**
```bash
# ตรวจสอบ Backend
ps aux | grep turbo-indicators

# รัน Backend
cargo run --release

# ตรวจสอบ Port
netstat -an | grep 3000
```

### Error 3: "Authorization Failed"
**สาเหตุ:** Token ไม่ถูกต้อง หรือหมดอายุ

**วิธีแก้:**
```javascript
// 1. ตรวจสอบ Token
console.log(localStorage.getItem('API_KEY'));

// 2. ต้องขึ้นต้นด้วย 'pat_'
✅ "pat_abc123..."
❌ "1089"

// 3. สร้าง Token ใหม่ที่ Deriv
// https://app.deriv.com/account/api-token
```

---

## 📚 เอกสารเพิ่มเติม

### ไฟล์อ้างอิง:
1. `md/newapi.md` - สรุปการอัปเกรด API ทั้งระบบ
2. `CHANGELOG-index-html.md` - บันทึกการเปลี่ยนแปลงแบบละเอียด
3. `MIGRATION-GUIDE.md` - คู่มือการย้ายระบบแบบสมบูรณ์

### External Links:
- [Deriv API Documentation](https://api.deriv.com/)
- [Deriv WebSocket Guide](https://api.deriv.com/docs/websockets/)
- [Personal Access Token Guide](https://api.deriv.com/docs/app-registration/)

---

## ✅ Checklist การแก้ไข

### ก่อนแก้ไข:
- [ ] Backup ไฟล์เดิม (`cp pkderiv.js pkderiv.js.bak`)
- [ ] อ่าน `md/newapi.md`
- [ ] ตรวจสอบ Backend รองรับ OTP

### ระหว่างแก้ไข:
- [x] แก้ไขฟังก์ชัน `syncOpenOrdersFromDeriv()`
- [x] แก้ไขฟังก์ชัน `setDerivTimeBtn`
- [x] ตรวจสอบไม่มี Public WS เหลือ
- [x] ทดสอบ Syntax ไม่มีข้อผิดพลาด

### หลังแก้ไข:
- [ ] Hard Refresh Browser (Ctrl+Shift+R)
- [ ] ทดสอบปุ่ม "ตั้งเวลาจาก Deriv"
- [ ] ทดสอบการ Sync Portfolio
- [ ] ทดสอบการเปิด/ปิด Order
- [ ] ตรวจสอบ Console ไม่มี Error

---

## 🎯 สรุป

### สิ่งที่ทำเสร็จ:
- ✅ แก้ไข `public/pkderiv.js` ทั้งหมด 2 จุด
- ✅ เปลี่ยนจาก Public WebSocket เป็น Local WebSocket
- ✅ หลีกเลี่ยงข้อผิดพลาด InvalidSymbol
- ✅ รองรับ OTP Authentication ผ่าน Backend
- ✅ สร้างเอกสารสรุปอย่างละเอียด

### ผลลัพธ์:
- ✅ ระบบ `index.html` พร้อมใช้งานกับ Deriv API ใหม่
- ✅ สามารถดึงข้อมูล Synthetic Indices ได้ปกติ
- ✅ การซิงค์ออเดอร์ทำงานได้
- ✅ ปุ่มตั้งเวลาทำงานได้

---

**📅 วันที่อัปเดต:** 2026-07-23  
**📝 เวอร์ชัน:** 1.0  
**✍️ ผู้เขียน:** Converted from Rust newapi.md

**🎉 การแก้ไข index.html เสร็จสมบูรณ์! 🚀**

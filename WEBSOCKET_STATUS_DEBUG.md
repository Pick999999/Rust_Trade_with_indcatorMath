# 🐛 Debug: Private WebSocket Status ไม่อัพเดท

## วันที่: 3 สิงหาคม 2569

---

## ❓ ปัญหา

1. **เมื่อ Private WS ถูกตัดจาก Deriv** → ไม่มีการอัพเดท status บน UI
2. **เมื่อเปิดหน้าเว็บใหม่** → Status ไม่ตรงกับความเป็นจริง

---

## 🔍 การทำงานที่ถูกต้อง

### 1. Rust ส่ง Status Update (Real-time)

**เมื่อ Private WS เปลี่ยนสถานะ** (`src/long_term.rs`):
```rust
// 1. กำลังเชื่อมต่อ
tx.send(json!({ 
    "type": "lt_ws_status", 
    "target": "private", 
    "status": "connecting" 
}));

// 2. เชื่อมต่อสำเร็จ
tx.send(json!({ 
    "type": "lt_ws_status", 
    "target": "private", 
    "status": "connected" 
}));

// 3. กำลัง Reconnect
tx.send(json!({ 
    "type": "lt_ws_status", 
    "target": "private", 
    "status": "reconnecting" 
}));

// 4. ตัดการเชื่อมต่อ
tx.send(json!({ 
    "type": "lt_ws_status", 
    "target": "private", 
    "status": "disconnected" 
}));
```

### 2. Frontend รับ Status (Real-time)

**ผ่าน WebSocket** (`long_term_trade.html`):
```javascript
ltWs.onmessage = (event) => {
    const msg = JSON.parse(event.data);
    
    if (msg.type === "lt_ws_status") {
        console.log(`📨 [WebSocket Message] lt_ws_status:`, msg);
        updateWsStatusDot(msg.target, msg.status);
        // อัพเดท UI (🟢/🟠/🔴)
    }
}
```

### 3. Frontend ดึง Status เมื่อเปิดหน้าเว็บ

**ผ่าน REST API** (`/api/status`):
```javascript
ltWs.onopen = () => {
    fetch('/api/status')
        .then(res => res.json())
        .then(status => {
            console.log("📊 [Status API] Received:", status);
            
            const candleStatus = status.candle_ws_status || 'disconnected';
            const privateStatus = status.private_ws_status || 'disconnected';
            
            console.log(`🔄 [Status] Candle WS: ${candleStatus}, Private WS: ${privateStatus}`);
            
            updateWsStatusDot('candle', candleStatus);
            updateWsStatusDot('private', privateStatus);
        });
}
```

---

## 🛠️ การแก้ไข

### 1. แก้ Duplicate Field ใน JSON (Rust)

**Before**:
```rust
Json(serde_json::json!({
    "candle_ws_status": candle_ws_status,
    "candle_ws_status": candle_ws_status,  // ← ซ้ำ!
    "private_ws_status": private_ws_status,
    "private_ws_status": private_ws_status  // ← ซ้ำ!
}))
```

**After**:
```rust
Json(serde_json::json!({
    "candle_ws_status": candle_ws_status,
    "private_ws_status": private_ws_status
}))
```

### 2. เพิ่ม Fallback เมื่อไม่มีค่า (Frontend)

**Before**:
```javascript
if (status.candle_ws_status) updateWsStatusDot('candle', status.candle_ws_status);
if (status.private_ws_status) updateWsStatusDot('private', status.private_ws_status);
// ถ้าเป็น undefined จะไม่ทำงาน
```

**After**:
```javascript
const candleStatus = status.candle_ws_status || 'disconnected';
const privateStatus = status.private_ws_status || 'disconnected';

updateWsStatusDot('candle', candleStatus);
updateWsStatusDot('private', privateStatus);
// มั่นใจว่าจะมีค่าเสมอ
```

### 3. เพิ่ม Debug Logging

**ใน `updateWsStatusDot()`**:
```javascript
function updateWsStatusDot(target, status) {
    console.log(`🔄 [updateWsStatusDot] target: ${target}, status: ${status}`);
    
    // ... code ...
    
    if (status === 'connected') {
        console.log(`✅ [WS Status] ${target} = CONNECTED (🟢)`);
    } else if (status === 'reconnecting' || status === 'connecting') {
        console.log(`⏳ [WS Status] ${target} = ${status.toUpperCase()} (🟠)`);
    } else {
        console.log(`❌ [WS Status] ${target} = DISCONNECTED (🔴)`);
    }
}
```

**ใน WebSocket Message Handler**:
```javascript
if (msg.type === "lt_ws_status") {
    console.log(`📨 [WebSocket Message] lt_ws_status:`, msg);
    updateWsStatusDot(msg.target, msg.status);
}
```

**ใน Status API Handler**:
```javascript
fetch('/api/status')
    .then(res => res.json())
    .then(status => {
        console.log("📊 [Status API] Received:", status);
        console.log(`🔄 [Status] Candle WS: ${candleStatus}, Private WS: ${privateStatus}`);
        // ...
    });
```

---

## 🧪 วิธีทดสอบ

### 1. ทดสอบ Real-time Update

1. เปิด Console (F12)
2. ดู log message: `📨 [WebSocket Message] lt_ws_status:`
3. ตัด internet หรือ restart Rust server
4. ดูว่า Private WS dot เปลี่ยนเป็น 🔴 หรือ 🟠

### 2. ทดสอบ Page Load

1. Restart Rust server (Private WS จะเป็น connected)
2. เปิดหน้าเว็บใหม่
3. ดู Console log: `📊 [Status API] Received:`
4. ตรวจสอบว่า `private_ws_status` มีค่าถูกต้อง
5. ดู log: `🔄 [updateWsStatusDot] target: private, status: connected`
6. ดูว่า Private WS dot เป็น 🟢

### 3. ทดสอบ Disconnection

1. ปิด internet หรือ force disconnect Deriv WS
2. ดู Rust console: `⚠️ [LongTerm Private] WebSocket disconnected`
3. ดู Browser console: `📨 [WebSocket Message] lt_ws_status: {target: "private", status: "reconnecting"}`
4. ดูว่า Private WS dot เปลี่ยนเป็น 🟠 หรือ 🔴

---

## 📊 Flow Diagram

```
[Deriv Server]
    ↓ (ถูกตัดสัญญาณ)
[Rust Private WS Handler]
    ↓ tx.send({"type": "lt_ws_status", "target": "private", "status": "reconnecting"})
[Rust WebSocket Broadcast]
    ↓
[Browser WebSocket (ltWs)]
    ↓ onmessage
[updateWsStatusDot("private", "reconnecting")]
    ↓
[UI dot เปลี่ยนเป็น 🟠]
```

**เมื่อเปิดหน้าเว็บใหม่**:
```
[Browser]
    ↓ ltWs.onopen
[Fetch /api/status]
    ↓
[Rust StorageState.lt_ws_status]
    ↓ HashMap["private"] = "reconnecting"
[Response JSON]
    ↓ {"private_ws_status": "reconnecting"}
[updateWsStatusDot("private", "reconnecting")]
    ↓
[UI dot แสดง 🟠]
```

---

## ✅ สรุป

1. ✅ **แก้ duplicate field** ใน JSON response
2. ✅ **เพิ่ม fallback** เมื่อไม่มีค่า status
3. ✅ **เพิ่ม debug logging** ทุกจุดที่สำคัญ
4. ✅ **Rust เก็บ status ใน HashMap** (StorageState)
5. ✅ **Frontend ดึง status เมื่อเปิดหน้า** (/api/status)
6. ✅ **Real-time update** ผ่าน WebSocket message

ตอนนี้สามารถ debug ได้ง่ายขึ้นด้วย console log! 🎉

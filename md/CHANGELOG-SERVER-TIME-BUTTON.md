# 📝 CHANGELOG - Server Time Button

**วันที่:** 2026-07-23  
**การเปลี่ยนแปลง:** เพิ่มปุ่มดึงเวลาจาก Deriv Server ใน Long Term Trade

---

## 🎯 สรุปการเปลี่ยนแปลง

เพิ่มปุ่ม "🕐 ดึงเวลา Server" ใน Long Term Trade เพื่อดึงเวลาปัจจุบันจาก Deriv.com และอัปเดต Start Time Picker อัตโนมัติ

Short Term Trade มีปุ่มนี้อยู่แล้วและทำงานได้ปกติ

---

## ✅ การแก้ไขในแต่ละไฟล์

### 1. **long_term_trade.html** - เพิ่มปุ่ม UI

#### ตำแหน่ง: บรรทัด ~380-383

**เพิ่มปุ่มใหม่:**
```html
<div class="date-input-group">
    <div class="date-range-label">เวลาเริ่ม (Start Time)</div>
    <input type="datetime-local" id="ltStartTime">
    <button type="button" class="fetch-btn" id="ltGetServerTimeBtn" 
        style="margin-top:4px; padding:4px 8px; font-size:11px; background:rgba(74,158,255,0.15); color:var(--accent); border:1px solid var(--accent);">
        🕐 ดึงเวลา Server
    </button>
</div>
```

**ผลลัพธ์:**
- ✅ ปุ่มแสดงใต้ input Start Time
- ✅ สี: สีฟ้า (accent color)
- ✅ Icon: 🕐 (นาฬิกา)

---

### 2. **long_term_trade.html** - เพิ่ม Event Listener

#### ตำแหน่ง: บรรทัด ~1169 (หลัง saveTradeLongTimeBtn)

**เพิ่ม Event Listener:**
```javascript
// ── Get Server Time Button ────────────────────────────
document.getElementById('ltGetServerTimeBtn')?.addEventListener('click', async () => {
    const btn = document.getElementById('ltGetServerTimeBtn');
    const originalText = btn.textContent;
    btn.textContent = "⏳ กำลังดึงเวลา...";
    btn.disabled = true;

    try {
        // Connect to local WebSocket to get Deriv server time
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
                    
                    document.getElementById('ltStartTime').value = localISOTime;
                    document.getElementById('ltStartTime').dispatchEvent(new Event('change'));
                    
                    btn.textContent = "✅ อัปเดตแล้ว!";
                    ltAddLog(`🕐 ดึงเวลาจาก Deriv Server สำเร็จ: ${date.toLocaleString('th-TH')}`, 'system');
                }
            } catch (e) {
                console.error('Error parsing server time', e);
                btn.textContent = "❌ ล้มเหลว";
            }
            
            setTimeout(() => {
                btn.textContent = originalText;
                btn.disabled = false;
            }, 2000);
            
            ws.close();
        };

        ws.onerror = () => {
            btn.textContent = "❌ เชื่อมต่อล้มเหลว";
            ltAddLog('❌ ไม่สามารถเชื่อมต่อ Deriv API เพื่อดึงเวลาได้', 'system');
            setTimeout(() => {
                btn.textContent = originalText;
                btn.disabled = false;
            }, 2000);
        };

    } catch (e) {
        console.error("Get server time error:", e);
        btn.textContent = "❌ ล้มเหลว";
        setTimeout(() => {
            btn.textContent = originalText;
            btn.disabled = false;
        }, 2000);
    }
});
```

---

### 3. **index_short_term.html** - ปุ่มมีอยู่แล้ว

#### ตำแหน่ง: บรรทัด ~1393

**HTML:**
```html
<button type="button" class="fetch-btn" id="setDerivTimeBtn" 
    style="background:var(--orange); color:#fff; border:none; padding:8px 12px; margin-right: 10px;">
    ⏱ ดึงเวลาจาก Deriv
</button>
```

**JavaScript:** (ใน `public/pkderiv.js` บรรทัด ~3552-3594)
```javascript
const setDerivTimeBtn = document.getElementById('setDerivTimeBtn');
if (setDerivTimeBtn) {
    setDerivTimeBtn.addEventListener('click', () => {
        const btn = setDerivTimeBtn;
        const originalText = btn.textContent;
        btn.textContent = "⏳ กำลังดึงเวลา...";
        btn.disabled = true;

        // เปลี่ยนจาก Public WS เป็นใช้ Local WS
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
            setTimeout(() => { btn.textContent = originalText; btn.disabled = false; }, 2000);
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

**สถานะ:** ✅ ทำงานได้แล้ว (ใช้ Local WebSocket ตาม newapi.md)

---

## 🔄 Flow การทำงาน

```
1. ผู้ใช้คลิกปุ่ม "🕐 ดึงเวลา Server"
   ↓
2. ปุ่มแสดง "⏳ กำลังดึงเวลา..." และ disabled
   ↓
3. เชื่อมต่อ Local WebSocket (ws://localhost/ws)
   ↓
4. ส่งคำสั่ง { time: 1 }
   ↓
5. Backend forward ไปยัง Deriv API (OTP WebSocket)
   ↓
6. Deriv API คืน { time: UNIX_TIMESTAMP }
   ↓
7. แปลงเป็น Local Time และ ISO Format
   ↓
8. อัปเดต Start Time Input
   ↓
9. Trigger 'change' event → อัปเดต Stop Time อัตโนมัติ
   ↓
10. ปุ่มแสดง "✅ อัปเดตแล้ว!" เป็นเวลา 2 วินาที
   ↓
11. กลับเป็นข้อความเดิม และ enable ปุ่มอีกครั้ง
```

---

## 📊 ตารางเปรียบเทียบ

| คุณสมบัติ | Short Term Trade | Long Term Trade |
|-----------|-----------------|-----------------|
| **ปุ่ม** | ⏱ ดึงเวลาจาก Deriv | 🕐 ดึงเวลา Server |
| **สี** | 🟠 Orange | 🔵 Blue (Accent) |
| **ตำแหน่ง** | ข้าง Clock | ใต้ Start Time Input |
| **ไฟล์ JS** | pkderiv.js | long_term_trade.html |
| **สถานะ** | ✅ ทำงานได้แล้ว | ✅ เพิ่มใหม่ |
| **WebSocket** | Local WS | Local WS |
| **Input ID** | startDate | ltStartTime |

---

## 🎨 UI Design

### Long Term Trade Button:
```css
background: rgba(74, 158, 255, 0.15);  /* สีฟ้าอ่อน */
color: var(--accent);                   /* สีฟ้า */
border: 1px solid var(--accent);        /* ขอบสีฟ้า */
padding: 4px 8px;                       /* ขนาดเล็ก */
font-size: 11px;                        /* ตัวอักษรเล็ก */
margin-top: 4px;                        /* ห่างจาก input */
```

### Short Term Trade Button:
```css
background: var(--orange);              /* สีส้ม */
color: #fff;                            /* ตัวอักษรขาว */
border: none;                           /* ไม่มีขอบ */
padding: 8px 12px;                      /* ขนาดปกติ */
```

---

## 🧪 การทดสอบ

### Test Case 1: Long Term Trade - ดึงเวลาสำเร็จ
**Steps:**
1. เปิดหน้า `long_term_trade.html`
2. คลิกปุ่ม "🕐 ดึงเวลา Server"

**คาดหวัง:**
- ✅ ปุ่มแสดง "⏳ กำลังดึงเวลา..."
- ✅ ปุ่ม disabled ชั่วคราว
- ✅ Start Time Input อัปเดตเป็นเวลาปัจจุบัน
- ✅ Stop Time อัปเดตอัตโนมัติ (ตาม Add Minutes)
- ✅ ปุ่มแสดง "✅ อัปเดตแล้ว!" 2 วินาที
- ✅ Log แสดง "🕐 ดึงเวลาจาก Deriv Server สำเร็จ"
- ✅ ปุ่มกลับเป็นปกติ

### Test Case 2: Short Term Trade - ดึงเวลาสำเร็จ
**Steps:**
1. เปิดหน้า `index_short_term.html`
2. คลิกปุ่ม "⏱ ดึงเวลาจาก Deriv"

**คาดหวัง:**
- ✅ ปุ่มแสดง "⏳ กำลังดึงเวลา..."
- ✅ ปุ่ม disabled ชั่วคราว
- ✅ Start Date Input อัปเดตเป็นเวลาปัจจุบัน
- ✅ Stop Date อัปเดตอัตโนมัติ
- ✅ ปุ่มแสดง "✅ อัปเดตเวลาแล้ว!" 2 วินาที
- ✅ ปุ่มกลับเป็นปกติ

### Test Case 3: WebSocket Error
**Steps:**
1. หยุด Backend (Rust server)
2. คลิกปุ่มดึงเวลา

**คาดหวัง:**
- ✅ ปุ่มแสดง "❌ เชื่อมต่อล้มเหลว"
- ✅ Alert หรือ Log แสดงข้อความ error
- ✅ ปุ่มกลับเป็นปกติหลัง 2 วินาที

### Test Case 4: Timezone Conversion
**Steps:**
1. คลิกปุ่มดึงเวลา
2. ตรวจสอบเวลาที่แสดง

**คาดหวัง:**
- ✅ เวลาแสดงเป็น Local Timezone (เช่น GMT+7 สำหรับไทย)
- ✅ รูปแบบเวลาถูกต้อง (YYYY-MM-DDTHH:MM)
- ✅ Input datetime-local รับค่าได้

---

## 💡 Technical Details

### WebSocket Message Format:

#### Request:
```json
{
    "time": 1
}
```

#### Response:
```json
{
    "msg_type": "time",
    "time": 1704067200
}
```

### Time Conversion:
```javascript
// Unix timestamp to Date object
const date = new Date(data.time * 1000);

// Get timezone offset in milliseconds
const tzoffset = (new Date()).getTimezoneOffset() * 60000;

// Convert to local time ISO string
const localISOTime = (new Date(date - tzoffset)).toISOString().slice(0, 16);
// Result: "2026-07-23T14:30"
```

### Event Dispatching:
```javascript
// Trigger change event to update Stop Time automatically
document.getElementById('ltStartTime').dispatchEvent(new Event('change'));
```

---

## 📊 สถิติการเปลี่ยนแปลง

| รายการ | จำนวน |
|--------|-------|
| ไฟล์ที่แก้ไข | 1 ไฟล์ (long_term_trade.html) |
| ปุ่มที่เพิ่ม | 1 ปุ่ม |
| Event Listener ที่เพิ่ม | 1 listener |
| บรรทัดที่เพิ่ม | ~60 บรรทัด |
| ไฟล์ที่ตรวจสอบแล้ว | 2 ไฟล์ (index_short_term + pkderiv.js) |

---

## 🎯 Benefits

### 1. **Convenience (ความสะดวก)**
- ✅ ไม่ต้องตั้งเวลาเอง
- ✅ คลิกเดียวได้เวลาที่ถูกต้อง
- ✅ ไม่ต้องกังวลเรื่อง Timezone

### 2. **Accuracy (ความแม่นยำ)**
- ✅ เวลาตรงกับ Deriv Server
- ✅ ไม่เกิดปัญหาเวลาเครื่องไม่ตรง
- ✅ Timezone conversion อัตโนมัติ

### 3. **Consistency (ความสอดคล้อง)**
- ✅ ทั้ง Short Term และ Long Term มีฟีเจอร์เดียวกัน
- ✅ ใช้ Local WebSocket เหมือนกัน (ตาม newapi.md)
- ✅ UX คล้ายกัน

### 4. **Reliability (ความน่าเชื่อถือ)**
- ✅ Error handling สมบูรณ์
- ✅ แสดง feedback ชัดเจน
- ✅ Auto recovery หลัง error

---

## 🔧 Backend Requirements

Backend ต้องรองรับการ forward คำสั่ง `time` ไปยัง Deriv API:

### Rust WebSocket Handler:
```rust
// Handle time request
if let Some(time) = msg.get("time") {
    // Forward to Deriv OTP WebSocket
    deriv_ws.send(json!({ "time": 1 })).await?;
    
    // Wait for response
    let response = deriv_ws.recv().await?;
    
    // Forward back to client
    client_ws.send(response).await?;
}
```

---

## 📚 ไฟล์ที่เกี่ยวข้อง

### ไฟล์ที่แก้ไข:
- ✅ `public/long_term_trade.html` - เพิ่มปุ่มและ event listener

### ไฟล์ที่ตรวจสอบ:
- ✅ `public/index_short_term.html` - มีปุ่มอยู่แล้ว
- ✅ `public/pkderiv.js` - มี event listener อยู่แล้ว

### เอกสาร:
- ✅ `CHANGELOG-SERVER-TIME-BUTTON.md` - เอกสารนี้
- ℹ️ `md/newapi.md` - การใช้ Local WebSocket
- ℹ️ `MIGRATION-GUIDE.md` - คู่มือการย้ายระบบ

---

## 🌟 Future Enhancements (ไอเดียเพิ่มเติม)

### 1. **Auto Sync Time**
ดึงเวลาอัตโนมัติเมื่อเปิดหน้า
```javascript
window.addEventListener('load', () => {
    // Auto get server time on page load
    document.getElementById('ltGetServerTimeBtn')?.click();
});
```

### 2. **Time Offset Setting**
เพิ่ม/ลดเวลาที่ดึงมาได้
```javascript
const offsetMinutes = parseInt(document.getElementById('timeOffset').value) || 0;
const adjustedDate = new Date(date.getTime() + (offsetMinutes * 60000));
```

### 3. **Show Server Time**
แสดงเวลา Server แบบ realtime
```html
<div id="serverTime">Server Time: --:--:--</div>
```

### 4. **Timezone Selection**
เลือก Timezone ที่ต้องการ
```html
<select id="timezoneSelect">
    <option value="Asia/Bangkok">Bangkok (GMT+7)</option>
    <option value="UTC">UTC (GMT+0)</option>
    <option value="America/New_York">New York (GMT-5)</option>
</select>
```

---

## ✅ สรุป

การเพิ่มปุ่มดึงเวลา Server เสร็จสมบูรณ์! ตอนนี้:

1. ✅ **Long Term Trade** มีปุ่ม "🕐 ดึงเวลา Server"
2. ✅ **Short Term Trade** มีปุ่ม "⏱ ดึงเวลาจาก Deriv" (อยู่แล้ว)
3. ✅ ทั้งสองปุ่มใช้ **Local WebSocket** (ตาม newapi.md)
4. ✅ อัปเดต Start Time อัตโนมัติเมื่อดึงเวลาสำเร็จ
5. ✅ Stop Time อัปเดตตามอัตโนมัติ
6. ✅ Error handling สมบูรณ์
7. ✅ UI feedback ชัดเจน

---

**🎉 Server Time Button Complete! ดึงเวลาได้ง่าย ๆ เพียงคลิกเดียว! 🕐**

**📅 วันที่:** 2026-07-23  
**🎯 สถานะ:** ✅ COMPLETED  
**🔧 Feature:** Server Time Sync Button  
**📱 Platform:** Long Term & Short Term Trade  
**🌐 Connection:** Local WebSocket (OTP)  
**👨‍💻 ผู้พัฒนา:** Kiro AI Assistant


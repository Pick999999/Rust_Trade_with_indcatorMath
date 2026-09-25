# CHANGELOG: Auto Trade Fix & Debug Logging

**Date**: 2026-07-25
**Status**: ✅ COMPLETED

---

## 🎯 Problem Summary

Auto Trade checkbox แสดงว่าเปิดใช้งานอยู่ แต่ Backend ไม่ทำการเทรดอัตโนมัติ เมื่อ toggle checkbox ปิด/เปิดใหม่และ reconnect จึงทำงานได้

**Root Cause**:
- เมื่อโหลดหน้าเว็บ settings จาก localStorage ถูกโหลด และ checkbox ถูก set แต่ `change` event ไม่ถูก trigger
- ทำให้ Backend ไม่ได้รับการแจ้งสถานะ Auto Trade
- แม้จะมีการแก้ไขด้วย `dispatchEvent(new Event('change'))` แต่อาจมี timing issue หรือ Backend ยังไม่ได้ compile ใหม่

---

## ✨ Changes Made

### 1. **Frontend Debug Logging** (`long_term_trade.html`)

#### 1.1 Auto Trade Toggle Handler
- เพิ่ม console log เมื่อ checkbox เปลี่ยนสถานะ
- เพิ่ม log response จาก Backend
- Lines ~2787-2798

```javascript
document.getElementById('ltAutoTradeToggle').addEventListener('change', async (e) => {
    const isAuto = e.target.checked;
    console.log(`🔄 [Frontend] Auto Trade checkbox changed to: ${isAuto}`);
    try {
        const res = await fetch('/api/longterm/toggle_auto', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ enabled: isAuto })
        });
        const data = await res.json();
        console.log(`✅ [Frontend] Auto Trade toggle response:`, data);
    } catch (err) {
        console.error("❌ [Frontend] Failed to toggle auto trade", err);
    }
});
```

#### 1.2 Settings Load Function
- เพิ่ม log เมื่อโหลด auto trade setting
- เพิ่ม 50ms delay ก่อน dispatch event เพื่อให้ checkbox update เสร็จก่อน
- เพิ่ม log เมื่อ dispatch change event
- Lines ~2959-2967

```javascript
if (settings.autoTrade !== undefined) {
    const autoTradeToggle = document.getElementById('ltAutoTradeToggle');
    console.log(`📥 [Frontend] Loading auto trade from settings: ${settings.autoTrade}`);
    autoTradeToggle.checked = settings.autoTrade;
    // รอให้ checkbox update ก่อน
    await new Promise(resolve => setTimeout(resolve, 50));
    autoTradeToggle.dispatchEvent(new Event('change'));
    console.log(`🔄 [Frontend] Auto Trade change event dispatched: ${settings.autoTrade}`);
}
```

#### 1.3 Go Trade Button Handler
- เพิ่ม log แสดงสถานะ Auto Trade เมื่อกดปุ่ม "Go Trade"
- Lines ~1306-1313

```javascript
document.getElementById('ltGoTradeBtn').addEventListener('click', async () => {
    // ...
    const autoTradeEnabled = document.getElementById('ltAutoTradeToggle').checked;
    console.log(`🚀 [Frontend] Go Trade clicked - Auto Trade is: ${autoTradeEnabled ? 'ENABLED' : 'DISABLED'}`);
    // ...
});
```

---

### 2. **Backend Debug Logging** (`main.rs`)

#### 2.1 Toggle Auto Endpoint
- เพิ่ม console log เมื่อได้รับคำสั่ง toggle auto trade
- Lines ~911-935

```rust
async fn handle_post_longterm_toggle_auto(
    State(state): State<AppState>,
    Json(payload): Json<LtToggleAutoRequest>,
) -> Json<serde_json::Value> {
    let mut setup = get_or_create_lt_setup();
    setup.auto_trade = payload.enabled;
    if let Ok(json_str) = serde_json::to_string_pretty(&setup) {
        let _ = std::fs::write(get_lt_setup_path(), json_str);
    }
    
    let msg = if payload.enabled {
        "⚙️ ระบบ Auto Trade: เปิด (ON)".to_string()
    } else {
        "⚙️ ระบบ Auto Trade: ปิด (OFF)".to_string()
    };
    
    // Log to both console and WebSocket
    println!("🔄 [Backend] Auto Trade toggled: {}", if payload.enabled { "ON" } else { "OFF" });
    
    let _ = state.tx.send(serde_json::json!({
        "type": "bot_log",
        "asset": "system",
        "data": { "message": msg }
    }));
    
    Json(serde_json::json!({ "status": "success", "enabled": payload.enabled }))
}
```

---

### 3. **Backend Analysis Logging** (`long_term.rs`)

#### 3.1 Initial Analysis (After Historical Candles Load)
- เพิ่ม log แสดงสถานะ auto_trade และเงื่อนไขที่เปิดใช้งาน
- เพิ่ม log เมื่อไม่มีเงื่อนไขตรง
- เพิ่ม log เมื่อ auto trade ถูกปิด
- Lines ~202-270

```rust
if let Some(last_result) = analysis.last() {
    let setup = crate::get_or_create_lt_setup();
    println!("🔍 [LongTerm] Initial analysis for {}: ema_cut_position={}, ema_cut_all_type={}, auto_trade={}", 
        asset, last_result.ema_cut_position, last_result.ema_cut_all_type, setup.auto_trade);
    
    if setup.auto_trade {
        println!("✅ [LongTerm] Auto Trade is ENABLED - checking conditions...");
        let cond_sm = setup.conditions.contains(&"condShortMedium".to_string());
        let cond_all = setup.conditions.contains(&"condLongCross".to_string());
        let cond_sl = setup.conditions.contains(&"condShortLong".to_string());
        println!("📋 [LongTerm] Conditions enabled: SM={}, SML={}, SL={}", cond_sm, cond_all, cond_sl);
        
        // ... condition checking ...
        
        if let Some(c_type) = trigger_type {
            // ... place order ...
        } else {
            println!("⏹️ [LongTerm] No matching condition on initial analysis for {}", asset);
        }
    } else {
        println!("⏸️ [LongTerm] Auto Trade is DISABLED - skipping initial analysis");
    }
}
```

#### 3.2 New Candle Update Analysis
- เพิ่ม log เมื่อเงื่อนไขตรงแต่ auto trade ถูกปิด
- Lines ~357-391

```rust
if let Some(c_type) = trigger_type {
    if setup.auto_trade {
        // ... existing cooldown and order logic ...
    } else {
        println!("⏸️ [LongTerm] Auto Trade DISABLED - condition matched but skipping (asset: {})", asset);
    }
}
```

---

## 📊 Expected Log Output

### Scenario 1: Auto Trade ENABLED (หลังโหลดหน้าเว็บ)

**Frontend Console**:
```
📥 [Frontend] Loading auto trade from settings: true
🔄 [Frontend] Auto Trade checkbox changed to: true
✅ [Frontend] Auto Trade toggle response: {status: "success", enabled: true}
🔄 [Frontend] Auto Trade change event dispatched: true
🚀 [Frontend] Go Trade clicked - Auto Trade is: ENABLED
```

**Backend Console**:
```
🔄 [Backend] Auto Trade toggled: ON
⚙️ ระบบ Auto Trade: เปิด (ON)
📈 [LongTerm] กำลังเชื่อมต่อ Deriv OTP WebSocket สำหรับ 4 assets...
📊 [LongTerm] ส่ง lt_candles_history สำหรับ vol25 (1000 แท่ง)
🔍 [LongTerm] Initial analysis for vol25: ema_cut_position=CrossUp, ema_cut_all_type=None, auto_trade=true
✅ [LongTerm] Auto Trade is ENABLED - checking conditions...
📋 [LongTerm] Conditions enabled: SM=true, SML=false, SL=false
🎯 [LongTerm INITIAL] เงื่อนไขตรง! ส่งคำสั่งซื้อ CALL vol25 (Entry: CrossUp, Code: SM)
```

### Scenario 2: Auto Trade DISABLED

**Backend Console**:
```
🔍 [LongTerm] Initial analysis for vol25: ema_cut_position=CrossUp, ema_cut_all_type=None, auto_trade=false
⏸️ [LongTerm] Auto Trade is DISABLED - skipping initial analysis
```

### Scenario 3: Condition Match but Auto Trade OFF

**Backend Console**:
```
⏸️ [LongTerm] Auto Trade DISABLED - condition matched but skipping (asset: vol25)
```

---

## 🔧 Testing Steps

1. **Stop Backend** (ถ้ากำลังรันอยู่)
   ```bash
   Ctrl+C
   ```

2. **Recompile Backend**
   ```bash
   cargo build --release
   ```

3. **Start Backend**
   ```bash
   cargo run --release
   ```

4. **Reload หน้าเว็บ** (F5)

5. **ตรวจสอบ Console Logs**
   - เปิด DevTools (F12)
   - ดู Console tab
   - ตรวจสอบว่ามี log `🔄 [Frontend] Auto Trade checkbox changed to: true`
   - ตรวจสอบว่ามี log `✅ [Frontend] Auto Trade toggle response`

6. **ตรวจสอบ Backend Console**
   - ดู terminal ที่รัน Backend
   - ต้องเห็น `🔄 [Backend] Auto Trade toggled: ON`
   - เมื่อกด "Go Trade" ต้องเห็น `🔍 [LongTerm] Initial analysis` และ `auto_trade=true`

7. **ทดสอบการเทรด**
   - เลือก asset (เช่น vol25)
   - เลือกเงื่อนไข (เช่น Short × Medium)
   - เปิด Auto Trade checkbox
   - กด "บันทึก"
   - กด "เชื่อมต่อสัญญาณ"
   - รอให้มีการโหลด historical candles
   - ถ้าเงื่อนไขตรง ต้องเห็น `🎯 [LongTerm INITIAL] เงื่อนไขตรง!`

---

## 🐛 Troubleshooting

### ปัญหา: Backend ยังไม่เทรดอัตโนมัติ

**ตรวจสอบ**:
1. Log `🔄 [Backend] Auto Trade toggled: ON` ปรากฏหรือไม่?
   - ถ้าไม่ = Frontend ไม่ได้ส่งคำสั่งไป Backend
   - แก้: ตรวจสอบ network tab ว่ามี request ไป `/api/longterm/toggle_auto` หรือไม่

2. Log `🔍 [LongTerm] Initial analysis` ปรากฏหรือไม่?
   - ถ้าไม่ = Backend ยังไม่ได้ compile ใหม่
   - แก้: รัน `cargo build --release` อีกครั้ง

3. Log แสดง `auto_trade=false` แทน `true`?
   - = `setup.json` ไม่ได้ save ค่า หรือ timing issue
   - แก้: ลองกด toggle off/on และบันทึกอีกครั้ง

4. Log แสดง `⏹️ [LongTerm] No matching condition`?
   - = ไม่มีเงื่อนไขตรงในแท่งสุดท้าย
   - รอแท่งใหม่ที่มีเงื่อนไขตรง

5. Log แสดง `📋 [LongTerm] Conditions enabled: SM=false, SML=false, SL=false`?
   - = ไม่มีเงื่อนไขถูกเลือกใน setup
   - แก้: เลือกเงื่อนไขอย่างน้อย 1 รายการและกดบันทึก

---

## 📝 Files Modified

### Frontend
- `public/long_term_trade.html`
  - Auto Trade toggle handler (~2787-2798)
  - Settings load function (~2959-2967)
  - Go Trade button handler (~1306-1313)

### Backend
- `src/main.rs`
  - `handle_post_longterm_toggle_auto` function (~911-935)

- `src/long_term.rs`
  - Initial analysis after historical candles (~202-270)
  - New candle update analysis (~357-391)

---

## ✅ Verification Checklist

- [x] Frontend dispatches `change` event เมื่อโหลด settings
- [x] Frontend เพิ่ม 50ms delay ก่อน dispatch event
- [x] Frontend log ทุก step ของ auto trade toggle
- [x] Backend log เมื่อได้รับ toggle request
- [x] Backend log เมื่อวิเคราะห์เงื่อนไขครั้งแรก (initial analysis)
- [x] Backend log แสดงสถานะ `auto_trade` และ conditions
- [x] Backend log เมื่อเงื่อนไขตรงแต่ auto trade ปิด
- [x] Backend log เมื่อไม่มีเงื่อนไขตรงใน initial analysis

---

## 🎉 Expected Result

หลัง compile ใหม่และโหลดหน้าเว็บ:
1. Auto Trade checkbox จะถูก sync กับ Backend ทันที
2. Log จะแสดงข้อมูลทุก step เพื่อ debug
3. ถ้า Auto Trade เปิดอยู่และมีเงื่อนไขตรง จะเทรดอัตโนมัติทันที
4. ไม่ต้อง toggle checkbox off/on เพื่อให้ auto trade ทำงาน

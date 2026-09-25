# พฤติกรรมการอัพเดทการตั้งค่าระหว่างเทรด

## ❓ คำถาม

> ถ้าระบบ main.rs กำลัง run และกำลังเทรดอยู่ ถ้าฉันเปลี่ยน input เช่นเปลี่ยน strategy ในหน้า index_short_term.html มันจะส่งค่าไป update ค่าตัวแปรใน main.rs หรือไม่ หรือต้องทำการบันทึกก่อน?

## ✅ คำตอบสั้น

**บันทึกอัตโนมัติ** แต่ **bot ที่กำลังรันจะไม่ได้รับการตั้งค่าใหม่**

ต้อง **หยุด bot แล้วเริ่มใหม่** เพื่อให้ใช้การตั้งค่าที่เปลี่ยน

---

## 📊 Flow การทำงาน

### 1. เมื่อเปลี่ยน Strategy ในหน้าเว็บ

```
ผู้ใช้เปลี่ยน Strategy
    ↓
suggestStrategySelect.addEventListener('change')
    ↓
updateJSON()  ← อัพเดท JSON textarea
    ↓
autoSaveSetup()  ← บันทึกอัตโนมัติ
    ↓
POST /api/setup  ← ส่งไปยัง backend
    ↓
Rust: save_setup()  ← บันทึกลงไฟล์ setup/setup.json
    ↓
✅ ไฟล์ถูกบันทึกเรียบร้อย
    ↓
⚠️ แต่ bot ที่กำลังรันยังใช้ค่าเก่า
```

### 2. การทำงานของ `autoSaveSetup()`

```javascript
// File: pkderiv.js
async function autoSaveSetup() {
    updateJSON();  // อัพเดท JSON textarea
    const currentSettings = collectSettings();  // รวบรวมการตั้งค่าทั้งหมด
    
    try {
        // ส่งไปบันทึกที่ backend ทันที
        await fetch('/api/setup', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(currentSettings)
        });
        
        console.log('✅ Auto-saved setup');
    } catch (e) {
        console.error("Could not auto-save setup:", e);
    }
}
```

**เมื่อไหร่ที่มีการบันทึกอัตโนมัติ?**
- เมื่อเปลี่ยน **Strategy** (`suggestStrategySelect`)
- เมื่อเปลี่ยน **Borrow Signal** toggle
- เมื่อเปลี่ยน **Whipsaw Zone** toggle
- เมื่อเปลี่ยน **Buy Method**

### 3. การทำงานของ Backend `save_setup()`

```rust
// File: main.rs
async fn save_setup(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    // 1. บันทึกลงไฟล์
    fs::write("setup/setup.json", 
              serde_json::to_string_pretty(&payload)?)?;
    
    // 2. ส่งคำสั่ง reload_history ไปยัง bot
    state.cmd_tx.send(serde_json::json!({
        "command": "reload_history"
    }));
    
    // 3. คำนวณ indicators ใหม่สำหรับ candles ที่แคชไว้
    // (อัพเดทกราฟ แต่ไม่เปลี่ยน strategy ของ bot)
    for (asset, cached_candles) in candle_cache {
        let new_analysis = recalculate_indicators(
            cached_candles, 
            new_config  // ← ใช้ config ใหม่
        );
        
        // 4. Broadcast กราฟใหม่ไปยัง WebSocket clients
        state.tx.send(new_analysis);
    }
    
    Json(serde_json::json!({"status": "success"}))
}
```

**สิ่งที่เกิดขึ้น:**
- ✅ ไฟล์ `setup/setup.json` ถูกอัพเดท
- ✅ ส่งคำสั่ง `reload_history` ไปยัง bot
- ✅ คำนวณ indicators ใหม่และอัพเดทกราฟ
- ✅ Broadcast ข้อมูลใหม่ไปยัง WebSocket clients
- ❌ **ไม่เปลี่ยน strategy ของ bot ที่กำลังรัน**

---

## 🤖 Bot อ่าน Config เมื่อไหร่?

### เมื่อเริ่มต้น Bot (`POST /api/trade`)

```rust
async fn handle_post_trade(
    State(state): State<AppState>,
    Json(payload): Json<AppConfigPayload>,  // ← รับ config จาก frontend
) -> Json<serde_json::Value> {
    // 1. ปิด bot เก่า (ถ้ามี)
    if let Some(old_bot) = bot_guard.take() {
        old_bot.abort();
    }
    
    // 2. เก็บ config ไว้ใช้
    let config = payload.clone();  // ← Config ที่ส่งมาจาก frontend
    
    // 3. สร้าง bot ใหม่
    let handle = tokio::spawn(async move {
        // ส่ง config นี้ให้ bot
        deriv::start_deriv_bot_multiplexed(
            app_id,
            api_token,
            account_id,
            config,  // ← ใช้ config นี้ตลอดการรัน
            assets,
            tx,
            cmd_rx,
            ot_signals
        ).await
    });
    
    *bot_guard = Some(handle);
    
    Json(serde_json::json!({"status": "success"}))
}
```

**สิ่งสำคัญ:**
- Bot อ่าน config **เพียงครั้งเดียว** เมื่อเริ่มต้น
- Config ถูก **move** เข้าไปใน async task
- Bot ใช้ config นี้ **ตลอดการรัน**
- **ไม่มี** mechanism สำหรับอัพเดท config ระหว่างรัน

---

## 🔄 วิธีให้ Bot ใช้การตั้งค่าใหม่

### วิธีที่ 1: หยุดและเริ่มใหม่ (แนะนำ)

```
1. กดปุ่ม "Stop Trade"
2. เปลี่ยนการตั้งค่า (มันจะบันทึกอัตโนมัติ)
3. กดปุ่ม "Go Trade" ใหม่
```

**ข้อดี:**
- ✅ แน่นอนว่าใช้การตั้งค่าใหม่
- ✅ ไม่มีความเสี่ยงจาก race condition
- ✅ Trade history สะอาด (มี start/stop ชัดเจน)

**ข้อเสีย:**
- ❌ ต้องหยุดเทรด (อาจพลาดโอกาส)
- ❌ ต้องทำ manual

### วิธีที่ 2: Reload Config ระหว่างรัน (ไม่แนะนำ)

**ปัจจุบันไม่มี feature นี้** แต่ถ้าต้องการเพิ่ม ต้อง:

1. เพิ่ม command channel สำหรับ reload config
2. Bot ต้อง check command ระหว่างรัน
3. ต้องระวัง race condition

```rust
// ตัวอย่าง (ยังไม่มีในโค้ด)
loop {
    // ตรวจสอบ command
    if let Ok(cmd) = cmd_rx.try_recv() {
        if cmd["command"] == "reload_config" {
            // อ่าน config ใหม่จากไฟล์
            let new_config = read_setup_file();
            // อัพเดท strategy
            current_strategy = new_config.trade.suggest_strategy;
        }
    }
    
    // ทำงานต่อ
    execute_trading_logic();
}
```

**ข้อดี:**
- ✅ ไม่ต้องหยุด bot
- ✅ สามารถเปลี่ยนการตั้งค่าได้ทันที

**ข้อเสีย:**
- ❌ ซับซ้อน มีโอกาส bug
- ❌ อาจเกิด race condition
- ❌ Strategy กลางคัน อาจทำให้สับสน
- ❌ ยากต่อการ debug

---

## 📝 สรุปพฤติกรรม

### ตารางสรุป

| การกระทำ | บันทึกลงไฟล์ | Bot ใช้ค่าใหม่ | ต้องหยุด Bot | กราฟอัพเดท |
|----------|--------------|---------------|-------------|------------|
| เปลี่ยน Strategy | ✅ อัตโนมัติ | ❌ ไม่ใช้ | ✅ ต้องหยุด | ✅ อัพเดท |
| เปลี่ยน EMA Period | ✅ อัตโนมัติ | ❌ ไม่ใช้ | ✅ ต้องหยุด | ✅ อัพเดท |
| เปลี่ยน Martingale | ✅ อัตโนมัติ | ❌ ไม่ใช้ | ✅ ต้องหยุด | - |
| เปลี่ยน Target Money | ✅ อัตโนมัติ | ❌ ไม่ใช้ | ✅ ต้องหยุด | - |
| เปลี่ยน Assets | ไม่บันทึกอัตโนมัติ | ❌ ไม่ใช้ | ✅ ต้องหยุด | - |

### สิ่งที่บันทึกอัตโนมัติ

```javascript
// Fields ที่มี auto-save
- suggestStrategy (ATRSpike, PKTrend, V1, V2, Manual)
- borrowSignal (true/false)
- whipsawZone (true/false)
- buyMethod (proposal/contract)
```

### สิ่งที่ต้องกด "Go Trade" เพื่อบันทึก

```javascript
// Fields ที่ไม่มี auto-save (บันทึกเมื่อกด Go Trade)
- assets (vol10, vol50, vol75)
- granularity (60, 120, 300, etc.)
- martingale settings
- targetMoney
- targetLot
- EMA periods
- indicator settings
```

---

## 🎯 Best Practices

### 1. การเปลี่ยนการตั้งค่าระหว่างเทรด

❌ **อย่าทำ:**
```
1. Bot กำลังรันอยู่
2. เปลี่ยน Strategy จาก ATRSpike → PKTrend
3. คาดหวังว่า bot จะใช้ strategy ใหม่ทันที
```

✅ **ทำแบบนี้:**
```
1. กด "Stop Trade"
2. เปลี่ยน Strategy จาก ATRSpike → PKTrend
3. กด "Go Trade" ใหม่
4. Bot จะใช้ strategy ใหม่
```

### 2. การทดสอบการตั้งค่าใหม่

```
1. เปิด Browser Console (F12)
2. เปลี่ยนการตั้งค่าที่ต้องการ
3. ดู console log ว่ามี "✅ Auto-saved setup" หรือไม่
4. เปิด Network tab ดูว่ามี POST /api/setup หรือไม่
5. ตรวจสอบ Response ต้องเป็น {"status": "success"}
6. หยุด bot เก่า
7. เริ่ม bot ใหม่
8. Bot จะใช้การตั้งค่าใหม่
```

### 3. การตรวจสอบว่าใช้การตั้งค่าใหม่แล้ว

```
1. เปิด Browser Console
2. พิมพ์: fetch('/api/setup').then(r=>r.json()).then(console.log)
3. ดูค่า "suggestStrategy" ว่าเป็นค่าใหม่หรือไม่
4. ถ้าใช่ → หยุดและเริ่ม bot ใหม่
5. ถ้าไม่ → บันทึกการตั้งค่าอีกครั้ง
```

---

## 🔍 Debugging

### ตรวจสอบว่าไฟล์ถูกบันทึกหรือไม่

```powershell
# Windows
type "setup\setup.json"

# หรือใช้ PowerShell
Get-Content "setup\setup.json" | ConvertFrom-Json | Format-List
```

### ตรวจสอบว่า Bot กำลังใช้ config ไหน

```rust
// ใน main.rs - เพิ่ม log
async fn handle_post_trade(...) -> ... {
    println!("🔧 Bot Config: Strategy = {}", payload.trade.suggest_strategy);
    println!("🔧 Bot Config: Max Loss Con = {}", payload.trade.max_loss_con);
    // ...
}
```

### ตรวจสอบผ่าน Terminal/Console

```bash
# ดู log ใน terminal
📝 [save_setup] Saved setup.json. Payload theme: ...
✅ [save_setup] Deserialized AppConfigPayload successfully.
🔄 [save_setup] Recalculating 300 candles for asset: vol10
📢 [save_setup] Broadcasted candles_history for vol10 to 2 subscribers.
```

---

## 📚 ดูเพิ่มเติม

- [trade-config-storage.md](./trade-config-storage.md) - การเก็บค่าการตั้งค่าการเทรด
- [api_routes.html](../api_routes.html) - API Documentation

---

## ✅ Checklist: การเปลี่ยนการตั้งค่า

- [ ] เปลี่ยนการตั้งค่าในหน้าเว็บ
- [ ] ตรวจสอบว่ามี auto-save (ดู Console log)
- [ ] กด "Stop Trade" เพื่อหยุด bot
- [ ] รอให้ bot หยุดเรียบร้อย (สถานะเป็น "หยุดเทรดแล้ว")
- [ ] กด "Go Trade" เพื่อเริ่ม bot ใหม่
- [ ] ตรวจสอบว่า bot ใช้การตั้งค่าใหม่ (ดู Terminal log)

---

**อัพเดทล่าสุด:** 28 สิงหาคม 2569  
**Version:** 1.0  
**ผู้เขียน:** PK Deriv Trade Documentation Team

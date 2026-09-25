# 📋 Changelog v1.3 — Multi-Machine State Sync
> วันที่แก้ไข: 22 เมษายน 2569 (2026)  
> สถานะ: ⏳ **รอทดสอบ**

---

## 🎯 ปัญหาที่แก้ไข

**เมื่อเปิดหน้าเว็บจากเครื่อง B หลังจากเครื่อง A เริ่มเทรดไปแล้ว:**
- ❌ ไม่มี log ย้อนหลัง → หน้าจอนิ่ง
- ❌ กราฟว่างเปล่า → ไม่มี candle data
- ❌ สถานะแสดง Disconnected → ต้องกด Go Trade ใหม่
- ❌ ถ้ากด Go Trade จากเครื่อง B → **เทรดซ้ำซ้อน!**

---

## 🔧 สิ่งที่แก้ไข

### ไฟล์ที่แก้: `src/main.rs`

#### 1. เพิ่ม Shared State ใน AppState (บรรทัด 109-116)
```rust
pub struct AppState {
    pub tx: ...,
    pub active_bots: ...,
    pub bot_logs: Arc<Mutex<Vec<serde_json::Value>>>,       // ใหม่!
    pub candle_data: Arc<Mutex<HashMap<String, serde_json::Value>>>, // ใหม่!
}
```
- `bot_logs` — เก็บ log ทั้งหมดที่ส่งผ่าน broadcast (จำกัด 500 entries)
- `candle_data` — เก็บข้อมูลแท่งเทียนแยกตาม asset (อัปเดตทุก tick)

#### 2. Storage Subscriber — Background Task (บรรทัด 754-811)
- ดักจับ **ทุก message** จาก broadcast channel อัตโนมัติ
- ประเภทที่เก็บ:
  - `candles_history` → เก็บข้อมูลกราฟแยกตาม asset
  - `ohlc_update` → อัปเดตแท่งเทียนล่าสุดลงใน snapshot
  - `bot_log` → เก็บ log (จำกัด 500 entries)
  - `balance_update` / `trade_result` → เก็บใน log เพื่อ replay
- ✅ **ไม่ต้องแก้ `deriv.rs` เลย** — ดักจับจาก broadcast โดยตรง

#### 3. WebSocket Snapshot สำหรับ Client ใหม่ (บรรทัด 127-169)
- เมื่อ client ใหม่ connect (เครื่อง B) → ส่ง snapshot ข้อมูลทั้งหมดก่อน:
  1. ส่ง `candle_data` ทุก asset → กราฟแสดงทันที
  2. ส่ง `bot_logs` ทั้งหมด → log ย้อนหลังแสดงทันที
- หลังจาก snapshot เสร็จ → เข้า broadcast loop ปกติ

#### 4. API `/api/status` ใหม่ (บรรทัด 707-725)
- Frontend เรียก endpoint นี้เพื่อตรวจสอบสถานะบอท
- Response:
```json
{
    "is_trading": true,
    "active_assets": ["vol10", "vol25_1s", "vol75"],
    "bot_count": 3,
    "log_count": 42,
    "candle_assets": ["vol10", "vol25_1s", "vol75"]
}
```

#### 5. Version bump: `v1.2` → `v1.3`

---

### ไฟล์ที่แก้: `php/index.html`

#### WebSocket onopen — ตรวจสถานะอัตโนมัติ (บรรทัด 2069-2085)
- เมื่อ WS connect สำเร็จ → เรียก `GET /api/status`
- ถ้า `is_trading = true`:
  - ตั้ง `isTradeRunning = true` → **ปุ่ม Go Trade ถูกป้องกันไม่ให้กดซ้ำ**
  - แสดง "Trading Active (3 bots: vol10, vol25_1s, vol75)"
  - แสดง log: "🔄 เชื่อมต่อสำเร็จ — พบบอทกำลังทำงาน 3 ตัว"
- ถ้า `is_trading = false`:
  - แสดง log: "🔌 เชื่อมต่อสำเร็จ — ยังไม่มีบอททำงาน"

---

## 📁 ไฟล์ที่ไม่ต้องแก้ไข
| ไฟล์ | เหตุผล |
|---|---|
| `src/deriv.rs` | Storage Subscriber ดักจับข้อมูลจาก broadcast อัตโนมัติ |
| `src/analysis.rs` | ไม่เกี่ยวข้อง |
| `src/get_action.rs` | ไม่เกี่ยวข้อง |
| `setup.json` | ไม่เกี่ยวข้อง |

---

## 🧪 วิธีทดสอบ

### ขั้นตอนที่ 1: Compile
```bash
cargo build --release
```
> ⚠️ ถ้ามี compile error ให้แจ้ง error message กลับมา

### ขั้นตอนที่ 2: ทดสอบเครื่อง A
1. รันโปรแกรม `cargo run` หรือ binary
2. เปิดเว็บ `http://localhost:3000`
3. กด **Go Trade** → เทรดปกติ
4. สังเกต: กราฟแสดง, log ทำงาน, สถานะ Connected

### ขั้นตอนที่ 3: ทดสอบเครื่อง B (หรือเปิด browser tab ใหม่)
1. เปิดเว็บ `http://<IP-เครื่อง-A>:3000` (หรือเปิด tab ใหม่ใน browser)
2. **สิ่งที่ควรเห็น:**
   - ✅ สถานะ: "Trading Active (X bots: ...)"
   - ✅ กราฟ: แสดงแท่งเทียนทันที (ไม่ต้องรอ)
   - ✅ Bot Log: เห็น log ย้อนหลังทั้งหมด
   - ✅ ปุ่ม Go Trade: กดแล้วไม่ทำอะไร (ป้องกันซ้ำซ้อน)

### ขั้นตอนที่ 4: ทดสอบ Edge Cases
- [ ] ปิด tab เครื่อง A แล้วเปิดใหม่ → ควรเห็นข้อมูลเดิม
- [ ] Stop Trade แล้วเปิด tab ใหม่ → ควรเห็น "ยังไม่มีบอททำงาน"
- [ ] เปิดหลาย tab พร้อมกัน → ทุก tab ควรได้ข้อมูลเหมือนกัน

---

## ⚠️ สิ่งที่ต้องรายงานกลับ

1. **Compile ผ่านหรือไม่?** → ถ้าไม่ผ่าน ส่ง error message มา
2. **เครื่อง B เห็นกราฟทันทีหรือไม่?**
3. **เครื่อง B เห็น log ย้อนหลังหรือไม่?**
4. **สถานะแสดง "Trading Active" หรือไม่?**
5. **กด Go Trade จากเครื่อง B ซ้ำได้หรือไม่?** (ควรไม่ได้)
6. **มีปัญหาอื่นๆ หรือไม่?**

---

## 🏗️ สถาปัตยกรรมใหม่

```
┌──────────────────────────────────────────────┐
│              Rust Backend (main.rs)           │
│                                               │
│  ┌─────────────┐   broadcast   ┌────────────┐│
│  │  deriv.rs    │──── tx ──────►│  Storage   ││
│  │  (bots)      │              │  Subscriber ││
│  └─────────────┘              └─────┬──────┘│
│         │                           │        │
│         │ broadcast                 ▼        │
│         │               ┌─────────────────┐  │
│         │               │  Shared State   │  │
│         │               │  - candle_data  │  │
│         │               │  - bot_logs     │  │
│         │               └────────┬────────┘  │
│         │                        │           │
│         ▼                        ▼           │
│  ┌──────────────────────────────────────┐    │
│  │         WebSocket Handler            │    │
│  │  1. ส่ง snapshot (candle + logs)      │    │
│  │  2. forward broadcast ต่อเนื่อง       │    │
│  └──────────────────────────────────────┘    │
│                    │                          │
│  ┌─────────────────┼──────────────────────┐  │
│  │  GET /api/status │                     │  │
│  └─────────────────┼──────────────────────┘  │
└────────────────────┼─────────────────────────┘
                     │
          ┌──────────┴──────────┐
          │                     │
    ┌─────▼─────┐        ┌─────▼─────┐
    │ เครื่อง A  │        │ เครื่อง B  │
    │ (Browser)  │        │ (Browser)  │
    │ กด GoTrade │        │ เห็นทุกอย่าง│
    │ ปกติ       │        │ ทันที!     │
    └───────────┘        └───────────┘
```

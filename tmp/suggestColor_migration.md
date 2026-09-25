# 📋 SuggestColor Strategy Migration — สถานะงาน

> **สร้างจาก:** LabATRV2/convertToRustByClaude.md  
> **วันที่:** 17 พฤษภาคม 2569  
> **สถานะรวม:** ✅ ทุก Step เสร็จสมบูรณ์! (Step 1-9)

---

## ✅ Step 1-6: Strategy Functions + Tests

| Step | งาน | สถานะ |
|------|------|--------|
| 1 | เพิ่ม `SuggestStrategy` enum (V1, V2, V3A, V3B) | ✅ |
| 2 | เพิ่ม `get_suggest_color_v2()` — threshold=1 | ✅ |
| 3 | เพิ่ม `get_suggest_color_v3a()` — EMA Consensus | ✅ |
| 4 | เพิ่ม `get_suggest_color_v3b()` — EMA Short + CutType | ✅ |
| 5 | เพิ่ม `get_suggest_color_by_strategy()` Dispatcher | ✅ |
| 6 | Unit Tests 24 ตัว | ✅ |

## ✅ Step 7-8: Production Wiring + Config

| Step | งาน | สถานะ |
|------|------|--------|
| 7 | `deriv.rs` ใช้ Dispatcher แทน hardcode V1 | ✅ |
| 8 | เพิ่ม `suggestStrategy` ใน `TradeConfig` / `setup.json` | ✅ |

## ✅ Step 9: Strategy Comparison Log

| Step | งาน | สถานะ |
|------|------|--------|
| 9 | บันทึกผลเปรียบเทียบทุก strategy ลง `strategy_comparison.json` | ✅ |

### Flow การทำงาน

```
Trade Entry (is_new_candle + should_trade)
├─ คำนวณ suggest จาก V1, V2, V3A, V3B (ใช้ loss_con แยกอิสระ)
├─ สร้าง pending_strategy_entry (JSON)
└─ ส่ง order จริงตาม activeStrategy

          ↓

Trade Settlement (contract ended)
├─ หาสีแท่งเทียนจริงจาก contract result:
│     CALL+Win=green  CALL+Loss=red
│     PUT+Win=red     PUT+Loss=green
├─ ตรวจ Win/Loss ของแต่ละ strategy
├─ อัปเดต per-strategy loss_con อิสระ
└─ บันทึกลง strategy_comparison.json
```

### โครงสร้าง JSON Output

```json
{
  "runno": 1,
  "assetCode": "R_10",
  "candleTimestamp": 1747456800,
  "candleTimeDisp": "14:00:00",
  "thisColor": "red",
  "emaShortDirection": "Down",
  "emaMediumDirection": "Up",
  "emaLongDirection": "Up",
  "isEmaShortCutType": "-",
  "activeStrategy": "V1",

  "suggestActionByV1": "CALL",
  "suggestColorByV1": "green",
  "lossConByV1": 0,
  "winStatusByV1": "Win",
  "lossConAfterV1": 0,

  "suggestActionByV2": "PUT",
  "suggestColorByV2": "red",
  "lossConByV2": 1,
  "winStatusByV2": "Loss",
  "lossConAfterV2": 2,

  "suggestActionByV3A": "CALL",
  "suggestColorByV3A": "green",
  "lossConByV3A": 0,
  "winStatusByV3A": "Win",
  "lossConAfterV3A": 0,

  "suggestActionByV3B": "PUT",
  "suggestColorByV3B": "red",
  "lossConByV3B": 2,
  "winStatusByV3B": "Loss",
  "lossConAfterV3B": 3,

  "actualNextColor": "green",
  "actualContractType": "CALL",
  "actualProfit": 0.95
}
```

### ตำแหน่งไฟล์

```
tradeData/
└── 05-2569/
    └── 17-05-2569/
        ├── order_tracker.log        ← เดิม
        ├── strategy_comparison.json ← ✅ ใหม่ (รวมทุก asset)
        ├── R_10/
        │   └── trades.json          ← เดิม (แยก asset)
        └── R_75/
            └── trades.json
```

---

## 📁 โครงสร้างไฟล์ที่แก้ไข

```
src/
├── get_action.rs    ← ✅ Step 1-6 (V1/V2/V3A/V3B + Dispatcher + Tests)
├── deriv.rs         ← ✅ Step 7+9 (Dispatcher + Strategy Comparison Log)
├── main.rs          ← ✅ Step 8 (suggestStrategy ใน TradeConfig)
├── analysis.rs      ← ไม่ต้องแก้
└── lib.rs           ← ไม่ต้องแก้
```

---

## 📊 ตารางเปรียบเทียบ 4 Strategies

| Version | Threshold | ดูอะไร | แนวคิด |
|---------|-----------|--------|--------|
| V1 | `>= 2` | `thisColor` | เล่นตามสีแท่งปัจจุบัน (default เดิม) |
| V2 | `>= 1` | `thisColor` | เหมือน V1 แต่ react เร็วกว่า |
| V3A | `>= 2` | EMA Short/Medium/Long direction | นับเสียง 3 EMA → ส่วนใหญ่ชนะ |
| V3B | `>= 2` | EMA Short direction + CutType | ดู EMA Short เป็นหลัก + crossover override |

---

## 🔧 คำสั่ง Test & Build

```bash
# Test
cargo test --bin turbo-indicators get_action::tests -- --nocapture

# Build
cargo build

# Run
cargo run
```

## 🔗 ต้นฉบับ
- JS Source: `D:\Rust\LabATRV2\getactionJS.js`
- แผนงานเดิม: `D:\Rust\LabATRV2\convertToRustByClaude.md`

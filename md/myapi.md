# รายละเอียด API ของระบบ Rust Backend (Axum Framework)

เอกสารนี้รวบรวมเส้นทาง (Routes) ทั้งหมดของระบบ Rust Backend ซึ่งได้รับการพัฒนาโดยใช้ Axum Framework ในโปรเจกต์ `indicators_Multiplex` สำหรับใช้สื่อสารกับ Frontend/Web UI

---

## 1. ตารางสรุป API Endpoints ทั้งหมด

| Method | Path | Handler Function | Description |
| :--- | :--- | :--- | :--- |
| **GET** | `/api/setup` | [get_setup](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex/src/main.rs#L426) | โหลดการตั้งค่าทั้งหมดจาก `setup.json` |
| **POST** | `/api/setup` | [save_setup](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex/src/main.rs#L472) | บันทึกการตั้งค่าระบบลงใน `setup.json` |
| **GET** | `/api/version` | [get_version](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex/src/main.rs#L1074) | ตรวจสอบรุ่น (Version) และเวลาคอมไพล์ระบบ |
| **POST** | `/api/trade` | [handle_post_trade](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex/src/main.rs#L212) | สั่งเริ่มทำงานบอทเทรด (Multiplexed Bot) |
| **POST** | `/api/stop` | [handle_post_stop](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex/src/main.rs#L294) | สั่งหยุดบอทเทรดทั้งหมด |
| **POST** | `/api/terminate` | [handle_post_terminate](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex/src/main.rs#L316) | หยุดการทำงานของบอทและสั่งปิด Backend ทันที |
| **POST** | `/api/sell` | [handle_post_sell](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex/src/main.rs#L441) | ส่งคำสั่งขายสัญญาล่วงหน้า (Sell) ไปยังบอท |
| **GET** | `/api/status` | [handle_get_status](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex/src/main.rs#L1247) | ดึงสถานะการทำงานปัจจุบันของบอทและยอดเงิน |
| **POST** | `/api/balance/sync` | [handle_post_sync_balance](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex/src/main.rs#L1266) | อัปเดตและประสานยอดเงิน (Balance) ล่าสุดจากโบรกเกอร์ |
| **GET** | `/api/history` | [get_history](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex/src/main.rs#L486) | ดึงข้อมูลประวัติการเทรดทั้งหมดตามวันที่กำหนด |
| **GET** | `/api/getTradeHistory` | [get_tradehistory_data](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex/src/main.rs#L553) | ดึงไฟล์ประวัติการเทรดของ asset รายตัว |
| **GET** | `/api/getStrategyComparison` | [get_strategy_comparison](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex/src/main.rs#L586) | ดึงรายงานเปรียบเทียบกลยุทธ์ตามวันที่ |
| **POST** | `/api/generate_analysis` | [handle_post_generate_analysis](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex/src/main.rs#L345) | สั่งดาวน์โหลดแท่งเทียนย้อนหลังมาวิเคราะห์และบันทึกไฟล์แคช |
| **POST** | `/getAnalysisdata` | [full_analysis::handle_post_analysis_data](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex/src/full_analysis.rs#L312) | ส่งอาร์เรย์แท่งเทียนดิบมาวิเคราะห์แบบ Real-time และส่งผลลัพธ์กลับ |
| **GET** | `/getanalysisdata` | [handle_get_analysis_data](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex/src/main.rs#L1093) | ดึงข้อมูลผลลัพธ์การวิเคราะห์ตามวันที่ระบุ (ตรวจสอบแคชก่อนสร้างใหม่) |
| **POST** | `/api/notify` | [handle_post_notify](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex/src/main.rs#L461) | ส่งข้อความแจ้งเตือนผ่าน Telegram Bot |
| **GET** | `/ws` | [ws_handler](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex/src/main.rs#L161) | การเชื่อมต่อ WebSocket สำหรับ Real-time streaming (JSON) |

---

## 2. รายละเอียดแต่ละ Endpoint

### หมวดตั้งค่าระบบและเวอร์ชัน (Setup & Version)

#### • `GET /api/setup`
* **ข้อมูลที่ตอบกลับ (JSON):** โครงสร้างการตั้งค่าระบบจาก `setup.json` (เช่น `meta`, `dateRange`, `granularity`, `assets`, `indicators`, `ema`, `trade`)
* **ตัวอย่างการใช้:** ใช้เมื่อโหลดหน้าเว็บครั้งแรกเพื่อนำข้อมูลการตั้งค่าเดิมขึ้นมาแสดงผล

#### • `POST /api/setup`
* **ข้อมูลที่รับเข้า (JSON Payload):** โครงสร้างการตั้งค่าระบบใหม่ทั้งหมด
* **ข้อมูลที่ตอบกลับ (JSON):**
  ```json
  { "status": "success" }
  ```

#### • `GET /api/version`
* **ข้อมูลที่ตอบกลับ (JSON):**
  ```json
  {
    "main_rs_version": "v1.3",
    "compiled_at": "YYYY-MM-DD HH:MM:SS"
  }
  ```

---

### หมวดควบคุมบอทเทรด (Bot Trading Control)

#### • `POST /api/trade`
* **ข้อมูลที่รับเข้า (JSON Payload):** โครงสร้าง `AppConfigPayload` ที่มีรายละเอียดการเทรดที่ตั้งใจจะเริ่ม
* **ข้อมูลที่ตอบกลับ (JSON):**
  ```json
  {
    "status": "success",
    "message": "Spawned [จำนวนบอท] bot(s): [ชื่อ Asset]. กำลังเชื่อมต่อ Deriv..."
  }
  ```

#### • `POST /api/stop`
* **ข้อมูลที่ตอบกลับ (JSON):**
  ```json
  {
    "status": "success",
    "message": "หยุดบอททั้งหมด [จำนวน] ตัวแล้ว"
  }
  ```

#### • `POST /api/terminate`
* **ข้อมูลที่ตอบกลับ (JSON):**
  ```json
  {
    "status": "success",
    "message": "ปิดระบบ Backend แล้ว! โปรแกรมจะออกสู่หน้าต่าง Prompt"
  }
  ```
  *(ระบบจะหยุดบอททั้งหมดและสั่งปิดตนเองแบบ Shutdown ทันทีหลังส่ง Response นี้ไป)*

#### • `POST /api/sell`
* **ข้อมูลที่รับเข้า (JSON Payload):**
  ```json
  {
    "contract_id": 123456789
  }
  ```
* **ข้อมูลที่ตอบกลับ (JSON):**
  ```json
  {
    "status": "success",
    "message": "ส่งคำสั่งขายไปยังบอทแล้ว"
  }
  ```

#### • `GET /api/status`
* **ข้อมูลที่ตอบกลับ (JSON):**
  ```json
  {
    "is_trading": true,
    "active_assets": ["R_10"],
    "bot_count": 1,
    "log_count": 25,
    "candle_assets": ["R_10"],
    "balance": 10045.20
  }
  ```

#### • `POST /api/balance/sync`
* **ข้อมูลที่ตอบกลับ (JSON):**
  ```json
  {
    "status": "success",
    "balance": 10045.20,
    "message": "Balance synced: $10045.20"
  }
  ```

---

### หมวดประวัติการเทรด (Trade History)

#### • `GET /api/history`
* **พารามิเตอร์ (Query Parameters):** `date` (ฟอร์แมต: `YYYY-MM-DD` ค.ศ.)
* **ข้อมูลที่ตอบกลับ (JSON Array):** อาร์เรย์ของรายการเทรดทั้งหมดในวันนั้น เรียงลำดับจากเวลาใหม่สุดไปเก่าสุด
  ```json
  [
    {
      "timeCandle": 1716300000,
      "Asset": "R_10",
      "action": "Buy",
      "profit": 1.25,
      ...
    }
  ]
  ```

#### • `GET /api/getTradeHistory`
* **พารามิเตอร์ (Query Parameters):**
  * `asset`: รหัส Asset (เช่น `R_10`)
  * `day_trade`: วันที่เทรด (ฟอร์แมต: `YYYY-MM-DD` ค.ศ.)
* **ข้อมูลที่ตอบกลับ (JSON):** อาร์เรย์ของรายการเทรดเฉพาะของ Asset นั้นๆ จากไฟล์ `tradeData/{MM-พ.ศ.}/{DD-MM-พ.ศ.}/{asset}/trades.json`

#### • `GET /api/getStrategyComparison`
* **พารามิเตอร์ (Query Parameters):** `day_trade` (ฟอร์แมต: `YYYY-MM-DD` ค.ศ.)
* ** https://pkderiv.shop/api/getStrategyComparison?dayTrade=2026-05-21
* **ข้อมูลที่ตอบกลับ (JSON):** อาร์เรย์เปรียบเทียบผลลัพธ์ของแต่ละกลยุทธ์จากไฟล์ `strategy_comparison.json`

---

### หมวดการวิเคราะห์ข้อมูล (Analysis Generation & Query)

#### • `POST /api/generate_analysis`
* **ข้อมูลที่รับเข้า (JSON Payload):**
  ```json
  {
    "asset": "R_10",
    "start_date": "2026-04-22T15:00",
    "stop_date": "2026-04-22T17:00"
  }
  ```
* **ข้อมูลที่ตอบกลับ (JSON):** ข้อมูลผลวิเคราะห์, จำนวนข้อมูลที่ได้ และเส้นทางบันทึกไฟล์แคชบน Disk
  ```json
  {
    "status": "success",
    "asset": "R_10",
    "count": 120,
    "start": "2026-04-22T15:00",
    "stop": "2026-04-22T17:00",
    "file_saved": "analysisData/04-2569/22-04-2569/R_10/analysis_2026-04-22T15-00_to_2026-04-22T17-00.json",
    "setup_saved": "analysisData/04-2569/22-04-2569/R_10/setup_2026-04-22T15-00_to_2026-04-22T17-00.json",
    "data": [ ... ]
  }
  ```

#### • `POST /getAnalysisdata`
* **ข้อมูลที่รับเข้า (JSON Payload):** อาร์เรย์ข้อมูลแท่งเทียนดิบ (`RawCandleInput`)
  ```json
  [
    { "epoch": 1716300000, "open": 100.5, "high": 101.2, "low": 99.8, "close": 100.9 },
    ...
  ]
  ```
* **ข้อมูลที่ตอบกลับ (JSON Array):** รายการแท่งเทียนที่ผ่านการวิเคราะห์อินดิเคเตอร์ต่างๆ (EMA, MACD, RSI, ADX, Bollinger Bands, SMC, Wick/Body Ratio)
  ```json
  [
    {
      "index": 0,
      "candletime": 1716300000,
      "candletime_display": "2026-05-21 17:00:00",
      "open": 100.5,
      ...
      "ema_short_value": 100.7,
      "smc": { ... }
    }
  ]
  ```

#### • `GET /getanalysisdata`
* **พารามิเตอร์ (Query Parameters):**
  * `assetcode`: รหัส Asset (เช่น `R_10`)
  * `sdate`: วันที่แบบ พ.ศ. (เช่น `13-05-2569`)
* **ข้อมูลที่ตอบกลับ (JSON):** หากมีแคชไฟล์อยู่แล้ว ระบบจะส่งแคชกลับไปทันที หากไม่มีระบบจะเริ่มดึงข้อมูลดิบจาก Deriv ย้อนหลังในช่วง `01:00` ถึง `23:59` น. ของวันดังกล่าวมาวิเคราะห์ บันทึกแคช และส่งข้อมูลกลับ

---

### หมวดการแจ้งเตือนและการสื่อสารแบบ Real-time

#### • `POST /api/notify`
* **ข้อมูลที่รับเข้า (JSON Payload):**
  ```json
  {
    "message": "ข้อความที่จะส่งแจ้งเตือน Telegram"
  }
  ```
* **ข้อมูลที่ตอบกลับ (JSON):**
  ```json
  {
    "status": "success",
    "message": "ส่ง Telegram เรียบร้อย"
  }
  ```

#### • `GET /ws`
* **การเชื่อมต่อ:** การเชื่อมต่อแบบทิศทางเดียวจาก Backend ไปยัง Frontend (Server-to-Client Stream) ผ่านโปรโตคอล WebSocket
* **ข้อมูลที่สตรีมกลับมา (JSON String):** 
  * เมื่อเชื่อมต่อสำเร็จระบบจะทำการส่ง **Snapshot** ของ `candle_data` และ `bot_logs` ย้อนหลังทั้งหมดที่อยู่ในหน่วยความจำ (สูงสุด 500 logs)
  * จากนั้นจะคอยสตรีมเหตุการณ์ Real-time (เช่น `"candles_history"`, `"ohlc_update"`, `"bot_log"`, `"balance_update"`, `"trade_result"`) ไปยัง Client

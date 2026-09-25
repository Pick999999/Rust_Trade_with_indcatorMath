# การเก็บค่าการตั้งค่าการเทรดใน main.rs

## 📋 สารบัญ
- [AppState (Global Runtime State)](#appstate-global-runtime-state)
- [TradeConfig (การตั้งค่าการเทรด)](#tradeconfig-การตั้งค่าการเทรด)
- [AppConfigPayload (การตั้งค่าทั้งหมด)](#appconfigpayload-การตั้งค่าทั้งหมด)
- [TradeControl (สถานะการเทรด)](#tradecontrol-สถานะการเทรด)
- [ไฟล์การตั้งค่า](#ไฟล์การตั้งค่า)
- [API Endpoints](#api-endpoints)

---

## AppState (Global Runtime State)

### คำอธิบาย
`AppState` เป็น struct ที่เก็บ **runtime state** ของระบบ ไม่ใช่การตั้งค่าการเทรด แต่เป็นข้อมูลที่เปลี่ยนแปลงระหว่างโปรแกรมทำงาน

### โครงสร้าง

```rust
#[derive(Clone)]
pub struct AppState {
    // WebSocket broadcast channel
    pub tx: Arc<broadcast::Sender<serde_json::Value>>,
    
    // Short Term command channel
    pub cmd_tx: Arc<broadcast::Sender<serde_json::Value>>,
    
    // Long Term command channel
    pub lt_cmd_tx: Arc<broadcast::Sender<serde_json::Value>>,
    
    // Short Term bot task handle
    pub active_bot: Arc<Mutex<Option<JoinHandle<()>>>>,
    
    // Long Term bot task handle
    pub long_term_bot: Arc<Mutex<Option<JoinHandle<()>>>>,
    
    // Overtime signals สำหรับควบคุมการทำงาน
    pub overtime_signals: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    
    // Bot logs (ประวัติการทำงาน)
    pub bot_logs: Arc<Mutex<Vec<serde_json::Value>>>,
    
    // Candle data cache (แคชข้อมูล candles)
    pub candle_data: Arc<Mutex<HashMap<String, serde_json::Value>>>,
    
    // Balance ล่าสุด
    pub last_balance: Arc<Mutex<f64>>,
    
    // Long Term WebSocket status
    pub lt_ws_status: Arc<Mutex<HashMap<String, String>>>,
}
```

### คุณสมบัติ

| คุณสมบัติ | ค่า |
|-----------|-----|
| **Global** | ✅ ใช่ (ส่งผ่าน Axum State injection) |
| **Thread-safe** | ✅ ใช้ `Arc<Mutex<>>` และ `Arc<AtomicBool>` |
| **Persistent** | ❌ หายเมื่อโปรแกรมปิด |
| **เก็บใน** | RAM (Memory) |

### การใช้งาน

```rust
// ใน handler function
async fn some_handler(State(state): State<AppState>) {
    // เข้าถึง balance
    let balance = *state.last_balance.lock().await;
    
    // เข้าถึง candle data
    let candle_guard = state.candle_data.lock().await;
    if let Some(vol10_data) = candle_guard.get("vol10") {
        // ใช้งาน data
    }
    
    // ส่ง broadcast message
    let _ = state.tx.send(serde_json::json!({
        "type": "update",
        "data": "some data"
    }));
}
```

---

## TradeConfig (การตั้งค่าการเทรด)

### คำอธิบาย
`TradeConfig` เก็บการตั้งค่าที่เกี่ยวกับการเทรด เช่น Martingale, เป้าหมาย, กลยุทธ์ ฯลฯ

**สำคัญ:** การตั้งค่าเหล่านี้เก็บใน **ไฟล์ `setup/setup.json`** ไม่ใช่ตัวแปร global ในโค้ด

### โครงสร้าง

```rust
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TradeConfig {
    // เงื่อนไขหยุดเทรด ("targetMoney", "targetLot", "unlimited")
    #[serde(rename = "conditionStopTrade")]
    pub condition_stop_trade: String,
    
    // เป้าหมายเงิน (เมื่อถึงจะหยุดเทรด)
    #[serde(rename = "targetMoney")]
    pub target_money: f64,
    
    // เป้าหมาย lot (จำนวน lot ที่ต้องการเทรด)
    #[serde(rename = "targetLot")]
    pub target_lot: f64,
    
    // แจ้งเตือน Telegram
    #[serde(rename = "notifyTelegram", default)]
    pub notify_telegram: bool,
    
    // เล่นเสียงเมื่อมีเหตุการณ์
    #[serde(rename = "playSound", default)]
    pub play_sound: bool,
    
    // การตั้งค่า Martingale
    pub martingale: Martingale,
    
    // ขายอัตโนมัติเมื่อใกล้หมดเวลา
    #[serde(rename = "autoSellTimeout", default)]
    pub auto_sell_timeout: bool,
    
    // ระยะเวลาก่อนหมดเวลา (วินาที)
    #[serde(rename = "autoSellTimeoutSeconds", default = "default_timeout_seconds")]
    pub auto_sell_timeout_seconds: i32,
    
    // วิธีการซื้อ ("proposal", "contract")
    #[serde(rename = "buyMethod", default = "default_buy_method")]
    pub buy_method: String,
    
    // จำนวนแพ้สูงสุดที่ยอมรับ
    #[serde(rename = "maxLossCon", default)]
    pub max_loss_con: u32,
    
    // กลยุทธ์ที่ใช้ ("V1", "V2", "ATRSpike", "PKTrend")
    #[serde(rename = "suggestStrategy", default = "default_suggest_strategy")]
    pub suggest_strategy: String,
    
    // อัตราแลกเปลี่ยน THB/USD
    #[serde(rename = "thbRate", default = "default_thb_rate")]
    pub thb_rate: f64,
    
    // ยืมสัญญาณจากเครื่องอื่น
    #[serde(rename = "borrowSignal", default)]
    pub borrow_signal: bool,
    
    // ใช้งาน Whipsaw Zone filter
    #[serde(rename = "whipsawZone", default = "default_whipsaw_zone")]
    pub whipsaw_zone: bool,
    
    // บันทึก Track Order snapshot
    #[serde(rename = "saveTrackOrder", default)]
    pub save_track_order: bool,
}

// ค่าเริ่มต้น
fn default_buy_method() -> String { "proposal".to_string() }
fn default_timeout_seconds() -> i32 { 90 }
fn default_suggest_strategy() -> String { "V1".to_string() }
fn default_thb_rate() -> f64 { 35.0 }
fn default_whipsaw_zone() -> bool { true }
```

### ตัวอย่างไฟล์ `setup/setup.json`

```json
{
  "meta": {
    "theme": "dark",
    "version": "1.0"
  },
  "granularity": 60,
  "assets": ["vol10", "vol50", "vol75"],
  "trade": {
    "conditionStopTrade": "targetMoney",
    "targetMoney": 100,
    "targetLot": 50,
    "notifyTelegram": true,
    "playSound": true,
    "martingale": {
      "enabled": true,
      "strategy": "exponential",
      "maxLevel": 5
    },
    "autoSellTimeout": true,
    "autoSellTimeoutSeconds": 90,
    "buyMethod": "proposal",
    "maxLossCon": 5,
    "suggestStrategy": "ATRSpike",
    "thbRate": 33.5,
    "borrowSignal": false,
    "whipsawZone": true,
    "saveTrackOrder": true
  }
}
```

---

## AppConfigPayload (การตั้งค่าทั้งหมด)

### คำอธิบาย
`AppConfigPayload` เป็น struct ที่รวมการตั้งค่าทั้งหมด รวมถึง `TradeConfig`, indicators, assets, และ granularity

### โครงสร้าง

```rust
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfigPayload {
    // Metadata (theme, version, etc.)
    pub meta: Meta,
    
    // ระยะเวลา candle (วินาที)
    pub granularity: i32,
    
    // ระยะเวลาสำหรับการตั้งค่า
    #[serde(rename = "granularitySettings")]
    pub granularity_settings: i32,
    
    // รายการ assets ที่เทรด
    pub assets: Vec<String>,
    
    // การตั้งค่า indicators (EMA, BB, ATR, RSI, etc.)
    pub indicators: Indicators,
    
    // การตั้งค่า EMA
    pub ema: EmaConfig,
    
    // การตั้งค่าการเทรด (← TradeConfig อยู่ตรงนี้)
    pub trade: TradeConfig,
}
```

### การอ่านการตั้งค่า

```rust
// อ่านจากไฟล์
let config_content = fs::read_to_string("setup/setup.json")
    .expect("Failed to read setup.json");

let config: AppConfigPayload = serde_json::from_str(&config_content)
    .expect("Failed to parse setup.json");

// เข้าถึงการตั้งค่าการเทรด
println!("Strategy: {}", config.trade.suggest_strategy);
println!("Max Loss Con: {}", config.trade.max_loss_con);
println!("Assets: {:?}", config.assets);
```

---

## TradeControl (สถานะการเทรด)

### คำอธิบาย
`TradeControl` เก็บสถานะปัจจุบันของการเทรด เช่น จำนวนรอบที่เทรดไปแล้ว, สถานะ (กำลังเทรด/หยุด), เวลาเริ่มต้นจริง ฯลฯ

**เก็บใน:** `tradeData/{เดือน-ปีพศ}/{วัน-เดือน-ปีพศ}/tradeControl.json`

### โครงสร้าง

```rust
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TradeControl {
    // วันที่เทรด (format: "DD-MM-YYYY")
    #[serde(rename = "DayTrade")]
    pub day_trade: String,
    
    // จำนวนรอบที่เทรดไปแล้ว
    #[serde(rename = "totalTrade")]
    pub total_trade: u32,
    
    // สถานะ: "กำลังเทรด" หรือ "ปิดเทรดอยู่"
    #[serde(rename = "TradeStatus")]
    pub trade_status: String,
    
    // เวลาเริ่มต้นที่ตั้งไว้ (schedule)
    #[serde(rename = "startTradeTime")]
    pub start_trade_time: String,
    
    // เวลาสิ้นสุดที่ตั้งไว้ (schedule)
    #[serde(rename = "stopTradeTime")]
    pub stop_trade_time: String,
    
    // ระยะเวลาที่เทรดจริง
    #[serde(rename = "durationTrade")]
    pub duration_trade: String,
    
    // เหตุผลที่หยุด ("กดหยุดเทรด", "ถึงเป้า", etc.)
    #[serde(rename = "actionStop")]
    pub action_stop: String,
    
    // เวลาที่เริ่มเทรดจริง
    #[serde(rename = "actualStartTime", default)]
    pub actual_start_time: String,
    
    // เวลาที่หยุดเทรดจริง
    #[serde(rename = "actualStopTime", default)]
    pub actual_stop_time: String,
    
    // ใช้ Schedule หรือไม่
    #[serde(rename = "useSchedule", default)]
    pub use_schedule: bool,
}
```

### ตัวอย่างไฟล์

```json
{
  "DayTrade": "18-08-2569",
  "totalTrade": 10,
  "TradeStatus": "ปิดเทรดอยู่",
  "startTradeTime": "2026-08-18 09:00:00",
  "stopTradeTime": "2026-08-18 17:00:00",
  "durationTrade": "5h 23m 15s",
  "actionStop": "ถึงเป้าหมาย",
  "actualStartTime": "2026-08-18 09:05:30",
  "actualStopTime": "2026-08-18 14:28:45",
  "useSchedule": true
}
```

### ฟังก์ชันที่เกี่ยวข้อง

```rust
// อ่าน TradeControl
fn get_or_create_trade_control() -> TradeControl {
    let path = get_trade_control_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(tc) = serde_json::from_str::<TradeControl>(&content) {
            return tc;
        }
    }
    // สร้างใหม่ถ้าไม่มี
    TradeControl {
        day_trade: chrono::Local::now().format("%d-%m-%Y").to_string(),
        total_trade: 0,
        trade_status: "ปิดเทรดอยู่".to_string(),
        // ... ค่าเริ่มต้นอื่นๆ
    }
}

// อัพเดท TradeControl
fn update_trade_control(
    status: &str, 
    increment_trade: bool, 
    action_stop_opt: Option<&str>
) -> TradeControl {
    let mut tc = get_or_create_trade_control();
    tc.trade_status = status.to_string();
    
    if increment_trade {
        tc.total_trade += 1;
    }
    
    // อัพเดทเวลาตามสถานะ
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    if status == "กำลังเทรด" {
        tc.actual_start_time = now;
        tc.actual_stop_time = "".to_string();
        tc.duration_trade = "".to_string();
    } else if status == "ปิดเทรดอยู่" {
        tc.actual_stop_time = now.clone();
        // คำนวณระยะเวลา
        // ... (โค้ดคำนวณ duration)
        if let Some(action) = action_stop_opt {
            tc.action_stop = action.to_string();
        }
    }
    
    // บันทึกลงไฟล์
    let _ = fs::write(
        get_trade_control_path(), 
        serde_json::to_string_pretty(&tc).unwrap()
    );
    
    tc
}
```

---

## ไฟล์การตั้งค่า

### ตารางสรุป

| ไฟล์ | เก็บอะไร | API Endpoint |
|------|---------|--------------|
| `setup/setup.json` | การตั้งค่าปัจจุบัน (TradeConfig + Indicators + Assets) | `/api/setup` |
| `setup/default/setup.json` | การตั้งค่าเริ่มต้น | `/api/setup/default` |
| `tradeData/{เดือน}/{วัน}/tradeControl.json` | สถานะการเทรดของวันนั้น | `/api/tradeControl` |
| `setup/longterm_settings.json` | การตั้งค่า Long Term Trade | `/api/longterm/settings` |
| `tradeData/{เดือน}/{วัน}/ltTradeControl.json` | สถานะ Long Term Trade | `/api/longterm/schedule` |

### โครงสร้างโฟลเดอร์

```
project_root/
├── setup/
│   ├── setup.json                    # Short Term Trade Config
│   ├── default/
│   │   └── setup.json                # Default Config
│   └── longterm_settings.json        # Long Term Trade Settings
│
└── tradeData/
    └── {เดือน-ปีพศ}/                # เช่น 08-2569
        └── {วัน-เดือน-ปีพศ}/         # เช่น 18-08-2569
            ├── tradeControl.json      # Short Term status
            ├── ltTradeControl.json    # Long Term status
            └── {asset}/               # เช่น 1HZ10V
                ├── trades.json        # Trade history
                └── track_orders.json  # Order tracking
```

---

## API Endpoints

### Short Term Trade Configuration

#### `GET /api/setup`
ดึงการตั้งค่าปัจจุบัน

**Response:**
```json
{
  "meta": { ... },
  "granularity": 60,
  "assets": ["vol10", "vol50"],
  "trade": { ... },
  "indicators": { ... }
}
```

#### `POST /api/setup`
บันทึกการตั้งค่า

**Request Body:**
```json
{
  "meta": { "theme": "dark" },
  "granularity": 60,
  "assets": ["vol10"],
  "trade": {
    "suggestStrategy": "ATRSpike",
    "maxLossCon": 5
  }
}
```

**Response:**
```json
{
  "status": "success"
}
```

#### `GET /api/setup/default`
ดึงการตั้งค่าเริ่มต้น

---

### Trade Control

#### `GET /api/tradeControl`
ดึงสถานะการเทรด

**Response:**
```json
{
  "DayTrade": "18-08-2569",
  "totalTrade": 5,
  "TradeStatus": "กำลังเทรด",
  "actualStartTime": "2026-08-18 09:00:00"
}
```

#### `POST /api/tradeControl`
อัพเดทสถานะการเทรด (ใช้ภายในระบบ)

---

### Long Term Trade

#### `GET /api/longterm/settings`
ดึงการตั้งค่า Long Term

#### `POST /api/longterm/settings`
บันทึกการตั้งค่า Long Term

#### `GET /api/longterm/schedule`
ดึงการตั้งค่า Schedule

#### `POST /api/longterm/schedule`
บันทึกการตั้งค่า Schedule

---

## การเข้าถึงจาก Frontend

### JavaScript Example

```javascript
// ดึงการตั้งค่า
async function loadSetup() {
  const response = await fetch('/api/setup');
  const setup = await response.json();
  
  console.log('Strategy:', setup.trade.suggestStrategy);
  console.log('Assets:', setup.assets);
  console.log('Max Loss Con:', setup.trade.maxLossCon);
  
  return setup;
}

// บันทึกการตั้งค่า
async function saveSetup(config) {
  const response = await fetch('/api/setup', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(config)
  });
  
  const result = await response.json();
  console.log('Save result:', result);
}

// ดึงสถานะการเทรด
async function getTradeControl() {
  const response = await fetch('/api/tradeControl');
  const tc = await response.json();
  
  console.log('Trade Status:', tc.TradeStatus);
  console.log('Total Trade:', tc.totalTrade);
  
  return tc;
}
```

---

## สรุปสำคัญ

### ❌ ไม่มีตัวแปร Global สำหรับการตั้งค่า

ระบบ **ไม่ได้เก็บการตั้งค่าการเทรดในตัวแปร global แบบ `static`** เหตุผล:
- ❌ ไม่สามารถแก้ไขค่า static ได้ง่ายใน runtime
- ❌ ต้องใช้ `lazy_static!` หรือ `OnceCell` ซึ่งซับซ้อน
- ✅ การใช้ไฟล์ JSON ทำให้แก้ไขได้ง่าย
- ✅ การตั้งค่ายังคงอยู่หลังปิดโปรแกรม (persistent)

### ✅ การตั้งค่าเก็บในไฟล์ JSON

```
setup/setup.json  →  อ่านทุกครั้งที่ใช้งาน
```

### ✅ Runtime State เก็บใน AppState

```rust
AppState (Global, Thread-safe)
├── tx (WebSocket broadcast)
├── active_bot (Bot handle)
├── bot_logs (Logs)
├── candle_data (Cache)
└── last_balance (Balance)
```

### ✅ ใช้ Axum State Injection

```rust
async fn handler(State(state): State<AppState>) {
    // เข้าถึง global state
    let balance = *state.last_balance.lock().await;
}
```

---

## เอกสารอ้างอิง

- [Axum State](https://docs.rs/axum/latest/axum/extract/struct.State.html)
- [Arc<Mutex<T>>](https://doc.rust-lang.org/std/sync/struct.Arc.html)
- [Serde JSON](https://docs.rs/serde_json/latest/serde_json/)
- [Tokio Broadcast](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html)

---

**เอกสารนี้อัพเดทล่าสุด:** 28 สิงหาคม 2569  
**Version:** 1.0  
**ผู้เขียน:** PK Deriv Trade Documentation Team

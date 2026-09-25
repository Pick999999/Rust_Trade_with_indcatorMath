# แผนงานการปรับปรุง: แยกเงื่อนไขการขาย (Exit Strategy) ตาม Trade Code (SM, SML, SL)

เพื่อให้ผู้ใช้สามารถกำหนด Exit Strategy (Target Profit, Sell on Next Cross, Duration Expiry) และเป้าหมายกำไร (Target Profit Percent) แยกต่างหากสำหรับแต่ละ Trade Code ได้ (SM, SML, SL) เราจะดำเนินการปรับปรุงตาม 3 ส่วนหลักดังนี้:

## 1. การปรับปรุงใน `long_term_trade.html` (UI)
ปัจจุบันมีชุดตัวเลือก Exit Strategy เพียงชุดเดียว จะต้องเปลี่ยนเป็นการแยกชุดตัวเลือกสำหรับแต่ละเงื่อนไข (Trade Code):

- **ลบ:** ชุด `name="ltExitStrategy"` และ `id="ltTargetProfitPercent"` เดิม
- **เพิ่ม:** ตัวเลือก Exit Strategy และ Target Profit แยกสำหรับแต่ละ Trade Code ภายในหน้า HTML:
  - **SM (EMA Short ✕ EMA Medium):**
    - Radio Buttons: `name="smExitStrategy"` (value: `targetProfit`, `nextCross`, `durationExpiry`)
    - Input: `id="smTargetProfitPercent"`
  - **SML (EMA Long ✕ Short & Medium):**
    - Radio Buttons: `name="smlExitStrategy"` (value: `targetProfit`, `nextCross`, `durationExpiry`)
    - Input: `id="smlTargetProfitPercent"`
  - **SL (EMA Short ✕ EMA Long):**
    - Radio Buttons: `name="slExitStrategy"` (value: `targetProfit`, `nextCross`, `durationExpiry`)
    - Input: `id="slTargetProfitPercent"`

*ข้อเสนอแนะ UI:* นำส่วนตั้งค่า Exit Strategy เข้าไปใส่ไว้ใต้ Condition Card แต่ละอัน หรือสร้าง Section ใหม่ที่แบ่งเป็น 3 คอลัมน์สำหรับ SM, SML และ SL เพื่อให้เห็นชัดเจนว่าแยกจากกัน

## 2. การปรับปรุงในส่วนของ Javascript (ในไฟล์ `long_term_trade.html`)
ต้องปรับปรุง script เพื่อดึงค่าที่แยกกันเหล่านี้และส่งไปให้ Rust:

**ตัวอย่างการดึงข้อมูล:**
```javascript
const smExitStrategy = document.querySelector('input[name="smExitStrategy"]:checked')?.value || 'targetProfit';
const smTargetProfit = parseFloat(document.getElementById('smTargetProfitPercent').value) || 10.0;

const smlExitStrategy = document.querySelector('input[name="smlExitStrategy"]:checked')?.value || 'targetProfit';
const smlTargetProfit = parseFloat(document.getElementById('smlTargetProfitPercent').value) || 10.0;

const slExitStrategy = document.querySelector('input[name="slExitStrategy"]:checked')?.value || 'targetProfit';
const slTargetProfit = parseFloat(document.getElementById('slTargetProfitPercent').value) || 10.0;
```

**ปรับปรุง Payload (ส่งไปที่ `/api/longterm/schedule`):**
แทนที่จะส่ง `exit_strategy` และ `target_profit` เดียว ให้ส่งแยกกัน:
```javascript
const schedulePayload = {
    // ข้อมูลเดิม
    assets, granularity, duration, conditions, amount, max_orders, start_time, stop_time, use_schedule,
    // ข้อมูลที่เพิ่มใหม่
    sm_exit_strategy: smExitStrategy,
    sm_target_profit: smTargetProfit,
    sml_exit_strategy: smlExitStrategy,
    sml_target_profit: smlTargetProfit,
    sl_exit_strategy: slExitStrategy,
    sl_target_profit: slTargetProfit
};
```
*(และต้องปรับปรุงส่วนที่ดึงค่าคืนจาก API มาอัปเดตบนหน้า UI ด้วยเมื่อโหลดหน้า)*

## 3. การปรับปรุงในฝั่ง Rust (`src/main.rs` และ `src/long_term.rs`)

### 3.1 ปรับแก้ Struct เพื่อรองรับตัวแปรใหม่ใน `src/main.rs`
เราต้องเปลี่ยนโครงสร้างของ `LongTermSetup` และ `LongTermScheduleRequest`

```rust
// ตัวอย่าง Struct ที่ต้องแก้ใน src/main.rs
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LongTermSetup {
    pub assets: Vec<String>,
    pub granularity: i32,
    pub duration: i32,
    pub conditions: Vec<String>,
    pub amount: f64,
    pub auto_trade: bool,
    #[serde(default = "default_max_orders")]
    pub max_orders: i32,
    
    // --- เปลี่ยน 2 ฟิลด์เดิม เป็น 6 ฟิลด์ใหม่ ---
    pub sm_exit_strategy: String,
    pub sm_target_profit: f64,
    pub sml_exit_strategy: String,
    pub sml_target_profit: f64,
    pub sl_exit_strategy: String,
    pub sl_target_profit: f64,
}

#[derive(Deserialize)]
pub struct LongTermScheduleRequest {
    // ... ฟิลด์เดิม ...
    pub sm_exit_strategy: Option<String>,
    pub sm_target_profit: Option<f64>,
    pub sml_exit_strategy: Option<String>,
    pub sml_target_profit: Option<f64>,
    pub sl_exit_strategy: Option<String>,
    pub sl_target_profit: Option<f64>,
}
```
เราจะต้องแก้ไข `get_or_create_lt_setup`, `handle_post_longterm_schedule`, และ `handle_get_longterm_schedule` ให้รับและบันทึกค่าเหล่านี้แทนค่าเดิม และตั้ง default ให้ครบถ้วน

### 3.2 ปรับแก้ฟังก์ชันเปิดออเดอร์ใน `src/long_term.rs` (เพื่อให้ตรวจสอบตาม trade_code)
เมื่อบอทตรวจพบสัญญาณ (Entry Signal) และกำลังจะส่งคำสั่งซื้อ (Sale Order) จะต้องเช็คว่า `trade_code` คืออะไร แล้วดึง Exit Strategy ของ Code นั้นๆ มาตั้งค่าให้กับ `ActiveOrder` ใหม่

```rust
// ตัวอย่าง Logic ที่ต้องปรับแก้ใน src/long_term.rs ตรงส่วนเปิด order
// ---------------------------------------------------------------------------------
let setup = crate::get_or_create_lt_setup();

// กำหนด Exit Strategy ให้ Order โดยแยกตาม Trade Code ที่เข้าเทรด
let (order_exit_strategy, order_target_profit) = match trade_code.as_str() {
    "SM" => (setup.sm_exit_strategy.clone(), setup.sm_target_profit),
    "SML" => (setup.sml_exit_strategy.clone(), setup.sml_target_profit),
    "SL" => (setup.sl_exit_strategy.clone(), setup.sl_target_profit),
    _ => ("targetProfit".to_string(), 10.0), // Default fallback
};

// หลังจากนั้นก็นำไปใส่ให้ตัวแปรใน ActiveOrder
let active_order = ActiveOrder {
    // ... ฟิลด์อื่นๆ
    exit_strategy: order_exit_strategy,
    target_profit: order_target_profit,
    trade_code: trade_code.clone(),
    // ...
};
```

เมื่อแก้ไขตามขั้นตอนนี้แล้ว จังหวะเช็คกำไร (Exit checking process) โค้ดเดิมที่ใช้ `order.exit_strategy` และ `order.target_profit` (เช่นในลูป WebSocket) จะทำงานตามปกติโดยไม่ต้องแก้ไขเพิ่มมากนัก เพราะเราฝังค่าที่แยกตาม trade_code ไว้ที่ออเดอร์ตั้งแต่ตอนเปิดเรียบร้อยแล้ว

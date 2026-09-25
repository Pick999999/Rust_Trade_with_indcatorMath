# สรุปฟีเจอร์และการทำงานของ `full_analysis_ver2.rs`

ไฟล์ [`src/full_analysis_ver2.rs`](../src/full_analysis_ver2.rs) คือ Engine หลักสำหรับการวิเคราะห์ข้อมูลแท่งเทียน (Full Technical Analysis Engine) ประสิทธิภาพสูงของระบบ ออกแบบมาเพื่อประมวลผลข้อมูลราคาแท่งเทียนแบบครบวงจร ทั้งการคำนวณ Indicator ทางเทคนิค, ตรวจจับพฤติกรรมราคา (Price Action & Anatomy), การวิเคราะห์โครงสร้างตลาด (SMC / Range / Volatility) และการจำแนกแนวโน้มด้วย `pkTrend`

---

## 1. หมวดหมู่ฟีเจอร์หลัก (Key Feature Groups)

### 📈 1.1 Triple EMA & Trend Engine (ระบบเส้นค่าเฉลี่ย 3 เส้น)
* **รองรับ 3 เส้นค่าเฉลี่ย:** Short, Medium, Long (กำหนดชนิดแยกอิสระได้: EMA, HMA, WMA, SMA)
* **การคำนวณ Slope & ความชัน:** คำนวณค่าความชันของแต่ละเส้น พร้อมตรวจสอบสถานะราคาพักตัว/วิ่งระนาบ (`ema_short_flat`, `ema_medium_flat`, `ema_long_flat`) ผ่านเกณฑ์ Threshold
* **การระบุทิศทางและการเลี้ยว:**
  * ทิศทาง: `Up` / `Down`
  * การกลับตัว/เลี้ยว: `TurnUp` / `TurnDown` / `-`
* **ความสัมพันธ์ระหว่างเส้น (Convergence / Divergence):**
  * วิเคราะห์การบีบเข้าหากัน (Convergence) หรือแยกห่างออกจากกัน (Divergence) ของ Short-Medium และ Medium-Long
  * วัดระยะห่าง (Gap) และแจ้งเตือนเมื่อเกิด Gap ที่มีนัยสำคัญ
* **ระบบตรวจจับการตัดกัน (Crossover Engine):**
  * `ema_cut_position`: Short ตัด Medium (`CrossUp`, `CrossDown`, `-`)
  * `ema_cut_long_type`: Medium ตัด Long (`CrossUp`, `CrossDown`, `-`)
  * `ema_cut_short_long_type`: Short ตัด Long (`CrossUp`, `CrossDown`, `-`)
  * `ema_cut_all_type`: การเรียงตัวและตัดกันครบทั้ง 3 เส้น (`AllCrossUp`, `AllCrossDown`, `-`)
* **Age Cut Code Tracking:**
  * `age_cut_candle_code12`: ติดตามอายุแท่งเทียนนับจากการตัดกันของ EMA 1-2 พร้อมบอกสถานะทิศทาง เช่น `5-CrossUp-Up:Up:Down`
  * `age_cut_candle_code123`: ติดตามอายุแท่งเทียนนับจากการตัดกันครบ 3 เส้น EMA 1-2-3

---

### 📊 1.2 Oscillators & Momentum Indicators (ตัวชี้วัดโมเมนตัมและความผันผวน)
* **MACD Subsystem:** คำนวณ MACD Line (`macd_12`), Signal Line (`macd_23`) พร้อมเก็บค่าประวัติแท่งก่อนหน้า (`previous_macd_12`, `previous_macd_23`)
* **RSI (Relative Strength Index):** คำนวณ RSI ความเร็วสูงแบบ Zero-Copy (14 Periods)
* **ADX (Average Directional Index):** คำนวณความแข็งแกร่งของเทรนด์ผ่าน True Range (+DM, -DM, DX, Smoothed ADX)
* **Choppiness Index (CI):** วัดสภาวะตลาดว่าเป็นเทรนด์ชัดเจน หรือเป็นตลาดสับขาหลอก/แกว่งตัวไซด์เวย์ (Choppy Market)

---

### 📉 1.3 Volatility & Band Systems (ระบบแถบความผันผวน)
* **Stateful ATR & Spike Filter:**
  * คำนวณ ATR แบบ Real-time Stateful (Period 3 และ Standard ATR)
  * ตรวจจับแท่งเทียนขนาดใหญ่ผิดปกติ (`is_abnormal_candle`, `is_abnormal_atr`)
  * ตรวจจับแท่ง Spike ที่แกว่งเกินเกณฑ์ ATR Multiplier (`is_atr`)
* **Bollinger Bands:**
  * ค่า Upper, Middle, Lower
  * คำนวณ Bandwidth % (`bb_bandwidth`)
  * ตรวจจับภาวะบีบตัวของราคา (`is_bb_squeeze`) ย้อนหลัง 20 แท่ง
  * ระบุตำแหน่งราคาเทียบกับ Band (`bb_position`: `AboveUpper`, `NearUpper`, `BelowLower`, `NearLower`)

---

### 🕯️ 1.4 Candle Anatomy & Alternating Patterns (กายวิภาคแท่งเทียนและรูปแบบสลับสี)
* **Candle Anatomy:** คำนวณขนาดและสัดส่วนของ Body, Upper Wick, Lower Wick ทั้งในรูปค่าราคาจริงและเปอร์เซ็นต์ (`u_wick_percent`, `body_percent`, `l_wick_percent`)
* **Alternating Pattern Detection (แท่งสลับสี เขียว-แดง):**
  * ตรวจจับแท่งเทียนสลับสีที่มีขนาด Body ใกล้เคียงกัน (`is_alternating_pattern`)
  * บันทึกความยาวลูกโซ่ของการสลับสีต่อเนื่อง (`alternating_sequence_length`)
  * แจ้งเตือนจุดทริกเกอร์เมื่อสลับสีตั้งแต่ 3 แท่งขึ้นไป (`is_alternating_trigger`)
  * แจ้งเตือนกรณีแท่งสลับสีที่มีการกระชากตัวแรงผิดปกติ (`is_alternating_spike`)

---

### 🎯 1.5 PK Trend Detection Engine (`pkTrend`) *(New)*
* รวมโมดูล [`pkDetectTrend.rs`](../src/pkDetectTrend.rs) เข้าเป็นหนึ่งในผลลัพธ์ของแต่ละแท่งเทียน
* **ตรวจจับรูปแบบแท่งเทียน 20 Cases (6 หมวดหมู่):**
  * `System`: ข้อมูลไม่เพียงพอ (`INSUFFICIENT_DATA`)
  * `Up`: ยืนยันขาขึ้น (`CONFIRMED_UP`)
  * `Down`: ยืนยันขาลง (`CONFIRMED_DOWN`)
  * `Rejected`: กับดักและแรงปฏิเสธราคา (`STRONG_BULL_TRAP`, `WEAK_UPPER_WICK`, `FAILED_FOLLOWTHROUGH_UP`, `STRONG_BEAR_TRAP`, `WEAK_LOWER_WICK`, `FAILED_FOLLOWTHROUGH_DOWN`)
  * `Sideways`: สภาวะพักตัวและไร้ทิศทาง (`INSIDE_BAR`, `DOJI_INDECISION`, `MIXED_SIGNAL`, `FLAT_RESISTANCE_TEST`, `NO_CLEAR_STRUCTURE`)
  * `Spike`: ความผันผวนสูงจาก ATR (`SPIKE_BULL_TRAP`, `SPIKE_BEAR_TRAP`, `SPIKE_CONTINUATION_UP`, `SPIKE_CONTINUATION_DOWN`, `SPIKE_WHIPSAW`, `SPIKE_NO_CLEAR_DIRECTION`)
* **ข้อมูลเชิงลึกใน `pkTrend`:**
  * `trend`: หมวดใหญ่ (`UpTrend`, `DownTrend`, `Rejected`, `Sideways`)
  * `caseCode` & `caseDesc`: รหัสย่อและชื่อเต็ม
  * `description`: คำอธิบายพฤติกรรมราคาภาษาไทย
  * `closePosition` & `bodyRatio`: ตำแหน่งปิดในแท่งและสัดส่วนเนื้อเทียน

---

### 🏛️ 1.6 Advanced Market Concepts (SMC & Range Detector)
* **Range Detector:** ตรวจจับกรอบราคาไซด์เวย์ คำนวณ `range_top`, `range_bottom`, `range_avg`, และสถานะ `range_state`
* **Smart Money Concepts (SMC):**
  * โครงสร้างราคา (Market Structure: BOS / CHoCH)
  * จุด Swing High / Swing Low
  * โซน Premium / Discount และ Equilibrium
  * Swing Trend & Internal Trend

---

### ⚡ 1.7 Tick Volatility & Trade Signal Integration
* **Tick Volatility:** วิเคราะห์ปริมาณ Tick, สัดส่วน Buy/Sell Tick, การเคลื่อนไหวเฉลี่ยและสูงสุด, Volatility Clustering
* **Strategy & Suggest Color Output:** รองรับการเชื่อมต่อกับระบบตัดสินใจ `get_action.rs` เพื่อสรุป `suggest_color`, `status_code`, `status_desc`, `win_con`, `loss_con`

---

## 2. โครงสร้างข้อมูลผลลัพธ์ (`FullAnalysisResult` JSON Schema)

```json
{
  "index": 120,
  "assetCode": "1HZ10V",
  "candletime": 1723700000,
  "candletime_display": "2026-08-15 15:00:00",
  "open": 100.50,
  "high": 105.80,
  "low": 100.10,
  "close": 105.20,
  "color": "green",
  "pip_size": 4.70,

  "ema_short_value": 104.80,
  "ema_short_direction": "Up",
  "ema_short_turn_type": "-",
  "ema_short_flat": "n",

  "ema_medium_value": 103.50,
  "ema_medium_direction": "Up",
  "ema_long_value": 101.20,
  "ema_long_direction": "Up",

  "macd_12": 1.30,
  "macd_23": 2.30,
  "rsi_value": 64.50,
  "adx_value": 28.30,
  "choppy_indicator": 42.10,

  "bb_values": {
    "upper": 106.50,
    "middle": 103.50,
    "lower": 100.50
  },
  "bb_position": "NearUpper",
  "bb_bandwidth": 5.80,
  "is_bb_squeeze": false,

  "atrValue": 1.25,
  "is_atr": false,
  "is_abnormal_candle": false,

  "u_wick": 0.60,
  "u_wick_percent": 10.53,
  "body": 4.70,
  "body_percent": 82.46,
  "l_wick": 0.40,
  "l_wick_percent": 7.01,

  "ema_cut_position": "-",
  "ema_cut_all_type": "AllCrossUp",
  "ageCutCandleCode12": "8-CrossUp-Up:Up:Up",
  "ageCutCandleCode123": "4-CrossUp-Up:Up:Up",

  "pkTrend": {
    "trend": "UpTrend",
    "caseCode": "UP-CONFIRM",
    "caseDesc": "CONFIRMED_UP",
    "group": "Up",
    "description": "ทำจุดสูงใหม่ ปิดสูงกว่าแท่งก่อนหน้า และปิดในโซนบนของแท่งตัวเอง ยืนยันแรงซื้อยังควบคุมตลาด",
    "isSpike": false,
    "extremeTrend": "UpTrend",
    "closePosition": 0.89,
    "bodyRatio": 0.82
  },

  "smc": {
    "swing_trend": "bullish",
    "internal_trend": "bullish",
    "premium_discount_zone": { ... }
  },
  "range_detector": {
    "in_range": false,
    "range_state": "Trending"
  },
  "is_alternating_pattern": false,
  "alternating_sequence_length": 0
}
```

---

## 3. สถาปัตยกรรมการรัน (Execution Targets)

1. **Native Rust HTTP API (Axum):**
   * ประมวลผลบน Server ฝั่ง Rust รองรับการประมวลผลข้อมูลขนาดใหญ่แบบ Parallel (Multi-threaded)
2. **WebAssembly (WASM Support):**
   * มีฟังก์ชัน `run_analysis_wasm(json_payload, config_json)` ให้เบราว์เซอร์หรือ Node.js เรียกใช้ Engine ตัวเดียวกันได้ตรงๆ โดยไม่ต้องพึ่งพา Server API

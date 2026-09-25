# 📊 FullAnalysisEngine.js

## 🎯 คำอธิบาย

**FullAnalysisEngine** เป็น Pure JavaScript Class สำหรับวิเคราะห์ข้อมูลแท่งเทียน (Candlestick) และคำนวณ Technical Indicators แบบครบวงจร โดยแปลงมาจาก **src/full_analysis_ver2.rs** ในโปรเจค Rust turbo-indicators

### ✨ คุณสมบัติหลัก

- ✅ **Pure JavaScript** - ไม่ต้องพึ่ง Library ภายนอก
- ✅ **รองรับ Indicators หลากหลาย** - EMA, SMA, HMA, WMA, MACD, RSI, ADX, ATR, BB, CI
- ✅ **EMA Cross Detection** - ตรวจจับ Cross Signals แบบละเอียด
- ✅ **Range Detector** - ตรวจจับกรอบ Range และ Breakout
- ✅ **Alternating Pattern** - ตรวจจับรูปแบบแท่งสลับสี
- ✅ **SMC Analysis** - Smart Money Concepts (Premium/Discount Zones)
- ✅ **Customizable** - ปรับค่า Parameters ได้ตามต้องการ
- ✅ **Performance** - ประมวลผล 100 แท่งใน ~10-20ms

---

## 📦 การติดตั้ง

### แบบที่ 1: ใช้ใน Browser

```html
<script src="./FullAnalysisEngine.js"></script>
<script>
    const engine = new FullAnalysisEngine();
</script>
```

### แบบที่ 2: ใช้ใน Node.js

```javascript
const FullAnalysisEngine = require('./FullAnalysisEngine.js');
const engine = new FullAnalysisEngine();
```

---

## 🚀 การใช้งานพื้นฐาน

### ตัวอย่างที่ 1: การใช้งานแบบง่าย

```javascript
// 1. สร้าง Instance
const engine = new FullAnalysisEngine();

// 2. เตรียมข้อมูลแท่งเทียน
const candles = [
    { epoch: 1704067200, open: 100.5, high: 101.2, low: 100.1, close: 100.8 },
    { epoch: 1704067260, open: 100.8, high: 101.5, low: 100.6, close: 101.3 },
    { epoch: 1704067320, open: 101.3, high: 102.0, low: 101.0, close: 101.8 },
    // ... เพิ่มแท่งเทียนต่อไป (แนะนำอย่างน้อย 60-100 แท่ง)
];

// 3. วิเคราะห์ (ใช้ค่า default)
const results = engine.performAnalysis(candles);

// 4. ใช้ผลลัพธ์
results.forEach(candle => {
    console.log(`Time: ${candle.candletime_display}`);
    console.log(`Close: ${candle.close}`);
    console.log(`EMA Short: ${candle.ema_short_value.toFixed(3)}`);
    console.log(`EMA Cross: ${candle.ema_cut_all_type}`);
    console.log(`RSI: ${candle.rsi_value.toFixed(2)}`);
    console.log(`ADX: ${candle.adx_value.toFixed(2)}`);
    console.log('---');
});
```

### ตัวอย่างที่ 2: การใช้งานแบบ Custom Config

```javascript
const engine = new FullAnalysisEngine();

// กำหนดค่า Config เอง
const config = {
    ema: {
        short: { period: 12, type: 'ema' },    // EMA 12
        medium: { period: 26, type: 'ema' },   // EMA 26
        long: { period: 50, type: 'hma' }      // HMA 50
    },
    indicators: {
        adxPeriod: 14,      // ADX Period
        atrPeriod: 10,      // ATR Period
        atrMulti: 1.5,      // ATR Multiplier
        bbPeriod: 20,       // Bollinger Bands Period
        ciPeriod: 14,       // Choppiness Index Period
        smcPeriod: 50       // SMC/RSI Period
    }
};

const results = engine.performAnalysis(candles, config, 'R_10');
```

### ตัวอย่างที่ 3: ใช้งานกับ Thresholds

```javascript
const engine = new FullAnalysisEngine();

// กำหนดค่า Thresholds สำหรับ Asset เฉพาะ
engine.setThresholds(
    0.0001,  // flatThreshold - EMA แบนหรือไม่
    0.0005,  // macdGapValue - MACD Gap เล็กน้อย
    0.5,     // altCandleAtrMultiplier
    1.5      // altCandleSpikeMultiplier
);

const results = engine.performAnalysis(candles, null, 'R_10');
```

---

## 📋 รูปแบบข้อมูล Input

### Candle Format
```javascript
{
    epoch: 1704067200,      // Unix timestamp (วินาที)
    open: 100.5,            // ราคาเปิด
    high: 101.2,            // ราคาสูงสุด
    low: 100.1,             // ราคาต่ำสุด
    close: 100.8            // ราคาปิด
}
```

### Config Format
```javascript
{
    ema: {
        short: { period: 9, type: 'ema' },      // ประเภท: 'ema', 'sma', 'hma', 'wma'
        medium: { period: 21, type: 'ema' },
        long: { period: 50, type: 'ema' }
    },
    indicators: {
        adxPeriod: 14,      // ADX Period (แนะนำ 14)
        atrPeriod: 7,       // ATR Period (แนะนำ 7-14)
        atrMulti: 1.3,      // ATR Multiplier สำหรับตรวจจับแท่งผิดปกติ
        bbPeriod: 20,       // Bollinger Bands Period (แนะนำ 20)
        ciPeriod: 14,       // Choppiness Index Period (แนะนำ 14)
        smcPeriod: 50       // RSI/SMC Period (แนะนำ 14-50)
    }
}
```

---

## 📊 ข้อมูล Output Fields

### 1. ข้อมูลพื้นฐาน (Basic Info)
- `index` - ลำดับแท่งเทียน
- `candletime` - Unix timestamp
- `candletime_display` - เวลาแสดงผล (YYYY-MM-DD HH:MM:SS)
- `open`, `high`, `low`, `close` - ราคา OHLC
- `color` - สีแท่งเทียน ("green" / "red" / "equal")
- `pip_size` - ขนาด Pip

### 2. EMA (Exponential Moving Average)
- `ema_short_value`, `ema_medium_value`, `ema_long_value` - ค่า EMA
- `ema_short_direction`, `ema_medium_direction`, `ema_long_direction` - ทิศทาง ("Up" / "Down")
- `ema_short_turn_type` - จุดเปลี่ยนทิศ ("TurnUp" / "TurnDown" / "-")
- `ema_short_slope_value` - ความชัน EMA
- `ema_short_flat` - EMA แบนหรือไม่ ("y" / "n")

### 3. EMA Cross Signals ⭐ สำคัญ!
- `ema_cut_position` - Short กับ Medium Cross ("CrossUp" / "CrossDown" / "-")
- `ema_cut_long_type` - Medium กับ Long Cross
- `ema_cut_short_long_type` - Short กับ Long Cross
- `ema_cut_all_type` - ทั้ง 3 เส้นเรียงกัน ("AllCrossUp" / "AllCrossDown" / "-")
- `ema_above` - Short vs Medium ("ShortAbove" / "MediumAbove")
- `ema_long_above` - Medium vs Long

### 4. MACD
- `macd_12` - MACD Line (EMA 12-26)
- `macd_23` - Signal Line (EMA 9 of MACD)
- `ema_convergence_type` - การลู่เข้า ("convergence" / "divergence")

### 5. Technical Indicators
- `rsi_value` - RSI (0-100)
- `adx_value` - ADX (แรงเทรนด์)
- `choppy_indicator` - Choppiness Index (ตลาดสับ)
- `atr_value` - ATR (ความผันผวน)

### 6. Bollinger Bands
- `bb_values` - { upper, middle, lower }
- `bb_position` - ตำแหน่ง ("AboveUpper" / "NearUpper" / "NearLower" / "BelowLower")
- `bb_bandwidth` - ความกว้าง BB (%)
- `is_bb_squeeze` - BB บีบแคบ (true/false)

### 7. Candle Anatomy
- `body` - ขนาดตัวเทียน
- `u_wick` - หางบน
- `l_wick` - หางล่าง
- `body_percent`, `u_wick_percent`, `l_wick_percent` - % ของ range
- `is_abnormal_candle` - แท่งผิดปกติ (true/false)

### 8. Range Detector
- `range_detector.in_range` - อยู่ในกรอบ Range (true/false)
- `range_detector.range_top`, `range_bottom` - ราคากรอบ
- `range_detector.range_state` - สถานะ ("unbroken" / "up" / "down")

### 9. Alternating Pattern
- `is_alternating_pattern` - เป็นรูปแบบสลับสี (true/false)
- `alternating_sequence_length` - ความยาวลำดับ
- `is_alternating_trigger` - Trigger สัญญาณ (>= 3 แท่ง)
- `is_alternating_spike` - มี Spike ผิดปกติ

### 10. SMC (Smart Money Concepts)
- `smc.swing_trend` - เทรนด์หลัก ("bullish" / "bearish")
- `smc.internal_trend` - เทรนด์ภายใน
- `smc.premium_discount_zone` - โซน Premium/Discount

---

## 🎯 กรณีการใช้งาน (Use Cases)

### 1. Trading Bot - EMA Cross Strategy
```javascript
const engine = new FullAnalysisEngine();
const results = engine.performAnalysis(candles);

// ตรวจจับสัญญาณ Entry
const lastCandle = results[results.length - 1];

if (lastCandle.ema_cut_all_type === 'AllCrossUp') {
    console.log('🟢 BUY Signal - All EMAs crossed up!');
    // เปิด CALL Order
}

if (lastCandle.ema_cut_all_type === 'AllCrossDown') {
    console.log('🔴 SELL Signal - All EMAs crossed down!');
    // เปิด PUT Order
}
```

### 2. Backtesting System
```javascript
const engine = new FullAnalysisEngine();
const results = engine.performAnalysis(historicalCandles);

let wins = 0, losses = 0;

results.forEach((candle, i) => {
    if (candle.ema_cut_position === 'CrossUp') {
        // จำลองเปิด Order และตรวจสอบผล
        const nextCandle = results[i + 1];
        if (nextCandle && nextCandle.close > candle.close) {
            wins++;
        } else {
            losses++;
        }
    }
});

console.log(`Win Rate: ${(wins / (wins + losses) * 100).toFixed(2)}%`);
```

### 3. Real-time Chart Analysis
```javascript
const engine = new FullAnalysisEngine();
let candleBuffer = [];

// เมื่อได้รับแท่งเทียนใหม่
function onNewCandle(candle) {
    candleBuffer.push(candle);
    
    // เก็บแค่ 100 แท่งล่าสุด
    if (candleBuffer.length > 100) {
        candleBuffer.shift();
    }
    
    // วิเคราะห์
    const results = engine.performAnalysis(candleBuffer);
    const latest = results[results.length - 1];
    
    // แสดงผลบน Chart
    updateChart(latest);
}
```

---

## ⚡ Performance Tips

1. **แนะนำจำนวนแท่งเทียน:** อย่างน้อย 60-100 แท่ง สำหรับ Indicators ที่ต้องใช้ Period ยาว
2. **Reuse Instance:** สร้าง `new FullAnalysisEngine()` ครั้งเดียว แล้วใช้ซ้ำ
3. **Limit Data:** ถ้ามีข้อมูลเยอะ ให้เก็บแค่ที่จำเป็น (เช่น 200 แท่งล่าสุด)
4. **Worker Thread:** ถ้าต้องประมวลผลหนัก ใช้ Web Worker

---

## 📌 หมายเหตุ

- **EMA Period** ที่แนะนำ: Short (9-12), Medium (21-26), Long (50)
- **Moving Average Types:** `ema`, `sma`, `hma`, `wma`
- **Minimum Candles:** ควรมีอย่างน้อย 60 แท่งสำหรับ Long EMA (50)
- **Time Zone:** Output แสดงเวลาตาม Local Time ของระบบ

---

## 🔗 ไฟล์ที่เกี่ยวข้อง

- `FullAnalysisEngine.js` - Class หลัก
- `FullAnalysisEngine.example.html` - ตัวอย่างการใช้งานแบบ Interactive
- `FullAnalysisEngine.README.md` - คู่มือนี้

---

## 📝 License

MIT License - แปลงมาจาก Rust turbo-indicators project

---

## 🤝 Contributing

หากพบ Bug หรือต้องการปรับปรุง สามารถแจ้งได้ที่ Repository หลัก

---

**Happy Coding! 🚀**

# วิเคราะห์สาเหตุ Loss Streak 6-7 ครั้ง ของกลยุทธ์ FTB

> วันที่วิเคราะห์: 15 มิถุนายน 2569  
> กลยุทธ์: FTB (Follow Trend B)  
> Asset: R_100 (Volatility 100 Index)  
> Schedule Trade No: #27

---

## สรุปปัญหา

| รายการ | รายละเอียด |
|--------|-----------|
| กลยุทธ์ | FTB — ดู EMA 3 เส้น (Short+Medium+Long) ถ้าทิศตรงกัน → เทรด |
| Noise Filter | เปิดทั้ง 3 (Alternate + Flat Case4 + Gap) |
| Loss Streak | 7 ครั้งต่อเนื่อง ($1→$2→$4→$8→$17→$32→$70) |
| noiseCode ทุกแท่ง | `none` — Noise Filter ไม่ block แม้แต่แท่งเดียว |
| ผลเสียหาย | -$1,737.51 ก่อนจะ Win คืนมา +$64.55 |

---

## 🔍 สาเหตุหลัก 3 ข้อ

### 1. FTB เข้าเทรดทุกแท่ง — ไม่ต้องรอ ATR Spike

**ไฟล์**: `src/deriv.rs` บรรทัด 796-802

```rust
let should_trade = if state.in_martingale {
    true
} else if current_config.trade.suggest_strategy == "FTA" || current_config.trade.suggest_strategy == "FTB" {
    true    // ← FTB ข้ามการเช็ค is_atr ไปเลย!
} else {
    prev.is_atr  // strategy อื่นต้องรอ ATR Spike
};
```

**ผลกระทบ**: FTB เทรดทุกแท่งเทียนใหม่ ไม่ว่าตลาดจะผันผวนหรือไม่  
ถ้า EMA ทั้ง 3 ชี้ทิศเดียวกัน → ระบบจะเทรดติดต่อกันทุกนาที ไม่มีตัวกรองว่าแท่งนั้น "แรง" พอหรือยัง

---

### 2. FTB ไม่พิจารณาความแรงของ Trend — ดูแค่ทิศทาง

**ไฟล์**: `src/get_action.rs` บรรทัด 172-184

```rust
pub fn get_suggest_color_ftb(analysis: &FullAnalysisResult, _loss_con: u32) -> String {
    if analysis.ema_short_direction != analysis.ema_medium_direction { return "idle"; }
    if analysis.ema_short_direction != analysis.ema_long_direction { return "idle"; }
    // เช็คแค่ว่า EMA ชี้ทิศเดียวกัน แต่ไม่ดูว่าเทรนด์แรงแค่ไหน
    if analysis.ema_short_direction == "Up" { "green" } else { "red" }
}
```

**ปัญหา**: ในตลาด sideway ที่มี micro-trend → EMA ทั้ง 3 อาจชี้ทิศเดียวกันได้  
เพราะ `ema_short_direction` คำนวณจาก `ema[i] >= ema[i-1]` เท่านั้น — แค่ขึ้น 0.001 ก็ถือว่า "Up"

---

### 3. Noise Filter ไม่ถูก Trigger ในสถานการณ์ micro-trend sideway

**ไฟล์**: `src/filterNoise.rs` + `thereshold.json`

**Config ปัจจุบัน**:
- **Flat (Case4)**: ต้อง `ema_short_flat = "y"` **และ** `ema_medium_flat = "y"`  
  → ถ้า EMA ขยับเกิน `flatThreshold = 0.071` แม้แค่นิดเดียว → ไม่ flat → ไม่ block
- **Gap**: ต้อง gap ≤ `MACDGapValue = 0.1`  
  → ถ้า EMA gap > 0.1 → ไม่ block
- **Alternate**: ต้องเป็น alternating pattern (สลับสี + body ใกล้กัน)  
  → ไม่เข้าเงื่อนไขเมื่อ EMA ชี้ทิศเดียว

**ผลลัพธ์**: ทั้ง 3 noise ไม่ trigger → `noiseCode = "none"` → เทรดทุกแท่ง → loss streak

---

## 📊 Indicator ที่คำนวณไว้ vs ที่ใช้จริง

| Indicator | มีใน FullAnalysisResult | FTB ใช้? | Noise Filter ใช้? | สถานะ |
|-----------|:---:|:---:|:---:|:---:|
| EMA Direction (Short/Med/Long) | ✅ | ✅ | ❌ | ใช้อยู่ |
| EMA Slope Value | ✅ | ❌ | ✅ (Flat) | ใช้บางส่วน |
| EMA Gap (Short-Med, Med-Long) | ✅ | ❌ | ✅ (Gap) | ใช้บางส่วน |
| Alternating Pattern | ✅ | ❌ | ✅ (Alt) | ใช้บางส่วน |
| **ADX** | ✅ | ❌ | ❌ | **ไม่ได้ใช้** |
| **Choppy Index** | ✅ | ❌ | ❌ | **ไม่ได้ใช้** |
| **RSI** | ✅ | ❌ | ❌ | **ไม่ได้ใช้** |
| **Bollinger Bands / Squeeze** | ✅ | ❌ | ❌ | **ไม่ได้ใช้** |
| **ATR / is_atr** | ✅ | ❌ (bypassed) | ❌ | **ไม่ได้ใช้** |
| **Range Detector** | ✅ | ❌ | ❌ | **ไม่ได้ใช้** |
| **EMA Convergence/Divergence** | ✅ | ❌ | ❌ | **ไม่ได้ใช้** |
| **MACD** | ✅ | ❌ | ❌ | **ไม่ได้ใช้** |
| **Candle Body %** | ✅ | ❌ | ❌ | **ไม่ได้ใช้** |
| **BB Position** | ✅ | ❌ | ❌ | **ไม่ได้ใช้** |

> **สรุป**: ระบบคำนวณ indicator ไว้เยอะมาก แต่ FTB ใช้แค่ EMA Direction 3 ตัว  
> และ Noise Filter ก็ใช้แค่ Flat/Gap/Alternate — **indicator อีก ~80% ไม่ได้ถูกนำมาประกอบการตัดสินใจเลย**

---

## 💡 แนวทางแก้ไข

### แนวทาง A: เพิ่ม Noise Filter ตัวใหม่ (เพิ่ม checkbox ใน UI)

#### A1. ADX Filter — กรองตลาดไม่มีเทรนด์
```rust
// filterNoise.rs — เพิ่มใหม่
if config.check_adx == "yes" {
    if data.adx_value < config.adx_threshold {  // default 25
        return (true, "weak_trend");
    }
}
```
**หลักการ**: ADX < 25 = ตลาดไม่มีเทรนด์ชัดเจน → ไม่ควรเทรดตาม trend

#### A2. Choppy Filter — กรองตลาด sideway
```rust
if config.check_choppy == "yes" {
    if data.choppy_indicator > config.choppy_threshold {  // default 50
        return (true, "choppy");
    }
}
```
**หลักการ**: Choppy Index > 50 = ตลาดกำลัง sideway → เทรด trend following จะแพ้

#### A3. Range Detector Filter — กรองเมื่ออยู่ใน range box
```rust
if config.check_range == "yes" {
    if data.range_detector.in_range {
        return (true, "in_range");
    }
}
```
**หลักการ**: ราคาอยู่ใน sideway box → ไม่ควรเทรด

#### A4. BB Squeeze Filter — กรองเมื่อ volatility ต่ำ
```rust
if config.check_bb_squeeze == "yes" {
    if data.is_bb_squeeze || data.bb_bandwidth < config.bb_bandwidth_threshold {
        return (true, "bb_squeeze");
    }
}
```
**หลักการ**: Bollinger Band squeeze = volatility ต่ำ → ยากจะทำนายทิศทาง

---

### แนวทาง B: แก้ FTB Logic ให้ฉลาดขึ้น

#### B1. เพิ่มเงื่อนไข ADX + Choppy ใน FTB
```rust
pub fn get_suggest_color_ftb(analysis: &FullAnalysisResult, _loss_con: u32) -> String {
    // เงื่อนไขเดิม: EMA ทิศเดียวกัน
    if analysis.ema_short_direction != analysis.ema_medium_direction { return "idle"; }
    if analysis.ema_short_direction != analysis.ema_long_direction { return "idle"; }
    
    // เงื่อนไขใหม่: ต้องมีเทรนด์แรงพอ
    if analysis.adx_value < 25.0 { return "idle"; }
    if analysis.choppy_indicator > 50.0 { return "idle"; }
    
    if analysis.ema_short_direction == "Up" { "green" } else { "red" }
}
```

#### B2. ไม่ bypass is_atr สำหรับ FTB (ทำให้ FTB ต้องรอ spike ด้วย)
```rust
// deriv.rs — แก้
let should_trade = if state.in_martingale {
    true
} else {
    prev.is_atr  // ทุก strategy ต้องรอ ATR Spike เหมือนกัน
};
```
> **ข้อควรระวัง**: B2 จะทำให้ FTB เทรดน้อยลงมาก

---

### แนวทาง C: เพิ่ม Indicator ใหม่ (ยังไม่มีในระบบ)

| Indicator | วัตถุประสงค์ | ความยาก |
|-----------|-------------|:---:|
| ATR Percentile | วัดว่า ATR ปัจจุบันอยู่ระดับไหนเทียบกับ 100 แท่งล่าสุด | ต่ำ |
| EMA Slope Strength | วัดความชันของ EMA (ไม่ใช่แค่ Up/Down) | ต่ำ (มี slope_value อยู่แล้ว) |
| Consecutive Direction Count | นับว่า EMA ชี้ทิศเดียวมากี่แท่งแล้ว | ต่ำ |
| Candle Strength Ratio | เทียบ body กับ wick ดูว่าแท่งมีแรงจริงหรือไม่ | ต่ำ (มี body_percent อยู่แล้ว) |

---

## 🎯 ลำดับความสำคัญที่แนะนำ

| ลำดับ | สิ่งที่ควรทำ | ผลที่คาดหวัง |
|:---:|-------------|-------------|
| 1 | เพิ่ม **ADX Filter** ใน noise (A1) | กรองตลาดไม่มี trend → ลด loss streak |
| 2 | เพิ่ม **Choppy Filter** ใน noise (A2) | กรองตลาด sideway → ลด loss streak |
| 3 | เพิ่ม **Range Detector Filter** ใน noise (A3) | กรองเมื่อราคาอยู่ใน box → ลด loss streak |
| 4 | แก้ FTB ให้เช็ค ADX ≥ 25 (B1) | FTB ฉลาดขึ้นจากต้นทาง |
| 5 | พิจารณาเรื่อง bypass is_atr (B2) | ลดจำนวนเทรดในแท่งที่ไม่แรงพอ |

---

## ไฟล์ที่เกี่ยวข้อง

| ไฟล์ | หน้าที่ |
|------|--------|
| `src/filterNoise.rs` | Noise filter logic (ต้องเพิ่ม filter ใหม่ที่นี่) |
| `src/get_action.rs` | FTB strategy logic (ต้องเพิ่มเงื่อนไขที่นี่) |
| `src/deriv.rs` | Trade entry logic (is_atr bypass อยู่ที่นี่) |
| `src/full_analysis_Ver2.rs` | คำนวณ indicator ทั้งหมด |
| `thereshold.json` | Config ค่า threshold ต่างๆ |
| `public/index.html` | UI สำหรับ checkbox noise filter |

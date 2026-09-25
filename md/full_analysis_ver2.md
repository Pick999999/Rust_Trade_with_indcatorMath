# Full Analysis Data (ver2)

ไฟล์นี้อธิบายโครงสร้างข้อมูล `FullAnalysisResult` ที่ได้จาก `full_analysis_ver2.rs` ซึ่งเป็นข้อมูลที่ถูกประมวลผลแล้วและส่งกลับไปยัง Frontend หรือใช้งานในระบบเทรด

## รูปแบบ JSON (JSON Structure)

```json
{
  "index": 0,
  "assetCode": "BTCUSD",
  "candletime": 1718712345,
  "candletime_display": "2024-06-18 15:30:00",
  "open": 65000.0,
  "high": 65100.0,
  "low": 64900.0,
  "close": 65050.0,
  "color": "green",
  "next_color": null,
  "pip_size": 50.0,
  "ema_short_value": 64980.5,
  "ema_short_direction": "Up",
  "ema_short_turn_type": "TurnUp",
  "ema_short_slope_value": 5.2,
  "emaslopeThereshold": 1.5,
  "diff": 0.5,
  "ema_short_flat": "No",
  "ema_medium_value": 64950.2,
  "ema_medium_direction": "Up",
  "ema_medium_turn_type": "-",
  "ema_medium_slope_value": 3.1,
  "ema_medium_flat": "No",
  "ema_long_value": 64800.0,
  "ema_long_direction": "Down",
  "ema_long_turn_type": "-",
  "ema_long_slope_value": -1.2,
  "ema_long_flat": "Yes",
  "short_medium_gap_value": 30.3,
  "is_short_medium_gap_occur": "Yes",
  "medium_long_gap_value": 150.2,
  "is_medium_long_gap_occur": "Yes",
  "ema_above": "ShortAbove",
  "ema_long_above": "MediumAbove",
  "macd_12": 150.5,
  "macd_23": 120.3,
  "previous_ema_short_value": 64975.3,
  "previous_ema_medium_value": 64947.1,
  "previous_ema_long_value": 64801.2,
  "previous_macd_12": 145.2,
  "previous_macd_23": 115.1,
  "ema_convergence_type": "divergence",
  "ema_long_convergence_type": "D",
  "choppy_indicator": 35.5,
  "adx_value": 25.4,
  "rsi_value": 55.2,
  "bb_values": {
    "upper": 65200.0,
    "middle": 64900.0,
    "lower": 64600.0
  },
  "bb_position": "NearUpper",
  "atrValue": 120.5,
  "is_abnormal_candle": false,
  "is_abnormal_atr": false,
  "is_atr": true,
  "u_wick": 50.0,
  "u_wick_percent": 25.0,
  "body": 50.0,
  "body_percent": 25.0,
  "l_wick": 100.0,
  "l_wick_percent": 50.0,
  "ema_cut_position": "CrossUp",
  "ema_cut_long_type": "-",
  "ema_cut_short_long_type": "CrossUp",
  "ema_cut_all_type": "-",
  "candles_since_ema_cut": 3,
  "ema_short_pos": "AboveHigh",
  "ema_medium_pos": "Body",
  "ema_long_pos": "BelowLow",
  "bb_bandwidth": 0.92,
  "is_bb_squeeze": false,
  "up_con_medium_ema": 5,
  "down_con_medium_ema": 0,
  "up_con_long_ema": 12,
  "down_con_long_ema": 0,
  "is_mark": "-",
  "status_code": "000",
  "status_desc": "Normal",
  "status_desc_0": "Normal",
  "hint_status": "-",
  "suggest_color": "green",
  "win_status": "-",
  "win_con": 0,
  "loss_con": 0,
  "smc": {
    "structures": [
      {
        "time": 1718710000,
        "price": 64800.0,
        "structure_type": "BOS",
        "direction": "bullish",
        "level": "major",
        "start_time": 1718705000
      }
    ],
    "swing_points": [
      {
        "time": 1718711000,
        "price": 64900.0,
        "swing_type": "HL",
        "swing": "strong"
      }
    ],
    "order_blocks": [],
    "fair_value_gaps": [],
    "equal_highs_lows": [],
    "premium_discount_zone": {
      "start_time": 1718705000,
      "end_time": 1718712345,
      "premium_top": 65220.5,
      "premium_bottom": 65050.0,
      "equilibrium": 65000.0,
      "discount_top": 65050.0,
      "discount_bottom": 64779.5
    },
    "strong_weak_levels": [],
    "swing_trend": "bullish",
    "internal_trend": "bullish"
  },
  "tick_volatility": {
    "tick_count": 120,
    "buy_tick_count": 70,
    "sell_tick_count": 50,
    "buy_sell_ratio": 1.4,
    "avg_tick_move": 2.5,
    "max_tick_move": 15.0,
    "sum_tick_move": 300.0,
    "volatility_clustering": 0.8,
    "volatility_level": "medium"
  },
  "range_detector": {
    "in_range": false,
    "range_top": 65150.0,
    "range_bottom": 64850.0,
    "range_avg": 65000.0,
    "range_state": "up"
  },
  "is_alternating_pattern": true,
  "alternating_sequence_length": 3,
  "is_alternating_trigger": true,
  "is_alternating_spike": false
}
```

## คำอธิบายฟิลด์ (Field Descriptions)

### ข้อมูลพื้นฐาน (Basic Info)
- **`index`** *(Integer)*: ลำดับของแท่งเทียน (เริ่มจาก 0)
- **`assetCode`** *(String)*: ชื่อสินทรัพย์ที่ประมวลผล เช่น "BTCUSD"
- **`candletime`** *(Integer)*: เวลาของแท่งเทียนในรูปแบบ Epoch (Unix timestamp)
- **`candletime_display`** *(String)*: เวลาของแท่งเทียนในรูปแบบที่อ่านได้ (YYYY-MM-DD HH:MM:SS) ตามเวลาท้องถิ่น
- **`open`, `high`, `low`, `close`** *(Float)*: ค่าราคาเปิด, สูงสุด, ต่ำสุด และปิดของแท่งเทียน
- **`color`** *(String)*: สีของแท่งเทียน ("green" = ปิดบวก, "red" = ปิดลบ, "equal" = ราคาคงที่)
- **`next_color`** *(String | null)*: สีของแท่งเทียนในอนาคต (มักใช้สำหรับการ backtest หรือประเมินผล)
- **`pip_size`** *(Float)*: ขนาดความกว้างของเนื้อเทียน (ส่วนต่างของ Open กับ Close แบบ Absolute)

### Exponential Moving Averages (EMA)
- **`ema_short_value`, `ema_medium_value`, `ema_long_value`** *(Float)*: ค่า EMA ระยะสั้น, ระยะกลาง, และระยะยาว ตามลำดับ
- **`ema_short_direction`, `ema_medium_direction`, `ema_long_direction`** *(String)*: ทิศทางของเส้น EMA ว่าชี้ขึ้น ("Up") หรือชี้ลง ("Down") เทียบกับแท่งก่อนหน้า
- **`ema_short_turn_type`, `ema_medium_turn_type`, `ema_long_turn_type`** *(String)*: รูปแบบการหักหัวของเส้น EMA ("TurnUp" = เริ่มชี้ขึ้น, "TurnDown" = เริ่มชี้ลง, "-" = ทิศทางเดิม)
- **`ema_short_slope_value`, `ema_medium_slope_value`, `ema_long_slope_value`** *(Float)*: ค่าความชันของเส้น EMA
- **`emaslopeThereshold`** *(Float)*: เกณฑ์ที่ใช้ตรวจสอบว่าความชันนั้นถือเป็นเส้นตรง (Flat) หรือไม่
- **`ema_short_flat`, `ema_medium_flat`, `ema_long_flat`** *(String)*: สถานะบ่งบอกว่าเส้น EMA อยู่ในแนวระนาบหรือไม่ (Flat)
- **`ema_above`** *(String)*: บอกว่าเส้นใดอยู่บนระหว่างเส้นระยะสั้นกับเส้นระยะกลาง ("ShortAbove", "MediumAbove")
- **`ema_long_above`** *(String)*: บอกว่าเส้นใดอยู่บนระหว่างเส้นระยะกลางกับเส้นระยะยาว ("MediumAbove", "LongAbove")

### EMA Gaps & Convergence
- **`short_medium_gap_value`** *(Float)*: ระยะห่างระหว่างเส้น EMA ระยะสั้นและระยะกลาง
- **`is_short_medium_gap_occur`** *(String)*: ตรวจสอบการเกิด Gap ระหว่างสั้นกับกลาง
- **`medium_long_gap_value`** *(Float)*: ระยะห่างระหว่างเส้น EMA ระยะกลางและระยะยาว
- **`is_medium_long_gap_occur`** *(String)*: ตรวจสอบการเกิด Gap ระหว่างกลางกับยาว
- **`ema_convergence_type`** *(String)*: บอกว่าระยะสั้นและกลางกำลังบีบเข้าหากัน ("convergence") หรือแยกออกจากกัน ("divergence")
- **`ema_long_convergence_type`** *(String)*: บอกว่าระยะกลางและยาวกำลังบีบเข้าหากัน ("C") หรือแยกออกจากกัน ("D")

### EMA Cross / Crossover
- **`ema_cut_position`** *(String)*: การตัดกันของเส้นระยะสั้นและระยะกลาง ("CrossUp", "CrossDown", หรือ "-")
- **`ema_cut_long_type`** *(String)*: การตัดกันของเส้นระยะกลางและระยะยาว
- **`ema_cut_short_long_type`** *(String)*: การตัดกันของเส้นระยะสั้นและระยะยาว
- **`ema_cut_all_type`** *(String)*: เส้นเรียงตัวกันตัดผ่าน ("AllCrossUp", "AllCrossDown")
- **`candles_since_ema_cut`** *(Integer)*: จำนวนแท่งเทียนที่ผ่านมาตั้งแต่เกิดการตัดกันครั้งล่าสุด

### ตำแหน่งราคาเทียบกับ EMA
- **`ema_short_pos`, `ema_medium_pos`, `ema_long_pos`** *(String)*: ตำแหน่งของเส้น EMA เมื่อเทียบกับโครงสร้างแท่งเทียน ("AboveHigh" = อยู่เหนือจุดสูงสุด, "UpperWick" = ผ่านไส้บน, "Body" = ตัดผ่านเนื้อเทียน, "LowerWick" = ผ่านไส้ล่าง, "BelowLow" = อยู่ใต้จุดต่ำสุด)

### ตัวชี้วัด (Indicators) - MACD, RSI, ADX, Bollinger Bands, ATR
- **`macd_12`, `macd_23`** *(Float)*: ค่า MACD Line และ Signal Line ตามลำดับ
- **`choppy_indicator`** *(Float)*: ค่า Choppiness Index บอกสภาวะตลาดว่าเป็นเทรนด์หรือไซต์เวย์ (ค่ายิ่งสูงยิ่งไม่มีเทรนด์)
- **`adx_value`** *(Float)*: ค่า ADX วัดความแข็งแกร่งของเทรนด์
- **`rsi_value`** *(Float)*: ค่า Relative Strength Index
- **`bb_values`** *(Object)*: ข้อมูลเส้นขอบ Bollinger Bands 
  - `upper` *(Float)*: เส้นขอบบน
  - `middle` *(Float)*: เส้นกึ่งกลาง (SMA)
  - `lower` *(Float)*: เส้นขอบล่าง
- **`bb_position`** *(String)*: ตำแหน่งของราคาเทียบกับ Bollinger Bands ("AboveUpper", "NearUpper", "NearLower", "BelowLower")
- **`bb_bandwidth`** *(Float)*: ความกว้างของ Bollinger Bands ในรูปแบบเปอร์เซ็นต์
- **`is_bb_squeeze`** *(Boolean)*: การหดแคบของกรอบ Bollinger (เป็นจุดเล็กสุดในรอบย้อนหลัง 20 แท่ง)
- **`atrValue`** *(Float)*: ค่า ATR (Average True Range) วัดความผันผวน
- **`is_abnormal_candle`** *(Boolean)*: ช่วงห่าง (High - Low) มากกว่า (ATR * Multiplier) ถือว่ายาวผิดปกติ
- **`is_abnormal_atr`** *(Boolean)*: ค่า ATR ปัจจุบันสูงกว่าในอดีต (5 แท่งที่แล้ว) ถึง 2 เท่า
- **`is_atr`** *(Boolean)*: แท่งนี้ตรงกับเงื่อนไขของ ATR 

### ข้อมูลย้อนหลัง (Previous Values)
- **`previous_ema_short_value`, `previous_ema_medium_value`, `previous_ema_long_value`** *(Float)*: ค่า EMA แท่งที่แล้ว
- **`previous_macd_12`, `previous_macd_23`** *(Float)*: ค่า MACD แท่งที่แล้ว

### องค์ประกอบแท่งเทียน (Candle Anatomy)
- **`u_wick`**, **`u_wick_percent`** *(Float)*: ขนาดไส้บน (Absolute) และคิดเป็นกี่เปอร์เซ็นต์ของทั้งแท่ง
- **`body`**, **`body_percent`** *(Float)*: ขนาดเนื้อเทียน (Absolute) และคิดเป็นเปอร์เซ็นต์
- **`l_wick`**, **`l_wick_percent`** *(Float)*: ขนาดไส้ล่าง (Absolute) และคิดเป็นเปอร์เซ็นต์

### รูปแบบสลับสี (Alternating Pattern)
- **`is_alternating_pattern`** *(Boolean)*: ตรวจจับการสลับสีของแท่งเทียน
- **`alternating_sequence_length`** *(Integer)*: จำนวนแท่งเทียนที่เกิดการสลับสีติดต่อกัน
- **`is_alternating_trigger`** *(Boolean)*: ถึงเกณฑ์ขั้นต่ำของการสลับสีที่สามารถส่งทริกเกอร์ได้ (มากกว่าหรือเท่ากับ 3 แท่ง)
- **`is_alternating_spike`** *(Boolean)*: มีการสลับสีโดยที่มีขนาดการเปลี่ยนกลับอย่างรุนแรง

### โครงสร้าง SMC (Smart Money Concepts)
- **`smc`** *(Object)*: ข้อมูลที่ใช้ระบุแนวรับ/แนวต้าน ตามหลักจิตวิทยา SMC
  - `structures` *(Array)*: ระบุการเบรคโครงสร้าง (BOS - Break of Structure, CHoCH - Change of Character)
  - `swing_points` *(Array)*: ระบุจุดแกว่งหลัก เช่น สูงขึ้น (Higher High) ต่ำลง (Lower Low)
  - `order_blocks`, `fair_value_gaps`, `equal_highs_lows`, `strong_weak_levels` *(Array)*: ข้อมูลเชิงลึกของ SMC (ระดับพื้นที่สำคัญ)
  - `premium_discount_zone` *(Object)*: ข้อมูลพื้นที่ Premium และ Discount เพื่อใช้ตัดสินความคุ้มค่าของการเข้าเทรด
  - `swing_trend`, `internal_trend` *(String)*: เทรนด์หลักและเทรนด์ย่อยอิงตามโครงสร้าง

### ความผันผวนย่อย (Tick Volatility)
- **`tick_volatility`** *(Object)*: การเคลื่อนไหวของราคาแบบละเอียดภายในแท่ง
  - `tick_count`: จำนวน Tick ทั้งหมด
  - `buy_tick_count`, `sell_tick_count`: จำนวน Tick ฝั่งซื้อ / ฝั่งขาย
  - `buy_sell_ratio`: สัดส่วนการซื้อต่อขาย
  - `avg_tick_move`, `max_tick_move`, `sum_tick_move`: ระยะห่างการเคลื่อนที่ (เฉลี่ย, สูงสุด, รวมทั้งหมด)
  - `volatility_clustering`: การกระจุกตัวของความผันผวน
  - `volatility_level`: ระดับความผันผวน ("low", "medium", "high")

### สภาวะไซด์เวย์ (Range Detector)
- **`range_detector`** *(Object)*: ตรวจจับสภาวะที่วิ่งในกรอบ
  - `in_range` *(Boolean)*: ราคาปัจจุบันอยู่ในกรอบแคบๆ หรือไม่
  - `range_top`, `range_bottom`, `range_avg` *(Float)*: ขอบบน, ขอบล่าง, และค่าเฉลี่ยของกรอบไซด์เวย์
  - `range_state` *(String)*: สภาวะของกรอบ เช่น "unbroken" (ยังอยู่ในกรอบ), "up" (เบรคทะลุขึ้น), "down" (เบรคทะลุลง)

### อื่นๆ / สถานะการส่งสัญญาณ (Status/Signal)
- **`up_con_medium_ema`, `down_con_medium_ema`** *(Integer)*: จำนวนแท่งที่ลอยตัวอยู่เหนือ/ใต้ EMA ระดับกลางติดต่อกัน
- **`up_con_long_ema`, `down_con_long_ema`** *(Integer)*: จำนวนแท่งที่ลอยตัวอยู่เหนือ/ใต้ EMA ระดับยาวติดต่อกัน
- **`is_mark`** *(String)*: สัญลักษณ์ Mark ทั่วไป
- **`status_code`, `status_desc`, `status_desc_0`** *(String)*: รหัสและคำอธิบายสถานะของสัญญาณที่จะส่งให้ Frontend
- **`hint_status`** *(String)*: ข้อแนะนำหรือคำใบ้เพิ่มเติม (Signal Hint)
- **`suggest_color`** *(String)*: สีที่ระบบแนะนำหรือคาดการณ์ว่าจะเป็น
- **`win_status`** *(String)*: สถานะแพ้/ชนะ (ในการทำ Backtest)
- **`win_con`, `loss_con`** *(Integer)*: จำนวนครั้งที่แพ้/ชนะติดต่อกัน

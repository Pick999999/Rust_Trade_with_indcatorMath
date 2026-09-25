# ข้อมูล Full Analysis (JSON Structure & Descriptions)

ข้อมูลผลลัพธ์ที่จะได้กลับมาเมื่อเรียกใช้งานวิเคราะห์ผ่าน API (จาก `full_analysis_Ver2.rs`) จะอยู่ในรูปแบบ Array ของ JSON Object โดยแต่ละ Object เป็นตัวแทนของข้อมูล 1 แท่งเทียน (`FullAnalysisResult`)

## โครงสร้าง JSON ของ 1 แท่งเทียน (FullAnalysisResult)

```json
{
  "index": 0,
  "assetCode": "EURUSD",
  "candletime": 1620000000,
  "candletime_display": "2021-05-03 00:00:00",
  "open": 1.2000,
  "high": 1.2050,
  "low": 1.1980,
  "close": 1.2030,
  "color": "Green",
  "next_color": null,
  "pip_size": 10.0,
  
  "ema_short_value": 1.2010,
  "ema_short_direction": "Up",
  "ema_short_turn_type": "None",
  
  "ema_medium_value": 1.1990,
  "ema_medium_direction": "Up",
  "ema_medium_turn_type": "-",
  
  "ema_long_value": 1.1900,
  "ema_long_direction": "Up",
  "ema_long_turn_type": "-",
  
  "ema_above": "Short",
  "ema_long_above": "Price",
  
  "macd_12": 0.0015,
  "macd_23": 0.0010,
  
  "previous_ema_short_value": 1.2005,
  "previous_ema_medium_value": 1.1985,
  "previous_ema_long_value": 1.1895,
  "previous_macd_12": 0.0012,
  "previous_macd_23": 0.0008,
  
  "ema_convergence_type": "Diverging",
  "ema_long_convergence_type": "Diverging",
  
  "choppy_indicator": 45.5,
  "adx_value": 25.4,
  "rsi_value": 65.2,
  
  "bb_values": {
    "upper": 1.2060,
    "middle": 1.2000,
    "lower": 1.1940
  },
  "bb_position": "Inside",
  
  "atrValue": 0.0050,
  "is_abnormal_candle": false,
  "is_abnormal_atr": false,
  
  "u_wick": 0.0020,
  "u_wick_percent": 28.5,
  "body": 0.0030,
  "body_percent": 42.8,
  "l_wick": 0.0020,
  "l_wick_percent": 28.5,
  
  "ema_cut_position": "-",
  "ema_cut_long_type": "-",
  "ema_cut_short_long_type": "-",
  "ema_cut_all_type": "-",
  "candles_since_ema_cut": 10,
  
  "ema_short_pos": "Above",
  "ema_medium_pos": "Above",
  "ema_long_pos": "Above",
  
  "bb_bandwidth": 0.0120,
  "is_bb_squeeze": false,
  
  "up_con_medium_ema": 1,
  "down_con_medium_ema": 0,
  "up_con_long_ema": 1,
  "down_con_long_ema": 0,
  
  "is_mark": "None",
  "status_code": "OK",
  "status_desc": "Normal trend",
  "status_desc_0": "No alert",
  "hint_status": "Buy",
  "suggest_color": "Green",
  "win_status": "Pending",
  "win_con": 0,
  "loss_con": 0,
  
  "smc": {
    "structures": [],
    "swing_points": [],
    "order_blocks": [],
    "fair_value_gaps": [],
    "equal_highs_lows": [],
    "premium_discount_zone": {
      "start_time": 1620000000,
      "end_time": 1620003600,
      "premium_top": 1.2050,
      "premium_bottom": 1.2025,
      "equilibrium": 1.2025,
      "discount_top": 1.2025,
      "discount_bottom": 1.2000
    },
    "strong_weak_levels": [],
    "swing_trend": "Bullish",
    "internal_trend": "Bullish"
  },
  
  "tick_volatility": {
    "tick_count": 150,
    "buy_tick_count": 90,
    "sell_tick_count": 60,
    "buy_sell_ratio": 1.5,
    "avg_tick_move": 0.0001,
    "max_tick_move": 0.0005,
    "sum_tick_move": 0.0150,
    "volatility_clustering": 0.8,
    "volatility_level": "Medium"
  },
  
  "range_detector": {
    "in_range": false,
    "range_top": 0.0,
    "range_bottom": 0.0,
    "range_avg": 0.0,
    "range_state": "Trending"
  },
  
  "is_alternating_pattern": true,
  "alternating_sequence_length": 2,
  "is_alternating_trigger": true,
  "is_alternating_spike": false
}
```

---

## คำอธิบาย Field (ภาษาไทย)

### 📌 ข้อมูลพื้นฐานและราคา (Basic Info & Price Data)
- **`index`** *(Number)*: ลำดับที่ของแท่งเทียนในชุดข้อมูล
- **`assetCode`** *(String)*: รหัสสินทรัพย์ (เช่น "EURUSD", "BTCUSD")
- **`candletime`** *(Number)*: เวลาของแท่งเทียนในรูปแบบ Unix Timestamp (วินาที)
- **`candletime_display`** *(String)*: เวลาที่ถูกจัดรูปแบบให้อ่านง่าย
- **`open`** *(Number)*: ราคาเปิดของแท่งเทียน
- **`high`** *(Number)*: ราคาสูงสุดของแท่งเทียน
- **`low`** *(Number)*: ราคาต่ำสุดของแท่งเทียน
- **`close`** *(Number)*: ราคาปิดของแท่งเทียน
- **`color`** *(String)*: สีของแท่งเทียน (เช่น "Green", "Red")
- **`next_color`** *(String | null)*: สีของแท่งเทียนถัดไป (ถ้ามีข้อมูล)
- **`pip_size`** *(Number)*: ขนาดของ Pip สำหรับสินทรัพย์นี้
- **`is_abnormal_candle`** *(Boolean)*: เป็นแท่งเทียนที่มีขนาดผิดปกติหรือไม่
- **`u_wick`** *(Number)*: ขนาดของไส้เทียนด้านบน (Upper Wick)
- **`u_wick_percent`** *(Number)*: สัดส่วนเปอร์เซ็นต์ของไส้เทียนด้านบนเทียบกับขนาดแท่งเทียน
- **`body`** *(Number)*: ขนาดของเนื้อเทียน (Body)
- **`body_percent`** *(Number)*: สัดส่วนเปอร์เซ็นต์ของเนื้อเทียนเทียบกับขนาดแท่งเทียน
- **`l_wick`** *(Number)*: ขนาดของไส้เทียนด้านล่าง (Lower Wick)
- **`l_wick_percent`** *(Number)*: สัดส่วนเปอร์เซ็นต์ของไส้เทียนด้านล่างเทียบกับขนาดแท่งเทียน

### 📌 ข้อมูล Moving Averages (EMA)
- **`ema_short_value`** *(Number)*: ค่าเส้นค่าเฉลี่ย EMA ระยะสั้น
- **`ema_short_direction`** *(String)*: ทิศทางของเส้น EMA สั้น (เช่น "Up", "Down")
- **`ema_short_turn_type`** *(String)*: ประเภทการกลับตัวของเส้น EMA สั้น
- **`ema_short_slope_value`** *(Number)*: ค่าความชันของ EMA ระยะสั้น (แท่งปัจจุบัน - แท่งก่อนหน้า)
- **`ema_short_flat`** *(String)*: สถานะความชัน EMA ระยะสั้น ("y" = Flat, "n" = ไม่ Flat) โดยเปรียบเทียบกับ `flatTheresholdValue`
- **`ema_medium_value`** *(Number)*: ค่าเส้นค่าเฉลี่ย EMA ระยะกลาง
- **`ema_medium_direction`** *(String)*: ทิศทางของเส้น EMA กลาง
- **`ema_medium_turn_type`** *(String)*: ประเภทการกลับตัวของเส้น EMA กลาง ("TurnUp", "TurnDown", "-")
- **`ema_medium_slope_value`** *(Number)*: ค่าความชันของ EMA ระยะกลาง (แท่งปัจจุบัน - แท่งก่อนหน้า)
- **`ema_medium_flat`** *(String)*: สถานะความชัน EMA ระยะกลาง ("y" = Flat, "n" = ไม่ Flat) โดยเปรียบเทียบกับ `flatTheresholdValue`
- **`ema_long_value`** *(Number)*: ค่าเส้นค่าเฉลี่ย EMA ระยะยาว
- **`ema_long_direction`** *(String)*: ทิศทางของเส้น EMA ยาว
- **`ema_long_turn_type`** *(String)*: ประเภทการกลับตัวของเส้น EMA ยาว ("TurnUp", "TurnDown", "-")
- **`ema_long_slope_value`** *(Number)*: ค่าความชันของ EMA ระยะยาว (แท่งปัจจุบัน - แท่งก่อนหน้า)
- **`ema_long_flat`** *(String)*: สถานะความชัน EMA ระยะยาว ("y" = Flat, "n" = ไม่ Flat) โดยเปรียบเทียบกับ `flatTheresholdValue`
- **`short_medium_gap_value`** *(Number)*: ระยะห่างระหว่าง EMA ระยะสั้นและระยะกลาง (ค่าสัมบูรณ์)
- **`is_short_medium_gap_occur`** *(String)*: สถานะระยะห่าง EMA สั้นและกลาง ("y" = ห่างน้อยกว่าหรือเท่ากับ MACDGapValue, "n" = ไม่ใช่)
- **`medium_long_gap_value`** *(Number)*: ระยะห่างระหว่าง EMA ระยะกลางและระยะยาว (ค่าสัมบูรณ์)
- **`is_medium_long_gap_occur`** *(String)*: สถานะระยะห่าง EMA กลางและยาว ("y" = ห่างน้อยกว่าหรือเท่ากับ MACDGapValue, "n" = ไม่ใช่)
- **`ema_above`** *(String)*: ตำแหน่งของราคาสัมพัทธ์กับ EMA สั้นและกลาง
- **`ema_long_above`** *(String)*: ตำแหน่งของราคาสัมพัทธ์กับ EMA ยาว
- **`previous_ema_short_value`** *(Number)*: ค่า EMA สั้นของแท่งก่อนหน้า
- **`previous_ema_medium_value`** *(Number)*: ค่า EMA กลางของแท่งก่อนหน้า
- **`previous_ema_long_value`** *(Number)*: ค่า EMA ยาวของแท่งก่อนหน้า
- **`ema_convergence_type`** *(String)*: รูปแบบการลู่เข้า/บานออก (Convergence/Divergence) ของ EMA
- **`ema_long_convergence_type`** *(String)*: การลู่เข้า/บานออกของราคาเทียบกับ EMA ยาว
- **`ema_cut_position`** *(String)*: การตัดกันของ EMA Short กับ EMA Medium ("CrossUp" = สั้นตัดขึ้นผ่านกลาง, "CrossDown" = สั้นตัดลงผ่านกลาง, "-" = ไม่มีการตัด)
- **`ema_cut_long_type`** *(String)*: การตัดกันของ EMA Medium กับ EMA Long ("CrossUp" = กลางตัดขึ้นผ่านยาว, "CrossDown" = กลางตัดลงผ่านยาว, "-" = ไม่มีการตัด)
- **`ema_cut_short_long_type`** *(String)*: การตัดกันของ EMA Short กับ EMA Long ("CrossUp" = สั้นตัดขึ้นผ่านยาว, "CrossDown" = สั้นตัดลงผ่านยาว, "-" = ไม่มีการตัด)
- **`ema_cut_all_type`** *(String)*: การตัดกันของ EMA ทั้ง 3 เส้น ("AllCrossUp" = Short > Medium > Long เรียงขึ้นครบ, "AllCrossDown" = Short < Medium < Long เรียงลงครบ, "-" = ยังไม่เรียงครบ)
- **`candles_since_ema_cut`** *(Number)*: จำนวนแท่งเทียนที่ผ่านมานับตั้งแต่มีการตัดกันของ EMA ครั้งล่าสุด
- **`ema_short_pos`**, **`ema_medium_pos`**, **`ema_long_pos`** *(String)*: ตำแหน่งของราคาเมื่อเทียบกับเส้น EMA นั้นๆ (เช่น "Above", "Below")
- **`up_con_medium_ema`**, **`down_con_medium_ema`**, **`up_con_long_ema`**, **`down_con_long_ema`** *(Number)*: ตัวนับหรือค่าความต่อเนื่องที่สนับสนุนแนวโน้มตามทิศทางของ EMA 

### 📌 ข้อมูล Oscillators & Indicators ทั่วไป
- **`macd_12`** *(Number)*: ค่า MACD (อ้างอิงช่วง 12 แท่ง หรือเส้น MACD ปกติ)
- **`macd_23`** *(Number)*: ค่า MACD Signal หรือ Histogram
- **`previous_macd_12`**, **`previous_macd_23`** *(Number)*: ค่า MACD ในแท่งก่อนหน้า
- **`choppy_indicator`** *(Number)*: ค่า Choppiness Index (ใช้วัดความแกว่งตัว หรือช่วง Sideways)
- **`adx_value`** *(Number)*: ค่าดัชนี ADX (ใช้วัดความแข็งแกร่งของเทรนด์)
- **`rsi_value`** *(Number)*: ค่า RSI (Relative Strength Index)
- **`atrValue`** *(Number)*: ค่า ATR (Average True Range) วัดความผันผวน
- **`is_abnormal_atr`** *(Boolean)*: ค่า ATR สูง/ต่ำ ผิดปกติหรือไม่

### 📌 ข้อมูล Bollinger Bands (`bb_values`)
- **`bb_values.upper`** *(Number)*: ขอบบนของ Bollinger Bands
- **`bb_values.middle`** *(Number)*: เส้นแกนกลาง (SMA) ของ Bollinger Bands
- **`bb_values.lower`** *(Number)*: ขอบล่างของ Bollinger Bands
- **`bb_position`** *(String)*: ตำแหน่งของราคาเทียบกับกรอบ Bollinger Bands
- **`bb_bandwidth`** *(Number)*: ความกว้างของกรอบ Bollinger Bands
- **`is_bb_squeeze`** *(Boolean)*: สถานะที่กรอบ Bollinger Bands บีบอัดตัว (Squeeze) หรือไม่

### 📌 ข้อมูล Smart Money Concepts (`smc`)
- **`structures`** *(Array)*: โครงสร้างของตลาด (เช่น BOS, CHoCH) โดยมีรายละเอียดดังนี้
  - `time`, `price`, `structure_type`, `direction`, `level`, `start_time`
- **`swing_points`** *(Array)*: จุดกลับตัวหรือจุด Swing High/Low
  - `time`, `price`, `swing_type`, `swing`
- **`order_blocks`** *(Array)*: ระดับราคาหรือโซนที่เป็น Order Blocks
- **`fair_value_gaps`** *(Array)*: โซนช่องว่างของราคา (FVG)
- **`equal_highs_lows`** *(Array)*: จุดยอดสูงสุด/ต่ำสุดที่เท่ากัน
- **`premium_discount_zone`** *(Object)*: โซน Premium / Discount สำหรับหาจุดเข้าทำกำไรที่ได้เปรียบ
  - แบ่งเป็น `start_time`, `end_time`, โซน `premium` (top/bottom), โซน `discount` (top/bottom) และ `equilibrium` (ค่ากึ่งกลาง)
- **`strong_weak_levels`** *(Array)*: แนวรับ/แนวต้านที่มีความแข็งแรงหรืออ่อนแอตามทฤษฎี SMC
- **`swing_trend`** *(String)*: ทิศทางแนวโน้มของวงสวิง (Swing Trend)
- **`internal_trend`** *(String)*: ทิศทางแนวโน้มย่อยภายใน (Internal Trend)

### 📌 ข้อมูลความผันผวนระดับ Tick (`tick_volatility`)
- **`tick_count`** *(Number)*: จำนวน Tick ที่เกิดในแท่งเทียนนี้
- **`buy_tick_count`**, **`sell_tick_count`** *(Number)*: จำนวน Tick ที่เป็นการซื้อ (Buy) และขาย (Sell)
- **`buy_sell_ratio`** *(Number)*: อัตราส่วนระหว่างฝั่งซื้อและฝั่งขาย
- **`avg_tick_move`** *(Number)*: ระยะการเคลื่อนที่เฉลี่ยต่อ Tick
- **`max_tick_move`** *(Number)*: ระยะการเคลื่อนที่สูงสุดภายใน Tick เดียว
- **`sum_tick_move`** *(Number)*: ระยะเคลื่อนที่ของ Tick ทั้งหมดรวมกัน
- **`volatility_clustering`** *(Number)*: การกระจุกตัวของความผันผวน
- **`volatility_level`** *(String)*: ระดับความผันผวน (เช่น Low, Medium, High)

### 📌 ข้อมูลตรวจสอบกรอบไซด์เวย์ (`range_detector`)
- **`in_range`** *(Boolean)*: ตลาดอยู่ในสภาวะกรอบแคบ/ไซด์เวย์หรือไม่
- **`range_top`**, **`range_bottom`** *(Number)*: ขอบเขตบนและล่างของกรอบไซด์เวย์
- **`range_avg`** *(Number)*: ค่าเฉลี่ยกลางของกรอบไซด์เวย์
- **`range_state`** *(String)*: สถานะของกรอบช่วงนี้ (เช่น "Trending", "Ranging")

### 📌 ข้อมูลรูปแบบแท่งเทียนสลับ (Alternating Candle Pattern)
- **`is_alternating_pattern`** *(Boolean)*: บ่งบอกว่าแท่งเทียนนี้เป็นส่วนหนึ่งของรูปแบบสลับสีและขนาดใกล้เคียงกันหรือไม่
- **`alternating_sequence_length`** *(Number)*: จำนวนแท่งเทียนที่เกิดการสลับสีติดต่อกัน (สะสม)
- **`is_alternating_trigger`** *(Boolean)*: เงื่อนไขการสลับสีครบถ้วนหรือไม่ (สลับสีกันตั้งแต่ 2 แท่งขึ้นไป)
- **`is_alternating_spike`** *(Boolean)*: บ่งบอกว่าเกิดแท่งเทียนสลับสีที่ขนาดเนื้อเทียนต่างจากแท่งก่อนหน้ามาก (มากกว่า 1.5 เท่าของ ATR) ซึ่งอาจเป็น Spike บอกสัญญาณกลับตัว

### 📌 ข้อมูลคำแนะนำและสัญญาณสำหรับเทรด
- **`is_mark`** *(String)*: จุด Mark สำคัญต่างๆ สำหรับบอท/ผู้ใช้
- **`status_code`** *(String)*: รหัสสถานะบอกอาการของตลาด
- **`status_desc`**, **`status_desc_0`** *(String)*: คำอธิบายสถานะเพิ่มเติม
- **`hint_status`** *(String)*: คำใบ้/คำแนะนำของสถานการณ์
- **`suggest_color`** *(String)*: สี/ทิศทางที่ระบบแนะนำสำหรับการเทรด
- **`win_status`** *(String)*: สถานะผลลัพธ์ (Win/Loss) ของสัญญาณการเทรดนี้
- **`win_con`**, **`loss_con`** *(Number)*: เงื่อนไขการชนะและการแพ้ที่ประเมินจากระบบ

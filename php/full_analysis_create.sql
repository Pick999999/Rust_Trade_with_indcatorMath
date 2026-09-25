CREATE TABLE `full_analysis_result` (
    `id` INT AUTO_INCREMENT PRIMARY KEY COMMENT 'รหัสประจำรายการ (PK)',
    `row_index` INT COMMENT 'ลำดับของแท่งเทียนในชุดข้อมูลอ้างอิง',
    `asset_code` VARCHAR(50) COMMENT 'รหัสคู่เงินหรือสินทรัพย์ (เช่น R_100)',
    `candletime` BIGINT COMMENT 'เวลาของแท่งเทียนในรูปแบบ Epoch Timestamp',
    `candletime_display` VARCHAR(50) COMMENT 'เวลาของแท่งเทียนแบบอ่านง่าย (เช่น 2026-05-28 12:00:00)',
    `open` DOUBLE COMMENT 'ราคาเปิดของแท่งเทียน',
    `high` DOUBLE COMMENT 'ราคาสูงสุดของแท่งเทียน',
    `low` DOUBLE COMMENT 'ราคาต่ำสุดของแท่งเทียน',
    `close` DOUBLE COMMENT 'ราคาปิดของแท่งเทียน',
    `color` VARCHAR(20) COMMENT 'สีของแท่งเทียน (Green, Red, Equal)',
    `next_color` VARCHAR(20) NULL COMMENT 'สีของแท่งเทียนถัดไป (ถ้ามี)',
    `pip_size` DOUBLE COMMENT 'ขนาดของเนื้อเทียน (Pip Size)',
    
    `ema_short_value` DOUBLE COMMENT 'ค่า EMA เส้นสั้น',
    `ema_short_direction` VARCHAR(20) COMMENT 'ทิศทางของ EMA เส้นสั้น (Up, Down)',
    `ema_short_turn_type` VARCHAR(20) COMMENT 'รูปแบบการหักหัวของ EMA เส้นสั้น (TurnUp, TurnDown, -)',
    
    `ema_medium_value` DOUBLE COMMENT 'ค่า EMA เส้นกลาง',
    `ema_medium_direction` VARCHAR(20) COMMENT 'ทิศทางของ EMA เส้นกลาง (Up, Down)',
    
    `ema_long_value` DOUBLE COMMENT 'ค่า EMA เส้นยาว',
    `ema_long_direction` VARCHAR(20) COMMENT 'ทิศทางของ EMA เส้นยาว (Up, Down)',
    
    `ema_above` VARCHAR(50) COMMENT 'สถานะเส้นใดอยู่บนระหว่าง Short กับ Medium',
    `ema_long_above` VARCHAR(50) COMMENT 'สถานะเส้นใดอยู่บนระหว่าง Medium กับ Long',
    
    `macd_12` DOUBLE COMMENT 'ค่า MACD Line (12, 26)',
    `macd_23` DOUBLE COMMENT 'ค่า Signal Line (9)',
    
    `previous_ema_short_value` DOUBLE COMMENT 'ค่า EMA เส้นสั้นของแท่งก่อนหน้า',
    `previous_ema_medium_value` DOUBLE COMMENT 'ค่า EMA เส้นกลางของแท่งก่อนหน้า',
    `previous_ema_long_value` DOUBLE COMMENT 'ค่า EMA เส้นยาวของแท่งก่อนหน้า',
    `previous_macd_12` DOUBLE COMMENT 'ค่า MACD Line ของแท่งก่อนหน้า',
    `previous_macd_23` DOUBLE COMMENT 'ค่า Signal Line ของแท่งก่อนหน้า',
    
    `ema_convergence_type` VARCHAR(50) COMMENT 'สถานะการบีบเข้าหรือถ่างออกระหว่าง Short และ Medium (Convergence/Divergence)',
    `ema_long_convergence_type` VARCHAR(50) COMMENT 'สถานะการบีบเข้าหรือถ่างออกระหว่าง Medium และ Long (C/D)',
    
    `choppy_indicator` DOUBLE COMMENT 'ค่า Choppiness Index บอกความผันผวนของตลาด',
    `adx_value` DOUBLE COMMENT 'ค่า ADX บอกความแรงของเทรนด์',
    `rsi_value` DOUBLE COMMENT 'ค่า RSI บอกสภาวะ Overbought/Oversold',
    
    `bb_values` JSON COMMENT 'ค่ากรอบ Bollinger Bands (Upper, Middle, Lower)',
    `bb_position` VARCHAR(50) COMMENT 'ตำแหน่งของราคาเทียบกับ Bollinger Bands (AboveUpper, NearUpper, ฯลฯ)',
    
    `atr_value` DOUBLE COMMENT 'ค่า ATR (Average True Range)',
    `is_abnormal_candle` BOOLEAN COMMENT 'เป็นแท่งเทียนที่ยาวผิดปกติกว่า ATR หรือไม่',
    `is_abnormal_atr` BOOLEAN COMMENT 'เป็นช่วงที่ค่า ATR สวิงรุนแรงผิดปกติหรือไม่',
    
    `u_wick` DOUBLE COMMENT 'ความยาวไส้เทียนด้านบน',
    `u_wick_percent` DOUBLE COMMENT 'สัดส่วนเปอร์เซ็นต์ของไส้เทียนด้านบนต่อทั้งแท่ง',
    `body` DOUBLE COMMENT 'ความยาวเนื้อเทียน',
    `body_percent` DOUBLE COMMENT 'สัดส่วนเปอร์เซ็นต์ของเนื้อเทียนต่อทั้งแท่ง',
    `l_wick` DOUBLE COMMENT 'ความยาวไส้เทียนด้านล่าง',
    `l_wick_percent` DOUBLE COMMENT 'สัดส่วนเปอร์เซ็นต์ของไส้เทียนด้านล่างต่อทั้งแท่ง',
    
    `ema_cut_position` VARCHAR(50) COMMENT 'ตำแหน่งที่ EMA ตัดกัน',
    `ema_cut_long_type` VARCHAR(50) COMMENT 'ประเภทการตัดกันที่เกี่ยวข้องกับเส้น Long',
    `candles_since_ema_cut` INT COMMENT 'จำนวนแท่งเทียนนับตั้งแต่ EMA ตัดกันครั้งล่าสุด',
    
    `ema_short_pos` VARCHAR(20) COMMENT 'ตำแหน่งของ EMA สั้นเทียบกับแท่งเทียน',
    `ema_medium_pos` VARCHAR(20) COMMENT 'ตำแหน่งของ EMA กลางเทียบกับแท่งเทียน',
    `ema_long_pos` VARCHAR(20) COMMENT 'ตำแหน่งของ EMA ยาวเทียบกับแท่งเทียน',
    
    `bb_bandwidth` DOUBLE COMMENT 'ความกว้างของแถบ Bollinger Bands (Bandwidth)',
    `is_bb_squeeze` BOOLEAN COMMENT 'อยู่ในสภาวะ Bollinger Bands บีบตัว (Squeeze) หรือไม่',
    
    `up_con_medium_ema` INT COMMENT 'จำนวนแท่งที่อยู่เหนือ EMA เส้นกลางต่อเนื่อง',
    `down_con_medium_ema` INT COMMENT 'จำนวนแท่งที่อยู่ใต้ EMA เส้นกลางต่อเนื่อง',
    `up_con_long_ema` INT COMMENT 'จำนวนแท่งที่อยู่เหนือ EMA เส้นยาวต่อเนื่อง',
    `down_con_long_ema` INT COMMENT 'จำนวนแท่งที่อยู่ใต้ EMA เส้นยาวต่อเนื่อง',
    
    `is_mark` VARCHAR(100) COMMENT 'เครื่องหมายกำกับพฤติกรรมพิเศษ',
    `status_code` VARCHAR(100) COMMENT 'รหัสสถานะของแท่งเทียนนี้',
    `status_desc` TEXT COMMENT 'คำอธิบายสถานะโดยละเอียด',
    `status_desc_0` TEXT COMMENT 'คำอธิบายสถานะระดับพื้นฐาน',
    `hint_status` VARCHAR(255) COMMENT 'คำแนะนำหรือสถานะรองอื่นๆ',
    `suggest_color` VARCHAR(50) COMMENT 'สีที่ระบบแนะนำให้เทรดในแท่งถัดไป',
    `win_status` VARCHAR(50) COMMENT 'สถานะผลแพ้ชนะ (Win/Loss) หลังเทียบกับ next_color',
    `win_con` INT COMMENT 'จำนวนการชนะติดต่อกัน',
    `loss_con` INT COMMENT 'จำนวนการแพ้ติดต่อกัน (Loss Streak)',
    
    `smc` JSON COMMENT 'ข้อมูล Smart Money Concepts (โครงสร้างราคา, FVG, OB)',
    `tick_volatility` JSON COMMENT 'ความผันผวนย่อยระดับ Tick (Tick count, Buy/Sell ratio)',
    `range_detector` JSON COMMENT 'ข้อมูลการแกว่งตัวในกรอบ (Range Top/Bottom, Breakout)',

    `created_at` TIMESTAMP DEFAULT CURRENT_TIMESTAMP COMMENT 'เวลาที่บันทึกข้อมูลลงฐานข้อมูล'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

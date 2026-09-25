# Long Term Trade — แผนงานที่เหลือ (Remaining Tasks)

> **อัปเดตล่าสุด:** 2026-07-14  
> **สถานะ:** ✅ HTML/UI เสร็จแล้ว — รอทำ Backend & Logic ต่อ

---

## ✅ สิ่งที่ทำเสร็จแล้ว

| รายการ | ไฟล์ | สถานะ |
|---|---|---|
| แผนการพัฒนา Plan 1 | `md/longTermTradePlan1.md` | ✅ เสร็จ |
| หน้า UI ทั้งหมด (HTML/CSS/JS พื้นฐาน) | `public/long_term_trade.html` | ✅ เสร็จ |
| Theme switching (3 themes เหมือน index.html) | ใน HTML | ✅ เสร็จ |
| Tab system (กราฟ, Open Orders, History, Bot Log) | ใน HTML | ✅ เสร็จ |
| Entry Conditions UI (3 checkbox จาก full_analysis_ver2) | ใน HTML | ✅ เสร็จ |
| Exit Strategy UI (3 radio cards) | ใน HTML | ✅ เสร็จ |
| Schedule UI | ใน HTML | ✅ เสร็จ |
| Live Signal + Indicator Gauges | ใน HTML | ✅ เสร็จ |
| Chart placeholder (LightweightCharts) | ใน HTML | ✅ เสร็จ |
| ฟังก์ชัน `updateSignalDisplay(analysis)` | ใน HTML | ✅ เสร็จ |
| ปุ่มลิงก์ "↩ กลับหน้า Multiplex" | ใน HTML | ✅ เสร็จ |

---

## 🔲 งานที่เหลือ — เรียงตามลำดับความสำคัญ

### Phase 1: เพิ่มปุ่มลิงก์ใน index.html เดิม
- [x] เพิ่มปุ่มที่ Header ของ `index.html` ลิงก์ไปยัง `long_term_trade.html`
- [x] เช่น `<a class="nav-btn" href="long_term_trade.html">📈 Long Term Trade</a>`
- [x] Timeframe ของกราฟ เพิ่มให้แสดง 1M ได้ด้วย 
- [x] แสดงกราฟแบบ tick data พร้อมทั้ง update กราฟทุกๆ  2 second
- [x] การเลือก asset ให้มี ได้มากกว่า 1 รายการโดยใช้ check box แบบใน index.html และการแสดงกราฟให้มี ปุ่มเลือก asset ที่จะแสดงได้ 
---

### Phase 2: Backend — WebSocket & Data Fetching
- [x] สร้าง WebSocket connection ไปยัง Deriv API (หรือใช้ Backend Rust เดิม)
- [x] Subscribe Candles (OHLC) ตาม Timeframe ที่เลือก (granularity: 900/1800/3600/7200/86400)
- [x] เมื่อได้ข้อมูล Candle → ส่งไปคำนวณ `full_analysis_ver2::perform_analysis()`
- [x] ส่งผลลัพธ์ `FullAnalysisResult` กลับมาที่หน้า HTML ผ่าน WebSocket

#### Fields สำคัญจาก FullAnalysisResult ที่ต้องใช้:
```
ema_cut_position          → "CrossUp" / "CrossDown" / "-"   (Short × Medium)
ema_cut_long_type         → "CrossUp" / "CrossDown" / "-"   (Medium × Long)
ema_cut_short_long_type   → "CrossUp" / "CrossDown" / "-"   (Short × Long)
ema_cut_all_type          → "AllCrossUp" / "AllCrossDown" / "-"
ema_short_value, ema_medium_value, ema_long_value
ema_short_direction, ema_medium_direction, ema_long_direction
ema_above                 → "ShortAbove" / "MediumAbove"
ema_long_above            → "MediumAbove" / "LongAbove"
ema_convergence_type      → "convergence" / "divergence"
adx_value, choppy_indicator, rsi_value
```

---

### Phase 3: Entry Logic (เงื่อนไขการเข้าเทรด)
- [ ] ตรวจสอบ EMA Cross ตามเงื่อนไขที่ผู้ใช้เปิดใช้:
  - Condition 1: `ema_cut_position` = CrossUp → BUY CALL, CrossDown → BUY PUT
  - Condition 2: `ema_cut_all_type` = AllCrossUp → BUY CALL, AllCrossDown → BUY PUT
  - Condition 3: `ema_cut_short_long_type` = CrossUp → BUY CALL, CrossDown → BUY PUT
- [ ] ตรวจ Filter เพิ่มเติม (ถ้าเปิดใช้):
  - ADX Filter: `adx_value >= ltAdxMin` → ถึงจะเข้าเทรด
  - Choppy Filter: `choppy_indicator <= ltChoppyMax` → ถึงจะเข้าเทรด
  - Convergence Filter: `ema_convergence_type == "divergence"` → ถึงจะเข้าเทรด
- [x] ตรวจ Schedule: เวลาปัจจุบันอยู่ในช่วง startTime - stopTime หรือไม่
- [x] ตรวจ Max Open Orders: จำนวน order ที่เปิดอยู่ < maxOrders หรือไม่
- [x] ถ้าผ่านทุกเงื่อนไข → ส่งคำสั่ง Buy ไปยัง Deriv API
- [x] ปุ่มสลับ Timeframe กราฟ 1M, 15M, 1H ทำงานและเชื่อมต่อข้อมูลกับ Backend API `/api/longterm/candles`

---

### Phase 4: Buy Order — ส่งคำสั่งซื้อ
> **💡 หมายเหตุ:** Duration ของการเทรด (Trade Duration) จะแยกเป็นอิสระจาก Timeframe ของกราฟที่แสดงผล เช่น ผู้ใช้สามารถตั้งกราฟให้แสดงที่ 1M แต่ตั้งค่า Trade Duration (ระยะเวลาของสัญญา) เป็น 15M, 30M หรืออื่นๆ ก็ได้
- [x] สร้าง Proposal Request สำหรับ Contract ประเภท CALL/PUT
  - `duration`: ตาม ltDurationSelect (เช่น 900 = 15m, 3600 = 1h)
  - `duration_unit`: "s" (seconds)
  - `amount`: ตาม ltStakeAmount
  - `basis`: "stake"
  - `contract_type`: "CALL" หรือ "PUT"
  - `symbol`: ตาม ltAssetSelect
- [x] ส่ง Buy Request จาก proposal_id ที่ได้
- [x] บันทึก Contract ID + Entry Signal + Entry Time

---

### Phase 5: Exit Logic (เงื่อนไขการขาย)
- [ ] Subscribe `proposal_open_contract` สำหรับทุก Contract ที่เปิดอยู่
- [ ] ตรวจสอบเงื่อนไขขายตาม Exit Strategy ที่เลือก:

#### Exit Strategy 1: Target Profit
- [ ] ดึง `profit` จาก open contract
- [ ] ถ้า `profit >= ltTargetProfit` → ส่งคำสั่ง Sell
- [ ] แสดง Profit Real-time ในตาราง Open Orders

#### Exit Strategy 2: Sell on Next Cross
- [ ] จดจำ Entry Signal (เช่น เข้าเป็น CALL จาก CrossUp)
- [ ] คำนวณ full_analysis ต่อเนื่อง ทุกแท่งเทียนที่ปิด
- [ ] ถ้าเกิด Cross ในทิศทางตรงข้าม (เช่น CrossDown ขณะที่ถือ CALL) → ส่งคำสั่ง Sell
- [ ] ต้องมี Minimum Hold Time (เช่น ถือขั้นต่ำ 1 แท่ง) เพื่อป้องกัน False Signal

#### Exit Strategy 3: Duration Expiry
- [ ] ไม่ต้องทำอะไร — ปล่อยให้ Contract หมดอายุเอง
- [ ] แสดง Countdown ในตาราง Open Orders

---

### Phase 6: Open Orders Tracking
- [ ] เมื่อ Buy สำเร็จ → เพิ่มแถวในตาราง `ltOrdersTable`
- [ ] อัปเดต Real-time: Current Spot, Profit/Loss, เวลาที่เหลือ
- [ ] ปุ่ม Sell (Manual) สำหรับขายด้วยตนเอง
- [ ] ปุ่มตั้ง Target Profit ต่อ Order (ปรับเปลี่ยนได้)
- [ ] เมื่อ Contract ปิด (sell สำเร็จ หรือ หมดอายุ) → ย้ายไปตาราง History

---

### Phase 7: Trade History
- [ ] บันทึกประวัติทุก Trade ลง Memory/LocalStorage
- [ ] แสดงในตาราง `ltHistoryTable`: เวลา, Asset, ประเภท, Duration, Entry Signal, Exit Reason, Stake, Profit, Balance
- [ ] (Optional) บันทึกลง Backend / File

---

### Phase 8: Bot Log
- [ ] Log ทุก Event สำคัญ:
  - `log-signal`: เมื่อเกิด EMA Cross Signal
  - `log-entry`: เมื่อส่งคำสั่ง Buy
  - `log-exit`: เมื่อส่งคำสั่ง Sell / Contract หมดอายุ
  - `log-win` / `log-loss`: ผลลัพธ์ Win/Loss
  - `log-system`: เริ่ม/หยุดระบบ, เปลี่ยน Schedule ฯลฯ
- [ ] แสดงจำนวน Log ที่ Tab Badge

---

### Phase 9: Chart — แสดง EMA & Trade Markers
- [ ] แสดงกราฟ Candlestick ตาม Timeframe ที่เลือก
- [ ] วาดเส้น EMA 3 เส้น (Short=เขียว, Medium=ส้ม, Long=แดง)
- [ ] Toggle แสดง/ซ่อน EMA แต่ละเส้น
- [ ] วาด Marker จุดที่เกิด EMA Cross (สัญลักษณ์ลูกศรขึ้น/ลง)
- [ ] วาด Marker จุดที่ Buy/Sell (Entry/Exit)

---

### Phase 10: Backend Rust (Optional — ถ้าต้องการ)
- [ ] เพิ่ม Route ใหม่ใน `main.rs` สำหรับ Long Term Trade
- [ ] เช่น `/lt/start`, `/lt/stop`, `/lt/status`
- [ ] หรือใช้ WebSocket channel แยกสำหรับ Long Term
- [ ] บันทึก Trade History ลง file (เช่น `tradeData/lt_trades.json`)

---

## 📋 ลำดับการทำงานแนะนำ

```
Phase 1 (ลิงก์ index.html)     → 5 นาที
Phase 2 (WebSocket & Data)      → 2-3 ชั่วโมง
Phase 3 (Entry Logic)           → 1-2 ชั่วโมง
Phase 4 (Buy Order)             → 1 ชั่วโมง
Phase 5 (Exit Logic)            → 2-3 ชั่วโมง
Phase 6 (Open Orders Tracking)  → 1-2 ชั่วโมง
Phase 7 (Trade History)         → 30 นาที
Phase 8 (Bot Log)               → 30 นาที
Phase 9 (Chart EMA & Markers)   → 1-2 ชั่วโมง
Phase 10 (Backend Rust)         → 2-3 ชั่วโมง (Optional)
────────────────────────────────────────────
รวมประมาณ                       → 10-16 ชั่วโมง
```

---

## ⚠️ สิ่งที่ต้องตัดสินใจก่อนทำต่อ

1. **WebSocket:** จะต่อตรงจาก HTML ไปยัง Deriv API เลย หรือจะผ่าน Backend Rust?
   - ถ้าผ่าน Rust: ต้องเพิ่ม Route + Logic ฝั่ง Backend
   - ถ้าต่อตรง: ต้องใช้ Deriv API Token จาก Frontend
2. **การคำนวณ full_analysis_ver2:** จะคำนวณฝั่งไหน?
   - ฝั่ง Backend (Rust): ส่ง Candle ไปให้ Rust คำนวณแล้วส่งผลกลับ
   - ฝั่ง Frontend (JS): Port Logic ของ full_analysis_ver2 มาเป็น JavaScript
   - **แนะนำ:** ใช้ฝั่ง Backend (Rust) เพราะมีโค้ดพร้อมแล้ว + แม่นยำกว่า
3. **Duration Unit:** Deriv API รองรับ duration "1d" (1 day) สำหรับ Synthetic Index หรือไม่? ต้องเช็ค Contract ที่เปิดได้

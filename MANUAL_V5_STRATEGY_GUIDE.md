# คู่มือระบบวิเคราะห์และกลยุทธ์เทรด Version ใหม่ (V5 Engine & Predictive Sequence Guide)
**Project**: `turbo-indicators_v2 / indicators_Multiplex_Ver1`  
**Document Version**: 5.0.0 (Rust Native Architecture)  
**Last Updated**: 2026-08-30  

---

## 📑 สารบัญ (Table of Contents)
1. [ภาพรวมการอัปเกรด (Overview)](#1-ภาพรวมการอัปเกรด-overview)
2. [โครงสร้างโมดูลและไฟล์ในระบบ (Module Architecture)](#2-โครงสร้างโมดูลและไฟล์ในระบบ-module-architecture)
3. [คู่มือ 27 รูปแบบ Price Action (PKTrend V5 Reference)](#3-คู่มือ-27-รูปแบบ-price-action-pktrend-v5-reference)
4. [คู่มือระบบทำนายและคู่รหัส 2-Bar Sequence Action (Forecast Sequence Reference)](#4-คู่มือระบบทำนายและคู่รหัส-2-bar-sequence-action-forecast-sequence-reference)
5. [การใช้งานผ่านหน้าจอ `index_short_term.html`](#5-การใช้งานผ่านหน้าจอ-index_short_termhtml)
6. [การเปรียบเทียบกลยุทธ์ (Strategy Comparison Dashboard)](#6-การเปรียบเทียบกลยุทธ์-strategy-comparison-dashboard)
7. [คำสั่งทดสอบและคอมไพล์ (Testing & Compilation Commands)](#7-คำสั่งทดสอบและคอมไพล์-testing--compilation-commands)

---

## 1. ภาพรวมการอัปเกรด (Overview)

ระบบเวอร์ชันใหม่ได้ทำการแปลงการคำนวณของ Indicative Engine จาก JavaScript ([`detectTrend_V5.js`](file:///d:/Rust/dynamicChart/detectTrend_V5.js) และ [`predictNextCandle.js`](file:///d:/Rust/dynamicChart/predictNextCandle.js)) มาเป็น **Rust Native Code** ที่มีความเร็วสูงระดับ Microsecond (SIMD-accelerated / Zero-overhead)

### 💡 จุดเด่นสำคัญ:
* **ไม่ทับโค้ดเดิม (Side-by-Side Coexistence)**: โค้ดเดิมทั้งหมด (`pkDetectTrend.rs`, `V1`, `V2`, `V3A-C`, `FTA`, `FTB`) ยังอยู่ครบ 100% สามารถเปิดบอทเทรดและเปรียบเทียบผลลัพธ์กันแบบ Real-time ได้
* **ครอบคลุม 27 Price Action Cases**: รองรับแท่งเทียนทุกสภาวะตลาด (ขาขึ้น, ขาลง, ไส้ยาวหลอก, กับดัก Bull/Bear Trap, สลับสี Whipsaw)
* **2-Bar Sequence Action Engine**: แมตช์คู่รหัสระหว่างแท่งก่อนหน้าและแท่งปัจจุบัน เพื่อหาจุดกลับตัวที่เฉียบคม
* **Tick Microstructure Analysis**: วิเคราะห์การไหลของราคา 10 วินาทีสุดท้ายก่อนปิดแท่ง (Closing Surge / Snapback)

---

## 2. โครงสร้างโมดูลและไฟล์ในระบบ (Module Architecture)

```text
D:\Rust\turbo-indicators\turbo-indicators_v2\indicators_Multiplex_Ver1/
├── src/
│   ├── pkDetectTrend_v5.rs      ← [NEW] โมดูล Price Action V5 (27 Cases, Structure, Whipsaw, Trend Score)
│   ├── predict_next_candle.rs   ← [NEW] โมดูลทำนายแท่งถัดไป & 2-Bar Sequence Action
│   ├── getActionByPKTrend.rs    ← [ORIGINAL] โมดูล PKTrend เดิม (V1)
│   ├── get_action.rs            ← Dispatcher กลยุทธ์ (เพิ่ม PKTrendV5 & ForecastSequence)
│   ├── deriv.rs                 ← Execution Bot & บันทึก Strategy Comparison
│   └── main.rs                  ← WebSocket Server & API Handlers
└── public/
    ├── index_short_term.html    ← หน้าจอควบคุมหลัก, Strategy Selector, Comparison Table
    └── pkderiv.js               ← Logic จัดการหน้าจอ, Diagnostic Modal, WebSocket Client
```

---

## 3. คู่มือ 27 รูปแบบ Price Action (PKTrend V5 Reference)

โมดูล [`pkDetectTrend_v5.rs`](file:///D:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/pkDetectTrend_v5.rs) จำแนกแท่งเทียนออกเป็น **27 รูปแบบ** พร้อมการคำนวณ **Trend Score (-100 ถึง +100)**:

| No. | Case Code | Category | ทิศทาง (Trend) | คำอธิบายรูปแบบ | Action แนะนำ |
|:---:|:---|:---:|:---:|:---|:---:|
| **1** | `UP-CONFIRM` | Up | UpTrend | แท่งเขียวเต็มแท่ง ทะลุ High เดิม ปิดชิดขอบบน | 🟢 **CALL** |
| **2** | `DN-CONFIRM` | Down | DownTrend | แท่งแดงเต็มแท่ง หลุด Low เดิม ปิดชิดขอบล่าง | 🔴 **PUT** |
| **3** | `SPK-CONTINUE-UP` | Spike | UpTrend | แท่งเขียวยาวพุ่งแรง (High Volatility) ปิดแน่นบน | 🟢 **CALL** |
| **4** | `SPK-CONTINUE-DN` | Spike | DownTrend | แท่งแดงยาวทุบแรง (High Volatility) ปิดแน่นล่าง | 🔴 **PUT** |
| **5** | `SPK-BULLTRAP` | Spike Trap | Rejected | เขียวพุ่งขึ้นหลอก แล้วทิ้งไส้บนยาว ปิดต่ำ | 🔴 **PUT** (Reversal) |
| **6** | `SPK-BEARTRAP` | Spike Trap | Rejected | แดงทุบลงหลอก แล้วดึงไส้ล่างยาว ปิดสูง | 🟢 **CALL** (Reversal) |
| **7** | `SPK-NODIRECTION` | Spike | Sideways | ไส้บนและล่างยาวทั้งคู่ ไร้ทิศทางชัดเจน | ⏳ **WAIT** (Idle) |
| **8** | `SPK-HESITANT-UP` | Spike | Sideways | พุ่งขึ้นแต่เนื้อเทียนเล็ก ลังเลบริเวณแนวต้าน | ⏳ **WAIT** (Idle) |
| **9** | `RJ-FOLLOWFAIL-UP` | Rejection | Sideways | พยายามขึ้นแต่ผ่าน High เดิมไม่ได้ โดนกดกลับ | ⏳ **WAIT** / 🔴 **PUT** |
| **10** | `RJ-FOLLOWFAIL-DN` | Rejection | Sideways | พยายามลงแต่ผ่าน Low เดิมไม่ได้ โดนเด้งกลับ | ⏳ **WAIT** / 🟢 **CALL** |
| **11** | `RJ-BULLTRAP-STRONG` | Trap | Rejected | ทะลุ High เดิม (New High) แต่โดนทุบกลับมาปิดล่าง | 🔴 **PUT** (Strong Reversal) |
| **12** | `RJ-BEARTRAP-STRONG` | Trap | Rejected | หลุด Low เดิม (New Low) แต่มีแรงซื้อดีดกลับมาปิดบน | 🟢 **CALL** (Strong Reversal) |
| **13** | `RJ-LOWWICK-WEAK` | Rejection | UpTrend | แท่งเขียวมีไส้ล่างยาว ปฏิเสธราคาต่ำ (Dip Buy) | 🟢 **CALL** |
| **14** | `RJ-UPWICK-WEAK` | Rejection | DownTrend | แท่งแดงมีไส้บนยาว ปฏิเสธราคาสูง (Rally Sell) | 🔴 **PUT** |
| **15** | `DJ-DOJI` | Indecision | Sideways | ราคาเปิดและปิดเกือบเท่ากัน (Doji แท้) | ⏳ **WAIT** |
| **16** | `DJ-HIGHWICK-REV` | Indecision | Rejected | โดจิไส้บนยาว ชนแนวต้านแล้วหมดแรง | 🔴 **PUT** |
| **17** | `DJ-LOWWICK-REV` | Indecision | Rejected | โดจิไส้ล่างยาว ชนแนวรับแล้วมีแรงเด้ง | 🟢 **CALL** |
| **18** | `IB-INSIDEBAR` | Breakout Pending | Sideways | แท่งลูกอยู่ในกรอบของแท่งแม่ (Inside Bar) | ⏳ **WAIT** |
| **19** | `SD-CHOPPY` | Sideways | Sideways | ไซด์เวย์กรอบแคบ วิ่งสะเปะสะปะ | ⏳ **WAIT** |
| **20** | `SYS-NODATA` | System | N/A | ข้อมูลแท่งเทียนไม่เพียงพอ (น้อยกว่า 3 แท่ง) | ⏳ **WAIT** |
| **21** | `EG-BULLISH` | Engulfing | UpTrend | แท่งเขียวกลืนกินแท่งแดงก่อนหน้ามิดทั้งแท่ง | 🟢 **CALL** |
| **22** | `EG-BEARISH` | Engulfing | DownTrend | แท่งแดงกลืนกินแท่งเขียวก่อนหน้ามิดทั้งแท่ง | 🔴 **PUT** |
| **23** | `SD-FLATSUPPORT` | Structure | UpTrend | ชนแนวรับเดิม 2 ครั้งไม่หลุด (Double Bottom) | 🟢 **CALL** |
| **24** | `SD-FLATRESISTANCE` | Structure | DownTrend | ชนแนวต้านเดิม 2 ครั้งไม่ผ่าน (Double Top) | 🔴 **PUT** |
| **25** | `EG-INDECISION` | Engulfing | Sideways | กลืนกินแต่ไส้ยาวทั้งสองข้าง ไม่เลือกทาง | ⏳ **WAIT** |
| **26** | `SPK-HESITANT-DN` | Spike | Sideways | ทุบลงแต่เนื้อเทียนเล็ก ลังเลบริเวณแนวรับ | ⏳ **WAIT** |
| **27** | `SPK-WHIPSAW` | Whipsaw | Sideways | สลับสีรุนแรง เกิด False Breakout ซ้ำซ้อน | ⏳ **WAIT** (Block Trade) |

### 🛡️ ระบบ Whipsaw 4-Stage Guard
จำแนกการสลับสีแท่งเทียนเพื่อความปลอดภัยสูงสุด:
1. `TRENDING` : สีเรียงตัวสม่ำเสมอ (เทรดได้ปกติ)
2. `CHOPPY` : เริ่มมีการสลับสี 2 แท่ง
3. `ENTERING_WHIPSAW` : สลับสี 3 แท่งติดต่อกัน (ลด Position Size หรือระวัง)
4. `CONFIRMED_WHIPSAW` : สลับสี 4 แท่งติดต่อกัน (**บล็อกการเทรดอัตโนมัติ ส่งสัญญาณ IDLE**)

---

## 4. คู่มือระบบทำนายและคู่รหัส 2-Bar Sequence Action (Forecast Sequence Reference)

โมดูล [`predict_next_candle.rs`](file:///D:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/predict_next_candle.rs) จะวิเคราะห์ความสัมพันธ์ระหว่าง **แท่งก่อนหน้า (Prev Bar)** ➔ **แท่งปัจจุบัน (Curr Bar)**

### ตัวอย่างคู่รหัสยอดนิยม (High-Probability Pairs):
* **`1 ➔ 11` (UP-CONFIRM ➔ RJ-BULLTRAP-STRONG)**:
  * *พฤติกรรม*: แท่งแรกขึ้นแรง แท่งสองทำ New High แต่ถูกทุบกลับมาปิดติดล่าง
  * *สัญญาณ*: 🔴 **PUT** (สับสวิตช์สวนทันที Confidence: HIGH, Score: 85)
* **`2 ➔ 12` (DN-CONFIRM ➔ RJ-BEARTRAP-STRONG)**:
  * *พฤติกรรม*: แท่งแรกทุบหนัก แท่งสองทำ New Low แต่ถูกซื้อลากกลับมาปิดติดบน
  * *สัญญาณ*: 🟢 **CALL** (สับสวิตช์สวนทันที Confidence: HIGH, Score: 85)
* **`1 ➔ 1` (UP-CONFIRM ➔ UP-CONFIRM)**:
  * *พฤติกรรม*: ขาขึ้นทรงพลังต่อเนื่อง Higher High + Higher Close
  * *สัญญาณ*: 🟢 **CALL** (Follow Momentum, Score: 80)
* **`2 ➔ 2` (DN-CONFIRM ➔ DN-CONFIRM)**:
  * *พฤติกรรม*: ขาลงทรงพลังต่อเนื่อง Lower Low + Lower Close
  * *สัญญาณ*: 🔴 **PUT** (Follow Momentum, Score: 80)
* **`17 ➔ 1` (DJ-LOWWICK-REV ➔ UP-CONFIRM)**:
  * *พฤติกรรม*: โดจิเด้งรับ ➔ ยืนยันแท่งเขียวตาม
  * *สัญญาณ*: 🟢 **CALL** (Morning Reversal Pattern)

---

## 5. การใช้งานผ่านหน้าจอ `index_short_term.html`

### 1) การเลือก Strategy
ไปที่ส่วน **🎯 Trading Strategy** บนแถบด้านบนขวา:
* เลือก **`PKTrend V5: 27 Patterns & Structure Engine`** เพื่อให้บอทตัดสินใจตามระบบ Price Action V5
* เลือก **`Forecast: 2-Bar Sequence Action (คู่รหัส)`** เพื่อให้บอทตัดสินใจตามระบบทำนายคู่รหัส

### 2) ตรวจสอบเงื่อนไข (Condition Codes)
คลิกที่ปุ่ม **"📋 View Condition Code Description"** เพื่อดูรายละเอียดรหัสสัญญาณ เช่น `PKT5-UP-CONFIRM`, `FC-PUT-BULLTRAP`, `PKT5-WHIPSAW-BLOCK`

---

## 6. การเปรียบเทียบกลยุทธ์ (Strategy Comparison Dashboard)

เปิดแท็บ **Strategy Comparison** เพื่อดูสถิติ Real-time ของทุก Strategy ที่รันขนานกัน:
1. **คอลัมน์ตาราง**:
   * แสดง `Suggest Color` / `Loss Streak ปัจจุบัน` / `ผล Win-Loss` ของแต่ละแท่งเทียน
   * เปรียบเทียบครบทั้ง **V1, V2, V3A, V3B, V3C, FTA, FTB, PKTrend V1, PKTrend V5, Forecast**
2. **🏆 Max Loss Streak Summary**:
   * แถวบนสุดจะสรุป **จำนวนไม้ที่แพ้ติดกันสูงสุด (Max Loss Streak)** ของทั้งวัน พร้อมระบุ Asset ที่เกิด
3. **ปุ่มแว่นขยาย 🔍 (Diagnostic Modal)**:
   * คลิกเพื่อดู **Reason (เหตุผลที่เข้าเทรด)** และ **Loss Factor (ปัจจัยที่ทำให้แพ้ เช่น ATR Spike หรือ Choppy)**

---

## 7. คำสั่งทดสอบและคอมไพล์ (Testing & Compilation Commands)

### 1) รัน Unit Tests ทั้งหมด (53 Tests)
```bash
cargo test --bin turbo-indicators
```

### 2) คอมไพล์และรัน Server
```bash
cargo run
```

### 3) ตรวจสอบความถูกต้องของโค้ด (Type Check)
```bash
cargo check
```

---
*จัดทำขึ้นเพื่อให้ทีมพัฒนาและผู้ใช้งานสามารถตรวจสอบ เปรียบเทียบ และปรับแต่งกลยุทธ์ได้อย่างถูกต้องและมีประสิทธิภาพสูงสุด*

// =============================================================================
// 📄 get_actionJS.js
// =============================================================================
// แปลงมาจาก: get_action.rs (Rust)
// วัตถุประสงค์: ระบบตัดสินใจการเทรด (Trade Decision Engine)
//              ใช้วิเคราะห์แท่งเทียนว่าควร Call, Put หรือ Wait
//              โดยอิงจากกลยุทธ์ ATR Spike + Martingale color strategy
//              ตรงตามต้นฉบับ PHP `clsATRCandleAnalyzer.php`
// =============================================================================

// =============================================================================
// 📌 SECTION 1: TradeAction — Enum สำหรับผลลัพธ์การตัดสินใจเทรด
// =============================================================================
// ใน Rust ใช้ enum TradeAction { Call, Put, Wait }
// ใน JavaScript ไม่มี enum จริง ๆ จึงใช้ Object.freeze() เพื่อล็อกค่าไม่ให้ถูกแก้ไข
// ทำให้เสมือนเป็น enum ที่ไม่สามารถเปลี่ยนแปลงค่าได้ (immutable)
//
// ความหมายของแต่ละค่า:
//   - "CALL" → ซื้อฝั่ง CALL (คาดว่าราคาจะขึ้น / แท่งสีเขียว)
//   - "PUT"  → ซื้อฝั่ง PUT  (คาดว่าราคาจะลง / แท่งสีแดง)
//   - "WAIT" → ยังไม่เข้าเทรด รอสัญญาณที่ชัดเจนกว่านี้
// =============================================================================
const TradeAction = Object.freeze({
    CALL: "CALL",   // เทียบเท่า TradeAction::Call ใน Rust
    PUT:  "PUT",    // เทียบเท่า TradeAction::Put  ใน Rust
    WAIT: "WAIT",   // เทียบเท่า TradeAction::Wait ใน Rust
});


// =============================================================================
// 📌 SECTION 2: AnalysisObject — โครงสร้างข้อมูลวิเคราะห์ 1 แท่งเทียน
// =============================================================================
// ใน Rust ใช้ struct AnalysisObject พร้อม derive(Serialize, Deserialize)
// ใน JavaScript ใช้ class เพื่อรับข้อมูลวิเคราะห์จาก Indicator Engine (lib.rs)
//
// ฟิลด์ทั้งหมดสะท้อนข้อมูลที่ได้จากการคำนวณ Indicator ต่าง ๆ เช่น:
//   - ข้อมูล OHLC (Open, High, Low, Close) ของแท่งเทียน
//   - ทิศทาง EMA (Exponential Moving Average) ระยะสั้น/กลาง/ยาว
//   - ค่า ATR (Average True Range) สำหรับวัดความผันผวน
//   - ค่า Choppiness Index สำหรับวัดว่าตลาด Sideways หรือ Trending
//   - ค่า ADX (Average Directional Index) สำหรับวัดความแข็งแรงของเทรนด์
//   - ค่า RSI (Relative Strength Index) สำหรับวัด Overbought/Oversold
// =============================================================================
class AnalysisObject {
    /**
     * สร้าง AnalysisObject จาก object ข้อมูลดิบ
     * 
     * @param {Object} data - ข้อมูลวิเคราะห์แท่งเทียน 1 แท่ง (มาจาก Indicator Engine)
     * 
     * ⚠️ หมายเหตุเรื่อง serde rename:
     *   ใน Rust ใช้ #[serde(rename = "timeDisplay")] เพื่อแปลงชื่อฟิลด์
     *   ตอน serialize/deserialize JSON
     *   ใน JavaScript ใช้ชื่อ camelCase ตรง ๆ ได้เลย เพราะ JSON ที่ส่งมา
     *   จะเป็น camelCase อยู่แล้ว (ตาม serde rename ฝั่ง Rust)
     */
    constructor(data = {}) {
        // ──────────────────────────────────────────────────────────────
        // 🕐 ข้อมูลเวลาและสินทรัพย์
        // ──────────────────────────────────────────────────────────────
        this.epoch = data.epoch || 0;               // Unix timestamp ของแท่งเทียน (หน่วยวินาที)
        this.asset = data.asset || "";              // ชื่อสินทรัพย์ เช่น "R_100", "R_75"
        this.timeDisplay = data.timeDisplay || "";  // เวลาที่แสดงผลอ่านง่าย เช่น "14:30:00"
                                                    // (Rust: time_display → serde rename → timeDisplay)

        // ──────────────────────────────────────────────────────────────
        // 📊 ข้อมูล OHLC (Open-High-Low-Close)
        // ──────────────────────────────────────────────────────────────
        this.open  = data.open  || 0;  // ราคาเปิดของแท่งเทียน
        this.high  = data.high  || 0;  // ราคาสูงสุดของแท่งเทียน
        this.low   = data.low   || 0;  // ราคาต่ำสุดของแท่งเทียน
        this.close = data.close || 0;  // ราคาปิดของแท่งเทียน

        // ──────────────────────────────────────────────────────────────
        // 🔥 ATR Spike Detection
        // ──────────────────────────────────────────────────────────────
        this.is_atr = data.is_atr || false;         // true = แท่งนี้เป็น ATR Spike (ความผันผวนสูงผิดปกติ)
                                                     // เป็นเงื่อนไขหลักในการตัดสินใจเข้าเทรด
                                                     // ถ้า false → ระบบจะ return Wait ทันที

        // ──────────────────────────────────────────────────────────────
        // 🎨 สีของแท่งเทียน
        // ──────────────────────────────────────────────────────────────
        this.this_color = data.this_color || "";     // สีแท่งเทียนปัจจุบัน: "green" (ขึ้น) หรือ "red" (ลง)
                                                     // ใช้ในกลยุทธ์ Martingale เมื่อ lossCon >= 3

        // ──────────────────────────────────────────────────────────────
        // 📈 EMA (Exponential Moving Average) — ค่าเฉลี่ยเคลื่อนที่แบบเอ็กซ์โพเนนเชียล
        // ──────────────────────────────────────────────────────────────
        this.ema_short_direction  = data.ema_short_direction  || "";  // ทิศทาง EMA ระยะสั้น: "up" หรือ "down"
        this.ema_medium_direction = data.ema_medium_direction || "";  // ทิศทาง EMA ระยะกลาง: "up" หรือ "down"
        this.ema_short_val        = data.ema_short_val        !== undefined ? data.ema_short_val : (data.ema_short_value || 0);   // ค่า EMA ระยะสั้น (ตัวเลข)
        this.ema_medium_val       = data.ema_medium_val       !== undefined ? data.ema_medium_val : (data.ema_medium_value || 0);   // ค่า EMA ระยะกลาง (ตัวเลข)
        this.ema_long_val         = data.ema_long_val         !== undefined ? data.ema_long_val : (data.ema_long_value || 0);   // ค่า EMA ระยะยาว (ตัวเลข)
        this.emaAbove             = data.emaAbove             || "";  // EMA อยู่เหนือ/ใต้ราคา: "above" หรือ "below"
                                                                      // (Rust: ema_above → serde rename → emaAbove)
        this.emaLongDirection     = data.emaLongDirection     || "";  // ทิศทาง EMA ระยะยาว: "up" หรือ "down"
                                                                      // (Rust: ema_long_direction → serde rename → emaLongDirection)

        // ──────────────────────────────────────────────────────────────
        // 📏 Indicator ตัวชี้วัดเพิ่มเติม
        // ──────────────────────────────────────────────────────────────
        this.atrValue        = data.atrValue        || 0;  // ค่า ATR (Average True Range) — วัดความผันผวน
                                                            // (Rust: atr_value → serde rename → atrValue)
        this.choppiNessIndex = data.choppiNessIndex || 0;  // ค่า Choppiness Index — วัดว่าตลาด Sideway หรือ Trending
                                                            // ยิ่งสูง = ยิ่ง Sideway, ยิ่งต่ำ = ยิ่ง Trending
                                                            // (Rust: choppiness_index → serde rename → choppiNessIndex)
        this.adx             = data.adx             || 0;  // ค่า ADX (Average Directional Index)
                                                            // วัดความแข็งแรงของเทรนด์ (ไม่สนทิศทาง)
                                                            // > 25 = เทรนด์แข็ง, < 20 = ตลาด Sideways
        this.rsi             = data.rsi             || 0;  // ค่า RSI (Relative Strength Index)
                                                            // > 70 = Overbought (ซื้อมากเกินไป)
                                                            // < 30 = Oversold  (ขายมากเกินไป)
    }
}


// =============================================================================
// 📌 SECTION 3: getSuggestColor() — กลยุทธ์หาสีที่ควรเทรด
// =============================================================================
// แปลงมาจาก: fn get_suggest_color(this_color: &str, loss_con: u32) -> String
//
// ── หลักการทำงาน (ตรงตาม PHP: findWinOnColor + getSuggestColor) ──
//
// กลยุทธ์นี้เป็น "Strategy Type 2" ซึ่งทำงานดังนี้:
//
// 1. ค่าเริ่มต้น: suggest = "green" เสมอ (เน้นเล่นฝั่ง CALL เป็นหลัก)
//    → เมื่อ lossCon = 0, 1, 2 จะได้ "green" ตลอด ไม่ว่าสีแท่งจะเป็นอะไร
//
// 2. Martingale Override (lossCon >= 3):
//    → เปลี่ยนกลยุทธ์! ไม่ยึดสีเขียวอีกต่อไป
//    → เล่นตามสีของแท่งเทียนปัจจุบัน:
//       - แท่งสีแดง → suggest = "red"  → จะออก PUT
//       - แท่งสีเขียว → suggest = "green" → จะออก CALL
//
// ── ทำไมถึงออกแบบแบบนี้? ──
//    - lossCon 0-2: ระบบยังเชื่อมั่นในทิศทาง CALL → ยืนหยัดไม่เปลี่ยน
//    - lossCon >= 3: แพ้ติดกัน 3 ครั้ง → สัญญาณว่าตลาดอาจเปลี่ยนทิศ
//      → เปลี่ยนไปเล่นตามสีแท่งปัจจุบันแทน (reactive strategy)
//
// @param {string} thisColor - สีของแท่งเทียนปัจจุบัน ("green" หรือ "red")
// @param {number} lossCon   - จำนวนครั้งที่แพ้ติดกัน (loss consecutive count)
// @returns {string}         - สีที่แนะนำ: "green" หรือ "red"
// =============================================================================
function getSuggestColor(thisColor, lossCon) {
    // Strategy Type 2: ค่าเริ่มต้น → suggest "green" เสมอ
    // (ตรงตาม Rust: let mut suggest = "green".to_string();)
    let suggest = "green";

    // Martingale override:
    // PHP เรียก getSuggestColor เฉพาะเมื่อ lossCon >= 3
    // เมื่อ lossCon >= 3 → เปลี่ยนไปเล่นตามสีแท่งเทียนปัจจุบัน
    // (ตรงตาม Rust: if loss_con >= 3 { ... })
    if (lossCon >= 3) {
        if (thisColor === "red") {
            // แท่งสีแดง (ราคาลง) → suggest เล่น red → จะแปลงเป็น PUT
            suggest = "red";
        } else {
            // แท่งสีเขียว (ราคาขึ้น) หรือสีอื่น ๆ → suggest เล่น green → จะแปลงเป็น CALL
            suggest = "green";
        }
    }

    // คืนค่า suggest color กลับไป
    return suggest;
}

// =============================================================================
// 📌 SECTION 4A: getSuggestColorFTA() — กลยุทธ์ FTA (Follow Trend A — 2 EMA Direction)
// =============================================================================
// ดูทิศทาง EMA 2 เส้น (Short + Medium) จาก AnalysisObject
//   - ema_short_direction ≠ ema_medium_direction → "idle" (ไม่เทรด)
//   - ทั้งคู่ = "Up"   → "green" (CALL)
//   - ทั้งคู่ = "Down" → "red"   (PUT)
//
// ⚠️ ไม่ใช้ lossCon ในการตัดสินใจ — ดู EMA direction อย่างเดียว
//
// @param {AnalysisObject} analysis - ข้อมูลวิเคราะห์แท่งเทียน 1 แท่ง
// @returns {string}                - "green", "red", หรือ "idle"
// =============================================================================
function getSuggestColorFTA(analysis) {
    if (analysis.ema_short_direction !== analysis.ema_medium_direction) {
        return "idle";
    }
    if (analysis.ema_short_direction === "Up") {
        return "green";
    }
    return "red";
}


// =============================================================================
// 📌 SECTION 4B: getSuggestColorFTB() — กลยุทธ์ FTB (Follow Trend B — 3 EMA Direction)
// =============================================================================
// ดูทิศทาง EMA 3 เส้น (Short + Medium + Long) จาก AnalysisObject
//   - ema_short_direction ≠ ema_medium_direction → "idle" (ไม่เทรด)
//   - ema_short_direction ≠ ema_long_direction  → "idle" (ไม่เทรด)
//   - ทั้งสาม = "Up"   → "green" (CALL)
//   - ทั้งสาม = "Down" → "red"   (PUT)
//
// ⚠️ ไม่ใช้ lossCon ในการตัดสินใจ — ดู EMA direction อย่างเดียว
//
// @param {AnalysisObject} analysis - ข้อมูลวิเคราะห์แท่งเทียน 1 แท่ง
// @returns {string}                - "green", "red", หรือ "idle"
// =============================================================================
function getSuggestColorFTB(analysis) {
    if (analysis.ema_short_direction !== analysis.ema_medium_direction) {
        return "idle";
    }
    // ต้อง check ema_long_direction ด้วย — ใช้ emaLongDirection (camelCase จาก Rust serde rename)
    const longDir = analysis.emaLongDirection || "";
    if (analysis.ema_short_direction !== longDir) {
        return "idle";
    }
    if (analysis.ema_short_direction === "Up") {
        return "green";
    }
    return "red";
}


// =============================================================================
// 📌 SECTION 5: getTradeAction() — ฟังก์ชันหลักสำหรับตัดสินใจเข้าเทรด
// =============================================================================
// แปลงมาจาก: fn get_trade_action(analysis: &AnalysisObject, loss_con: u32) -> TradeAction
//
// เลียนแบบจาก PHP `clsATRCandleAnalyzer.php`
//
// ── ขั้นตอนการทำงาน ──
//
// ขั้นที่ 1: ตรวจสอบ ATR Spike
//   → ถ้าแท่งนี้ไม่ใช่ ATR Spike (is_atr = false) → return WAIT ทันที
//   → ATR Spike คือแท่งที่มีความผันผวนสูงเกินค่า ATR threshold
//   → ถ้าไม่มี Spike = ตลาดสงบ ไม่มีสัญญาณเทรด
//
// ขั้นที่ 2: หา Suggest Color
//   → เรียก getSuggestColor() เพื่อดูว่าควรเล่นสีอะไร
//   → ขึ้นอยู่กับ lossCon (จำนวนแพ้ติดกัน) และสีแท่งปัจจุบัน
//
// ขั้นที่ 3: แปลง Suggest Color → Trade Action
//   → "green" → CALL (คาดราคาขึ้น)
//   → "red"   → PUT  (คาดราคาลง)
//   → อื่น ๆ  → WAIT (ไม่เทรด — กรณี edge case)
//
// @param {AnalysisObject} analysis - ข้อมูลวิเคราะห์แท่งเทียน 1 แท่ง
// @param {number} lossCon          - จำนวนครั้งที่แพ้ติดกัน (loss consecutive count)
// @returns {string}                - ค่า TradeAction: "CALL", "PUT", หรือ "WAIT"
// =============================================================================
function getTradeAction(analysis, lossCon) {
    const strategyElement = document.getElementById("suggestStrategySelect");
    const strategy = strategyElement ? strategyElement.value : "V1";

    // ──────────────────────────────────────────────────────────────
    // ขั้นที่ 1: ตรวจสอบ ATR Spike (ยกเว้น FTA และ FTB ที่ไม่ต้องรอ ATR Spike)
    // ──────────────────────────────────────────────────────────────
    if (strategy !== "FTA" && strategy !== "FTB") {
        if (!analysis.is_atr) {
            return TradeAction.WAIT;
        }
    }

    // ──────────────────────────────────────────────────────────────
    // ขั้นที่ 2: หา Suggest Color
    // ──────────────────────────────────────────────────────────────
    let suggest;
    switch (strategy) {
        case "FTA": suggest = getSuggestColorFTA(analysis); break;
        case "FTB": suggest = getSuggestColorFTB(analysis); break;
        default: suggest = getSuggestColor(analysis.this_color, lossCon); break;
    }

    // แสดง log สำหรับ debug
    console.log(
        `🎯 Signal Check! strategy=${strategy}, thisColor=${analysis.this_color}, lossCon=${lossCon}, suggest=${suggest}`
    );

    // ──────────────────────────────────────────────────────────────
    // ขั้นที่ 3: แปลง Suggest Color → Trade Action
    // ──────────────────────────────────────────────────────────────
    switch (suggest) {
        case "green":
            return TradeAction.CALL;
        case "red":
            return TradeAction.PUT;
        default:
            return TradeAction.WAIT;
    }
}


// =============================================================================
// 📌 SECTION 6: Export — ส่งออกทุกอย่างเพื่อให้ไฟล์อื่นเรียกใช้ได้
// =============================================================================
// ใน Rust ใช้ `pub` keyword เพื่อ export
// ใน JavaScript ใช้ module.exports (CommonJS) หรือ export (ES Module)
//
// ⚠️ หมายเหตุ: ถ้าใช้ใน browser โดยตรง (ไม่ผ่าน bundler)
//    สามารถลบ block นี้ออกได้ เพราะ function/class จะอยู่ใน global scope อยู่แล้ว
//    แต่ถ้าใช้กับ Node.js หรือ bundler → ต้องใช้ export
// =============================================================================

// ── สำหรับ ES Module (import/export) ──
// export { TradeAction, AnalysisObject, getSuggestColor, getSuggestColorFTA, getSuggestColorFTB, getTradeAction };

// ── สำหรับ CommonJS (require) ──
// module.exports = { TradeAction, AnalysisObject, getSuggestColor, getSuggestColorFTA, getSuggestColorFTB, getTradeAction };

// ── ปัจจุบัน: ใช้แบบ browser global (ไม่ต้อง export) ──
// ทุก function/class/const ด้านบนจะอยู่ใน global scope ของ browser อัตโนมัติ
// สามารถเรียกใช้ได้เลย เช่น:
//   const action = getTradeAction(analysisData, 0);
//   console.log(action); // "CALL", "PUT", หรือ "WAIT"
//   const ftaSuggest = getSuggestColorFTA(analysisData);
//   console.log(ftaSuggest); // "green", "red", หรือ "idle"

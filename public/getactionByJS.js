/**
 * ============================================================================
 * โมดูลวิเคราะห์สีที่ควรเทรด (Trade Action Strategy Engine)
 * ============================================================================
 *
 * โมดูลนี้ทำหน้าที่ในการประเมินและคืนค่าสีที่ควรเทรด (green = Call, red = Put, idle = Wait)
 * โดยการทำงานของทุกๆ กลยุทธ์ (Function) จะต้องการพารามิเตอร์ 2 ตัวหลักที่รับมาจากระบบ ได้แก่:
 *
 * 1. `analysis` (FullAnalysisResult Object)
 *    - แหล่งที่มา: เป็นก้อนข้อมูลออบเจกต์ที่สรุปผลการวิเคราะห์ตลาด (Market Analysis) ของแท่งเทียนปัจจุบัน
 *      ซึ่งโดยปกติจะประมวลผลมาจาก Backend (Rust) หรือระบบรวบรวมอินดิเคเตอร์
 *    - ข้อมูลที่สำคัญที่ถูกนำมาใช้: `color` (สีแท่งเทียนปัจจุบัน), `ema_short_direction`,
 *      `ema_medium_direction`, `ema_long_direction` (ทิศทาง EMA), และ `ema_cut_position`
 *
 * 2. `lossCon` (Number)
 *    - แหล่งที่มา: จำนวนครั้งที่แพ้ต่อเนื่อง (Consecutive Losses) ที่ระบบเก็บสถานะการเทรดเอาไว้
 *    - หน้าที่: ใช้เป็นจุดเปลี่ยนเงื่อนไข (Threshold) บางกลยุทธ์จะทำงานก็ต่อเมื่อแพ้สะสมถึงจุดที่กำหนด
 *
 * ============================================================================
 * 📌 สรุปวิธีการและตรรกะของแต่ละกลยุทธ์ (Strategy Logic)
 * ============================================================================
 *
 * 🟢 [V1] Default Strategy (รูปแบบเดิม)
 *    - วิธีการ: หากจำนวนครั้งที่แพ้ (lossCon) ยังน้อยกว่า 2 จะแนะนำ "green" เสมอ
 *      แต่ถ้าแพ้ตั้งแต่ 2 ครั้งขึ้นไป (lossCon >= 2) ระบบจะเปลี่ยนมาแนะนำให้เทรด "ตามสีของแท่งเทียนปัจจุบัน" (Follow color)
 * ***** ปกติแล้ว (ตอนยังแพ้ไม่ถึง 2 ไม้): ระบบจะบังคับ "Call" เสมอ (ถูกล็อกค่าไว้เป็น "green") ไม่ว่าแท่ง Spike ที่เจอจะเป็นสีอะไรก็ตาม ระบบก็จะไม่สน จะกด Call อย่างเดียว

แต่พอแพ้ครบ 2 ไม้ปุ๊บ (loss_con >= 2): ระบบจะปลดล็อกการบังคับ Call ทิ้งไป แล้วหันมา มองสีของแท่ง Spike ล่าสุดที่เพิ่งเกิด แทน ว่ามันเป็นสีอะไร เพื่อนำมากำหนดทิศทางออเดอร์ใหม่:

ถ้าระบบเจอแท่ง Spike ใหม่เป็น สีแดง 👉 ทิศทางออเดอร์จะเปลี่ยนไปเปิด Put (แทงลง) ทันที
ถ้าระบบเจอแท่ง Spike ใหม่เป็น สีเขียว 👉 ทิศทางออเดอร์จะเปิด Call (แทงขึ้น) ตามเดิม
สรุปสั้นๆ คือ สิ่งที่เปลี่ยนไปคือ "ฝั่งที่จะแทง (Call/Put)" จะเปลี่ยนไปอิงตาม "สีของแท่งเทียน Spike" แทนการหลับหูหลับตากด Call เพียงฝั่งเดียวครับ


ทางแก้: ถ้าระบบเก็บสถิติแท่งเทียนล่าสุดแล้วพบว่ากราฟกำลังสลับสี (ในโค้ดของคุณเหมือนจะมีตัวแปร is_alternating_pattern และ choppy_indicator อยู่) เมื่อเข้าสภาวะนี้ให้ระบบทำ 2 อย่าง:
ตัวเลือก A: หยุดเทรดชั่วคราว (Pause) จนกว่ากราฟจะเบรกเอาต์หลุดกรอบไซด์เวย์
ตัวเลือก B: สลับโหมดเป็น "เทรดสวน (Reversal)" คือเจอ Spike เขียวให้กด Put, เจอ Spike แดงให้กด Call แทน
 *
 * 🟢 [V2] Early React Strategy (ตอบสนองไว)
 *    - วิธีการ: เหมือน V1 ทุกประการ แต่ปรับเกณฑ์ให้ไวกว่าเดิม คือถ้าแพ้แค่ 1 ครั้งขึ้นไป (lossCon >= 1)
 *      ก็จะเปลี่ยนไปเล่นตามสีของแท่งเทียนปัจจุบันทันที
 *
 * 🟢 [V3A] EMA Direction Consensus (โหวตเสียงข้างมากจาก EMA)
 *    - วิธีการ: ทำงานเมื่อแพ้ (lossCon >= 2) โดยใช้เส้น EMA 3 เส้น (Short, Medium, Long) มาโหวตทิศทาง
 *      หากมีอย่างน้อย 2 เส้นชี้ไปในทางเดียวกัน จะแนะนำให้เทรดไปในทิศทางนั้น (Up = green, Down = red)
 *
 * 🟢 [V3B] EMA Short Direction + CutType Hybrid (ทิศทางเส้นสั้นผสมจุดตัด)
 *    - วิธีการ: ทำงานเมื่อแพ้ (lossCon >= 2) ดูทิศทางเส้น EMA Short เป็นหลัก แต่ถ้าในจังหวะนั้น
 *      มีจุดตัดของเส้น EMA (CutUp / CutDown) เกิดขึ้น จะนำสัญญาณจุดตัดนั้นมาใช้งานแทน (Override)
 *
 * 🟢 [V3C] EMA Medium Direction Only (เส้นกลางเส้นเดียว)
 *    - วิธีการ: ทำงานเมื่อแพ้ (lossCon >= 2) โดยจะดูทิศทางของเส้น EMA Medium เพียงเส้นเดียวเพื่อลดสัญญาณรบกวน
 *      ถ้าชี้ Up = green, ชี้ Down = red
 *
 * 🟢 [FTA] Follow Trend A (ตามเทรนด์ระยะสั้น-กลาง)
 *    - วิธีการ: ไม่สนใจ lossCon ระบบจะเปรียบเทียบทิศทางของ EMA Short และ EMA Medium
 *      ถ้าทิศทางตรงกันทั้งคู่ถึงจะแนะนำให้เทรดตามเทรนด์นั้น หากขัดแย้งกันจะแนะนำให้หยุดเทรด ("idle")
 *
 * 🟢 [FTB] Follow Trend B (ตามเทรนด์ระยะสั้น-กลาง-ยาว)
 *    - วิธีการ: ไม่สนใจ lossCon เหมือน FTA แต่เข้มงวดกว่า โดยจะเช็ค EMA ทั้ง 3 เส้น (Short, Medium, Long)
 *      ต้องชี้ไปในทิศทางเดียวกันทั้งหมดจึงจะให้เทรด หากมีแม้แต่เส้นเดียวขัดแย้งกัน จะแนะนำให้หยุดเทรด ("idle")
 *
 * ============================================================================
 */

/**
 * @typedef {Object} StrategyDecision
 * @property {string} suggest_color
 * @property {string} reason
 * @property {string[]} conditions_matched
 * @property {string} code
 */

/**
 * @typedef {Object} FullAnalysisResult
 * @property {string} color
 * @property {string} ema_short_direction
 * @property {string} ema_medium_direction
 * @property {string} ema_long_direction
 * @property {string} ema_cut_position
 * // ... other properties that match the Rust struct
 */

const SuggestStrategy = {
    V1: 'V1',
    V2: 'V2',
    V3A: 'V3A',
    V3B: 'V3B',
    V3C: 'V3C',
    FTA: 'FTA',
    FTB: 'FTB',
    PKTrend: 'PKTrend'
};

// =============================================================================
// 📌 V1 — Default Strategy (เดิม, Backward Compatible)
// =============================================================================
function getSuggestColorV1(analysis, lossCon, analysisArray) {
    let suggest = "green";
    if (lossCon >= 2) {
        if (analysis.color === "red") {
            suggest = "red";
        } else {
            suggest = "green";
        }
    }
    return suggest;
}

function getSuggestColorV1WithReason(analysis, lossCon, analysisArray) {
    let suggest = "green";
    let reason;
    let conditions = [];
    let code = "V1-A";

    if (lossCon >= 2) {
        if (analysis.color === "red") {
            suggest = "red";
            reason = "loss_con >= 2 -> follow thisColor (red)";
            code = "V1-C";
        } else {
            suggest = "green";
            reason = "loss_con >= 2 -> follow thisColor (green)";
            code = "V1-B";
        }
        conditions.push(`loss_con=${lossCon}`);
    } else {
        reason = "loss_con < 2 -> Default (green)";
        conditions.push(`loss_con=${lossCon}`);
    }

    return { suggest_color: suggest, reason, conditions_matched: conditions, code };
}

// =============================================================================
// 📌 V2 — Early React Strategy (ใหม่)
// =============================================================================
function getSuggestColorV2(analysis, lossCon, analysisArray) {
    let suggest = "red";
    if (lossCon === 1 && analysis.color === "green") {
        suggest = "red";
    } else if (lossCon >= 1) {
        if (analysis.color === "red") {
            suggest = "red";
        } else {
            suggest = "green";
        }
    }

    // ═══ Borrow Signal (Whipsaw) — เมื่อแพ้ติดกัน 3 ครั้ง สลับสี ═══
    if (lossCon >= 3) {
        const borrowToggle = document.getElementById('borrowSignalToggle');
        if (borrowToggle && borrowToggle.checked) {
            // Reverse color: whipsaw mode
            if (analysis.color === "red") {
                suggest = "green";
            } else {
                suggest = "red";
            }
        }
    }

    return suggest;
}

function getSuggestColorV2WithReason(analysis, lossCon, analysisArray) {
    let suggest = "red";
    let reason;
    let conditions = [];
    let code = "V2-A";

    if (lossCon === 1 && analysis.color === "green") {
        suggest = "red";
        reason = "loss_con == 1 and first color green -> force put (red)";
        code = "V2-D";
        conditions.push(`loss_con=${lossCon}`);
    } else if (lossCon >= 1) {
        if (analysis.color === "red") {
            suggest = "red";
            reason = "loss_con >= 1 -> follow thisColor (red)";
            code = "V2-C";
        } else {
            suggest = "green";
            reason = "loss_con >= 1 -> follow thisColor (green)";
            code = "V2-B";
        }
        conditions.push(`loss_con=${lossCon}`);
    } else {
        reason = "loss_con < 1 -> Default (red)";
        conditions.push(`loss_con=${lossCon}`);
    }

    // ═══ Borrow Signal (Whipsaw) — เมื่อแพ้ติดกัน 3 ครั้ง สลับสี ═══
    if (lossCon >= 3) {
        const borrowToggle = document.getElementById('borrowSignalToggle');
        if (borrowToggle && borrowToggle.checked) {
            // Reverse color: whipsaw mode
            if (analysis.color === "red") {
                suggest = "green";
            } else {
                suggest = "red";
            }
            reason = `loss_con >= 3 -> Whipsaw detected, reverse color (${suggest})`;
            code = "V2-Borrow";
            conditions.push("borrow_v2_whipsaw");
        }
    }

    return { suggest_color: suggest, reason, conditions_matched: conditions, code };
}

// =============================================================================
// 📌 V3A — EMA Direction Consensus (ใหม่)
// =============================================================================
function getSuggestColorV3A(analysis, lossCon, analysisArray) {
    let suggest = "green";
    if (lossCon >= 2) {
        let upVotes = 0;
        if (analysis.ema_short_direction === "Up") upVotes += 1;
        if (analysis.ema_medium_direction === "Up") upVotes += 1;
        if (analysis.ema_long_direction === "Up") upVotes += 1;

        if (upVotes >= 2) {
            suggest = "green";
        } else {
            suggest = "red";
        }
    }
    return suggest;
}

function getSuggestColorV3AWithReason(analysis, lossCon, analysisArray) {
    let suggest = "green";
    let reason = "Default (green) loss_con < 2";
    let conditions = [];
    let code = "V3A-A";

    if (lossCon >= 2) {
        let upVotes = 0;
        if (analysis.ema_short_direction === "Up") { upVotes += 1; conditions.push("EMA Short Up"); }
        if (analysis.ema_medium_direction === "Up") { upVotes += 1; conditions.push("EMA Medium Up"); }
        if (analysis.ema_long_direction === "Up") { upVotes += 1; conditions.push("EMA Long Up"); }

        if (upVotes >= 2) {
            suggest = "green";
            reason = `Majority EMA Up (${upVotes}/3)`;
            code = "V3A-B";
        } else {
            suggest = "red";
            reason = `Majority EMA Down (${3 - upVotes}/3)`;
            code = "V3A-C";
        }
    } else {
        conditions.push(`loss_con=${lossCon}`);
    }

    return { suggest_color: suggest, reason, conditions_matched: conditions, code };
}

// =============================================================================
// 📌 V3B — EMA Short Direction + CutType Hybrid (ใหม่)
// =============================================================================
function getSuggestColorV3B(analysis, lossCon, analysisArray) {
    let suggest = "green";
    if (lossCon >= 2) {
        if (analysis.ema_short_direction === "Up") {
            suggest = "green";
        } else {
            suggest = "red";
        }

        if (analysis.ema_cut_position === "CutUp") {
            suggest = "green";
        } else if (analysis.ema_cut_position === "CutDown") {
            suggest = "red";
        }
    }
    return suggest;
}

function getSuggestColorV3BWithReason(analysis, lossCon, analysisArray) {
    let suggest = "green";
    let reason = "Default (green) loss_con < 2";
    let conditions = [];
    let code = "V3B-A";

    if (lossCon >= 2) {
        if (analysis.ema_short_direction === "Up") {
            suggest = "green";
            reason = "EMA Short Up";
            conditions.push("EMA Short Up");
            code = "V3B-B";
        } else {
            suggest = "red";
            reason = "EMA Short Down";
            conditions.push("EMA Short Down");
            code = "V3B-C";
        }

        if (analysis.ema_cut_position === "CutUp") {
            suggest = "green";
            reason = "CutUp Override";
            conditions.push("CutUp Signal");
            code = "V3B-D";
        } else if (analysis.ema_cut_position === "CutDown") {
            suggest = "red";
            reason = "CutDown Override";
            conditions.push("CutDown Signal");
            code = "V3B-E";
        }
    } else {
        conditions.push(`loss_con=${lossCon}`);
    }

    return { suggest_color: suggest, reason, conditions_matched: conditions, code };
}

// =============================================================================
// 📌 V3C — EMA Medium Direction Only (ใหม่)
// =============================================================================
function getSuggestColorV3C(analysis, lossCon, analysisArray) {
    let suggest = "green";
    if (lossCon >= 2) {
        if (analysis.ema_medium_direction === "Up") {
            suggest = "green";
        } else {
            suggest = "red";
        }
    }
    return suggest;
}

function getSuggestColorV3CWithReason(analysis, lossCon, analysisArray) {
    let suggest = "green";
    let reason = "Default (green) loss_con < 2";
    let conditions = [];
    let code = "V3C-A";

    if (lossCon >= 2) {
        if (analysis.ema_medium_direction === "Up") {
            suggest = "green";
            reason = "EMA Medium Up";
            conditions.push("EMA Medium Up");
            code = "V3C-B";
        } else {
            suggest = "red";
            reason = `EMA Medium Down (${analysis.ema_medium_direction})`;
            conditions.push(`EMA Medium ${analysis.ema_medium_direction}`);
            code = "V3C-C";
        }
    } else {
        conditions.push(`loss_con=${lossCon}`);
    }

    return { suggest_color: suggest, reason, conditions_matched: conditions, code };
}

// =============================================================================
// 📌 FTA — Follow Trend A (EMA Short + Medium Direction)
// =============================================================================
function getSuggestColorFTA(analysis, lossCon, analysisArray) {
    if (analysis.ema_short_direction !== analysis.ema_medium_direction) {
        return "idle";
    }
    if (analysis.ema_short_direction === "Up") {
        return "green";
    } else {
        return "red";
    }
}

function getSuggestColorFTAWithReason(analysis, lossCon, analysisArray) {
    let conditions = [
        `EMA Short ${analysis.ema_short_direction}`,
        `EMA Medium ${analysis.ema_medium_direction}`,
    ];

    if (analysis.ema_short_direction !== analysis.ema_medium_direction) {
        return {
            suggest_color: "idle",
            reason: `Short(${analysis.ema_short_direction}) ≠ Medium(${analysis.ema_medium_direction}) → Idle`,
            conditions_matched: conditions,
            code: "FTA-C"
        };
    }

    if (analysis.ema_short_direction === "Up") {
        return {
            suggest_color: "green",
            reason: "Short=Medium=Up → CALL",
            conditions_matched: conditions,
            code: "FTA-A"
        };
    } else {
        return {
            suggest_color: "red",
            reason: "Short=Medium=Down → PUT",
            conditions_matched: conditions,
            code: "FTA-B"
        };
    }
}

// =============================================================================
// 📌 FTB — Follow Trend B (EMA Short + Medium + Long Direction)
// =============================================================================
function getSuggestColorFTB(analysis, lossCon, analysisArray) {
    if (analysis.ema_short_direction !== analysis.ema_medium_direction) {
        return "idle";
    }
    if (analysis.ema_short_direction !== analysis.ema_long_direction) {
        return "idle";
    }
    if (analysis.ema_short_direction === "Up") {
        return "green";
    } else {
        return "red";
    }
}

function getSuggestColorFTBWithReason(analysis, lossCon, analysisArray) {
    let conditions = [
        `EMA Short ${analysis.ema_short_direction}`,
        `EMA Medium ${analysis.ema_medium_direction}`,
        `EMA Long ${analysis.ema_long_direction}`,
    ];

    if (analysis.ema_short_direction !== analysis.ema_medium_direction) {
        return {
            suggest_color: "idle",
            reason: `Short(${analysis.ema_short_direction}) ≠ Medium(${analysis.ema_medium_direction}) → Idle`,
            conditions_matched: conditions,
            code: "FTB-C"
        };
    }

    if (analysis.ema_short_direction !== analysis.ema_long_direction) {
        return {
            suggest_color: "idle",
            reason: `Short/Medium(${analysis.ema_short_direction}) ≠ Long(${analysis.ema_long_direction}) → Idle`,
            conditions_matched: conditions,
            code: "FTB-C"
        };
    }

    if (analysis.ema_short_direction === "Up") {
        return {
            suggest_color: "green",
            reason: "Short=Medium=Long=Up → CALL",
            conditions_matched: conditions,
            code: "FTB-A"
        };
    } else {
        return {
            suggest_color: "red",
            reason: "Short=Medium=Long=Down → PUT",
            conditions_matched: conditions,
            code: "FTB-B"
        };
    }
}

// =============================================================================
// 📌 PKTrend — PK Trend Spot Evaluator Strategy
// =============================================================================
function getSuggestColorPKTrend(analysis, lossCon, analysisArray) {
    if (!analysis || !analysis.pkTrend) return "idle";
    const pk = analysis.pkTrend;
    const opt = { minTrendScoreStrong: 40, minCloseConvictionUp: 0.7, maxCloseConvictionDown: 0.3, allowTrapReversal: true, allowSpikeContinuation: true };
    if (typeof evaluatePkTrendEntry === 'function') {
        const res = evaluatePkTrendEntry(pk, opt);
        if (res.signal === 'BUY') return "green";
        if (res.signal === 'SELL') return "red";
        return "idle";
    }
    return "idle";
}

function getSuggestColorPKTrendWithReason(analysis, lossCon, analysisArray) {
    if (!analysis || !analysis.pkTrend) {
        return { suggest_color: "idle", reason: "No pkTrend data", conditions_matched: [], code: "PKT-NODATA" };
    }
    const pk = analysis.pkTrend;
    const opt = { minTrendScoreStrong: 40, minCloseConvictionUp: 0.7, maxCloseConvictionDown: 0.3, allowTrapReversal: true, allowSpikeContinuation: true };
    const conditions = [
        `Trend: ${pk.trend || 'None'}`,
        `Score: ${pk.trendScore}`,
        `CaseCode: ${pk.caseCode}`
    ];
    if (pk.closePosition !== undefined && pk.closePosition !== null) {
        conditions.push(`ClosePos: ${Number(pk.closePosition).toFixed(2)}`);
    }

    if (typeof evaluatePkTrendEntry === 'function') {
        const res = evaluatePkTrendEntry(pk, opt);
        let suggest_color = "idle";
        if (res.signal === 'BUY') suggest_color = "green";
        else if (res.signal === 'SELL') suggest_color = "red";
        return {
            suggest_color,
            reason: res.reason,
            conditions_matched: conditions,
            code: `PKT-${pk.caseCode || 'N/A'}`
        };
    }
    return { suggest_color: "idle", reason: "PkTrendEvaluator not loaded", conditions_matched: conditions, code: "PKT-NOEVAL" };
}

// =============================================================================
// 📌 Dispatcher — เลือก strategy ตาม enum (พร้อมตรวจสอบ isSelectedToAction)
// =============================================================================
function getSuggestColorByStrategy(strategy, analysis, lossCon, analysisArray) {
    // 🛡️ ตรวจสอบว่า Case Code ของแท่งเทียนนี้ได้รับอนุญาตให้เทรดหรือไม่ (isSelectedToAction == 'y')
    if (typeof window !== 'undefined' && typeof window.isCaseCodeSelectedToAction === 'function') {
        var codeNo = analysis ? (analysis.code_no || analysis.codeNo || (analysis.pkt_code_no !== undefined ? analysis.pkt_code_no : null)) : null;
        var caseCode = analysis ? (analysis.case_code || analysis.caseCode || analysis.pkt_case_code || null) : null;
        if ((codeNo !== null || caseCode !== null) && !window.isCaseCodeSelectedToAction(codeNo, caseCode)) {
            return "idle";
        }
    }

    switch (strategy) {
        case SuggestStrategy.V1: return getSuggestColorV1(analysis, lossCon, analysisArray);
        case SuggestStrategy.V2: return getSuggestColorV2(analysis, lossCon, analysisArray);
        case SuggestStrategy.V3A: return getSuggestColorV3A(analysis, lossCon, analysisArray);
        case SuggestStrategy.V3B: return getSuggestColorV3B(analysis, lossCon, analysisArray);
        case SuggestStrategy.V3C: return getSuggestColorV3C(analysis, lossCon, analysisArray);
        case SuggestStrategy.FTA: return getSuggestColorFTA(analysis, lossCon, analysisArray);
        case SuggestStrategy.FTB: return getSuggestColorFTB(analysis, lossCon, analysisArray);
        case SuggestStrategy.PKTrend: return getSuggestColorPKTrend(analysis, lossCon, analysisArray);
        default: return getSuggestColorV1(analysis, lossCon, analysisArray); // Default fallback
    }
}

function getSuggestColorByStrategyWithReason(strategy, analysis, lossCon, analysisArray) {
    // 🛡️ ตรวจสอบว่า Case Code ของแท่งเทียนนี้ได้รับอนุญาตให้เทรดหรือไม่ (isSelectedToAction == 'y')
    if (typeof window !== 'undefined' && typeof window.isCaseCodeSelectedToAction === 'function') {
        var codeNo = analysis ? (analysis.code_no || analysis.codeNo || (analysis.pkt_code_no !== undefined ? analysis.pkt_code_no : null)) : null;
        var caseCode = analysis ? (analysis.case_code || analysis.caseCode || analysis.pkt_case_code || null) : null;
        if ((codeNo !== null || caseCode !== null) && !window.isCaseCodeSelectedToAction(codeNo, caseCode)) {
            return {
                suggest_color: "idle",
                reason: "⛔ ข้ามการเทรด: Case Code [No. " + (codeNo !== null ? codeNo : "—") + "] " + (caseCode || "") + " ถูกตั้งค่า isSelectedToAction = n (ปิดการทำงาน)",
                conditions_matched: ["isSelectedToAction=n"],
                code: "CASE-DISABLED"
            };
        }
    }

    switch (strategy) {
        case SuggestStrategy.V1: return getSuggestColorV1WithReason(analysis, lossCon, analysisArray);
        case SuggestStrategy.V2: return getSuggestColorV2WithReason(analysis, lossCon, analysisArray);
        case SuggestStrategy.V3A: return getSuggestColorV3AWithReason(analysis, lossCon, analysisArray);
        case SuggestStrategy.V3B: return getSuggestColorV3BWithReason(analysis, lossCon, analysisArray);
        case SuggestStrategy.V3C: return getSuggestColorV3CWithReason(analysis, lossCon, analysisArray);
        case SuggestStrategy.FTA: return getSuggestColorFTAWithReason(analysis, lossCon, analysisArray);
        case SuggestStrategy.FTB: return getSuggestColorFTBWithReason(analysis, lossCon, analysisArray);
        case SuggestStrategy.PKTrend: return getSuggestColorPKTrendWithReason(analysis, lossCon, analysisArray);
        default: return getSuggestColorV1WithReason(analysis, lossCon, analysisArray); // Default fallback
    }
}

// Export for Node.js / ES Module / Browser
if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        SuggestStrategy,
        getSuggestColorV1,
        getSuggestColorV1WithReason,
        getSuggestColorV2,
        getSuggestColorV2WithReason,
        getSuggestColorV3A,
        getSuggestColorV3AWithReason,
        getSuggestColorV3B,
        getSuggestColorV3BWithReason,
        getSuggestColorV3C,
        getSuggestColorV3CWithReason,
        getSuggestColorFTA,
        getSuggestColorFTAWithReason,
        getSuggestColorFTB,
        getSuggestColorFTBWithReason,
        getSuggestColorPKTrend,
        getSuggestColorPKTrendWithReason,
        getSuggestColorByStrategy,
        getSuggestColorByStrategyWithReason
    };
} else if (typeof window !== 'undefined') {
    window.StrategyEngine = {
        SuggestStrategy,
        getSuggestColorV1,
        getSuggestColorV1WithReason,
        getSuggestColorV2,
        getSuggestColorV2WithReason,
        getSuggestColorV3A,
        getSuggestColorV3AWithReason,
        getSuggestColorV3B,
        getSuggestColorV3BWithReason,
        getSuggestColorV3C,
        getSuggestColorV3CWithReason,
        getSuggestColorFTA,
        getSuggestColorFTAWithReason,
        getSuggestColorFTB,
        getSuggestColorFTBWithReason,
        getSuggestColorPKTrend,
        getSuggestColorPKTrendWithReason,
        getSuggestColorByStrategy,
        getSuggestColorByStrategyWithReason
    };
}


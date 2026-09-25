use serde::{Deserialize, Serialize};
use crate::full_analysis_ver2::FullAnalysisResult;

// ตัวแทนผลลัพธ์ที่จะคืนค่าไปให้ระบบเทรดหลักนำไปออกออเดอร์
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum TradeAction {
    Call,
    Put,
    Idle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyDecision {
    pub suggest_color: String,
    pub reason: String,
    pub conditions_matched: Vec<String>,
    pub code: String,
}

// =============================================================================
// 📌 ActionFixObj — สำหรับ BorrowSignal Feature
// =============================================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct MethodAction {
    pub action: String,
    pub winstatus: String,
    pub winCon: u32,
    pub lossCon: u32,
    pub BorrowSignalFrom: String,
}

impl MethodAction {
    /// อัปเดตผลลัพธ์การเทรด และปรับ winCon / lossCon ตามเงื่อนไข
    pub fn update_status(&mut self, new_winstatus: &str) {
        self.winstatus = new_winstatus.to_string();
        if new_winstatus == "Win" {
            self.winCon += 1;
            self.lossCon = 0;
        } else if new_winstatus == "Loss" {
            self.winCon = 0;
            self.lossCon += 1;
        } else {
            // กรณีเป็นค่าอื่น เช่น "Wait", "Idle" 
            // สามารถเลือกเคลียร์ค่า หรือคงค่าเดิมไว้ก็ได้
            // self.winCon = 0;
            // self.lossCon = 0;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct MethodList {
    pub V1: MethodAction,
    pub V2: MethodAction,
    pub V2WhipSaw: MethodAction,
    pub V3A: MethodAction,
    pub V3B: MethodAction,
    pub V3C: MethodAction,
    pub FTA: MethodAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ActionFixObj {
    pub assetCode: String,
    pub TradeNo: u32,
    pub methodList: MethodList,
}

impl ActionFixObj {
    /// Helper สำหรับอัปเดต winStatus / winCon / lossCon ของกลยุทธ์ที่ระบุ
    pub fn update_method_status(&mut self, method_name: &str, new_winstatus: &str) {
        match method_name {
            "V1" => self.methodList.V1.update_status(new_winstatus),
            "V2" => self.methodList.V2.update_status(new_winstatus),
            "V2WhipSaw" => self.methodList.V2WhipSaw.update_status(new_winstatus),
            "V3A" => self.methodList.V3A.update_status(new_winstatus),
            "V3B" => self.methodList.V3B.update_status(new_winstatus),
            "V3C" => self.methodList.V3C.update_status(new_winstatus),
            "FTA" => self.methodList.FTA.update_status(new_winstatus),
            _ => {}
        }
    }
}

// =============================================================================
// 📌 SuggestStrategy — Enum สำหรับเลือกกลยุทธ์ suggest color
// =============================================================================
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SuggestStrategy {
    V1,  // lossCon >= 2 → follow thisColor (default, เดิม)
    V2,  // lossCon >= 1 → follow thisColor (early react)
    V3A, // lossCon >= 2 → EMA Direction Consensus (นับเสียง 3 EMA)
    V3B, // lossCon >= 2 → EMA Short Direction + CutType Hybrid
    V3C, // lossCon >= 2 → EMA Medium Direction Only (ดู EMA Medium อย่างเดียว)
    FTA, // Follow Trend A: ดู EMA Short + Medium direction → ถ้าตรงกัน = เทรด, ไม่ตรง = idle
    FTB, // Follow Trend B: ดู EMA Short + Medium + Long direction → ถ้าตรงกันทั้ง 3 = เทรด, ไม่ตรง = idle
    PKTrend, // PKTrend: หา Action จาก getActionByPKTrend.rs (Trend follow, Traps, Whipsaw filter)
    PKTrendV5, // PKTrendV5: ระบบเทรดตาม Price Action V5 ครบ 27 รูปแบบ & Structure
    ForecastSequence, // ForecastSequence: ระบบทำนายแท่งถัดไป & คู่รหัส 2-Bar Sequence Action
    PKTrendSelectCaseCode, // PKTrendSelectCaseCode: หา Action ตามตาราง setup/strategy_case_codes.json
}

// =============================================================================
// 📌 V1 — Default Strategy (เดิม, Backward Compatible)
// =============================================================================
/// กลยุทธ์หาสีที่ควรเทรด — ตรงตามต้นฉบับ PHP `getSuggestColor` Strategy Type 2
///
/// หลักการ (ตรงตาม PHP findWinOnColor + getSuggestColor):
///   - lossCon 0, 1, 2 → suggest = "green" เสมอ (ยืนหยัด CALL)
///   - lossCon >= 3     → เปลี่ยนเล่นตามสีแท่งปัจจุบัน (red→red, green→green)
pub fn get_suggest_color(analysis: &FullAnalysisResult, loss_con: u32) -> String {
    let mut suggest = "green".to_string();
    if loss_con >= 2 {
        if analysis.color == "red" {
            suggest = "red".to_string();
        } else {
            suggest = "green".to_string();
        }
    }
    suggest
}

// =============================================================================
// 📌 V2 — Early React Strategy (ใหม่)
// =============================================================================
/// เหมือน V1 แต่ threshold = 1 (react เร็วกว่า)
/// - lossCon 0 → suggest = "red" เสมอ
/// - lossCon >= 1 → เล่นตามสีแท่งปัจจุบัน
pub fn get_suggest_color_v2(analysis: &FullAnalysisResult, loss_con: u32, _action_fix_obj: Option<&ActionFixObj>) -> String {
    let mut suggest = "red".to_string();
    if loss_con == 1 && analysis.color == "green" {
        suggest = "red".to_string();
    } else if loss_con >= 1 {
        if analysis.color == "red" {
            suggest = "red".to_string();
        } else {
            suggest = "green".to_string();
        }
    }

    if loss_con >= 3 {
        // เมื่อแพ้ติดกันตั้งแต่ 3 ครั้งขึ้นไป (loss_con >= 3) ให้เปลี่ยนไปขอ Suggest Color จาก getActionByPKTrend.rs แทน
        let pktrend_decision = crate::getActionByPKTrend::get_action_by_full_analysis(analysis, None);
        suggest = pktrend_decision.suggest_color;
    }

    suggest
}

pub fn get_suggest_color_v2_whipsaw(analysis: &FullAnalysisResult, _loss_con: u32, _action_fix_obj: &ActionFixObj) -> String {
    if analysis.color == "red" {
        "green".to_string()
    } else {
        "red".to_string()
    }
}

// =============================================================================
// 📌 V3A — EMA Direction Consensus (ใหม่)
// =============================================================================
/// นับเสียง EMA 3 ตัว (Short, Medium, Long)
/// - ถ้าส่วนใหญ่ (>= 2 จาก 3) ชี้ขึ้น → suggest "green" (CALL)
/// - ถ้าส่วนใหญ่ชี้ลง → suggest "red" (PUT)
///
/// จุดเด่น: กรอง noise ได้ดี เพราะต้อง consensus (อย่างน้อย 2/3 เห็นด้วย)
pub fn get_suggest_color_v3a(analysis: &FullAnalysisResult, loss_con: u32) -> String {
    let mut suggest = "green".to_string();
    if loss_con >= 2 {
        let mut up_votes: u32 = 0;
        if analysis.ema_short_direction == "Up" { up_votes += 1; }
        if analysis.ema_medium_direction == "Up" { up_votes += 1; }
        if analysis.ema_long_direction == "Up" { up_votes += 1; }
        if up_votes >= 2 {
            suggest = "green".to_string();
        } else {
            suggest = "red".to_string();
        }
    }
    suggest
}

// =============================================================================
// 📌 V3B — EMA Short Direction + CutType Hybrid (ใหม่)
// =============================================================================
/// ดู EMA Short direction เป็นหลัก + ถ้ามี crossover signal → override
///
/// จุดเด่น: จับ trend reversal ได้เร็ว ด้วย CutType signal
/// จุดอ่อน: อาจ overreact ในตลาด sideway
pub fn get_suggest_color_v3b(analysis: &FullAnalysisResult, loss_con: u32) -> String {
    let mut suggest = "green".to_string();
    if loss_con >= 2 {
        if analysis.ema_short_direction == "Up" {
            suggest = "green".to_string();
        } else {
            suggest = "red".to_string();
        }
        if analysis.ema_cut_position == "CutUp" {
            suggest = "green".to_string();
        } else if analysis.ema_cut_position == "CutDown" {
            suggest = "red".to_string();
        }
    }
    suggest
}

// =============================================================================
// 📌 V3C — EMA Medium Direction Only (ใหม่)
// =============================================================================
/// ดัดแปลงจาก V3A แต่พิจารณาเฉพาะ EMA Medium Direction เท่านั้น
/// - ถ้า EMA Medium ชี้ขึ้น ("Up") → suggest "green" (CALL)
/// - ถ้า EMA Medium ไม่ใช่ "Up" → suggest "red" (PUT)
///
/// จุดเด่น: ใช้ EMA กลางเป็นตัวกรองเดี่ยว — ลด noise จาก EMA Short
///         แต่ไม่ช้าเท่า EMA Long
/// จุดอ่อน: ไม่มี consensus หรือ crossover signal ช่วยยืนยัน
pub fn get_suggest_color_v3c(analysis: &FullAnalysisResult, loss_con: u32) -> String {
    let mut suggest = "green".to_string();
    if loss_con >= 2 {
        if analysis.ema_medium_direction == "Up" {
            suggest = "green".to_string();
        } else {
            suggest = "red".to_string();
        }
    }
    suggest
}

// =============================================================================
// 📌 FTA — Follow Trend A (EMA Short + Medium Direction)
// =============================================================================
/// ดูทิศทาง EMA 2 เส้น (Short + Medium) จาก FullAnalysisResult
/// - ema_short_direction ≠ ema_medium_direction → "idle" (ไม่เทรด)
/// - ทั้งคู่ = "Up"   → "green" (CALL)
/// - ทั้งคู่ = "Down" → "red"   (PUT)
///
/// ⚠️ ไม่ใช้ lossCon ในการตัดสินใจ — ดู EMA direction อย่างเดียว
pub fn get_suggest_color_fta(analysis: &FullAnalysisResult, _loss_con: u32) -> String {
    if analysis.ema_short_direction != analysis.ema_medium_direction {
        return "idle".to_string();
    }
    if analysis.ema_short_direction == "Up" {
        "green".to_string()
    } else {
        "red".to_string()
    }
}

// =============================================================================
// 📌 FTB — Follow Trend B (EMA Short + Medium + Long Direction)
// =============================================================================
/// ดูทิศทาง EMA 3 เส้น (Short + Medium + Long) จาก FullAnalysisResult
/// - ema_short_direction ≠ ema_medium_direction → "idle" (ไม่เทรด)
/// - ทั้งสาม ≠ กัน → "idle" (ไม่เทรด)
/// - ทั้งสาม = "Up"   → "green" (CALL)
/// - ทั้งสาม = "Down" → "red"   (PUT)
///
/// ⚠️ ไม่ใช้ lossCon ในการตัดสินใจ — ดู EMA direction อย่างเดียว
pub fn get_suggest_color_ftb(analysis: &FullAnalysisResult, _loss_con: u32) -> String {
    if analysis.ema_short_direction != analysis.ema_medium_direction {
        return "idle".to_string();
    }
    if analysis.ema_short_direction != analysis.ema_long_direction {
        return "idle".to_string();
    }
    if analysis.ema_short_direction == "Up" {
        "green".to_string()
    } else {
        "red".to_string()
    }
}

// =============================================================================
// 📌 Dispatcher — เลือก strategy ตาม enum
// =============================================================================
/// เรียกใช้ suggest color ตาม strategy ที่เลือก
/// - V1, V2 ใช้แค่ this_color + loss_con
/// - V3A, V3B ใช้ AnalysisObject เต็ม (มี EMA data)
pub fn get_suggest_color_by_strategy(
    strategy: &SuggestStrategy,
    analysis: &FullAnalysisResult,
    loss_con: u32,
    action_fix_obj: Option<&ActionFixObj>,
) -> String {
    match strategy {
        SuggestStrategy::V1 => get_suggest_color(analysis, loss_con),
        SuggestStrategy::V2 => get_suggest_color_v2(analysis, loss_con, action_fix_obj),
        SuggestStrategy::V3A => get_suggest_color_v3a(analysis, loss_con),
        SuggestStrategy::V3B => get_suggest_color_v3b(analysis, loss_con),
        SuggestStrategy::V3C => get_suggest_color_v3c(analysis, loss_con),
        SuggestStrategy::FTA => get_suggest_color_fta(analysis, loss_con),
        SuggestStrategy::FTB => get_suggest_color_ftb(analysis, loss_con),
        SuggestStrategy::PKTrend => get_suggest_color_pktrend(analysis, loss_con),
        SuggestStrategy::PKTrendV5 => get_suggest_color_pktrend_v5(analysis, loss_con),
        SuggestStrategy::ForecastSequence => get_suggest_color_forecast_sequence(analysis, loss_con),
        SuggestStrategy::PKTrendSelectCaseCode => crate::getActionByPKTrendVerSelectCaseCode::get_suggest_color_case_code(analysis, loss_con),
    }
}

// =============================================================================
// 📌 StrategyDecision Functions (With Reasoning)
// =============================================================================
pub fn get_suggest_color_with_reason(analysis: &FullAnalysisResult, loss_con: u32) -> StrategyDecision {
    let mut suggest = "green".to_string();
    let reason;
    let mut conditions = vec![];
    let mut code = "V1-A".to_string();

    if loss_con >= 2 {
        if analysis.color == "red" {
            suggest = "red".to_string();
            reason = "loss_con >= 2 -> follow thisColor (red)".to_string();
            code = "V1-C".to_string();
        } else {
            suggest = "green".to_string();
            reason = "loss_con >= 2 -> follow thisColor (green)".to_string();
            code = "V1-B".to_string();
        }
        conditions.push(format!("loss_con={}", loss_con));
    } else {
        reason = "loss_con < 2 -> Default (green)".to_string();
        conditions.push(format!("loss_con={}", loss_con));
    }

    StrategyDecision { suggest_color: suggest, reason, conditions_matched: conditions, code }
}

pub fn get_suggest_color_v2_with_reason(analysis: &FullAnalysisResult, loss_con: u32, _action_fix_obj: Option<&ActionFixObj>) -> StrategyDecision {
    let mut suggest = "red".to_string();
    let mut reason;
    let mut conditions = vec![];
    let mut code = "V2-A".to_string();

    if loss_con == 1 && analysis.color == "green" {
        suggest = "red".to_string();
        reason = "loss_con == 1 and first color green -> force put (red)".to_string();
        code = "V2-D".to_string();
        conditions.push(format!("loss_con={}", loss_con));
    } else if loss_con >= 1 {
        if analysis.color == "red" {
            suggest = "red".to_string();
            reason = "loss_con >= 1 -> follow thisColor (red)".to_string();
            code = "V2-C".to_string();
        } else {
            suggest = "green".to_string();
            reason = "loss_con >= 1 -> follow thisColor (green)".to_string();
            code = "V2-B".to_string();
        }
        conditions.push(format!("loss_con={}", loss_con));
    } else {
        reason = "loss_con < 1 -> Default (red)".to_string();
        conditions.push(format!("loss_con={}", loss_con));
    }

    if loss_con >= 3 {
        // เมื่อแพ้ติดกันตั้งแต่ 3 ครั้งขึ้นไป (loss_con >= 3) ให้เปลี่ยนไปขอ Suggest Color จาก getActionByPKTrend.rs แทน
        let pktrend_decision = crate::getActionByPKTrend::get_action_by_full_analysis(analysis, None);
        suggest = pktrend_decision.suggest_color;
        reason = format!("loss_con >= 3 -> ยืมสัญญาณ PKTrend: {}", pktrend_decision.reason);
        code = format!("V2-PKT-{}", if pktrend_decision.case_code.is_empty() { "N/A" } else { &pktrend_decision.case_code });
        conditions.push(format!("borrow_pktrend_{}", pktrend_decision.case_code));
    }

    StrategyDecision { suggest_color: suggest, reason, conditions_matched: conditions, code }
}

pub fn get_suggest_color_v3a_with_reason(analysis: &FullAnalysisResult, loss_con: u32) -> StrategyDecision {
    let mut suggest = "green".to_string();
    let mut reason = "Default (green) loss_con < 2".to_string();
    let mut conditions = vec![];
    let mut code = "V3A-A".to_string();

    if loss_con >= 2 {
        let mut up_votes: u32 = 0;
        if analysis.ema_short_direction == "Up" { up_votes += 1; conditions.push("EMA Short Up".to_string()); }
        if analysis.ema_medium_direction == "Up" { up_votes += 1; conditions.push("EMA Medium Up".to_string()); }
        if analysis.ema_long_direction == "Up" { up_votes += 1; conditions.push("EMA Long Up".to_string()); }
        if up_votes >= 2 {
            suggest = "green".to_string();
            reason = format!("Majority EMA Up ({}/3)", up_votes);
            code = "V3A-B".to_string();
        } else {
            suggest = "red".to_string();
            reason = format!("Majority EMA Down ({}/3)", 3 - up_votes);
            code = "V3A-C".to_string();
        }
    } else {
        conditions.push(format!("loss_con={}", loss_con));
    }

    StrategyDecision { suggest_color: suggest, reason, conditions_matched: conditions, code }
}

pub fn get_suggest_color_v3b_with_reason(analysis: &FullAnalysisResult, loss_con: u32) -> StrategyDecision {
    let mut suggest = "green".to_string();
    let mut reason = "Default (green) loss_con < 2".to_string();
    let mut conditions = vec![];
    let mut code = "V3B-A".to_string();

    if loss_con >= 2 {
        if analysis.ema_short_direction == "Up" {
            suggest = "green".to_string();
            reason = "EMA Short Up".to_string();
            conditions.push("EMA Short Up".to_string());
            code = "V3B-B".to_string();
        } else {
            suggest = "red".to_string();
            reason = "EMA Short Down".to_string();
            conditions.push("EMA Short Down".to_string());
            code = "V3B-C".to_string();
        }

        if analysis.ema_cut_position == "CutUp" {
            suggest = "green".to_string();
            reason = "CutUp Override".to_string();
            conditions.push("CutUp Signal".to_string());
            code = "V3B-D".to_string();
        } else if analysis.ema_cut_position == "CutDown" {
            suggest = "red".to_string();
            reason = "CutDown Override".to_string();
            conditions.push("CutDown Signal".to_string());
            code = "V3B-E".to_string();
        }
    } else {
        conditions.push(format!("loss_con={}", loss_con));
    }

    StrategyDecision { suggest_color: suggest, reason, conditions_matched: conditions, code }
}

pub fn get_suggest_color_v3c_with_reason(analysis: &FullAnalysisResult, loss_con: u32) -> StrategyDecision {
    let mut suggest = "green".to_string();
    let mut reason = "Default (green) loss_con < 2".to_string();
    let mut conditions = vec![];
    let mut code = "V3C-A".to_string();

    if loss_con >= 2 {
        if analysis.ema_medium_direction == "Up" {
            suggest = "green".to_string();
            reason = "EMA Medium Up".to_string();
            conditions.push("EMA Medium Up".to_string());
            code = "V3C-B".to_string();
        } else {
            suggest = "red".to_string();
            reason = format!("EMA Medium Down ({})", analysis.ema_medium_direction);
            conditions.push(format!("EMA Medium {}", analysis.ema_medium_direction));
            code = "V3C-C".to_string();
        }
    } else {
        conditions.push(format!("loss_con={}", loss_con));
    }

    StrategyDecision { suggest_color: suggest, reason, conditions_matched: conditions, code }
}

pub fn get_suggest_color_fta_with_reason(analysis: &FullAnalysisResult, _loss_con: u32) -> StrategyDecision {
    let conditions = vec![
        format!("EMA Short {}", analysis.ema_short_direction),
        format!("EMA Medium {}", analysis.ema_medium_direction),
    ];

    if analysis.ema_short_direction != analysis.ema_medium_direction {
        return StrategyDecision {
            suggest_color: "idle".to_string(),
            reason: format!("Short({}) ≠ Medium({}) → Idle", analysis.ema_short_direction, analysis.ema_medium_direction),
            conditions_matched: conditions,
            code: "FTA-C".to_string(),
        };
    }

    if analysis.ema_short_direction == "Up" {
        StrategyDecision {
            suggest_color: "green".to_string(),
            reason: "Short=Medium=Up → CALL".to_string(),
            conditions_matched: conditions,
            code: "FTA-A".to_string(),
        }
    } else {
        StrategyDecision {
            suggest_color: "red".to_string(),
            reason: "Short=Medium=Down → PUT".to_string(),
            conditions_matched: conditions,
            code: "FTA-B".to_string(),
        }
    }
}

pub fn get_suggest_color_ftb_with_reason(analysis: &FullAnalysisResult, _loss_con: u32) -> StrategyDecision {
    let conditions = vec![
        format!("EMA Short {}", analysis.ema_short_direction),
        format!("EMA Medium {}", analysis.ema_medium_direction),
        format!("EMA Long {}", analysis.ema_long_direction),
    ];

    if analysis.ema_short_direction != analysis.ema_medium_direction {
        return StrategyDecision {
            suggest_color: "idle".to_string(),
            reason: format!("Short({}) ≠ Medium({}) → Idle", analysis.ema_short_direction, analysis.ema_medium_direction),
            conditions_matched: conditions,
            code: "FTB-C".to_string(),
        };
    }

    if analysis.ema_short_direction != analysis.ema_long_direction {
        return StrategyDecision {
            suggest_color: "idle".to_string(),
            reason: format!("Short/Medium({}) ≠ Long({}) → Idle", analysis.ema_short_direction, analysis.ema_long_direction),
            conditions_matched: conditions,
            code: "FTB-C".to_string(),
        };
    }

    if analysis.ema_short_direction == "Up" {
        StrategyDecision {
            suggest_color: "green".to_string(),
            reason: "Short=Medium=Long=Up → CALL".to_string(),
            conditions_matched: conditions,
            code: "FTB-A".to_string(),
        }
    } else {
        StrategyDecision {
            suggest_color: "red".to_string(),
            reason: "Short=Medium=Long=Down → PUT".to_string(),
            conditions_matched: conditions,
            code: "FTB-B".to_string(),
        }
    }
}

pub fn get_suggest_color_pktrend(analysis: &FullAnalysisResult, _loss_con: u32) -> String {
    let decision = crate::getActionByPKTrend::get_action_by_full_analysis(analysis, None);
    decision.suggest_color
}

pub fn get_suggest_color_pktrend_with_reason(analysis: &FullAnalysisResult, _loss_con: u32) -> StrategyDecision {
    let decision = crate::getActionByPKTrend::get_action_by_full_analysis(analysis, None);
    let mut conditions = vec![
        format!("Trend: {}", analysis.pk_trend.trend.unwrap_or("None")),
        format!("Score: {}", analysis.pk_trend.trend_score),
        format!("CaseCode: {}", analysis.pk_trend.case_code),
    ];
    if let Some(pos) = analysis.pk_trend.close_position {
        conditions.push(format!("ClosePos: {:.2}", pos));
    }

    StrategyDecision {
        suggest_color: decision.suggest_color,
        reason: decision.reason,
        conditions_matched: conditions,
        code: format!("PKT-{}", if decision.case_code.is_empty() { "N/A" } else { &decision.case_code }),
    }
}

pub fn is_case_code_selected_to_action(case_code: &str) -> bool {
    let candidate_paths = [
        "public/case_codes.json",
        "case_codes.json",
        "../dynamicChart/case_codes.json",
    ];

    for path in &candidate_paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                let cases = val.get("cases").and_then(|c| c.as_array()).or_else(|| val.as_array());
                if let Some(list) = cases {
                    for item in list {
                        let match_case_code = item.get("caseCode").and_then(|v| v.as_str()) == Some(case_code);
                        if match_case_code {
                            let is_selected = item.get("isSelectedToAction")
                                .and_then(|v| v.as_str())
                                .map(|s| s.eq_ignore_ascii_case("y"))
                                .unwrap_or(true);
                            return is_selected;
                        }
                    }
                }
            }
        }
    }

    // Default fallback: allow core patterns (1-6, 9-12)
    match case_code {
        "UP-CONFIRM" | "DN-CONFIRM" | "SPK-CONTINUE-UP" | "SPK-CONTINUE-DN" |
        "SPK-BULLTRAP" | "SPK-BEARTRAP" | "EG-BULLISH" | "EG-BEARISH" |
        "RJ-BULLTRAP-STRONG" | "RJ-BEARTRAP-STRONG" => true,
        _ => false,
    }
}

pub fn get_suggest_color_pktrend_v5(analysis: &FullAnalysisResult, loss_con: u32) -> String {
    get_suggest_color_pktrend_v5_with_reason(analysis, loss_con).suggest_color
}

pub fn get_suggest_color_pktrend_v5_with_reason(analysis: &FullAnalysisResult, _loss_con: u32) -> StrategyDecision {
    let case_code = analysis.pk_trend.case_code;
    let trend_score = analysis.pk_trend.trend_score;
    let trend_str = analysis.pk_trend.trend.unwrap_or("Sideways");
    let whipsaw_status = analysis.pk_trend.whipsaw_status.as_str();
    let close_pos = analysis.pk_trend.close_position.unwrap_or(0.5);

    let conditions = vec![
        format!("Trend: {}", trend_str),
        format!("Score: {}", trend_score),
        format!("CaseCode: {}", case_code),
        format!("Whipsaw: {}", whipsaw_status),
    ];

    // 🛡️ GATEKEEPER: ตรวจสอบกับ case_codes.json ว่า isSelectedToAction == "y" หรือไม่
    if !is_case_code_selected_to_action(case_code) {
        return StrategyDecision {
            suggest_color: "idle".to_string(),
            reason: format!("⛔ ข้ามการเทรด: Case Code [{}] ถูกตั้งค่า isSelectedToAction = n (ปิดการทำงาน)", if case_code.is_empty() { "NODATA" } else { case_code }),
            conditions_matched: conditions,
            code: "CASE-DISABLED".to_string(),
        };
    }

    let mut suggest_color = "idle".to_string();
    let mut reason = String::new();
    let mut code = format!("PKT5-{}", if case_code.is_empty() { "NODATA" } else { case_code });

    if whipsaw_status == "CONFIRMED_WHIPSAW" {
        suggest_color = "idle".to_string();
        reason = format!("⚠️ Whipsaw Zone สลับสีรุนแรง ({}) - งดเข้าเทรด", analysis.pk_trend.color_sequence);
        code = "PKT5-WHIPSAW-BLOCK".to_string();
    } else {
        match case_code {
            "UP-CONFIRM" | "EG-BULLISH" | "SPK-CONTINUE-UP" => {
                if trend_score >= 30 && close_pos >= 0.55 {
                    suggest_color = "green".to_string();
                    reason = format!("🟢 {} ขาขึ้นยืนยันแข็งแกร่ง (Score: {}, ClosePos: {:.2})", case_code, trend_score, close_pos);
                } else {
                    suggest_color = "idle".to_string();
                    reason = format!("⏳ {} ขาขึ้นแต่คะแนนหรือปิดไม่เด็ดขาดพอ (Score: {})", case_code, trend_score);
                }
            }
            "DN-CONFIRM" | "EG-BEARISH" | "SPK-CONTINUE-DN" => {
                if trend_score <= -30 && close_pos <= 0.45 {
                    suggest_color = "red".to_string();
                    reason = format!("🔴 {} ขาลงยืนยันแข็งแกร่ง (Score: {}, ClosePos: {:.2})", case_code, trend_score, close_pos);
                } else {
                    suggest_color = "idle".to_string();
                    reason = format!("⏳ {} ขาลงแต่คะแนนหรือปิดไม่เด็ดขาดพอ (Score: {})", case_code, trend_score);
                }
            }
            "RJ-BEARTRAP-STRONG" | "SPK-BEARTRAP" => {
                suggest_color = "green".to_string();
                reason = format!("🟢 {} กับดักขาลง (Bear Trap) ดักขายแล้วดีดกลับขึ้นแรง -> CALL", case_code);
            }
            "RJ-BULLTRAP-STRONG" | "SPK-BULLTRAP" => {
                suggest_color = "red".to_string();
                reason = format!("🔴 {} กับดักขาขึ้น (Bull Trap) ดักซื้อแล้วทุบกลับลงแรง -> PUT", case_code);
            }
            "SD-FLATSUPPORT" => {
                suggest_color = "green".to_string();
                reason = "🟢 ชนแนวรับเดิมแล้วเด้งกลับ (Double Bottom Support) -> CALL".to_string();
            }
            "SD-FLATRESISTANCE" => {
                suggest_color = "red".to_string();
                reason = "🔴 ชนแนวต้านเดิมซ้ำไม่ผ่าน (Double Top Resistance) -> PUT".to_string();
            }
            _ => {
                suggest_color = "idle".to_string();
                reason = format!("⏳ {} สภาวะตลาดพักตัว/ไร้ทิศทางชัดเจน", case_code);
            }
        }
    }

    StrategyDecision {
        suggest_color,
        reason,
        conditions_matched: conditions,
        code,
    }
}

pub fn get_suggest_color_forecast_sequence(analysis: &FullAnalysisResult, loss_con: u32) -> String {
    get_suggest_color_forecast_sequence_with_reason(analysis, loss_con).suggest_color
}

pub fn get_suggest_color_forecast_sequence_with_reason(analysis: &FullAnalysisResult, _loss_con: u32) -> StrategyDecision {
    let case_code = analysis.pk_trend.case_code;
    let trend_str = analysis.pk_trend.trend.unwrap_or("Sideways");
    let close_pos = analysis.pk_trend.close_position.unwrap_or(0.5);

    let mut suggest_color = "idle".to_string();
    let mut reason = String::new();
    let mut code = "FC-WAIT".to_string();
    let conditions = vec![
        format!("Trend: {}", trend_str),
        format!("CaseCode: {}", case_code),
        format!("ClosePos: {:.2}", close_pos),
    ];

    match case_code {
        "UP-CONFIRM" | "EG-BULLISH" | "SPK-CONTINUE-UP" => {
            suggest_color = "green".to_string();
            reason = format!("🔮 Action Forecast [CALL] - ส่งต่อโมเมนตัมขาขึ้น ({})", case_code);
            code = format!("FC-CALL-{}", case_code);
        }
        "DN-CONFIRM" | "EG-BEARISH" | "SPK-CONTINUE-DN" => {
            suggest_color = "red".to_string();
            reason = format!("🔮 Action Forecast [PUT] - ส่งต่อโมเมนตัมขาลง ({})", case_code);
            code = format!("FC-PUT-{}", case_code);
        }
        "RJ-BEARTRAP-STRONG" | "SPK-BEARTRAP" => {
            suggest_color = "green".to_string();
            reason = "🔮 2-Bar Sequence [Bear Trap Reversal] -> CALL".to_string();
            code = "FC-CALL-BEARTRAP".to_string();
        }
        "RJ-BULLTRAP-STRONG" | "SPK-BULLTRAP" => {
            suggest_color = "red".to_string();
            reason = "🔮 2-Bar Sequence [Bull Trap Reversal] -> PUT".to_string();
            code = "FC-PUT-BULLTRAP".to_string();
        }
        "RJ-LOWWICK-WEAK" | "RJ-FOLLOWFAIL-DN" => {
            suggest_color = "green".to_string();
            reason = "🔮 Dip Buy Rejection - มีแรงซื้อดันก้นแท่ง -> CALL".to_string();
            code = "FC-CALL-DIPBUY".to_string();
        }
        "RJ-UPWICK-WEAK" | "RJ-FOLLOWFAIL-UP" => {
            suggest_color = "red".to_string();
            reason = "🔮 Rally Sell Rejection - มีแรงขายกดปลายแท่ง -> PUT".to_string();
            code = "FC-PUT-RALLYSELL".to_string();
        }
        _ => {
            suggest_color = "idle".to_string();
            reason = format!("⏳ สภาวะตลาด {} - ควรรอยืนยันสัญญาณ", case_code);
            code = format!("FC-WAIT-{}", case_code);
        }
    }

    StrategyDecision {
        suggest_color,
        reason,
        conditions_matched: conditions,
        code,
    }
}

pub fn get_suggest_color_by_strategy_with_reason(
    strategy: &SuggestStrategy,
    analysis: &FullAnalysisResult,
    loss_con: u32,
    action_fix_obj: Option<&ActionFixObj>,
) -> StrategyDecision {
    match strategy {
        SuggestStrategy::V1 => get_suggest_color_with_reason(analysis, loss_con),
        SuggestStrategy::V2 => get_suggest_color_v2_with_reason(analysis, loss_con, action_fix_obj),
        SuggestStrategy::V3A => get_suggest_color_v3a_with_reason(analysis, loss_con),
        SuggestStrategy::V3B => get_suggest_color_v3b_with_reason(analysis, loss_con),
        SuggestStrategy::V3C => get_suggest_color_v3c_with_reason(analysis, loss_con),
        SuggestStrategy::FTA => get_suggest_color_fta_with_reason(analysis, loss_con),
        SuggestStrategy::FTB => get_suggest_color_ftb_with_reason(analysis, loss_con),
        SuggestStrategy::PKTrend => get_suggest_color_pktrend_with_reason(analysis, loss_con),
        SuggestStrategy::PKTrendV5 => get_suggest_color_pktrend_v5_with_reason(analysis, loss_con),
        SuggestStrategy::ForecastSequence => get_suggest_color_forecast_sequence_with_reason(analysis, loss_con),
        SuggestStrategy::PKTrendSelectCaseCode => crate::getActionByPKTrendVerSelectCaseCode::get_suggest_color_case_code_with_reason(analysis, loss_con),
    }
}

pub fn analyze_loss_factor(
    _strategy_name: &str,
    _decision: &StrategyDecision,
    analysis: Option<&FullAnalysisResult>,
    actual_color: &str,
) -> String {
    if let Some(fa) = analysis {
        let mut factors = vec![];
        if fa.is_atr {
            factors.push("ATR Spike (High Volatility)");
        }
        if fa.choppy_indicator > 61.8 {
            factors.push("Choppy Market (Sideway)");
        }
        if fa.adx_value < 20.0 {
            factors.push("Weak Trend (ADX < 20)");
        }
        if fa.rsi_value > 70.0 && actual_color == "red" {
            factors.push("Overbought Reversal");
        }
        if fa.rsi_value < 30.0 && actual_color == "green" {
            factors.push("Oversold Reversal");
        }
        
        if factors.is_empty() {
            "Normal market variance".to_string()
        } else {
            factors.join(", ")
        }
    } else {
        "No Full Analysis Data".to_string()
    }
}

// =============================================================================
// 📌 get_trade_action — ฟังก์ชันหลัก (ใช้ V1 default)
// =============================================================================
pub fn get_trade_action(
    analysis: &FullAnalysisResult,
    loss_con: u32,
    noise_config: Option<&crate::filter_noise::NoiseFilterConfig>,
) -> TradeAction {
    if !analysis.is_atr {
        return TradeAction::Idle;
    }

    if let Some(config) = noise_config {
        let (is_idle, case_code) = crate::filter_noise::check_noise_filter(analysis, config);
        if is_idle {
            println!("🛑 Noise Filter Idle! caseCode={}", case_code);
            return TradeAction::Idle;
        }
    }

    let suggest = get_suggest_color(analysis, loss_con);
    println!(
        "🎯 Spike detected! thisColor={}, lossCon={}, suggest={}",
        analysis.color, loss_con, suggest
    );
    match suggest.as_str() {
        "green" => TradeAction::Call,
        "red" => TradeAction::Put,
        _ => TradeAction::Idle,
    }
}

// =============================================================================
// 📌 Unit Tests
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::full_analysis_ver2::*;

    /// Helper: สร้าง FullAnalysisResult สำหรับ test
    fn make_analysis(
        this_color: &str,
        ema_short_dir: &str,
        ema_medium_dir: &str,
        ema_long_dir: &str,
        cut_type: &str,
    ) -> FullAnalysisResult {
        FullAnalysisResult {
            index: 0,
            asset_code: "R_10".to_string(),
            candletime: 0,
            candletime_display: "".to_string(),
            open: 0.0, high: 0.0, low: 0.0, close: 0.0,
            color: this_color.to_string(),
            next_color: None,
            pip_size: 0.0,
            ema_short_value: 0.0,
            ema_short_direction: ema_short_dir.to_string(),
            ema_short_turn_type: "-".to_string(),
            ema_short_slope_value: 0.0,
            emaslope_thereshold: 0.0,
            diff: 0.0,
            ema_short_flat: "-".to_string(),
            ema_medium_value: 0.0,
            ema_medium_direction: ema_medium_dir.to_string(),
            ema_medium_turn_type: "-".to_string(),
            ema_medium_slope_value: 0.0,
            ema_medium_flat: "-".to_string(),
            ema_long_value: 0.0,
            ema_long_direction: ema_long_dir.to_string(),
            ema_long_turn_type: "-".to_string(),
            ema_long_slope_value: 0.0,
            ema_long_flat: "-".to_string(),
            short_medium_gap_value: 0.0,
            is_short_medium_gap_occur: "-".to_string(),
            medium_long_gap_value: 0.0,
            is_medium_long_gap_occur: "-".to_string(),
            ema_above: "Short".to_string(),
            ema_long_above: "-".to_string(),
            macd_12: 0.0, macd_23: 0.0,
            previous_ema_short_value: 0.0,
            previous_ema_medium_value: 0.0,
            previous_ema_long_value: 0.0,
            previous_macd_12: 0.0, previous_macd_23: 0.0,
            ema_convergence_type: "-".to_string(),
            ema_long_convergence_type: "-".to_string(),
            choppy_indicator: 0.0,
            adx_value: 0.0,
            rsi_value: 0.0,
            bb_values: BollingerBands { upper: 0.0, middle: 0.0, lower: 0.0 },
            bb_position: "-".to_string(),
            atr_value: 0.0,
            is_abnormal_candle: false,
            is_abnormal_atr: false,
            is_atr: true,
            u_wick: 0.0, u_wick_percent: 0.0,
            body: 0.0, body_percent: 0.0,
            l_wick: 0.0, l_wick_percent: 0.0,
            ema_cut_position: cut_type.to_string(),
            ema_cut_long_type: "-".to_string(),
            ema_cut_short_long_type: "-".to_string(),
            ema_cut_all_type: "-".to_string(),
            candles_since_ema_cut: 0,
            age_cut_candle_code12: "-".to_string(),
            age_cut_candle_code123: "-".to_string(),
            ema_short_pos: "-".to_string(),
            ema_medium_pos: "-".to_string(),
            ema_long_pos: "-".to_string(),
            bb_bandwidth: 0.0,
            is_bb_squeeze: false,
            is_bb_flat_choppy: false,
            bb_middle_slope: 0.0,
            bb_line_upper_cut_pos: "-".to_string(),
            bb_line_middle_cut_pos: "-".to_string(),
            bb_line_low_cut_pos: "-".to_string(),
            kama_er: 0.0,
            candle_body_ratio: 0.0,
            color_switch_count: 0,
            is_choppy_micro: false,
            macro_chop: None,
            is_choppy_macro: None,
            is_choppy_combined: false,
            up_con_medium_ema: 0, down_con_medium_ema: 0,
            up_con_long_ema: 0, down_con_long_ema: 0,
            is_mark: "-".to_string(),
            status_code: "-".to_string(),
            status_desc: "-".to_string(),
            status_desc_0: "-".to_string(),
            hint_status: "-".to_string(),
            suggest_color: "-".to_string(),
            win_status: "-".to_string(),
            win_con: 0, loss_con: 0,
            smc: SmcData {
                structures: vec![], swing_points: vec![],
                order_blocks: vec![], fair_value_gaps: vec![],
                equal_highs_lows: vec![],
                premium_discount_zone: SmcPremiumDiscountZone {
                    start_time: 0, end_time: 0,
                    premium_top: 0.0, premium_bottom: 0.0,
                    equilibrium: 0.0, discount_top: 0.0, discount_bottom: 0.0,
                },
                strong_weak_levels: vec![],
                swing_trend: "-".to_string(),
                internal_trend: "-".to_string(),
            },
            tick_volatility: TickVolatility {
                tick_count: 0, buy_tick_count: 0, sell_tick_count: 0,
                buy_sell_ratio: 0.0, avg_tick_move: 0.0, max_tick_move: 0.0,
                sum_tick_move: 0.0, volatility_clustering: 0.0,
                volatility_level: "-".to_string(),
            },
            range_detector: RangeDetectorData {
                in_range: false, range_top: 0.0, range_bottom: 0.0,
                range_avg: 0.0, range_state: "-".to_string(),
            },
            pk_trend: Default::default(),
            is_alternating_pattern: false,
            alternating_sequence_length: 0,
            is_alternating_trigger: false,
            is_alternating_spike: false,
        }
    }

    // ── V1 Tests ──
    #[test]
    fn v1_loss_con_0_returns_green() {
        let a = make_analysis("red", "Down", "Down", "Down", "-");
        assert_eq!(get_suggest_color(&a, 0), "green");
    }

    #[test]
    fn v1_loss_con_2_red_returns_red() {
        let a = make_analysis("red", "Down", "Down", "Down", "-");
        assert_eq!(get_suggest_color(&a, 2), "red");
    }

    #[test]
    fn v1_loss_con_3_green_returns_green() {
        let a = make_analysis("green", "Up", "Up", "Up", "-");
        assert_eq!(get_suggest_color(&a, 3), "green");
    }

    // ── V2 Tests ──
    #[test]
    fn v2_loss_con_0_returns_red() {
        let a = make_analysis("red", "Down", "Down", "Down", "-");
        assert_eq!(get_suggest_color_v2(&a, 0, None), "red");
    }

    #[test]
    fn v2_loss_con_1_red_returns_red() {
        let a = make_analysis("red", "Down", "Down", "Down", "-");
        assert_eq!(get_suggest_color_v2(&a, 1, None), "red");
    }

    #[test]
    fn v2_loss_con_1_green_returns_red() {
        let a = make_analysis("green", "Up", "Up", "Up", "-");
        assert_eq!(get_suggest_color_v2(&a, 1, None), "red");
    }

    // ── V3A Tests ──
    #[test]
    fn v3a_loss_con_0_returns_green() {
        let a = make_analysis("red", "Down", "Down", "Down", "-");
        assert_eq!(get_suggest_color_v3a(&a, 0), "green");
    }

    #[test]
    fn v3a_all_up_returns_green() {
        let a = make_analysis("red", "Up", "Up", "Up", "-");
        assert_eq!(get_suggest_color_v3a(&a, 2), "green");
    }

    #[test]
    fn v3a_all_down_returns_red() {
        let a = make_analysis("green", "Down", "Down", "Down", "-");
        assert_eq!(get_suggest_color_v3a(&a, 2), "red");
    }

    #[test]
    fn v3a_two_up_one_down_returns_green() {
        let a = make_analysis("red", "Up", "Up", "Down", "-");
        assert_eq!(get_suggest_color_v3a(&a, 2), "green");
    }

    #[test]
    fn v3a_one_up_two_down_returns_red() {
        let a = make_analysis("green", "Up", "Down", "Down", "-");
        assert_eq!(get_suggest_color_v3a(&a, 2), "red");
    }

    // ── V3B Tests ──
    #[test]
    fn v3b_loss_con_0_returns_green() {
        let a = make_analysis("red", "Down", "Down", "Down", "-");
        assert_eq!(get_suggest_color_v3b(&a, 0), "green");
    }

    #[test]
    fn v3b_ema_short_up_returns_green() {
        let a = make_analysis("red", "Up", "Down", "Down", "-");
        assert_eq!(get_suggest_color_v3b(&a, 2), "green");
    }

    #[test]
    fn v3b_ema_short_down_returns_red() {
        let a = make_analysis("green", "Down", "Up", "Up", "-");
        assert_eq!(get_suggest_color_v3b(&a, 2), "red");
    }

    #[test]
    fn v3b_cut_up_override() {
        let a = make_analysis("red", "Down", "Down", "Down", "CutUp");
        assert_eq!(get_suggest_color_v3b(&a, 2), "green");
    }

    #[test]
    fn v3b_cut_down_override() {
        let a = make_analysis("green", "Up", "Up", "Up", "CutDown");
        assert_eq!(get_suggest_color_v3b(&a, 2), "red");
    }

    #[test]
    fn v3b_ema_short_down_cut_up_override_green() {
        let a = make_analysis("red", "Down", "Down", "Down", "CutUp");
        assert_eq!(get_suggest_color_v3b(&a, 2), "green");
    }

    // ── Dispatcher Tests ──
    #[test]
    fn dispatcher_routes_v1() {
        let a = make_analysis("red", "Up", "Up", "Up", "-");
        assert_eq!(
            get_suggest_color_by_strategy(&SuggestStrategy::V1, &a, 2, None),
            "red"
        );
    }

    #[test]
    fn dispatcher_routes_v2() {
        let a = make_analysis("red", "Up", "Up", "Up", "-");
        assert_eq!(
            get_suggest_color_by_strategy(&SuggestStrategy::V2, &a, 1, None),
            "red"
        );
    }

    #[test]
    fn dispatcher_routes_v3a() {
        let a = make_analysis("red", "Up", "Up", "Up", "-");
        assert_eq!(
            get_suggest_color_by_strategy(&SuggestStrategy::V3A, &a, 2, None),
            "green"
        );
    }

    #[test]
    fn dispatcher_routes_v3b() {
        let a = make_analysis("green", "Down", "Down", "Down", "CutDown");
        assert_eq!(
            get_suggest_color_by_strategy(&SuggestStrategy::V3B, &a, 2, None),
            "red"
        );
    }

    // ── get_trade_action Tests ──
    #[test]
    fn trade_action_wait_when_no_atr() {
        let mut a = make_analysis("green", "Up", "Up", "Up", "-");
        a.is_atr = false;
        assert_eq!(get_trade_action(&a, 0, None), TradeAction::Idle);
    }

    #[test]
    fn trade_action_call_when_atr_green() {
        let a = make_analysis("green", "Up", "Up", "Up", "-");
        assert_eq!(get_trade_action(&a, 0, None), TradeAction::Call);
    }

    #[test]
    fn trade_action_put_when_atr_red_high_loss() {
        let a = make_analysis("red", "Down", "Down", "Down", "-");
        assert_eq!(get_trade_action(&a, 3, None), TradeAction::Put);
    }

    // ── FTA Tests ──
    #[test]
    fn fta_both_up_returns_green() {
        let a = make_analysis("red", "Up", "Up", "Down", "-");
        assert_eq!(get_suggest_color_fta(&a, 0), "green");
    }

    #[test]
    fn fta_both_down_returns_red() {
        let a = make_analysis("green", "Down", "Down", "Up", "-");
        assert_eq!(get_suggest_color_fta(&a, 0), "red");
    }

    #[test]
    fn fta_mismatch_returns_idle() {
        let a = make_analysis("green", "Up", "Down", "Up", "-");
        assert_eq!(get_suggest_color_fta(&a, 0), "idle");
    }

    #[test]
    fn fta_ignores_loss_con() {
        let a = make_analysis("red", "Up", "Up", "Down", "-");
        assert_eq!(get_suggest_color_fta(&a, 0), "green");
        assert_eq!(get_suggest_color_fta(&a, 5), "green");
    }

    // ── FTB Tests ──
    #[test]
    fn ftb_all_up_returns_green() {
        let a = make_analysis("red", "Up", "Up", "Up", "-");
        assert_eq!(get_suggest_color_ftb(&a, 0), "green");
    }

    #[test]
    fn ftb_all_down_returns_red() {
        let a = make_analysis("green", "Down", "Down", "Down", "-");
        assert_eq!(get_suggest_color_ftb(&a, 0), "red");
    }

    #[test]
    fn ftb_short_medium_mismatch_returns_idle() {
        let a = make_analysis("green", "Up", "Down", "Up", "-");
        assert_eq!(get_suggest_color_ftb(&a, 0), "idle");
    }

    #[test]
    fn ftb_long_mismatch_returns_idle() {
        let a = make_analysis("green", "Up", "Up", "Down", "-");
        assert_eq!(get_suggest_color_ftb(&a, 0), "idle");
    }

    #[test]
    fn ftb_ignores_loss_con() {
        let a = make_analysis("red", "Up", "Up", "Up", "-");
        assert_eq!(get_suggest_color_ftb(&a, 0), "green");
        assert_eq!(get_suggest_color_ftb(&a, 10), "green");
    }

    // ── Dispatcher Tests for FTA/FTB ──
    #[test]
    fn dispatcher_routes_fta() {
        let a = make_analysis("red", "Up", "Up", "Down", "-");
        assert_eq!(
            get_suggest_color_by_strategy(&SuggestStrategy::FTA, &a, 0, None),
            "green"
        );
    }

    #[test]
    fn dispatcher_routes_ftb() {
        let a = make_analysis("green", "Down", "Down", "Down", "-");
        assert_eq!(
            get_suggest_color_by_strategy(&SuggestStrategy::FTB, &a, 0, None),
            "red"
        );
    }

    #[test]
    fn dispatcher_routes_pktrend() {
        let mut a = make_analysis("green", "Up", "Up", "Up", "-");
        a.pk_trend.trend = Some("UpTrend");
        a.pk_trend.case_code = "UP-CONFIRM";
        a.pk_trend.group = "Up";
        a.pk_trend.trend_score = 60;
        a.pk_trend.trend_strength = crate::pkDetectTrend_v5::TrendStrength::StrongUp;
        a.pk_trend.close_position = Some(0.8);
        a.pk_trend.is_whipsaw = false;

        assert_eq!(
            get_suggest_color_by_strategy(&SuggestStrategy::PKTrend, &a, 0, None),
            "green"
        );

        let decision = get_suggest_color_by_strategy_with_reason(&SuggestStrategy::PKTrend, &a, 0, None);
        assert_eq!(decision.suggest_color, "green");
        assert_eq!(decision.code, "PKT-UP-CONFIRM");
    }

    #[test]
    fn dispatcher_routes_pktrend_v5() {
        let mut a = make_analysis("green", "Up", "Up", "Up", "-");
        a.pk_trend.trend = Some("UpTrend");
        a.pk_trend.case_code = "SPK-CONTINUE-UP";
        a.pk_trend.trend_score = 65;
        a.pk_trend.close_position = Some(0.85);
        a.pk_trend.whipsaw_status = crate::pkDetectTrend_v5::WhipsawStatus::Trending;

        assert_eq!(
            get_suggest_color_by_strategy(&SuggestStrategy::PKTrendV5, &a, 0, None),
            "green"
        );
        let decision = get_suggest_color_by_strategy_with_reason(&SuggestStrategy::PKTrendV5, &a, 0, None);
        assert_eq!(decision.suggest_color, "green");
        assert_eq!(decision.code, "PKT5-SPK-CONTINUE-UP");
    }

    #[test]
    fn dispatcher_routes_forecast_sequence() {
        let mut a = make_analysis("red", "Down", "Down", "Down", "-");
        a.pk_trend.trend = Some("DownTrend");
        a.pk_trend.case_code = "DN-CONFIRM";
        a.pk_trend.close_position = Some(0.15);

        assert_eq!(
            get_suggest_color_by_strategy(&SuggestStrategy::ForecastSequence, &a, 0, None),
            "red"
        );
        let decision = get_suggest_color_by_strategy_with_reason(&SuggestStrategy::ForecastSequence, &a, 0, None);
        assert_eq!(decision.suggest_color, "red");
        assert_eq!(decision.code, "FC-PUT-DN-CONFIRM");
    }
}

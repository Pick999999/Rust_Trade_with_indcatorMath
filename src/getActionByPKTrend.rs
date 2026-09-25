//! getActionByPKTrend - PK Trend Trade Spot Evaluation Engine
//!
//! แปลงตรรกะจาก `public/js/PkTrendEvaluator.js` สำหรับประเมินสัญญาณและ Action เข้าเทรด
//! (CALL / PUT / WAIT) โดยใช้ผลการวิเคราะห์ `FullAnalysisResult` (analysisdata)
//! จาก `full_analysis_ver2.rs` ร่วมกับ `pkDetectTrend::TrendDetectResult`

use serde::{Deserialize, Serialize};
use crate::full_analysis_ver2::FullAnalysisResult;
use crate::pkDetectTrend_v5::TrendDetectResult;

/// สัญญาณการเข้าเทรดระดับดิบ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeSpotSignal {
    #[serde(rename = "BUY")]
    Buy,
    #[serde(rename = "SELL")]
    Sell,
    #[serde(rename = "WAIT")]
    Wait,
}

impl TradeSpotSignal {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Buy => "BUY",
            Self::Sell => "SELL",
            Self::Wait => "WAIT",
        }
    }
}

/// Action สำหรับระบบเทรดหลัก (Deriv / Binary Options)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeType {
    #[serde(rename = "CALL")]
    Call,
    #[serde(rename = "PUT")]
    Put,
    #[serde(rename = "WAIT")]
    Wait,
}

impl TradeType {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Call => "CALL",
            Self::Put => "PUT",
            Self::Wait => "WAIT",
        }
    }
}

/// พารามิเตอร์การตั้งค่าเกณฑ์การประเมิน (ตรงกับ DEFAULT_OPTIONS ใน PkTrendEvaluator.js)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PkTrendOptions {
    /// คะแนนเทรนด์ขั้นต่ำ (ค่าเริ่มต้น: 40)
    #[serde(rename = "minTrendScoreStrong")]
    pub min_trend_score_strong: i32,
    /// ตำแหน่งราคาปิดฝั่ง Up (0.0 - 1.0, ค่าเริ่มต้น: 0.70)
    #[serde(rename = "minCloseConvictionUp")]
    pub min_close_conviction_up: f64,
    /// ตำแหน่งราคาปิดฝั่ง Down (0.0 - 1.0, ค่าเริ่มต้น: 0.30)
    #[serde(rename = "maxCloseConvictionDown")]
    pub max_close_conviction_down: f64,
    /// สัญญาณสวนเทรนด์จาก Trap (Bull/Bear Trap) (ค่าเริ่มต้น: true)
    #[serde(rename = "allowTrapReversal")]
    pub allow_trap_reversal: bool,
    /// สัญญาณตามแรงแท่ง Spike ต่อเนื่อง (ค่าเริ่มต้น: true)
    #[serde(rename = "allowSpikeContinuation")]
    pub allow_spike_continuation: bool,
}

impl Default for PkTrendOptions {
    fn default() -> Self {
        Self {
            min_trend_score_strong: 40,
            min_close_conviction_up: 0.70,
            max_close_conviction_down: 0.30,
            allow_trap_reversal: true,
            allow_spike_continuation: true,
        }
    }
}

/// ผลลัพธ์การประเมินจุดเข้าเทรด 1 แท่งเทียน
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PkTrendDecision {
    /// คำตัดสินสัญญาณ: 'BUY' | 'SELL' | 'WAIT'
    pub signal: TradeSpotSignal,
    /// Action การเทรด: 'CALL' | 'PUT' | 'WAIT'
    #[serde(rename = "tradeType")]
    pub trade_type: TradeType,
    /// สีที่แนะนำให้เทรด: "green" | "red" | "idle"
    #[serde(rename = "suggestColor")]
    pub suggest_color: String,
    /// เหตุผลการตัดสินใจ
    pub reason: String,
    /// ระดับความแรง เช่น "ULTRA_STRONG_UP", "REVERSAL"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strength: Option<String>,
    /// แท่งนี้เข้าข่ายอันตราย/หลอก ควรระวังเป็นพิเศษ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub danger: Option<bool>,
    /// Case Code ของ pkTrend
    #[serde(rename = "caseCode")]
    pub case_code: String,
}

/// ผลลัพธ์สำหรับแท่งเทียนที่ผ่านการกรอง (Spot) ใน Dataset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotTradeResult {
    pub index: usize,
    pub candletime: i64,
    pub candletime_display: String,
    pub close: f64,
    pub signal: TradeSpotSignal,
    #[serde(rename = "tradeType")]
    pub trade_type: TradeType,
    #[serde(rename = "suggestColor")]
    pub suggest_color: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strength: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub danger: Option<bool>,
    #[serde(rename = "caseCode")]
    pub case_code: String,
}

// ────────────────────────────────────────────────────────────────────────────
// Core Evaluation Logic
// ────────────────────────────────────────────────────────────────────────────

/// ประเมินสัญญาณเข้าเทรดจากโครงสร้าง `TrendDetectResult` 1 แท่งเทียน
/// (ตรงกับ `evaluatePkTrendEntry` / `evaluateTradeSpot` ใน PkTrendEvaluator.js)
pub fn evaluate_pk_trend_entry(
    pk_trend: &TrendDetectResult,
    options: Option<&PkTrendOptions>,
) -> PkTrendDecision {
    let default_opt = PkTrendOptions::default();
    let opt = options.unwrap_or(&default_opt);

    let case_code = pk_trend.case_code;
    let group = pk_trend.group;
    let trend_score = pk_trend.trend_score;
    let close_pos = pk_trend.close_position.unwrap_or(0.5);
    let trend_opt = pk_trend.trend;
    let trend_str = trend_opt.unwrap_or("");
    let extreme_trend_str = pk_trend.extreme_trend.unwrap_or("");

    // 0.0) เช็คกรณีข้อมูลไม่เพียงพอ หรือ System Data
    if group == "System" || trend_opt.is_none() || case_code == "SYS-NODATA" {
        return PkTrendDecision {
            signal: TradeSpotSignal::Wait,
            trade_type: TradeType::Wait,
            suggest_color: "idle".to_string(),
            reason: "ข้อมูลไม่พอ (SYS-NODATA)".to_string(),
            strength: None,
            danger: None,
            case_code: case_code.to_string(),
        };
    }

    // 0.1) กันสัญญาณหลอกก่อนเสมอ — whipsaw ต้องถูกกรองทิ้งไม่ว่า trendScore จะสูงแค่ไหน
    if pk_trend.is_whipsaw {
        return PkTrendDecision {
            signal: TradeSpotSignal::Wait,
            trade_type: TradeType::Wait,
            suggest_color: "idle".to_string(),
            reason: format!(
                "{} — เสี่ยงโดนหลอกฟันปลาสูง ({})",
                pk_trend.whipsaw_status.as_str(), case_code
            ),
            strength: None,
            danger: Some(true),
            case_code: case_code.to_string(),
        };
    }

    // 0.2) ตลาดพัก ไม่มีทิศทางชัดเจน
    if group == "Sideways" {
        return PkTrendDecision {
            signal: TradeSpotSignal::Wait,
            trade_type: TradeType::Wait,
            suggest_color: "idle".to_string(),
            reason: format!("Sideways: {}", case_code),
            strength: None,
            danger: None,
            case_code: case_code.to_string(),
        };
    }

    // 0.3) กลุ่ม spike-trap อันตราย (stop hunt / liquidity grab) — เตือนไม่ให้เข้าเสมอ
    if case_code == "SPK-BULLTRAP"
        || case_code == "SPK-BEARTRAP"
        || case_code == "SPK-NODIRECTION"
        || case_code == "SPK-HESITANT-UP"
    {
        return PkTrendDecision {
            signal: TradeSpotSignal::Wait,
            trade_type: TradeType::Wait,
            suggest_color: "idle".to_string(),
            reason: format!("{} — ความเสี่ยงกลับตัวสูง ไม่ควรเข้า", case_code),
            strength: None,
            danger: Some(true),
            case_code: case_code.to_string(),
        };
    }

    // ===== 1) Trend-following entry: ตามทิศเทรนด์หลัก =====
    let is_trend_follow_group = match group {
        "Up" | "Down" | "Engulfing" => true,
        "Spike" => opt.allow_spike_continuation,
        _ => false,
    };

    if is_trend_follow_group && trend_str != "Sideways" && trend_str != "Rejected" {
        if trend_str == "UpTrend"
            && trend_score >= opt.min_trend_score_strong
            && close_pos >= opt.min_close_conviction_up
        {
            return PkTrendDecision {
                signal: TradeSpotSignal::Buy,
                trade_type: TradeType::Call,
                suggest_color: "green".to_string(),
                reason: format!(
                    "{} ({}) score={} closePos={:.2}",
                    case_code, pk_trend.trend_strength.as_str(), trend_score, close_pos
                ),
                strength: Some(pk_trend.trend_strength.as_str().to_string()),
                danger: None,
                case_code: case_code.to_string(),
            };
        }

        if trend_str == "DownTrend"
            && trend_score <= -opt.min_trend_score_strong
            && close_pos <= opt.max_close_conviction_down
        {
            return PkTrendDecision {
                signal: TradeSpotSignal::Sell,
                trade_type: TradeType::Put,
                suggest_color: "red".to_string(),
                reason: format!(
                    "{} ({}) score={} closePos={:.2}",
                    case_code, pk_trend.trend_strength.as_str(), trend_score, close_pos
                ),
                strength: Some(pk_trend.trend_strength.as_str().to_string()),
                danger: None,
                case_code: case_code.to_string(),
            };
        }
    }

    // ===== 2) Reversal entry: bull trap / bear trap ที่ยืนยันแล้ว =====
    if opt.allow_trap_reversal && group == "Rejected" {
        if case_code == "RJ-BULLTRAP-STRONG" && extreme_trend_str == "UpTrend" && close_pos <= 0.20 {
            return PkTrendDecision {
                signal: TradeSpotSignal::Sell,
                trade_type: TradeType::Put,
                suggest_color: "red".to_string(),
                reason: format!("Bull Trap ยืนยัน closePos={:.2}", close_pos),
                strength: Some("REVERSAL".to_string()),
                danger: None,
                case_code: case_code.to_string(),
            };
        }

        if case_code == "RJ-BEARTRAP-STRONG" && extreme_trend_str == "DownTrend" && close_pos >= 0.80 {
            return PkTrendDecision {
                signal: TradeSpotSignal::Buy,
                trade_type: TradeType::Call,
                suggest_color: "green".to_string(),
                reason: format!("Bear Trap ยืนยัน closePos={:.2}", close_pos),
                strength: Some("REVERSAL".to_string()),
                danger: None,
                case_code: case_code.to_string(),
            };
        }
    }

    // Fallback: ไม่เข้าเงื่อนไข
    PkTrendDecision {
        signal: TradeSpotSignal::Wait,
        trade_type: TradeType::Wait,
        suggest_color: "idle".to_string(),
        reason: format!(
            "{} ไม่เข้าเงื่อนไขที่ตั้งไว้ (score={})",
            if case_code.is_empty() { "N/A" } else { case_code },
            trend_score
        ),
        strength: None,
        danger: None,
        case_code: case_code.to_string(),
    }
}

/// ประเมิน Action โดยตรงจาก `FullAnalysisResult` (analysisdata) ของ `full_analysis_ver2.rs`
pub fn get_action_by_full_analysis(
    analysis: &FullAnalysisResult,
    options: Option<&PkTrendOptions>,
) -> PkTrendDecision {
    evaluate_pk_trend_entry(&analysis.pk_trend, options)
}

/// คัดกรอง Array ของ `FullAnalysisResult` ทั้งหมด เพื่อหาเฉพาะแท่งที่เกิด Trade Spot (Call / Put)
/// (ตรงกับ `filterCandlesByPkTrend` ใน PkTrendEvaluator.js)
pub fn filter_candles_by_pk_trend(
    candles: &[FullAnalysisResult],
    options: Option<&PkTrendOptions>,
) -> Vec<SpotTradeResult> {
    candles
        .iter()
        .filter_map(|candle| {
            let decision = get_action_by_full_analysis(candle, options);
            if decision.signal != TradeSpotSignal::Wait {
                Some(SpotTradeResult {
                    index: candle.index,
                    candletime: candle.candletime,
                    candletime_display: candle.candletime_display.clone(),
                    close: candle.close,
                    signal: decision.signal,
                    trade_type: decision.trade_type,
                    suggest_color: decision.suggest_color,
                    reason: decision.reason,
                    strength: decision.strength,
                    danger: decision.danger,
                    case_code: decision.case_code,
                })
            } else {
                None
            }
        })
        .collect()
}

/// คัดกรอง Slice ของ `TrendDetectResult` ตรง ๆ
/// (ตรงกับ `filterPkTrendArray` ใน PkTrendEvaluator.js)
pub fn filter_pk_trend_slice(
    pk_trends: &[TrendDetectResult],
    options: Option<&PkTrendOptions>,
) -> Vec<(usize, PkTrendDecision)> {
    pk_trends
        .iter()
        .enumerate()
        .filter_map(|(idx, pk)| {
            let decision = evaluate_pk_trend_entry(pk, options);
            if decision.signal != TradeSpotSignal::Wait {
                Some((idx, decision))
            } else {
                None
            }
        })
        .collect()
}

// ────────────────────────────────────────────────────────────────────────────
// Unit Tests
// ────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkDetectTrend_v5::{ScoreBreakdown, TrendDetectResult, TrendStrength, WhipsawStatus};

    fn create_mock_pk_trend(
        case_code: &'static str,
        group: &'static str,
        trend: Option<&'static str>,
        trend_score: i32,
        trend_strength: TrendStrength,
        close_position: Option<f64>,
        extreme_trend: Option<&'static str>,
        is_whipsaw: bool,
    ) -> TrendDetectResult {
        TrendDetectResult {
            code_no: None,
            trend,
            case_code,
            case_desc: "TEST",
            group,
            description: "Test",
            is_spike: group == "Spike",
            extreme_trend,
            close_position,
            body_ratio: Some(0.6),
            range_ratio: Some(1.0),
            structure: None,
            color_sequence: "GGGG".to_string(),
            color_switches: 0,
            is_whipsaw,
            whipsaw_status: if is_whipsaw { WhipsawStatus::ConfirmedWhipsaw } else { WhipsawStatus::Trending },
            whipsaw_warning: "".to_string(),
            trend_score,
            trend_strength,
            score_breakdown: ScoreBreakdown::default(),
            volume_ratio: None,
            volume_confirmed: None,
        }
    }

    #[test]
    fn test_whipsaw_filter() {
        let pk = create_mock_pk_trend(
            "UP-CONFIRM", "Up", Some("UpTrend"), 80, TrendStrength::UltraStrongUp, Some(0.85), None, true,
        );
        let decision = evaluate_pk_trend_entry(&pk, None);
        assert_eq!(decision.signal, TradeSpotSignal::Wait);
        assert_eq!(decision.danger, Some(true));
        assert!(decision.reason.contains("CONFIRMED_WHIPSAW"));
    }

    #[test]
    fn test_sideways_filter() {
        let pk = create_mock_pk_trend(
            "SD-INSIDEBAR", "Sideways", Some("Sideways"), 10, TrendStrength::Neutral, Some(0.50), None, false,
        );
        let decision = evaluate_pk_trend_entry(&pk, None);
        assert_eq!(decision.signal, TradeSpotSignal::Wait);
        assert_eq!(decision.suggest_color, "idle");
    }

    #[test]
    fn test_dangerous_spike_filter() {
        let pk = create_mock_pk_trend(
            "SPK-BULLTRAP", "Spike", Some("UpTrend"), 45, TrendStrength::StrongUp, Some(0.20), None, false,
        );
        let decision = evaluate_pk_trend_entry(&pk, None);
        assert_eq!(decision.signal, TradeSpotSignal::Wait);
        assert_eq!(decision.danger, Some(true));
    }

    #[test]
    fn test_uptrend_following() {
        let pk = create_mock_pk_trend(
            "UP-CONFIRM", "Up", Some("UpTrend"), 65, TrendStrength::StrongUp, Some(0.85), None, false,
        );
        let decision = evaluate_pk_trend_entry(&pk, None);
        assert_eq!(decision.signal, TradeSpotSignal::Buy);
        assert_eq!(decision.trade_type, TradeType::Call);
        assert_eq!(decision.suggest_color, "green");
        assert_eq!(decision.strength, Some("STRONG_UP".to_string()));
    }

    #[test]
    fn test_downtrend_following() {
        let pk = create_mock_pk_trend(
            "DN-CONFIRM", "Down", Some("DownTrend"), -65, TrendStrength::StrongDown, Some(0.15), None, false,
        );
        let decision = evaluate_pk_trend_entry(&pk, None);
        assert_eq!(decision.signal, TradeSpotSignal::Sell);
        assert_eq!(decision.trade_type, TradeType::Put);
        assert_eq!(decision.suggest_color, "red");
    }

    #[test]
    fn test_reversal_bull_and_bear_trap() {
        // Bull Trap -> Expect SELL / PUT
        let pk_bull_trap = create_mock_pk_trend(
            "RJ-BULLTRAP-STRONG", "Rejected", Some("UpTrend"), 30, TrendStrength::MildUp, Some(0.10), Some("UpTrend"), false,
        );
        let d1 = evaluate_pk_trend_entry(&pk_bull_trap, None);
        assert_eq!(d1.signal, TradeSpotSignal::Sell);
        assert_eq!(d1.trade_type, TradeType::Put);
        assert_eq!(d1.suggest_color, "red");
        assert_eq!(d1.strength, Some("REVERSAL".to_string()));

        // Bear Trap -> Expect BUY / CALL
        let pk_bear_trap = create_mock_pk_trend(
            "RJ-BEARTRAP-STRONG", "Rejected", Some("DownTrend"), -30, TrendStrength::MildDown, Some(0.90), Some("DownTrend"), false,
        );
        let d2 = evaluate_pk_trend_entry(&pk_bear_trap, None);
        assert_eq!(d2.signal, TradeSpotSignal::Buy);
        assert_eq!(d2.trade_type, TradeType::Call);
        assert_eq!(d2.suggest_color, "green");
        assert_eq!(d2.strength, Some("REVERSAL".to_string()));
    }

    #[test]
    fn test_filter_candles_by_pk_trend() {
        let pk_buy = create_mock_pk_trend(
            "UP-CONFIRM", "Up", Some("UpTrend"), 65, TrendStrength::StrongUp, Some(0.85), None, false,
        );
        let pk_wait = create_mock_pk_trend(
            "SD-INSIDEBAR", "Sideways", Some("Sideways"), 10, TrendStrength::Neutral, Some(0.50), None, false,
        );

        let candle1 = crate::full_analysis_ver2::FullAnalysisResult {
            index: 0,
            asset_code: "R_100".to_string(),
            candletime: 1600000000,
            candletime_display: "2026-08-18 10:00:00".to_string(),
            open: 100.0,
            high: 105.0,
            low: 99.0,
            close: 104.5,
            color: "green".to_string(),
            next_color: None,
            pip_size: 4.5,
            ema_short_value: 0.0,
            ema_short_direction: "Up".to_string(),
            ema_short_turn_type: "-".to_string(),
            ema_short_slope_value: 0.0,
            emaslope_thereshold: 0.0,
            diff: 0.0,
            ema_short_flat: "n".to_string(),
            ema_medium_value: 0.0,
            ema_medium_direction: "Up".to_string(),
            ema_medium_turn_type: "-".to_string(),
            ema_medium_slope_value: 0.0,
            ema_medium_flat: "n".to_string(),
            ema_long_value: 0.0,
            ema_long_direction: "Up".to_string(),
            ema_long_turn_type: "-".to_string(),
            ema_long_slope_value: 0.0,
            ema_long_flat: "n".to_string(),
            short_medium_gap_value: 0.0,
            is_short_medium_gap_occur: "n".to_string(),
            medium_long_gap_value: 0.0,
            is_medium_long_gap_occur: "n".to_string(),
            ema_above: "ShortAbove".to_string(),
            ema_long_above: "MediumAbove".to_string(),
            macd_12: 0.0,
            macd_23: 0.0,
            previous_ema_short_value: 0.0,
            previous_ema_medium_value: 0.0,
            previous_ema_long_value: 0.0,
            previous_macd_12: 0.0,
            previous_macd_23: 0.0,
            ema_convergence_type: "divergence".to_string(),
            ema_long_convergence_type: "D".to_string(),
            choppy_indicator: 30.0,
            adx_value: 25.0,
            rsi_value: 60.0,
            bb_values: crate::full_analysis_ver2::BollingerBands { upper: 106.0, middle: 100.0, lower: 94.0 },
            bb_position: "NearUpper".to_string(),
            atr_value: 2.0,
            is_abnormal_candle: false,
            is_abnormal_atr: false,
            is_atr: false,
            u_wick: 0.5,
            u_wick_percent: 8.3,
            body: 4.5,
            body_percent: 75.0,
            l_wick: 1.0,
            l_wick_percent: 16.7,
            ema_cut_position: "-".to_string(),
            ema_cut_long_type: "-".to_string(),
            ema_cut_short_long_type: "-".to_string(),
            ema_cut_all_type: "-".to_string(),
            candles_since_ema_cut: 0,
            age_cut_candle_code12: "-".to_string(),
            age_cut_candle_code123: "-".to_string(),
            ema_short_pos: "Body".to_string(),
            ema_medium_pos: "Body".to_string(),
            ema_long_pos: "Body".to_string(),
            bb_bandwidth: 12.0,
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
            up_con_medium_ema: 0,
            down_con_medium_ema: 0,
            up_con_long_ema: 0,
            down_con_long_ema: 0,
            is_mark: "n".to_string(),
            status_code: "0".to_string(),
            status_desc: "-".to_string(),
            status_desc_0: "-".to_string(),
            hint_status: "".to_string(),
            suggest_color: "".to_string(),
            win_status: "".to_string(),
            win_con: 0,
            loss_con: 0,
            smc: crate::full_analysis_ver2::SmcData {
                structures: vec![],
                swing_points: vec![],
                order_blocks: vec![],
                fair_value_gaps: vec![],
                equal_highs_lows: vec![],
                premium_discount_zone: crate::full_analysis_ver2::SmcPremiumDiscountZone {
                    start_time: 0, end_time: 0, premium_top: 0.0, premium_bottom: 0.0, equilibrium: 0.0, discount_top: 0.0, discount_bottom: 0.0,
                },
                strong_weak_levels: vec![],
                swing_trend: "bullish".to_string(),
                internal_trend: "bullish".to_string(),
            },
            tick_volatility: crate::full_analysis_ver2::TickVolatility {
                tick_count: 0, buy_tick_count: 0, sell_tick_count: 0, buy_sell_ratio: 0.5, avg_tick_move: 0.0, max_tick_move: 0.0, sum_tick_move: 0.0, volatility_clustering: 0.0, volatility_level: "Low".to_string(),
            },
            range_detector: crate::full_analysis_ver2::RangeDetectorData {
                in_range: false, range_top: 0.0, range_bottom: 0.0, range_avg: 0.0, range_state: "none".to_string(),
            },
            pk_trend: pk_buy,
            is_alternating_pattern: false,
            alternating_sequence_length: 0,
            is_alternating_trigger: false,
            is_alternating_spike: false,
        };

        let mut candle2 = candle1.clone();
        candle2.index = 1;
        candle2.pk_trend = pk_wait;

        let candles = vec![candle1, candle2];
        let spots = filter_candles_by_pk_trend(&candles, None);

        assert_eq!(spots.len(), 1);
        assert_eq!(spots[0].index, 0);
        assert_eq!(spots[0].signal, TradeSpotSignal::Buy);
        assert_eq!(spots[0].trade_type, TradeType::Call);
        assert_eq!(spots[0].suggest_color, "green");
    }
}

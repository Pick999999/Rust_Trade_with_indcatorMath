//! predict_next_candle - Next-Candle Predictive & 2-Bar Sequence Action Engine
//! Translated from `predictNextCandle.js`
//!
//! ทำนายทิศทางและแนวโน้มของแท่งเทียนถัดไป (Next Candle Prediction)
//! รองรับ 2 โหมด:
//! 1. Pure Candle Mode (ไม่ใช้ Tick Data) — Price Action + V5 Cases + Multi-Bar Context
//! 2. Hybrid Mode (ผสมผสาน Candle + Last 10s Tick Flow) — แม่นยำสูงสุดสำหรับ Realtime
//!
//! พร้อม 2-Bar Sequence Action Engine (วิเคราะห์คู่รหัสแท่งก่อนหน้า -> แท่งปัจจุบัน)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::pkDetectTrend_v5::{
    detect_trend, TrendCandle, TrendDetectResult, TrendStrength, WhipsawStatus,
};

/// โหมดการทำนายแท่งถัดไป
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredictMode {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "pure_candle")]
    PureCandle,
    #[serde(rename = "hybrid")]
    Hybrid,
}

impl PredictMode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::PureCandle => "pure_candle",
            Self::Hybrid => "hybrid",
        }
    }
}

/// พารามิเตอร์การตั้งค่าการทำนาย
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictOptions {
    pub mode: PredictMode,
    #[serde(rename = "timeframeSec")]
    pub timeframe_sec: u64,
}

impl Default for PredictOptions {
    fn default() -> Self {
        Self {
            mode: PredictMode::Auto,
            timeframe_sec: 60,
        }
    }
}

/// ข้อมูล Tick สำหรับการวิเคราะห์ Microstructure
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TickInput {
    #[serde(rename = "candleTimestamp")]
    pub candle_timestamp: i64,
    #[serde(rename = "closePrices")]
    pub close_price: f64,
}

/// ผลการวิเคราะห์ Microstructure ของ Tick ช่วง 10 วินาทีสุดท้าย (Last 10s)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickAnalysisResult {
    #[serde(rename = "totalTicks")]
    pub total_ticks: usize,
    #[serde(rename = "last10sTickCount")]
    pub last_10s_tick_count: usize,
    #[serde(rename = "delta10s")]
    pub delta_10s: f64,
    #[serde(rename = "upTicks")]
    pub up_ticks: usize,
    #[serde(rename = "downTicks")]
    pub down_ticks: usize,
    #[serde(rename = "isClosingSurgeUp")]
    pub is_closing_surge_up: bool,
    #[serde(rename = "isClosingSurgeDown")]
    pub is_closing_surge_down: bool,
    #[serde(rename = "isBullSnapback")]
    pub is_bull_snapback: bool,
    #[serde(rename = "isBearSnapback")]
    pub is_bear_snapback: bool,
    #[serde(rename = "tickBullScore")]
    pub tick_bull_score: u32,
    #[serde(rename = "tickBearScore")]
    pub tick_bear_score: u32,
}

/// ผลลัพธ์การจับคู่รหัส 2-Bar Sequence Matrix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceResult {
    #[serde(rename = "prevCodeNo")]
    pub prev_code_no: Option<u8>,
    #[serde(rename = "currCodeNo")]
    pub curr_code_no: Option<u8>,
    #[serde(rename = "prevCaseCode")]
    pub prev_case_code: String,
    #[serde(rename = "currCaseCode")]
    pub curr_case_code: String,
    #[serde(rename = "pairKey")]
    pub pair_key: String,
    #[serde(rename = "pairDisplay")]
    pub pair_display: String,
    #[serde(rename = "patternName")]
    pub pattern_name: String,
    pub action: &'static str,
    pub confidence: &'static str,
    pub score: u32,
    pub reason: String,
    #[serde(rename = "matchType")]
    pub match_type: &'static str,
}

/// ผลการพยากรณ์แท่งถัดไปแบบเต็มรูปแบบ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionResult {
    #[serde(rename = "nextBarBias")]
    pub next_bar_bias: &'static str,
    #[serde(rename = "expectedAction")]
    pub expected_action: &'static str,
    pub confidence: &'static str,
    pub score: u32,
    #[serde(rename = "bullScore")]
    pub bull_score: u32,
    #[serde(rename = "bearScore")]
    pub bear_score: u32,
    pub mode: &'static str,
    pub reasons: Vec<String>,
    #[serde(rename = "trendResult", skip_serializing_if = "Option::is_none")]
    pub trend_result: Option<TrendDetectResult>,
    #[serde(rename = "sequenceResult", skip_serializing_if = "Option::is_none")]
    pub sequence_result: Option<SequenceResult>,
    #[serde(rename = "tickAnalysis", skip_serializing_if = "Option::is_none")]
    pub tick_analysis: Option<TickAnalysisResult>,
}

/// วิเคราะห์คู่รหัส 2 แท่งต่อเนื่อง (2-Bar Sequence Action Engine)
pub fn evaluate_sequence_action<C: TrendCandle>(candles: &[C], index: usize) -> SequenceResult {
    if candles.is_empty() || index < 1 || index >= candles.len() {
        return SequenceResult {
            prev_code_no: None,
            curr_code_no: None,
            prev_case_code: "SYS-NODATA".to_string(),
            curr_case_code: "SYS-NODATA".to_string(),
            pair_key: "N/A".to_string(),
            pair_display: "—".to_string(),
            pattern_name: "ข้อมูลไม่เพียงพอ".to_string(),
            action: "WAIT",
            confidence: "LOW",
            score: 50,
            reason: "ต้องมีข้อมูลแท่งเทียนอย่างน้อย 2 แท่งในการจับคู่รหัส".to_string(),
            match_type: "NONE",
        };
    }

    let curr_trend = detect_trend(candles, index, None);
    let prev_trend = detect_trend(candles, index - 1, None);

    let p_code_no = prev_trend.code_no.unwrap_or(0);
    let c_code_no = curr_trend.code_no.unwrap_or(0);
    let p_code = prev_trend.case_code;
    let c_code = curr_trend.case_code;

    let pair_key = format!("{}->{}", p_code_no, c_code_no);
    let pair_display = format!("[No. {} ➔ {}] {} ➔ {}", p_code_no, c_code_no, p_code, c_code);

    let curr_candle = &candles[index];
    let curr_is_bullish = curr_candle.close() >= curr_candle.open();

    // กฎคู่รหัสเฉพาะ (High-Probability 2-Bar Transition Rules)
    let matched_rule: Option<(&'static str, &'static str, &'static str, u32, &'static str)> = match pair_key.as_str() {
        // 🚀 Inside Bar Breakouts (17 -> ...)
        "17->1"  => Some(("Inside Bar Breakout UP", "CALL", "HIGH", 85, "Inside Bar สะสมพลังแล้วระเบิดทะลุทำ Higher High ขาขึ้นชัดเจน")),
        "17->2"  => Some(("Inside Bar Breakout DOWN", "PUT", "HIGH", 85, "Inside Bar สะสมพลังแล้วระเบิดทะลุทำ Lower Low ขาลงชัดเจน")),
        "17->9"  => Some(("Inside Bar Bullish Engulfing", "CALL", "MAX", 92, "สะสมพลังในกรอบแล้วตามด้วยแท่งเขียวกลืนกินเต็มแท่ง (Strong Bull Break)")),
        "17->10" => Some(("Inside Bar Bearish Engulfing", "PUT", "MAX", 92, "สะสมพลังในกรอบแล้วตามด้วยแท่งแดงกลืนกินเต็มแท่ง (Strong Bear Break)")),
        "17->3"  => Some(("Inside Bar Spike Breakout UP", "CALL", "HIGH", 88, "Spike พุ่งทะลุกรอบ Inside Bar รุนแรง")),
        "17->4"  => Some(("Inside Bar Spike Breakdown DOWN", "PUT", "HIGH", 88, "Spike ทิ่มทะลุกรอบ Inside Bar รุนแรง")),
        "17->17" => Some(("Double Inside Bar Compression", "WAIT", "HIGH", 90, "Inside Bar ซ้อน 2 แท่ง บีบตัวแคบสุดขีด รอเลือกทาง")),

        // ⚠️ Traps & Exhaustion
        "1->5"   => Some(("UpTrend Spike Bull Trap", "PUT", "MAX", 88, "ขาขึ้นพุ่งทำ New High แล้วโดนทุบรูดมิดแท่ง (Bull Trap) เสี่ยงกลับตัวลงรุนแรง")),
        "1->11"  => Some(("UpTrend Strong Bull Trap", "PUT", "MAX", 86, "พยายามทำ High แต่ถูกแรงขายปฏิเสธอย่างรุนแรง (Strong Bull Trap)")),
        "2->6"   => Some(("DownTrend Spike Bear Trap", "CALL", "MAX", 88, "ขาลงทิ่มทำ New Low แล้วมีแรงซื้อกระชากกลับปิดเต็มแท่ง (Bear Trap) กลับตัวขึ้นแรง")),
        "2->12"  => Some(("DownTrend Strong Bear Trap", "CALL", "MAX", 86, "พยายามทำ Low แต่ถูกแรงซื้อดันกลับอย่างแข็งแกร่ง (Strong Bear Trap)")),
        "3->5"   => Some(("Double Spike Exhaustion Top", "PUT", "MAX", 90, "Spike พุ่งสุดตัวแล้วตามด้วย Bull Trap จบคลื่นขาขึ้นทันที")),
        "4->6"   => Some(("Double Spike Exhaustion Bottom", "CALL", "MAX", 90, "Spike ทิ่มสุดตัวแล้วตามด้วย Bear Trap จบคลื่นขาลงทันที")),

        // 🎯 Trap Confirmation
        "6->1"   => Some(("Bear Trap Confirmed UP", "CALL", "MAX", 94, "Bear Trap ดักขายสำเร็จ และแท่งปัจจุบันดันทำ Higher High ตอกย้ำขาขึ้นชัดเจน")),
        "12->1"  => Some(("Strong Bear Trap Confirmed UP", "CALL", "MAX", 92, "ยืนยันการกลับตัวขึ้นหลังเกิด Strong Bear Trap")),
        "5->2"   => Some(("Bull Trap Confirmed DOWN", "PUT", "MAX", 94, "Bull Trap ดักซื้อสำเร็จ และแท่งปัจจุบันทุบทำ Lower Low ตอกย้ำขาลงชัดเจน")),
        "11->2"  => Some(("Strong Bull Trap Confirmed DOWN", "PUT", "MAX", 92, "ยืนยันการกลับตัวลงหลังเกิด Strong Bull Trap")),

        // 📈 Continuation & Dip Buying
        "1->1"   => Some(("UpTrend Solid Continuation", "CALL", "HIGH", 82, "ทำ Higher High ต่อเนื่อง เทรนด์ขาขึ้นแข็งแกร่ง")),
        "2->2"   => Some(("DownTrend Solid Continuation", "PUT", "HIGH", 82, "ทำ Lower Low ต่อเนื่อง เทรนด์ขาลงแข็งแกร่ง")),
        "1->16"  => Some(("UpTrend Pullback & Dip Buy", "CALL", "HIGH", 80, "ขาขึ้นย่อตัวลงมาติดแนวรับเกิดไส้ล่างยาวดันกลับ (Buy on Dip)")),
        "2->15"  => Some(("DownTrend Pullback & Sell Rally", "PUT", "HIGH", 80, "ขาลงดีดตัวขึ้นไปติดแนวต้านเกิดไส้บนยาวกดลง (Sell on Rally)")),
        "1->23"  => Some(("UpTrend Retest Support", "CALL", "HIGH", 76, "ขาขึ้นย่อทดสอบแนวรับเดิมแล้วเด้งกลับ")),
        "2->22"  => Some(("DownTrend Retest Resistance", "PUT", "HIGH", 76, "ขาลงดีดทดสอบแนวต้านเดิมแล้วไม่ผ่าน")),

        // ⚠️ Exhaustion & Indecision
        "1->13"  => Some(("UpTrend Momentum Loss", "WAIT", "MEDIUM", 62, "ขาขึ้นเริ่มส่งต่อโมเมนตัมไม่ผ่าน (Failed Followthrough)")),
        "2->14"  => Some(("DownTrend Momentum Loss", "WAIT", "MEDIUM", 62, "ขาลงเริ่มส่งต่อโมเมนตัมไม่ผ่าน (Failed Followthrough)")),
        "1->15"  => Some(("UpTrend Upper Wick Rejection", "WAIT", "MEDIUM", 65, "ทำ New High แต่มีไส้บนยาว กดดันโมเมนตัมฝั่งซื้อ")),
        "2->16"  => Some(("DownTrend Lower Wick Rejection", "WAIT", "MEDIUM", 65, "ทำ New Low แต่มีไส้ล่างยาว มีแรงซื้อพยุงก้นแท่ง")),
        "21->17" => Some(("Double Squeeze Compression", "WAIT", "MAX", 95, "ตลาดบีบตัวแคบต่อเนื่อง 2 แท่ง ปริมาณการเทรดนิ่งสนิท รอระเบิดทิศทาง")),
        "21->21" => Some(("Dual Doji Indecision", "WAIT", "HIGH", 90, "Doji ต่อเนื่อง 2 แท่ง สองฝั่งสู้กันเสมอกัน ไร้ทิศทาง")),
        "21->9"  => Some(("Morning Star Bullish Break", "CALL", "HIGH", 84, "จาก Doji ลังเล เปลี่ยนเป็นแท่งเขียวกลืนกินเต็มแท่ง (Morning Reversal)")),
        "21->10" => Some(("Evening Star Bearish Break", "PUT", "HIGH", 84, "จาก Doji ลังเล เปลี่ยนเป็นแท่งแดงกลืนกินเต็มแท่ง (Evening Reversal)")),
        "21->1"  => Some(("Doji Breakout UP", "CALL", "HIGH", 80, "หลุดจาก Doji ลังเลด้วยแท่ง Up Confirm")),
        "21->2"  => Some(("Doji Breakdown DOWN", "PUT", "HIGH", 80, "หลุดจาก Doji ลังเลด้วยแท่ง Down Confirm")),
        "18->19" => Some(("Mixed Signal Choppiness", "WAIT", "HIGH", 85, "สัญญาณขัดแย้ง High/Low สลับไปมา ติด Whipsaw")),
        "19->18" => Some(("Mixed Signal Choppiness", "WAIT", "HIGH", 85, "สัญญาณขัดแย้ง High/Low สลับไปมา ติด Whipsaw")),
        "27->1"  => Some(("Spike Whipsaw Breakout UP", "CALL", "MEDIUM", 72, "หลุดพ้นจาก Whipsaw ด้วยแท่ง Up Confirm")),
        "27->2"  => Some(("Spike Whipsaw Breakdown DOWN", "PUT", "MEDIUM", 72, "หลุดพ้นจาก Whipsaw ด้วยแท่ง Down Confirm")),
        _ => None,
    };

    if let Some((p_name, act, conf, sc, rsn)) = matched_rule {
        return SequenceResult {
            prev_code_no: Some(p_code_no),
            curr_code_no: Some(c_code_no),
            prev_case_code: p_code.to_string(),
            curr_case_code: c_code.to_string(),
            pair_key,
            pair_display,
            pattern_name: p_name.to_string(),
            action: act,
            confidence: conf,
            score: sc,
            reason: rsn.to_string(),
            match_type: "EXACT_RULE",
        };
    }

    // Dynamic Transition Rule
    let (dynamic_act, dynamic_conf, dynamic_score, dynamic_p_name, dynamic_rsn) = if curr_trend.trend == Some("UpTrend") {
        if curr_is_bullish {
            ("CALL", "MEDIUM", 68, format!("UpTrend Momentum ({}➔{})", p_code_no, c_code_no), "ส่งต่อโมเมนตัมขาขึ้นด้วยแท่งเขียว".to_string())
        } else {
            ("WAIT", "LOW", 55, format!("UpTrend Hesitation ({}➔{})", p_code_no, c_code_no), "โครงสร้างขึ้นแต่แท่งปัจจุบันปิดแดง ควรชะลอรอยืนยัน".to_string())
        }
    } else if curr_trend.trend == Some("DownTrend") {
        if !curr_is_bullish {
            ("PUT", "MEDIUM", 68, format!("DownTrend Momentum ({}➔{})", p_code_no, c_code_no), "ส่งต่อโมเมนตัมขาลงด้วยแท่งแดง".to_string())
        } else {
            ("WAIT", "LOW", 55, format!("DownTrend Hesitation ({}➔{})", p_code_no, c_code_no), "โครงสร้างลงแต่แท่งปัจจุบันปิดเขียว ควรชะลอรอยืนยัน".to_string())
        }
    } else {
        ("WAIT", "MEDIUM", 60, format!("Sideways/Transition ({}➔{})", p_code_no, c_code_no), "สภาวะตลาดไร้ทิศทางชัดเจน รอการเลือกทาง".to_string())
    };

    SequenceResult {
        prev_code_no: Some(p_code_no),
        curr_code_no: Some(c_code_no),
        prev_case_code: p_code.to_string(),
        curr_case_code: c_code.to_string(),
        pair_key,
        pair_display,
        pattern_name: dynamic_p_name,
        action: dynamic_act,
        confidence: dynamic_conf,
        score: dynamic_score,
        reason: dynamic_rsn,
        match_type: "DYNAMIC_TRANSITION",
    }
}

/// ทำนายทิศทางแท่งถัดไปสำหรับแท่งที่ `index`
pub fn predict_next_candle<C: TrendCandle>(
    candles: &[C],
    index: usize,
    candle_ticks: Option<&[TickInput]>,
    options: Option<&PredictOptions>,
) -> PredictionResult {
    let default_opts = PredictOptions::default();
    let opts = options.unwrap_or(&default_opts);

    let mode_str = match opts.mode {
        PredictMode::Auto => {
            if candle_ticks.map(|t| t.len() >= 5).unwrap_or(false) {
                "hybrid"
            } else {
                "pure_candle"
            }
        }
        PredictMode::Hybrid => "hybrid",
        PredictMode::PureCandle => "pure_candle",
    };

    if candles.is_empty() || index < 2 || index >= candles.len() {
        return PredictionResult {
            next_bar_bias: "NEUTRAL",
            expected_action: "WAIT",
            confidence: "LOW",
            score: 50,
            bull_score: 50,
            bear_score: 50,
            mode: mode_str,
            reasons: vec!["ข้อมูลแท่งเทียนไม่เพียงพอสำหรับการทำนาย (ต้องการอย่างน้อย 3 แท่ง)".to_string()],
            trend_result: None,
            sequence_result: None,
            tick_analysis: None,
        };
    }

    let current = &candles[index];
    let trend_result = detect_trend(candles, index, None);
    let case_code = trend_result.case_code;
    let structure = trend_result.structure.unwrap_or_default();
    let close_pos = trend_result.close_position.unwrap_or(0.5);
    let body_ratio = trend_result.body_ratio.unwrap_or(0.0);

    let mut bull_raw: u32 = 0;
    let mut bear_raw: u32 = 0;
    let mut reasons: Vec<String> = Vec::new();

    // =========================================================================
    // SECTION 1: Pure Candle Scoring (Max 100 คะแนน)
    // =========================================================================

    // 1.1 V5 Case Signal Scoring (40 คะแนน)
    match case_code {
        "UP-CONFIRM" => {
            bull_raw += 40;
            reasons.push("🟢 ทำ Higher High และปิดยืนโซนบน (Confirmed Up)".to_string());
        }
        "EG-BULLISH" => {
            bull_raw += 42;
            reasons.push("🟢 Bullish Engulfing กลืนแท่งก่อนหน้าสมบูรณ์".to_string());
        }
        "SPK-CONTINUE-UP" => {
            bull_raw += 40;
            reasons.push("🟢 Spike Breakout พุ่งแรงและปิดยืนโซนบน".to_string());
        }
        "RJ-BEARTRAP-STRONG" | "SPK-BEARTRAP" => {
            bull_raw += 42;
            reasons.push("🟢 Bear Trap! ดักขายแล้วมีแรงซื้อดันกลับขึ้นปิดเต็มแท่ง (Reversal Up)".to_string());
        }

        "DN-CONFIRM" => {
            bear_raw += 40;
            reasons.push("🔴 ทำ Lower Low และปิดกดโซนล่าง (Confirmed Down)".to_string());
        }
        "EG-BEARISH" => {
            bear_raw += 42;
            reasons.push("🔴 Bearish Engulfing กลืนแท่งก่อนหน้าสมบูรณ์".to_string());
        }
        "SPK-CONTINUE-DN" => {
            bear_raw += 40;
            reasons.push("🔴 Spike Breakout ทิ่มลงแรงและปิดกดโซนล่าง".to_string());
        }
        "RJ-BULLTRAP-STRONG" | "SPK-BULLTRAP" => {
            bear_raw += 42;
            reasons.push("🔴 Bull Trap! ดักซื้อแล้วถูกแรงขายทุบกลับลงปิดต่ำ (Reversal Down)".to_string());
        }

        "RJ-UPWICK-WEAK" | "RJ-FOLLOWFAIL-UP" => {
            bear_raw += 26;
            bull_raw += 6;
            reasons.push("⚠️ ไส้บนยาวหรือโมเมนตัมขาขึ้นเริ่มแผ่ว (มีแรงขายกดปลายแท่ง)".to_string());
        }
        "RJ-LOWWICK-WEAK" | "RJ-FOLLOWFAIL-DN" => {
            bull_raw += 26;
            bear_raw += 6;
            reasons.push("⚠️ ไส้ล่างยาวหรือโมเมนตัมขาลงเริ่มแผ่ว (มีแรงซื้อดันก้นแท่ง)".to_string());
        }
        "SPK-HESITANT-UP" => {
            bull_raw += 20;
            bear_raw += 12;
            reasons.push("⚡ Spike ขึ้นแต่ปิดไม่เด็ดขาด (มีแรงซื้อแต่ยังลังเล)".to_string());
        }
        "SPK-HESITANT-DN" => {
            bear_raw += 20;
            bull_raw += 12;
            reasons.push("⚡ Spike ลงแต่ปิดไม่เด็ดขาด (มีแรงขายแต่ยังลังเล)".to_string());
        }
        "SD-FLATRESISTANCE" => {
            bear_raw += 24;
            reasons.push("🧱 ชนแนวต้านเดิมซ้ำแต่ไม่ผ่าน (มีโอกาสย่อตัว)".to_string());
        }
        "SD-FLATSUPPORT" => {
            bull_raw += 24;
            reasons.push("🧱 ชนแนวรับเดิมซ้ำแล้วเด้งกลับ (มีโอกาสดีดขึ้น)".to_string());
        }
        "SD-INSIDEBAR" => {
            bull_raw += 10;
            bear_raw += 10;
            reasons.push("📦 Inside Bar ตลาดบีบตัวรอเลือกทาง".to_string());
        }
        "SD-DOJI" => {
            bull_raw += 10;
            bear_raw += 10;
            reasons.push("⚪ Doji แรงซื้อแรงขายสมดุล".to_string());
        }
        "SPK-WHIPSAW" => {
            bull_raw += 6;
            bear_raw += 6;
            reasons.push("🌪️ Spike Whipsaw สับขาหลอกและผันผวนรุนแรง ไร้ทิศทาง".to_string());
        }
        _ => {
            bull_raw += 10;
            bear_raw += 10;
        }
    }

    // 1.2 Structure Scoring (25 คะแนน)
    if structure.higher_high && structure.higher_low {
        bull_raw += 25;
        reasons.push("📈 โครงสร้างขาขึ้นแข็งแกร่ง (HH + HL)".to_string());
    } else if structure.lower_low && structure.lower_high {
        bear_raw += 25;
        reasons.push("📉 โครงสร้างขาลงแข็งแกร่ง (LL + LH)".to_string());
    } else if structure.higher_high {
        bull_raw += 14;
        bear_raw += 4;
    } else if structure.lower_low {
        bear_raw += 14;
        bull_raw += 4;
    }

    // 1.3 Close Position & Body Strength (20 คะแนน)
    if close_pos >= 0.75 {
        bull_raw += 20;
        if body_ratio >= 0.5 {
            bull_raw += 5;
        }
    } else if close_pos <= 0.25 {
        bear_raw += 20;
        if body_ratio >= 0.5 {
            bear_raw += 5;
        }
    } else if close_pos >= 0.55 {
        bull_raw += 10;
    } else if close_pos <= 0.45 {
        bear_raw += 10;
    }

    // 1.4 Streak / Consecutive Momentum (15 คะแนน)
    let mut up_streak = 0;
    let mut down_streak = 0;
    let start_s = if index >= 4 { index - 4 } else { 0 };
    for s in (start_s..=index).rev() {
        let c_candle = &candles[s];
        if c_candle.close() > c_candle.open() {
            if down_streak == 0 {
                up_streak += 1;
            } else {
                break;
            }
        } else if c_candle.close() < c_candle.open() {
            if up_streak == 0 {
                down_streak += 1;
            } else {
                break;
            }
        }
    }

    if up_streak >= 1 && up_streak <= 3 {
        bull_raw += 15;
    } else if up_streak >= 5 {
        bear_raw += 10;
        reasons.push("⚠️ แท่งเขียวต่อเนื่อง 5 แท่ง (ระวังพักตัว / Overextended)".to_string());
    }

    if down_streak >= 1 && down_streak <= 3 {
        bear_raw += 15;
    } else if down_streak >= 5 {
        bull_raw += 10;
        reasons.push("⚠️ แท่งแดงต่อเนื่อง 5 แท่ง (ระวังดีดตัว / Overextended)".to_string());
    }

    // =========================================================================
    // 1.5 Whipsaw Zone & Color Alternation Analysis (Whipsaw Guard Filter)
    // =========================================================================
    let whipsaw_status = trend_result.whipsaw_status;
    let is_whipsaw = trend_result.is_whipsaw;
    let color_seq = &trend_result.color_sequence;
    let color_switches = trend_result.color_switches;
    let trend_strength = trend_result.trend_strength;

    let is_confirmed_whipsaw = whipsaw_status == WhipsawStatus::ConfirmedWhipsaw
        || trend_strength == TrendStrength::WhipsawZone
        || color_switches >= 3;
    let is_entering_whipsaw = whipsaw_status == WhipsawStatus::EnteringWhipsaw
        || (color_switches == 2 && is_whipsaw);

    if is_confirmed_whipsaw {
        // ดึงคะแนนเข้าสู่โซนสมดุล (Dampen extreme directional bias) เพื่อตัดสัญญาณหลอก
        bull_raw = ((bull_raw as f64 * 0.35) + 20.0).round() as u32;
        bear_raw = ((bear_raw as f64 * 0.35) + 20.0).round() as u32;
        reasons.push(format!("🚫 [Whipsaw Guard] ตรวจพบการสลับสีฟันปลา 4 แท่งติด ({}) สภาวะ Whipsaw รุนแรง", color_seq));
    } else if is_entering_whipsaw {
        bull_raw = ((bull_raw as f64 * 0.70) + 10.0).round() as u32;
        bear_raw = ((bear_raw as f64 * 0.70) + 10.0).round() as u32;
        reasons.push(format!("⚠️ [Whipsaw Alert] ตลาดเริ่มสลับสี 3 แท่งติด ({}) เสี่ยงต่อการกลับตัวหลอก", color_seq));
    }

    // =========================================================================
    // SECTION 2: Tick Microstructure Analysis (Last 10s Closing Flow)
    // =========================================================================
    let mut tick_analysis: Option<TickAnalysisResult> = None;
    let mut tick_weight = 0.0;

    if mode_str == "hybrid" {
        if let Some(ticks) = candle_ticks {
            if ticks.len() >= 5 {
                tick_weight = 0.35;
                let candle_start = current.time();
                let candle_end = candle_start + opts.timeframe_sec as i64;
                let last10s_start = candle_end - 10;

                let mut last10s_ticks: Vec<&TickInput> = ticks
                    .iter()
                    .filter(|t| t.candle_timestamp >= last10s_start && t.candle_timestamp <= candle_end)
                    .collect();

                if last10s_ticks.len() < 3 {
                    let slice_count = std::cmp::max(3, ticks.len() / 5);
                    last10s_ticks = ticks[ticks.len().saturating_sub(slice_count)..].iter().collect();
                }

                if !last10s_ticks.is_empty() {
                    let tick_prices: Vec<f64> = last10s_ticks.iter().map(|t| t.close_price).collect();
                    let p_start10s = tick_prices[0];
                    let p_end10s = tick_prices[tick_prices.len() - 1];
                    let delta_10s = p_end10s - p_start10s;

                    let mut max_10s = tick_prices[0];
                    let mut min_10s = tick_prices[0];
                    for &p in &tick_prices {
                        if p > max_10s { max_10s = p; }
                        if p < min_10s { min_10s = p; }
                    }
                    let range_10s = max_10s - min_10s;

                    let mut up_ticks = 0;
                    let mut down_ticks = 0;
                    for t_idx in 1..tick_prices.len() {
                        if tick_prices[t_idx] > tick_prices[t_idx - 1] {
                            up_ticks += 1;
                        } else if tick_prices[t_idx] < tick_prices[t_idx - 1] {
                            down_ticks += 1;
                        }
                    }

                    let last3_count = std::cmp::min(4, tick_prices.len());
                    let p_last3_start = tick_prices[tick_prices.len() - last3_count];
                    let delta_last3 = p_end10s - p_last3_start;

                    let is_closing_surge_up = delta_10s > 0.0 && p_end10s >= max_10s - (range_10s * 0.15) && up_ticks > down_ticks;
                    let is_closing_surge_down = delta_10s < 0.0 && p_end10s <= min_10s + (range_10s * 0.15) && down_ticks > up_ticks;
                    let is_bull_snapback = delta_10s < 0.0 && delta_last3 > 0.0 && (delta_last3 / (if range_10s > 0.0 { range_10s } else { 1.0 })) >= 0.4;
                    let is_bear_snapback = delta_10s > 0.0 && delta_last3 < 0.0 && (delta_last3.abs() / (if range_10s > 0.0 { range_10s } else { 1.0 })) >= 0.4;

                    let mut tick_bull_score = 50;
                    let mut tick_bear_score = 50;

                    if is_closing_surge_up {
                        tick_bull_score = 90;
                        tick_bear_score = 10;
                        reasons.push("⚡ [Tick 10s] Closing Surge พุ่งขึ้นแรงและปิดยืนจุดสูงสุด".to_string());
                    } else if is_closing_surge_down {
                        tick_bear_score = 90;
                        tick_bull_score = 10;
                        reasons.push("⚡ [Tick 10s] Closing Surge ทิ่มลงแรงและปิดกดจุดต่ำสุด".to_string());
                    } else if is_bear_snapback {
                        tick_bear_score = 80;
                        tick_bull_score = 20;
                        reasons.push("⚡ [Tick 10s] เกิด Bearish Snapback โดนตบลงแรงในวินาทีสุดท้าย".to_string());
                    } else if is_bull_snapback {
                        tick_bull_score = 80;
                        tick_bear_score = 20;
                        reasons.push("⚡ [Tick 10s] เกิด Bullish Snapback มีแรงดีดกลับขึ้นในวินาทีสุดท้าย".to_string());
                    } else if delta_10s > 0.0 {
                        tick_bull_score = 65;
                        tick_bear_score = 35;
                        reasons.push("⚡ [Tick 10s] ราคาขยับขึ้นบวกสุทธิในช่วง 10 วิสุดท้าย".to_string());
                    } else if delta_10s < 0.0 {
                        tick_bear_score = 65;
                        tick_bull_score = 35;
                        reasons.push("⚡ [Tick 10s] ราคาขยับลงลบสุทธิในช่วง 10 วิสุดท้าย".to_string());
                    }

                    tick_analysis = Some(TickAnalysisResult {
                        total_ticks: ticks.len(),
                        last_10s_tick_count: last10s_ticks.len(),
                        delta_10s: (delta_10s * 100.0).round() / 100.0,
                        up_ticks,
                        down_ticks,
                        is_closing_surge_up,
                        is_closing_surge_down,
                        is_bull_snapback,
                        is_bear_snapback,
                        tick_bull_score,
                        tick_bear_score,
                    });
                }
            }
        }
    }

    // =========================================================================
    // SECTION 3: Final Combination & Decision Matrix
    // =========================================================================
    let candle_total = std::cmp::max(1, bull_raw + bear_raw) as f64;
    let candle_bull_pct = (bull_raw as f64 / candle_total) * 100.0;
    let candle_bear_pct = (bear_raw as f64 / candle_total) * 100.0;

    let (final_bull_score_f, final_bear_score_f) = if let Some(ref ta) = tick_analysis {
        if tick_weight > 0.0 {
            (
                (candle_bull_pct * 0.65) + (ta.tick_bull_score as f64 * 0.35),
                (candle_bear_pct * 0.65) + (ta.tick_bear_score as f64 * 0.35),
            )
        } else {
            (candle_bull_pct, candle_bear_pct)
        }
    } else {
        (candle_bull_pct, candle_bear_pct)
    };

    let final_bull_score = final_bull_score_f.round().clamp(1.0, 99.0) as u32;
    let final_bear_score = final_bear_score_f.round().clamp(1.0, 99.0) as u32;

    let net_diff = (final_bull_score as i32 - final_bear_score as i32).abs() as u32;
    let dominant_score = std::cmp::max(final_bull_score, final_bear_score);

    let (next_bar_bias, expected_action, confidence) = if is_confirmed_whipsaw {
        reasons.push("🛑 [Action Blocked] บังคับ WAIT เนื่องจากอยู่ใน Whipsaw Zone เพื่อป้องกันการโดนหลอก".to_string());
        ("NEUTRAL", "WAIT", "LOW")
    } else {
        let required_spread = if is_entering_whipsaw { 24 } else { 14 };
        let (nbb, ea) = if final_bull_score > final_bear_score + required_spread {
            ("BULLISH", "CALL")
        } else if final_bear_score > final_bull_score + required_spread {
            ("BEARISH", "PUT")
        } else {
            ("NEUTRAL", "WAIT")
        };

        let conf = if dominant_score >= 75 && net_diff >= 30 && !is_entering_whipsaw {
            "HIGH"
        } else if dominant_score >= 58 && net_diff >= 15 {
            if is_entering_whipsaw { "LOW" } else { "MEDIUM" }
        } else {
            "LOW"
        };

        (nbb, ea, conf)
    };

    let sequence_result = Some(evaluate_sequence_action(candles, index));

    PredictionResult {
        next_bar_bias,
        expected_action,
        confidence,
        score: dominant_score,
        bull_score: final_bull_score,
        bear_score: final_bear_score,
        mode: mode_str,
        reasons,
        trend_result: Some(trend_result),
        sequence_result,
        tick_analysis,
    }
}

/// ทำนายทั้งชุดข้อมูลแท่งเทียน
pub fn predict_next_candle_series<C: TrendCandle>(
    candles: &[C],
    ticks_map: Option<&HashMap<i64, Vec<TickInput>>>,
    options: Option<&PredictOptions>,
) -> Vec<PredictionResult> {
    candles
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let ticks = ticks_map.and_then(|m| m.get(&c.time()).map(|v| v.as_slice()));
            predict_next_candle(candles, i, ticks, options)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predict_next_candle_basic() {
        let candles = vec![
            (10.0, 12.0, 9.0, 11.0),
            (11.0, 13.0, 10.0, 12.0),
            (12.0, 15.0, 11.5, 14.5), // Strong Up Confirm
        ];
        let res = predict_next_candle(&candles, 2, None, None);
        assert_eq!(res.next_bar_bias, "BULLISH");
        assert_eq!(res.expected_action, "CALL");
        assert!(res.bull_score > res.bear_score);
    }

    #[test]
    fn test_sequence_action_bull_trap() {
        let candles = vec![
            (9.0, 10.0, 8.5, 9.5),
            (9.5, 11.0, 9.0, 10.5),
            (10.5, 12.5, 10.0, 12.0), // index 2: Up Confirm (No. 1)
            (12.0, 14.0, 10.0, 10.2), // index 3: Higher High, dumped to close at low (Strong Bull Trap No. 11)
        ];
        let seq = evaluate_sequence_action(&candles, 3);
        assert_eq!(seq.action, "PUT");
        assert_eq!(seq.pair_key, "1->11");
        assert_eq!(seq.match_type, "EXACT_RULE");
    }

    #[test]
    fn test_predict_whipsaw_guard() {
        let candles = vec![
            (10.0, 11.0, 9.8, 10.8), // Green
            (10.8, 11.2, 9.5, 9.6),  // Red
            (9.6, 11.5, 9.4, 11.4),  // Green
            (11.4, 11.6, 9.2, 9.3),  // Red (4 switches -> Confirmed Whipsaw)
        ];
        let res = predict_next_candle(&candles, 3, None, None);
        assert_eq!(res.expected_action, "WAIT");
        assert_eq!(res.next_bar_bias, "NEUTRAL");
        assert_eq!(res.confidence, "LOW");
        assert!(res.reasons.iter().any(|r| r.contains("Whipsaw Guard")));
    }
}

//! getActionByPKTrendVerSelectCaseCode.rs
//!
//! ตรรกะการตัดสินใจ Action และ Suggest Color จากการตั้งค่าใน `setup/strategy_case_codes.json`
//! โดยดึงข้อมูลจาก `pkTrend` (TrendDetectResult) ใน FullAnalysisResult
//! และค้นหาตาม `code_no` หรือ `case_code`
//!
//! - หาก action = "call" -> แนะนำ Action: CALL / suggest_color: "green"
//! - หาก action = "put"  -> แนะนำ Action: PUT  / suggest_color: "red"
//! - หาก action = "idle", ค่าว่าง "" หรือไม่พบ -> แนะนำ Action: IDLE / suggest_color: "idle"

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::full_analysis_ver2::FullAnalysisResult;
use crate::pkDetectTrend_v5::TrendDetectResult;
use crate::get_action::StrategyDecision;

/// การตั้งค่า Pre-filters สำหรับคัดกรองก่อนเข้าหา Case Code
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyFilterConfig {
    #[serde(rename = "isuseAdx", default = "default_y")]
    pub isuse_adx: String,
    #[serde(rename = "adxValue", default = "default_adx_value")]
    pub adx_value: f64,
    #[serde(rename = "isuseChoppy", default = "default_y")]
    pub isuse_choppy: String,
    #[serde(rename = "choppyValue", default = "default_choppy_value")]
    pub choppy_value: f64,
    #[serde(rename = "isuseBbSqueeze", default = "default_y")]
    pub isuse_bb_squeeze: String,
    #[serde(rename = "bbBandwidthThreshold", default = "default_bb_bw_value")]
    pub bb_bandwidth_threshold: f64,
}

fn default_y() -> String { "y".to_string() }
fn default_adx_value() -> f64 { 25.0 }
fn default_choppy_value() -> f64 { 38.2 }
fn default_bb_bw_value() -> f64 { 0.5 }

impl Default for StrategyFilterConfig {
    fn default() -> Self {
        Self {
            isuse_adx: "y".to_string(),
            adx_value: 25.0,
            isuse_choppy: "y".to_string(),
            choppy_value: 38.2,
            isuse_bb_squeeze: "y".to_string(),
            bb_bandwidth_threshold: 0.5,
        }
    }
}

/// รายการการตั้งค่า 1 รูปแบบจาก `strategy_case_codes.json`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyCaseCodeRule {
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub code_no: Option<i64>,
    #[serde(default)]
    pub case_code: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub case_desc: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub group_name: Option<String>,
    #[serde(default)]
    pub trend: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// โครงสร้างรองรับไฟล์ JSON ทั้ง 2 รูปแบบ: Object { filters, cases } หรือ Array [ cases ]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StrategyCaseCodesPayload {
    Structured {
        #[serde(default)]
        filters: StrategyFilterConfig,
        cases: Vec<StrategyCaseCodeRule>,
    },
    Array(Vec<StrategyCaseCodeRule>),
}

/// ผลลัพธ์การตัดสินใจจากโมดูล Case Code Selector
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CaseCodeDecision {
    /// Action: "CALL" | "PUT" | "IDLE"
    pub action: String,
    /// สีที่แนะนำ: "green" | "red" | "idle"
    #[serde(rename = "suggestColor")]
    pub suggest_color: String,
    /// code_no ของแท่งเทียน (ถ้ามี)
    #[serde(rename = "codeNo")]
    pub code_no: Option<u8>,
    /// case_code ของแท่งเทียน
    #[serde(rename = "caseCode")]
    pub case_code: String,
    /// เหตุผลการตัดสินใจ
    pub reason: String,
    /// กฎที่แมตช์ได้จาก JSON (ถ้ามี)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_rule: Option<StrategyCaseCodeRule>,
}

/// โครงสร้างจัดเก็บ Cache ในหน่วยความจำพร้อมตรวจสอบเวลาแก้ไขไฟล์ (Hot Reload)
struct RulesCache {
    filters: Option<StrategyFilterConfig>,
    rules: Vec<StrategyCaseCodeRule>,
    resolved_path: Option<PathBuf>,
    last_modified: Option<SystemTime>,
}

static CACHE: RwLock<RulesCache> = RwLock::new(RulesCache {
    filters: None,
    rules: Vec::new(),
    resolved_path: None,
    last_modified: None,
});

/// รายชื่อ path ที่เป็นไปได้ในการค้นหา `strategy_case_codes.json`
const CANDIDATE_PATHS: &[&str] = &[
    "setup/strategy_case_codes.json",
    "indicators_Multiplex_Ver1/setup/strategy_case_codes.json",
    "D:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/setup/strategy_case_codes.json",
    "../setup/strategy_case_codes.json",
    "public/setup/strategy_case_codes.json",
];

/// ค้นหา path ของไฟล์ `strategy_case_codes.json` ที่มีอยู่จริง
fn find_strategy_file_path() -> Option<PathBuf> {
    for path_str in CANDIDATE_PATHS {
        let p = Path::new(path_str);
        if p.exists() && p.is_file() {
            return Some(p.to_path_buf());
        }
    }
    None
}

/// โหลดการตั้งค่าทั้ง filters และ cases จากไฟล์ JSON โดยมีระบบ Cache และ Hot-reload
pub fn load_strategy_config() -> (StrategyFilterConfig, Vec<StrategyCaseCodeRule>) {
    // 1. ตรวจสอบว่า cache ปัจจุบันยังใช้งานได้หรือไม่
    {
        let cache = CACHE.read();
        if let Some(ref path) = cache.resolved_path {
            if let Ok(metadata) = fs::metadata(path) {
                if let Ok(modified) = metadata.modified() {
                    if Some(modified) == cache.last_modified && !cache.rules.is_empty() {
                        return (cache.filters.clone().unwrap_or_default(), cache.rules.clone());
                    }
                }
            }
        }
    }

    // 2. หากยังไม่มี cache หรือไฟล์มีการแก้ไข ให้อัปเดต cache ใหม่
    let mut cache = CACHE.write();
    let file_path = cache.resolved_path.clone().or_else(find_strategy_file_path);

    if let Some(path) = file_path {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(payload) = serde_json::from_str::<StrategyCaseCodesPayload>(&content) {
                let (filters, rules) = match payload {
                    StrategyCaseCodesPayload::Structured { filters, cases } => (filters, cases),
                    StrategyCaseCodesPayload::Array(cases) => (StrategyFilterConfig::default(), cases),
                };
                let modified = fs::metadata(&path).ok().and_then(|m| m.modified().ok());
                cache.filters = Some(filters.clone());
                cache.rules = rules.clone();
                cache.resolved_path = Some(path);
                cache.last_modified = modified;
                return (filters, rules);
            }
        }
    }

    // กรณีหาไฟล์ไม่เจอ หรือ parse ไม่สำเร็จ ให้คืนค่า rules เดิมใน cache
    (cache.filters.clone().unwrap_or_default(), cache.rules.clone())
}

/// โหลดเฉพาะรายการ rules (สำหรับความเข้ากันได้ย้อนหลัง)
pub fn load_strategy_rules() -> Vec<StrategyCaseCodeRule> {
    load_strategy_config().1
}

/// ประเมิน Action โดยตรงจาก `TrendDetectResult` 1 แท่งเทียน
pub fn evaluate_by_case_code(pk_trend: &TrendDetectResult) -> CaseCodeDecision {
    let rules = load_strategy_rules();
    let candle_code_no = pk_trend.code_no;
    let candle_case_code = pk_trend.case_code.trim();

    // ค้นหากฎที่ตรงกัน:
    // 1. ลองแมตช์ด้วย code_no ก่อน (ถ้ามี candle_code_no)
    // 2. หรือแมตช์ด้วย case_code (case-insensitive)
    let matched = rules.iter().find(|rule| {
        if let (Some(c_no), Some(r_no)) = (candle_code_no, rule.code_no) {
            if (c_no as i64) == r_no {
                return true;
            }
        }
        if !candle_case_code.is_empty() && !rule.case_code.is_empty() {
            if rule.case_code.trim().eq_ignore_ascii_case(candle_case_code) {
                return true;
            }
        }
        false
    });

    if let Some(rule) = matched {
        let act_lower = rule.action.trim().to_lowercase();
        let (action, suggest_color) = match act_lower.as_str() {
            "call" => ("CALL".to_string(), "green".to_string()),
            "put" => ("PUT".to_string(), "red".to_string()),
            _ => ("IDLE".to_string(), "idle".to_string()),
        };

        let reason = if action == "IDLE" {
            format!(
                "CaseCode [{}] (code_no: {:?}) ตั้งค่า Action เป็น Idle/ว่าง ใน strategy_case_codes.json",
                candle_case_code, candle_code_no
            )
        } else {
            format!(
                "แมตช์ CaseCode [{}] (code_no: {:?}) -> Action [{}] จาก strategy_case_codes.json ({})",
                candle_case_code,
                candle_code_no,
                action,
                rule.case_desc.as_deref().unwrap_or("-")
            )
        };

        CaseCodeDecision {
            action,
            suggest_color,
            code_no: candle_code_no,
            case_code: candle_case_code.to_string(),
            reason,
            matched_rule: Some(rule.clone()),
        }
    } else {
        CaseCodeDecision {
            action: "IDLE".to_string(),
            suggest_color: "idle".to_string(),
            code_no: candle_code_no,
            case_code: candle_case_code.to_string(),
            reason: format!(
                "ไม่พบการตั้งค่าสำหรับ CaseCode [{}] (code_no: {:?}) ใน strategy_case_codes.json",
                candle_case_code, candle_code_no
            ),
            matched_rule: None,
        }
    }
}

/// ประเมิน Action จาก `FullAnalysisResult` (analysisdata)
/// โดยตรวจสอบ Pre-filters (BB Squeeze, ADX, Choppiness) ก่อน หากไม่ผ่านจะส่ง IDLE ทันที
pub fn get_action_by_full_analysis(analysis: &FullAnalysisResult) -> CaseCodeDecision {
    let (filters, _) = load_strategy_config();

    // 1. ตรวจสอบเงื่อนไข BB Squeeze (ถ้าตั้ง 'y' แล้วตลาดบีบตัวรุนแรง ให้งดเข้าออเดอร์)
    if filters.isuse_bb_squeeze.eq_ignore_ascii_case("y") {
        if analysis.is_bb_squeeze || analysis.bb_bandwidth < filters.bb_bandwidth_threshold {
            return CaseCodeDecision {
                action: "IDLE".to_string(),
                suggest_color: "idle".to_string(),
                code_no: analysis.pk_trend.code_no,
                case_code: analysis.pk_trend.case_code.to_string(),
                reason: format!(
                    "ติดตัวกรอง BB Squeeze (ตลาดบีบตัวรุนแรง is_squeeze: {}, bw: {:.4} < {:.4}) -> IDLE",
                    analysis.is_bb_squeeze, analysis.bb_bandwidth, filters.bb_bandwidth_threshold
                ),
                matched_rule: None,
            };
        }
    }

    // 2. ตรวจสอบเงื่อนไข ADX (ถ้าตั้ง 'y' แล้วแรงเทรนด์ไม่ถึงเกณฑ์ ให้งดเข้าออเดอร์)
    if filters.isuse_adx.eq_ignore_ascii_case("y") {
        if analysis.adx_value < filters.adx_value {
            return CaseCodeDecision {
                action: "IDLE".to_string(),
                suggest_color: "idle".to_string(),
                code_no: analysis.pk_trend.code_no,
                case_code: analysis.pk_trend.case_code.to_string(),
                reason: format!(
                    "ติดตัวกรอง ADX (ค่า ADX {:.2} < เกณฑ์ {:.2} แรงเทรนด์ไม่พอ) -> IDLE",
                    analysis.adx_value, filters.adx_value
                ),
                matched_rule: None,
            };
        }
    }

    // 3. ตรวจสอบเงื่อนไข Choppiness (ถ้าตั้ง 'y' แล้วค่าความช็อปปี้สูงเกินเกณฑ์ คือตลาด Sideways ให้งดเข้าออเดอร์)
    if filters.isuse_choppy.eq_ignore_ascii_case("y") {
        if analysis.choppy_indicator > filters.choppy_value {
            return CaseCodeDecision {
                action: "IDLE".to_string(),
                suggest_color: "idle".to_string(),
                code_no: analysis.pk_trend.code_no,
                case_code: analysis.pk_trend.case_code.to_string(),
                reason: format!(
                    "ติดตัวกรอง Choppiness (ค่า CI {:.2} > เกณฑ์ {:.2} ตลาด Sideways) -> IDLE",
                    analysis.choppy_indicator, filters.choppy_value
                ),
                matched_rule: None,
            };
        }
    }

    // เมื่อผ่าน Pre-filters ทั้งหมดแล้ว ค่อยไปค้นหากฎตาม Case Code
    evaluate_by_case_code(&analysis.pk_trend)
}

/// ฟังก์ชันสำหรับขอ `suggest_color` สำหรับระบบเทรดหลัก (`"green"` | `"red"` | `"idle"`)
pub fn get_suggest_color_case_code(analysis: &FullAnalysisResult, _loss_con: u32) -> String {
    let decision = get_action_by_full_analysis(analysis);
    decision.suggest_color
}

/// ฟังก์ชันสำหรับขอ `StrategyDecision` พร้อมเหตุผลและ Code สำหรับรายงานและวิเคราะห์
pub fn get_suggest_color_case_code_with_reason(
    analysis: &FullAnalysisResult,
    _loss_con: u32,
) -> StrategyDecision {
    let decision = get_action_by_full_analysis(analysis);
    let conditions = vec![
        format!("CaseCode: {}", decision.case_code),
        format!("CodeNo: {:?}", decision.code_no),
        format!("Action: {}", decision.action),
    ];

    let code = format!(
        "PKT-CC-{}",
        if decision.case_code.is_empty() {
            "NODATA"
        } else {
            &decision.case_code
        }
    );

    StrategyDecision {
        suggest_color: decision.suggest_color,
        reason: decision.reason,
        conditions_matched: conditions,
        code,
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Unit Tests
// ────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkDetectTrend_v5::{ScoreBreakdown, TrendStrength, WhipsawStatus};

    fn create_mock_pk(code_no: Option<u8>, case_code: &'static str) -> TrendDetectResult {
        TrendDetectResult {
            code_no,
            trend: Some("UpTrend"),
            case_code,
            case_desc: "TEST",
            group: "Up",
            description: "Test",
            is_spike: false,
            extreme_trend: None,
            close_position: Some(0.8),
            body_ratio: Some(0.6),
            range_ratio: Some(1.0),
            structure: None,
            color_sequence: "G".to_string(),
            color_switches: 0,
            is_whipsaw: false,
            whipsaw_status: WhipsawStatus::Trending,
            whipsaw_warning: "".to_string(),
            trend_score: 50,
            trend_strength: TrendStrength::StrongUp,
            score_breakdown: ScoreBreakdown::default(),
            volume_ratio: None,
            volume_confirmed: None,
        }
    }

    #[test]
    fn test_mock_rules_evaluation() {
        // ทดสอบ inject กฎจำลองเข้า cache เพื่อทดสอบลอจิก
        let test_rules = vec![
            StrategyCaseCodeRule {
                id: Some(1),
                code_no: Some(1),
                case_code: "UP-CONFIRM".to_string(),
                action: "put".to_string(),
                case_desc: Some("CONFIRMED_UP".to_string()),
                category: Some("CORE".to_string()),
                group_name: Some("Up".to_string()),
                trend: Some("UpTrend".to_string()),
                description: None,
            },
            StrategyCaseCodeRule {
                id: Some(5),
                code_no: Some(5),
                case_code: "SPK-BULLTRAP".to_string(),
                action: "call".to_string(),
                case_desc: Some("SPIKE_BULL_TRAP".to_string()),
                category: Some("CORE".to_string()),
                group_name: Some("Spike".to_string()),
                trend: Some("Rejected".to_string()),
                description: None,
            },
            StrategyCaseCodeRule {
                id: Some(3),
                code_no: Some(3),
                case_code: "SPK-CONTINUE-UP".to_string(),
                action: "idle".to_string(),
                case_desc: Some("SPIKE_CONTINUATION_UP".to_string()),
                category: Some("CORE".to_string()),
                group_name: Some("Spike".to_string()),
                trend: Some("UpTrend".to_string()),
                description: None,
            },
            StrategyCaseCodeRule {
                id: Some(6),
                code_no: Some(6),
                case_code: "SPK-BEARTRAP".to_string(),
                action: "".to_string(),
                case_desc: Some("SPIKE_BEAR_TRAP".to_string()),
                category: Some("CORE".to_string()),
                group_name: Some("Spike".to_string()),
                trend: Some("Rejected".to_string()),
                description: None,
            },
        ];

        {
            let mut cache = CACHE.write();
            cache.rules = test_rules;
            cache.resolved_path = Some(PathBuf::from("setup/strategy_case_codes.json"));
            cache.last_modified = Some(SystemTime::now());
        }

        // 1. Case: UP-CONFIRM -> action: "put" -> PUT / red
        let pk1 = create_mock_pk(Some(1), "UP-CONFIRM");
        let d1 = evaluate_by_case_code(&pk1);
        assert_eq!(d1.action, "PUT");
        assert_eq!(d1.suggest_color, "red");

        // 2. Case: SPK-BULLTRAP -> action: "call" -> CALL / green
        let pk2 = create_mock_pk(Some(5), "SPK-BULLTRAP");
        let d2 = evaluate_by_case_code(&pk2);
        assert_eq!(d2.action, "CALL");
        assert_eq!(d2.suggest_color, "green");

        // 3. Case: SPK-CONTINUE-UP -> action: "idle" -> IDLE / idle
        let pk3 = create_mock_pk(Some(3), "SPK-CONTINUE-UP");
        let d3 = evaluate_by_case_code(&pk3);
        assert_eq!(d3.action, "IDLE");
        assert_eq!(d3.suggest_color, "idle");

        // 4. Case: SPK-BEARTRAP -> action: "" (blank) -> IDLE / idle
        let pk4 = create_mock_pk(Some(6), "SPK-BEARTRAP");
        let d4 = evaluate_by_case_code(&pk4);
        assert_eq!(d4.action, "IDLE");
        assert_eq!(d4.suggest_color, "idle");

        // 5. Case: รหัสที่ไม่มีใน JSON -> IDLE / idle
        let pk5 = create_mock_pk(Some(99), "UNKNOWN-CODE");
        let d5 = evaluate_by_case_code(&pk5);
        assert_eq!(d5.action, "IDLE");
        assert_eq!(d5.suggest_color, "idle");
    }
}

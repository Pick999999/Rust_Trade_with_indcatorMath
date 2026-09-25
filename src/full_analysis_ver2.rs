use axum::{Json, extract::State};
use std::fs;
use serde::{Deserialize, Serialize};
use crate::AppState;
use turbo_indicators::OHLCV;
use turbo_indicators::{RSI, ChoppinessIndex, HMA, WMA, simd, StatefulATR};
use chrono::{TimeZone, Local};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfigPayload {
    pub ema: EmaConfigGroup,
    pub indicators: IndicatorsConfigGroup,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EmaConfigGroup {
    pub short: EmaConfig,
    pub medium: EmaConfig,
    pub long: EmaConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EmaConfig {
    pub period: u32,
    #[serde(rename = "type")]
    pub ema_type: String,
}

fn default_bb_std_dev() -> f64 { 2.0 }
fn default_bb_ma_type() -> String { "sma".to_string() }
fn default_squeeze_percentile() -> f64 { 20.0 }
fn default_squeeze_lookback() -> usize { 120 }
fn default_flat_slope_threshold() -> f64 { 0.008 }
fn default_flat_min_bars() -> usize { 3 }

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IndicatorsConfigGroup {
    pub adx_period: u32,
    pub atr_period: u32,
    pub atr_multi: f64,
    pub bb_period: u32,
    #[serde(default = "default_bb_std_dev", alias = "bbStdDev", alias = "bb_std_dev")]
    pub bb_std_dev: f64,
    #[serde(default = "default_bb_ma_type", alias = "bbMaType", alias = "bb_ma_type")]
    pub bb_ma_type: String,
    #[serde(default = "default_squeeze_percentile", alias = "squeezePct", alias = "squeeze_pct", alias = "squeezePercentile")]
    pub squeeze_percentile: f64,
    #[serde(default = "default_squeeze_lookback", alias = "squeezeLookback", alias = "squeeze_lookback")]
    pub squeeze_lookback: usize,
    #[serde(default = "default_flat_slope_threshold", alias = "flatSlopeThreshold", alias = "flat_slope_threshold")]
    pub flat_slope_threshold: f64,
    #[serde(default = "default_flat_min_bars", alias = "flatMinBars", alias = "flat_min_bars")]
    pub flat_min_bars: usize,
    pub ci_period: u32,
    pub smc_period: u32,
}

fn default_choppy_combined_mode() -> String { "or".to_string() }
fn default_choppy_group_size() -> usize { 15 }
fn default_choppy_period() -> usize { 14 }
fn default_choppy_threshold() -> f64 { 61.8 }
fn default_kama_er_threshold() -> f64 { 0.35 }
fn default_body_ratio_threshold() -> f64 { 0.30 }
fn default_min_switches() -> usize { 3 }

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChoppyCombinedConfig {
    #[serde(default = "default_choppy_combined_mode")]
    pub mode: String,
    #[serde(default = "default_choppy_group_size", alias = "group_size")]
    pub group_size: usize,
    #[serde(default = "default_choppy_period", alias = "chop_period")]
    pub chop_period: usize,
    #[serde(default = "default_choppy_threshold", alias = "chop_threshold", alias = "choppy_threshold")]
    pub chop_threshold: f64,
    #[serde(default = "default_kama_er_threshold", alias = "kama_er_threshold", alias = "er_threshold")]
    pub kama_er_threshold: f64,
    #[serde(default = "default_body_ratio_threshold", alias = "body_ratio_threshold")]
    pub body_ratio_threshold: f64,
    #[serde(default = "default_min_switches", alias = "min_switches")]
    pub min_switches: usize,
}

impl Default for ChoppyCombinedConfig {
    fn default() -> Self {
        Self {
            mode: default_choppy_combined_mode(),
            group_size: default_choppy_group_size(),
            chop_period: default_choppy_period(),
            chop_threshold: default_choppy_threshold(),
            kama_er_threshold: default_kama_er_threshold(),
            body_ratio_threshold: default_body_ratio_threshold(),
            min_switches: default_min_switches(),
        }
    }
}

pub fn load_choppy_combined_config() -> ChoppyCombinedConfig {
    // 1. ลองอ่านจาก setup/settings.json ก่อน (Approach 1)
    if let Ok(content) = fs::read_to_string("setup/settings.json") {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(cfg_val) = v.pointer("/settings/parameters/choppy_combined") {
                if let Ok(cfg) = serde_json::from_value::<ChoppyCombinedConfig>(cfg_val.clone()) {
                    return cfg;
                }
            }
        }
    }

    // 2. Fallback ไปที่ setup/setup.json
    if let Ok(content) = fs::read_to_string("setup/setup.json") {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(cfg_val) = v.pointer("/indicators/choppy_combined") {
                if let Ok(cfg) = serde_json::from_value::<ChoppyCombinedConfig>(cfg_val.clone()) {
                    return cfg;
                }
            }
            if let Some(cfg_val) = v.pointer("/indicators/choppyCombined") {
                if let Ok(cfg) = serde_json::from_value::<ChoppyCombinedConfig>(cfg_val.clone()) {
                    return cfg;
                }
            }
        }
    }

    // 3. Fallback ไปยัง Default
    ChoppyCombinedConfig::default()
}

/// ข้อมูลแท่งใหญ่ที่เกิดจากการ Aggregate แท่งย่อย 1M พร้อม Micro & Macro Metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedBigCandle {
    pub start_epoch: i64,
    pub end_epoch: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub candle_count: usize,
    // Micro metrics
    pub kama_er: f64,
    pub body_ratio: f64,
    pub switch_count: usize,
    pub is_choppy_micro: bool,
    // Macro metrics
    pub macro_chop: Option<f64>,
    pub is_choppy_macro: Option<bool>,
    // Combined result
    pub is_choppy_combined: bool,
}


// โครงสร้างรับข้อมูลดิบจาก Frontend
#[derive(Debug, Deserialize, Serialize, Clone, utoipa::ToSchema)]
pub struct RawCandleInput {
    /// Candle timestamp in epoch seconds
    pub epoch: i64,
    /// Open price
    pub open: f64,
    /// High price
    pub high: f64,
    /// Low price
    pub low: f64,
    /// Close price
    pub close: f64,
}

// โครงสร้างสำหรับ SMC
#[derive(Debug, Serialize, Clone)]
pub struct SmcStructure {
    pub time: i64,
    pub price: f64,
    pub structure_type: String,
    pub direction: String,
    pub level: String,
    pub start_time: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct SmcSwingPoint {
    pub time: i64,
    pub price: f64,
    pub swing_type: String,
    pub swing: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct SmcPremiumDiscountZone {
    pub start_time: i64,
    pub end_time: i64,
    pub premium_top: f64,
    pub premium_bottom: f64,
    pub equilibrium: f64,
    pub discount_top: f64,
    pub discount_bottom: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct SmcData {
    pub structures: Vec<SmcStructure>,
    pub swing_points: Vec<SmcSwingPoint>,
    pub order_blocks: Vec<String>, 
    pub fair_value_gaps: Vec<String>, 
    pub equal_highs_lows: Vec<String>, 
    pub premium_discount_zone: SmcPremiumDiscountZone,
    pub strong_weak_levels: Vec<String>, 
    pub swing_trend: String,
    pub internal_trend: String,
}

// โครงสร้างสำหรับ Tick Volatility
#[derive(Debug, Serialize, Clone)]
pub struct TickVolatility {
    pub tick_count: i32,
    pub buy_tick_count: i32,
    pub sell_tick_count: i32,
    pub buy_sell_ratio: f64,
    pub avg_tick_move: f64,
    pub max_tick_move: f64,
    pub sum_tick_move: f64,
    pub volatility_clustering: f64,
    pub volatility_level: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct RangeDetectorData {
    pub in_range: bool,
    pub range_top: f64,
    pub range_bottom: f64,
    pub range_avg: f64,
    pub range_state: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct BollingerBands {
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

// โครงสร้างผลลัพธ์ที่จะส่งกลับให้ Frontend
#[derive(Debug, Serialize, Clone)]
pub struct FullAnalysisResult {
    pub index: usize,
    #[serde(rename = "assetCode")]
    pub asset_code: String,
    pub candletime: i64,
    pub candletime_display: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub color: String,
    pub next_color: Option<String>,
    pub pip_size: f64,
    
    pub ema_short_value: f64,
    pub ema_short_direction: String,
    pub ema_short_turn_type: String,
    pub ema_short_slope_value: f64,
    #[serde(rename = "emaslopeThereshold")]
    pub emaslope_thereshold: f64,
    pub diff: f64,
    pub ema_short_flat: String,
    
    pub ema_medium_value: f64,
    pub ema_medium_direction: String,
    pub ema_medium_turn_type: String,
    pub ema_medium_slope_value: f64,
    pub ema_medium_flat: String,
    
    pub ema_long_value: f64,
    pub ema_long_direction: String,
    pub ema_long_turn_type: String,
    pub ema_long_slope_value: f64,
    pub ema_long_flat: String,
    
    pub short_medium_gap_value: f64,
    pub is_short_medium_gap_occur: String,
    pub medium_long_gap_value: f64,
    pub is_medium_long_gap_occur: String,
    
    pub ema_above: String,
    pub ema_long_above: String,
    
    pub macd_12: f64,
    pub macd_23: f64,
    
    pub previous_ema_short_value: f64,
    pub previous_ema_medium_value: f64,
    pub previous_ema_long_value: f64,
    pub previous_macd_12: f64,
    pub previous_macd_23: f64,
    
    pub ema_convergence_type: String,
    pub ema_long_convergence_type: String,
    
    pub choppy_indicator: f64,
    pub adx_value: f64,
    pub rsi_value: f64,
    
    pub bb_values: BollingerBands,
    pub bb_position: String,
    
    #[serde(rename = "atrValue")]
    pub atr_value: f64,
    pub is_abnormal_candle: bool,
    pub is_abnormal_atr: bool,
    pub is_atr: bool,
    
    pub u_wick: f64,
    pub u_wick_percent: f64,
    pub body: f64,
    pub body_percent: f64,
    pub l_wick: f64,
    pub l_wick_percent: f64,
    
    pub ema_cut_position: String,
    pub ema_cut_long_type: String,
    pub ema_cut_short_long_type: String,
    pub ema_cut_all_type: String,
    pub candles_since_ema_cut: i32,
    
    #[serde(rename = "ageCutCandleCode12")]
    pub age_cut_candle_code12: String,
    #[serde(rename = "ageCutCandleCode123")]
    pub age_cut_candle_code123: String,
    
    pub ema_short_pos: String,
    pub ema_medium_pos: String,
    pub ema_long_pos: String,
    
    pub bb_bandwidth: f64,
    pub is_bb_squeeze: bool,
    #[serde(default)]
    pub is_bb_flat_choppy: bool,
    #[serde(default)]
    pub bb_middle_slope: f64,
    #[serde(default, rename = "BBLineupper_CutPos", alias = "bb_line_upper_cut_pos", alias = "bbLineUpperCutPos")]
    pub bb_line_upper_cut_pos: String,
    #[serde(default, rename = "BBLineMiddle_CutPos", alias = "bb_line_middle_cut_pos", alias = "bbLineMiddleCutPos")]
    pub bb_line_middle_cut_pos: String,
    #[serde(default, rename = "BBLineLow_CutPos", alias = "bb_line_low_cut_pos", alias = "bbLineLowCutPos")]
    pub bb_line_low_cut_pos: String,
    
    // -- KAMA & Choppiness Combined Fields --
    #[serde(default)]
    pub kama_er: f64,
    #[serde(default)]
    pub candle_body_ratio: f64,
    #[serde(default)]
    pub color_switch_count: usize,
    #[serde(default)]
    pub is_choppy_micro: bool,
    #[serde(default)]
    pub macro_chop: Option<f64>,
    #[serde(default)]
    pub is_choppy_macro: Option<bool>,
    #[serde(default)]
    pub is_choppy_combined: bool,
    
    pub up_con_medium_ema: i32,
    pub down_con_medium_ema: i32,
    pub up_con_long_ema: i32,
    pub down_con_long_ema: i32,
    
    pub is_mark: String,
    pub status_code: String,
    pub status_desc: String,
    pub status_desc_0: String,
    pub hint_status: String,
    pub suggest_color: String,
    pub win_status: String,
    pub win_con: i32,
    pub loss_con: i32,
    
    pub smc: SmcData,
    pub tick_volatility: TickVolatility,
    pub range_detector: RangeDetectorData,
    #[serde(rename = "pkTrend")]
    pub pk_trend: crate::pkDetectTrend_v5::TrendDetectResult,
    pub is_alternating_pattern: bool,
    pub alternating_sequence_length: i32,
    pub is_alternating_trigger: bool,
    pub is_alternating_spike: bool,
}

// ────────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────────

fn right_align(vals: Vec<f64>, n: usize) -> Vec<f64> {
    let mut res = vec![0.0; n];
    if vals.len() <= n {
        let offset = n - vals.len();
        for i in 0..vals.len() {
            res[offset + i] = vals[i];
        }
    }
    res
}

fn compute_ma(prices: &[f64], period: usize, ma_type: &str) -> Vec<f64> {
    let mut ma_values = vec![0.0; prices.len()];
    if prices.len() < period || period == 0 {
        return ma_values;
    }

    let ma_t = ma_type.to_lowercase();
    if ma_t == "sma" {
        for i in period - 1..prices.len() {
            let sum: f64 = prices[i + 1 - period..=i].iter().sum();
            ma_values[i] = sum / period as f64;
        }
    } else if ma_t == "hma" {
        let result = HMA::new(period).compute(prices);
        let offset = prices.len() - result.values.len();
        for i in 0..result.values.len() {
            ma_values[offset + i] = result.values[i];
        }
    } else if ma_t == "wma" || ma_t == "whma" {
        let result = WMA::new(period).compute(prices);
        let offset = prices.len() - result.values.len();
        for i in 0..result.values.len() {
            ma_values[offset + i] = result.values[i];
        }
    } else {
        // Default to EMA
        let alpha = 2.0 / (period + 1) as f64;
        let mut current_ema = prices[..period].iter().sum::<f64>() / period as f64;
        ma_values[period - 1] = current_ema;

        for i in period..prices.len() {
            current_ema = simd::ema_update(current_ema, prices[i], alpha);
            ma_values[i] = current_ema;
        }
    }

    ma_values
}

fn compute_bb(prices: &[f64], period: usize, std_dev: f64, ma_type: &str) -> Vec<BollingerBands> {
    let mut bbs = vec![BollingerBands { upper: 0.0, middle: 0.0, lower: 0.0 }; prices.len()];
    if prices.len() < period || period == 0 {
        return bbs;
    }
    let ma_values = compute_ma(prices, period, ma_type);

    for i in period - 1..prices.len() {
        let ma = ma_values[i];
        let window = &prices[i + 1 - period..=i];
        let mean = window.iter().sum::<f64>() / period as f64;
        let variance = window.iter().map(|value| {
            let diff = mean - *value;
            diff * diff
        }).sum::<f64>() / period as f64;
        let std = variance.sqrt();
        bbs[i] = BollingerBands {
            middle: ma,
            upper: ma + std_dev * std,
            lower: ma - std_dev * std,
        };
    }
    bbs
}

fn get_candle_position(ema: f64, open: f64, high: f64, low: f64, close: f64) -> String {
    let max_body = open.max(close);
    let min_body = open.min(close);
    if ema > high {
        "AboveHigh".to_string()
    } else if ema <= high && ema > max_body {
        "UpperWick".to_string()
    } else if ema <= max_body && ema >= min_body {
        "Body".to_string()
    } else if ema < min_body && ema >= low {
        "LowerWick".to_string()
    } else {
        "BelowLow".to_string()
    }
}

/// คำนวณรหัสตำแหน่งของเส้น Indicator (เช่น เส้น Bollinger Bands 3 เส้น) เทียบกับแท่งเทียน
/// ตามนิยามตาราง indycutpos:
/// - "abx"  : above max (val > high)
/// - "abxo" : between max-open (high >= val > open ในแท่ง Bearish)
/// - "abxc" : between max-close (high >= val > close ในแท่ง Bullish)
/// - "inb"  : insideBody (min_body <= val <= max_body)
/// - "abno" : between min-open (low <= val < open ในแท่ง Bullish)
/// - "abnc" : between min-close (low <= val < close ในแท่ง Bearish)
/// - "bln"  : below min (val < low)
pub fn get_cut_pos_code(val: f64, open: f64, high: f64, low: f64, close: f64) -> String {
    if val <= 0.0 {
        return "".to_string();
    }
    let max_body = open.max(close);
    let min_body = open.min(close);
    if val > high {
        "abx".to_string()
    } else if val > max_body {
        if close >= open {
            "abxc".to_string()
        } else {
            "abxo".to_string()
        }
    } else if val >= min_body {
        "inb".to_string()
    } else if val >= low {
        if close >= open {
            "abno".to_string()
        } else {
            "abnc".to_string()
        }
    } else {
        "bln".to_string()
    }
}

/// คำนวณ Kaufman Efficiency Ratio และ Micro Price Action Metrics จากกลุ่มแท่งย่อย 1M
pub fn compute_micro_kama_metrics(
    group: &[OHLCV],
    body_ratio_threshold: f64,
    er_threshold: f64,
    min_switches: usize,
) -> (f64, f64, usize, bool) {
    if group.is_empty() {
        return (0.0, 0.0, 0, false);
    }

    let open = group[0].open;
    let close = group[group.len() - 1].close;
    let high = group.iter().map(|c| c.high).fold(f64::NEG_INFINITY, f64::max);
    let low = group.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);

    let net_change = (close - open).abs();
    let range = high - low;
    let body_ratio = if range > 0.0 { net_change / range } else { 0.0 };

    // Kaufman Efficiency Ratio: Net Change / Sum of 1M Step Changes
    let mut total_path = 0.0;
    let mut prev_price = open;
    for c in group {
        total_path += (c.close - prev_price).abs();
        prev_price = c.close;
    }
    let efficiency_ratio = if total_path > 0.0 { net_change / total_path } else { 0.0 };

    // นับจำนวนครั้งที่สีแท่งย่อยสลับสีกัน (ข้าม doji)
    let mut switch_count = 0usize;
    let mut prev_color: Option<bool> = None; // true = green, false = red
    for c in group {
        let color = if c.close > c.open {
            Some(true)
        } else if c.close < c.open {
            Some(false)
        } else {
            None
        };
        if let Some(col) = color {
            if let Some(p_col) = prev_color {
                if col != p_col {
                    switch_count += 1;
                }
            }
            prev_color = Some(col);
        }
    }

    let is_choppy_micro = body_ratio <= body_ratio_threshold
        && efficiency_ratio <= er_threshold
        && switch_count >= min_switches;

    (efficiency_ratio, body_ratio, switch_count, is_choppy_micro)
}

/// รวมแท่งย่อย 1M เป็นแท่งใหญ่ พร้อมคำนวณ Dreiss Choppiness Index (Macro) และ Micro KAMA
pub fn analyze_choppiness_combined(
    candles: &[OHLCV],
    cfg: &ChoppyCombinedConfig,
) -> Vec<AggregatedBigCandle> {
    if candles.is_empty() || cfg.group_size == 0 {
        return Vec::new();
    }

    // 1. จัดกลุ่มแท่งย่อยเป็นแท่งใหญ่ (Big Candles)
    let mut big_candles = Vec::new();
    for chunk in candles.chunks(cfg.group_size) {
        if chunk.is_empty() { continue; }
        let (er, body_ratio, switches, is_micro) =
            compute_micro_kama_metrics(chunk, cfg.body_ratio_threshold, cfg.kama_er_threshold, cfg.min_switches);

        let high = chunk.iter().map(|c| c.high).fold(f64::NEG_INFINITY, f64::max);
        let low = chunk.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);

        big_candles.push(AggregatedBigCandle {
            start_epoch: chunk[0].timestamp,
            end_epoch: chunk[chunk.len() - 1].timestamp,
            open: chunk[0].open,
            high,
            low,
            close: chunk[chunk.len() - 1].close,
            candle_count: chunk.len(),
            kama_er: (er * 10000.0).round() / 10000.0,
            body_ratio: (body_ratio * 10000.0).round() / 10000.0,
            switch_count: switches,
            is_choppy_micro: is_micro,
            macro_chop: None,
            is_choppy_macro: None,
            is_choppy_combined: false,
        });
    }

    // 2. คำนวณ Rolling Dreiss Choppiness Index บนแท่งใหญ่
    let len = big_candles.len();
    let chop_period = cfg.chop_period;
    let log_period = (chop_period as f64).ln();

    if len >= chop_period && chop_period > 1 {
        for i in chop_period..=len {
            let window = &big_candles[i - chop_period..i];
            let mut atr_sum = 0.0;
            for j in 1..window.len() {
                let hl = window[j].high - window[j].low;
                let hc = (window[j].high - window[j - 1].close).abs();
                let lc = (window[j].low - window[j - 1].close).abs();
                atr_sum += hl.max(hc).max(lc);
            }
            let win_high = window.iter().map(|c| c.high).fold(f64::NEG_INFINITY, f64::max);
            let win_low = window.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
            let range = win_high - win_low;

            let chop = if range > 0.0 {
                (100.0 * (atr_sum / range).ln()) / log_period
            } else {
                0.0
            };

            let rounded_chop = (chop * 100.0).round() / 100.0;
            big_candles[i - 1].macro_chop = Some(rounded_chop);
            big_candles[i - 1].is_choppy_macro = Some(rounded_chop >= cfg.chop_threshold);
        }
    }

    // 3. รวมผล Macro + Micro
    let mode = cfg.mode.to_lowercase();
    for item in &mut big_candles {
        let is_macro = item.is_choppy_macro;
        let is_micro = item.is_choppy_micro;

        item.is_choppy_combined = match mode.as_str() {
            "macroonly" => is_macro.unwrap_or(false),
            "microonly" => is_micro,
            "and" => {
                is_macro.map(|m| m && is_micro).unwrap_or(is_micro)
            }
            _ => {
                // "or" (default)
                is_macro.unwrap_or(false) || is_micro
            }
        };
    }

    big_candles
}

// ADX Function (simplified)
fn compute_adx_simple(ohlcv: &[OHLCV], period: usize) -> Vec<f64> {
    let n = ohlcv.len();
    let mut adx_vals = vec![0.0; n];
    if n < period * 2 || period == 0 { return adx_vals; }

    let mut tr_sm = 0.0;
    let mut pdm_sm = 0.0;
    let mut ndm_sm = 0.0;
    
    for i in 1..=period {
        let hl = ohlcv[i].high - ohlcv[i].low;
        let hc = (ohlcv[i].high - ohlcv[i-1].close).abs();
        let lc = (ohlcv[i].low - ohlcv[i-1].close).abs();
        let tr = hl.max(hc).max(lc);
        
        let up = ohlcv[i].high - ohlcv[i-1].high;
        let down = ohlcv[i-1].low - ohlcv[i].low;
        let mut pdm = 0.0;
        let mut ndm = 0.0;
        if up > down && up > 0.0 { pdm = up; }
        if down > up && down > 0.0 { ndm = down; }
        
        tr_sm += tr;
        pdm_sm += pdm;
        ndm_sm += ndm;
    }
    
    let mut dx_vals = vec![0.0; n];
    for i in period..n {
        if i > period {
            let hl = ohlcv[i].high - ohlcv[i].low;
            let hc = (ohlcv[i].high - ohlcv[i-1].close).abs();
            let lc = (ohlcv[i].low - ohlcv[i-1].close).abs();
            let tr = hl.max(hc).max(lc);
            
            let up = ohlcv[i].high - ohlcv[i-1].high;
            let down = ohlcv[i-1].low - ohlcv[i].low;
            let mut pdm = 0.0;
            let mut ndm = 0.0;
            if up > down && up > 0.0 { pdm = up; }
            if down > up && down > 0.0 { ndm = down; }
            
            tr_sm = tr_sm - (tr_sm / period as f64) + tr;
            pdm_sm = pdm_sm - (pdm_sm / period as f64) + pdm;
            ndm_sm = ndm_sm - (ndm_sm / period as f64) + ndm;
        }
        
        let pdi = if tr_sm > 0.0 { 100.0 * pdm_sm / tr_sm } else { 0.0 };
        let ndi = if tr_sm > 0.0 { 100.0 * ndm_sm / tr_sm } else { 0.0 };
        let diff = (pdi - ndi).abs();
        let sum = pdi + ndi;
        dx_vals[i] = if sum > 0.0 { 100.0 * diff / sum } else { 0.0 };
    }
    
    let mut adx = 0.0;
    for i in period..period * 2 {
        adx += dx_vals[i];
    }
    adx /= period as f64;
    adx_vals[period * 2 - 1] = adx;
    
    for i in period * 2..n {
        adx = (adx * (period as f64 - 1.0) + dx_vals[i]) / period as f64;
        adx_vals[i] = adx;
    }
    
    adx_vals
}

// RangeDetector – แปลงจาก RangeDetector.js
fn compute_range_detector(ohlcvs: &[OHLCV], closes: &[f64], length: usize, mult: f64, atr_len: usize) -> Vec<RangeDetectorData> {
    let n = closes.len();
    let mut results: Vec<RangeDetectorData> = Vec::with_capacity(n);
    for _ in 0..n {
        results.push(RangeDetectorData {
            in_range: false, range_top: 0.0, range_bottom: 0.0,
            range_avg: 0.0, range_state: "none".to_string(),
        });
    }
    if n <= length { return results; }

    let sma = compute_ma(closes, length, "sma");

    // ATR แบบ simple average (ตาม JS logic)
    let mut atr_arr = vec![0.0f64; n];
    for i in 0..n {
        if i < 1 {
            atr_arr[i] = (ohlcvs[0].high - ohlcvs[0].low) * mult;
        } else {
            let start = if i >= atr_len { i + 1 - atr_len } else { 1 };
            let mut sum = 0.0;
            let mut cnt = 0usize;
            for k in start..=i {
                let tr = (ohlcvs[k].high - ohlcvs[k].low)
                    .max((ohlcvs[k].high - ohlcvs[k-1].close).abs())
                    .max((ohlcvs[k].low - ohlcvs[k-1].close).abs());
                sum += tr;
                cnt += 1;
            }
            atr_arr[i] = if cnt > 0 { (sum / cnt as f64) * mult } else { 0.0 };
        }
    }

    // Count: จำนวนแท่งใน length ที่ทะลุกรอบ SMA ± ATR*mult
    let mut count_arr = vec![0usize; n];
    for i in (length - 1)..n {
        if sma[i] == 0.0 { continue; }
        let mut c = 0usize;
        for k in 0..length {
            if (closes[i - k] - sma[i]).abs() > atr_arr[i] { c += 1; }
        }
        count_arr[i] = c;
    }

    // สร้าง Range box + track per-candle
    let mut box_top = 0.0f64;
    let mut box_bottom = 0.0f64;
    let mut box_avg = 0.0f64;
    let mut box_state = String::from("none");
    let mut box_right_idx: usize = 0;
    let mut has_box = false;

    for i in length..n {
        let count = count_arr[i];
        let count_prv = count_arr[i - 1];
        let ma = sma[i];
        let atr_val = atr_arr[i];
        if ma == 0.0 { continue; }

        if count == 0 && count_prv != 0 {
            let start_bar = i - length;
            if has_box && start_bar <= box_right_idx {
                let new_top = (ma + atr_val).max(box_top);
                let new_bot = (ma - atr_val).min(box_bottom);
                box_top = new_top;
                box_bottom = new_bot;
                box_right_idx = i;
                box_avg = (new_top + new_bot) / 2.0;
            } else {
                box_top = ma + atr_val;
                box_bottom = ma - atr_val;
                box_right_idx = i;
                box_avg = ma;
                box_state = "unbroken".to_string();
                has_box = true;
            }
        } else if count == 0 && has_box {
            box_right_idx = i;
        }

        // ตรวจ Breakout
        if has_box {
            if closes[i] > box_top { box_state = "up".to_string(); }
            else if closes[i] < box_bottom { box_state = "down".to_string(); }
        }

        // บันทึกผลต่อแท่ง
        if has_box {
            results[i] = RangeDetectorData {
                in_range: box_state == "unbroken",
                range_top: box_top,
                range_bottom: box_bottom,
                range_avg: box_avg,
                range_state: box_state.clone(),
            };
        }
    }

    results
}



pub fn perform_analysis(payload: &[RawCandleInput], config: Option<AppConfigPayload>, asset_name: Option<&str>) -> Vec<FullAnalysisResult> {
    let n = payload.len();
    if n == 0 { return vec![]; }
    
    // โหลด flatTheresholdValue จาก thereshold.json
    let mut flat_threshold = 0.0;
    let mut macd_gap_value = 0.0;
    let mut alt_candle_atr_multiplier = 0.5;
    let mut alt_candle_spike_multiplier = 1.5;
    if let Some(asset) = asset_name {
        if let Ok(content) = std::fs::read_to_string("setup/thereshold.json") {
            if let Ok(root) = serde_json::from_str::<serde_json::Value>(&content) {
                let arr_opt = if root.is_array() {
                    root.as_array()
                } else if let Some(t) = root.get("thresholds") {
                    t.as_array()
                } else {
                    None
                };
                
                if let Some(arr) = arr_opt {
                    for t in arr {
                        let match_asset = t.get("asset").and_then(|v| v.as_str()).map(|a| a == asset).unwrap_or(false);
                        let match_code = t.get("assetCode").and_then(|v| v.as_str()).map(|a| a == asset).unwrap_or(false);
                        if match_asset || match_code {
                            if let Some(f) = t.get("flatTheresholdValue").and_then(|v| v.as_f64()) {
                                flat_threshold = f;
                            }
                            if let Some(m) = t.get("MACDGapValue").and_then(|v| v.as_f64()) {
                                macd_gap_value = m;
                            }
                            if let Some(m) = t.get("altCandleAtrMultiplier").and_then(|v| v.as_f64()) {
                                alt_candle_atr_multiplier = m;
                            }
                            if let Some(m) = t.get("altCandleSpikeMultiplier").and_then(|v| v.as_f64()) {
                                alt_candle_spike_multiplier = m;
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
    
    // กำหนดค่า parameter จาก config หรือใช้ค่า default
    let ema_s_period = config.as_ref().map(|c| c.ema.short.period as usize).unwrap_or(9);
    let ema_m_period = config.as_ref().map(|c| c.ema.medium.period as usize).unwrap_or(21);
    let ema_l_period = config.as_ref().map(|c| c.ema.long.period as usize).unwrap_or(50);
    
    let ema_s_type = config.as_ref().map(|c| c.ema.short.ema_type.clone()).unwrap_or_else(|| "ema".to_string());
    let ema_m_type = config.as_ref().map(|c| c.ema.medium.ema_type.clone()).unwrap_or_else(|| "ema".to_string());
    let ema_l_type = config.as_ref().map(|c| c.ema.long.ema_type.clone()).unwrap_or_else(|| "ema".to_string());

    let adx_period = config.as_ref().map(|c| c.indicators.adx_period as usize).unwrap_or(14);
    let atr_period = config.as_ref().map(|c| c.indicators.atr_period as usize).unwrap_or(7);
    let atr_multi = config.as_ref().map(|c| c.indicators.atr_multi).unwrap_or(1.3);
    let bb_period = config.as_ref().map(|c| c.indicators.bb_period as usize).unwrap_or(20);
    let bb_std_dev = config.as_ref().map(|c| c.indicators.bb_std_dev).unwrap_or(2.0);
    let bb_ma_type = config.as_ref().map(|c| c.indicators.bb_ma_type.clone()).unwrap_or_else(|| "sma".to_string());
    let squeeze_percentile = config.as_ref().map(|c| c.indicators.squeeze_percentile).unwrap_or(20.0);
    let squeeze_lookback = config.as_ref().map(|c| c.indicators.squeeze_lookback).unwrap_or(120);
    let flat_slope_threshold = config.as_ref().map(|c| c.indicators.flat_slope_threshold).unwrap_or(0.008);
    let flat_min_bars = config.as_ref().map(|c| c.indicators.flat_min_bars).unwrap_or(3);
    let ci_period = config.as_ref().map(|c| c.indicators.ci_period as usize).unwrap_or(14);
    let smc_period = config.as_ref().map(|c| c.indicators.smc_period as usize).unwrap_or(50);

    // แปลง RawCandleInput เป็น OHLCV
    let ohlcvs: Vec<OHLCV> = payload.iter().map(|c| OHLCV {
        open: c.open,
        high: c.high,
        low: c.low,
        close: c.close,
        volume: 0.0,
        timestamp: c.epoch,
    }).collect();

    let closes: Vec<f64> = ohlcvs.iter().map(|c| c.close).collect();

    // คำนวณ KAMA & Choppiness Combined
    let choppy_cfg = load_choppy_combined_config();
    let big_candles = analyze_choppiness_combined(&ohlcvs, &choppy_cfg);

    // -- Indicator Setup --
    let ema_short = compute_ma(&closes, ema_s_period, &ema_s_type);
    let ema_medium = compute_ma(&closes, ema_m_period, &ema_m_type);
    let ema_long = compute_ma(&closes, ema_l_period, &ema_l_type);

    // คำนวณ MACD (12, 26, 9) - inline ตาม algorithm ใน FullAnalysisEngine.js
    let (macd_val, macd_signal) = {
        let fast_period: usize = 12;
        let slow_period: usize = 26;
        let signal_period: usize = 9;

        let mut macd_line_arr = vec![0.0f64; n];
        let mut signal_line_arr = vec![0.0f64; n];

        if n >= slow_period {
            // Initialize fast/slow EMA with SMA
            let mut sum_fast = 0.0f64;
            for k in 0..fast_period { sum_fast += closes[k]; }
            let mut fast_ema = sum_fast / fast_period as f64;

            let mut sum_slow = 0.0f64;
            for k in 0..slow_period { sum_slow += closes[k]; }
            let mut slow_ema = sum_slow / slow_period as f64;

            let fast_alpha = 2.0 / (fast_period + 1) as f64;
            let slow_alpha = 2.0 / (slow_period + 1) as f64;

            // Compute MACD line from slow_period onward
            let mut raw_macd: Vec<f64> = Vec::with_capacity(n - slow_period);
            for k in slow_period..n {
                fast_ema = closes[k] * fast_alpha + fast_ema * (1.0 - fast_alpha);
                slow_ema = closes[k] * slow_alpha + slow_ema * (1.0 - slow_alpha);
                let macd_v = fast_ema - slow_ema;
                raw_macd.push(macd_v);
                macd_line_arr[k] = macd_v;
            }

            // Compute Signal line (EMA of raw MACD)
            if raw_macd.len() >= signal_period {
                let mut sum_signal = 0.0f64;
                for k in 0..signal_period { sum_signal += raw_macd[k]; }
                let mut signal_ema = sum_signal / signal_period as f64;

                let signal_alpha = 2.0 / (signal_period + 1) as f64;
                let offset = slow_period + signal_period - 1;

                // Apply one EMA step at the signal_period-1 position
                signal_ema = raw_macd[signal_period - 1] * signal_alpha + signal_ema * (1.0 - signal_alpha);
                signal_line_arr[offset] = signal_ema;

                for k in signal_period..raw_macd.len() {
                    signal_ema = raw_macd[k] * signal_alpha + signal_ema * (1.0 - signal_alpha);
                    let main_idx = slow_period + k;
                    signal_line_arr[main_idx] = signal_ema;
                }
            }
        }

        (macd_line_arr, signal_line_arr)
    };

    // คำนวณ RSI (ใช้ smc_period เป็น RSI ตามตัวอย่างในระบบเดิม หรือจะแยกก็ได้)
    let rsi_vals = right_align(RSI::new(smc_period).compute(&closes), n);

    // คำนวณ Choppiness Index
    let chop_vals = right_align(ChoppinessIndex::new(ci_period).compute(&ohlcvs), n);

    // คำนวณ ATR
    let mut stateful_atr = StatefulATR::new(atr_period);
    let mut atr_vals = vec![0.0; n];
    for (i, candle) in ohlcvs.iter().enumerate() {
        if let Some(atr) = stateful_atr.update(candle) {
            atr_vals[i] = atr;
        }
    }

    let mut stateful_atr_3 = StatefulATR::new(3);
    let mut atr_3_vals = vec![0.0; n];
    for (i, candle) in ohlcvs.iter().enumerate() {
        if let Some(atr) = stateful_atr_3.update(candle) {
            atr_3_vals[i] = atr;
        }
    }

    // คำนวณ Bollinger Bands
    let bb_vals = compute_bb(&closes, bb_period, bb_std_dev, &bb_ma_type);

    let mut bb_bandwidths = vec![0.0; n];
    for i in 0..n {
        if bb_vals[i].middle > 0.0 {
            bb_bandwidths[i] = (bb_vals[i].upper - bb_vals[i].lower) / bb_vals[i].middle * 100.0;
        }
    }

    // คำนวณ BB Squeeze (Rolling Window Percentile - 0-bar lag causal)
    let mut squeeze_flags = vec![false; n];
    for i in 0..n {
        let window_start = if i >= squeeze_lookback { i - squeeze_lookback } else { 0 };
        let window_end = i;
        if window_end.saturating_sub(window_start) >= 5 {
            let mut window_bws: Vec<f64> = (window_start..window_end).map(|j| bb_bandwidths[j]).collect();
            window_bws.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let pct_idx = ((window_bws.len() as f64) * squeeze_percentile / 100.0).floor() as usize;
            let threshold = window_bws[pct_idx.min(window_bws.len().saturating_sub(1))];
            squeeze_flags[i] = bb_bandwidths[i] <= threshold;
        }
    }

    // คำนวณ Flat / Choppy Zone (เส้นกลางแบนราบ และ ผ่ากลางแท่งเทียน)
    let mut flat_choppy_flags = vec![false; n];
    let mut middle_slopes = vec![0.0; n];
    let mut raw_flat_matches = vec![false; n];

    for i in 1..n {
        let mid = bb_vals[i].middle;
        let prev_mid = bb_vals[i - 1].middle;
        if mid > 0.0 && prev_mid > 0.0 {
            let slope_pct = ((mid - prev_mid).abs() / prev_mid) * 100.0;
            middle_slopes[i] = slope_pct;
            let c = &payload[i];
            let pierces = c.low <= mid && mid <= c.high;
            if slope_pct <= flat_slope_threshold && pierces {
                raw_flat_matches[i] = true;
            }
        }
    }

    // Group contiguous blocks where length >= flat_min_bars
    let mut start_opt: Option<usize> = None;
    for i in 0..n {
        if raw_flat_matches[i] {
            if start_opt.is_none() {
                start_opt = Some(i);
            }
        } else {
            if let Some(start_idx) = start_opt {
                let len = i - start_idx;
                if len >= flat_min_bars {
                    for k in start_idx..i {
                        flat_choppy_flags[k] = true;
                    }
                }
                start_opt = None;
            }
        }
    }
    if let Some(start_idx) = start_opt {
        let len = n - start_idx;
        if len >= flat_min_bars {
            for k in start_idx..n {
                flat_choppy_flags[k] = true;
            }
        }
    }

    // คำนวณ ADX
    let adx_vals = compute_adx_simple(&ohlcvs, adx_period);

    // คำนวณ RangeDetector (length=20, mult=1.0, atr_len=200)
    let range_detector_vals = compute_range_detector(&ohlcvs, &closes, 20, 1.0, 200);

    // เตรียมชุดแท่งเทียนสำหรับ pkDetectTrend_v5 (พร้อมคำนวณ is_atr ของแต่ละแท่ง)
    let trend_candles: Vec<crate::pkDetectTrend_v5::CandleInput> = (0..n)
        .map(|idx| {
            let c = &payload[idx];
            let r = c.high - c.low;
            let is_spike = atr_vals[idx] > 0.0 && r > atr_vals[idx] * atr_multi;
            crate::pkDetectTrend_v5::CandleInput {
                open: c.open,
                high: c.high,
                low: c.low,
                close: c.close,
                volume: None,
                is_atr: is_spike,
                time: c.epoch,
            }
        })
        .collect();

    let mut results = Vec::with_capacity(n);
    let mut alt_seq_len = 0;
    
    // State tracking for ageCutCandleCode12 / ageCutCandleCode123
    let mut last_cut12_idx: i64 = -1;
    let mut last_cut12_type = String::from("-");
    let mut last_cut123_idx: i64 = -1;
    let mut last_cut123_type = String::from("-");
    
    for i in 0..n {
        let candle = &payload[i];
        
        let pip_size = (candle.open - candle.close).abs();
        let color = if candle.close > candle.open { "green" } else if candle.close < candle.open { "red" } else { "equal" };
        
        // Anatomy
        let range = candle.high - candle.low;
        let body = (candle.open - candle.close).abs();
        let u_wick = candle.high - candle.open.max(candle.close);
        let l_wick = candle.open.min(candle.close) - candle.low;
        
        let u_wick_percent = if range > 0.0 { (u_wick / range) * 100.0 } else { 0.0 };
        let body_percent = if range > 0.0 { (body / range) * 100.0 } else { 0.0 };
        let l_wick_percent = if range > 0.0 { (l_wick / range) * 100.0 } else { 0.0 };

        let mut is_alternating_pattern = false;
        let mut is_alternating_spike = false;
        if i > 0 {
            let prev_candle = &payload[i - 1];
            let prev_color = if prev_candle.close > prev_candle.open { "green" } else if prev_candle.close < prev_candle.open { "red" } else { "equal" };
            let prev_body = (prev_candle.open - prev_candle.close).abs();
            
            let is_alt_color = (color == "green" && prev_color == "red") || (color == "red" && prev_color == "green");
            let body_diff = (prev_body - body).abs();
            let threshold = atr_3_vals[i] * alt_candle_atr_multiplier;
            let is_similar_body = threshold > 0.0 && body_diff < threshold;
            
            if is_alt_color && is_similar_body {
                if alt_seq_len == 0 {
                    alt_seq_len = 2;
                } else {
                    alt_seq_len += 1;
                }
                is_alternating_pattern = true;
            } else {
                alt_seq_len = 0;
            }

            if is_alt_color && atr_3_vals[i] > 0.0 && body_diff > (atr_3_vals[i] * alt_candle_spike_multiplier) {
                is_alternating_spike = true;
            }
        } else {
            alt_seq_len = 0;
        }
        let is_alternating_trigger = alt_seq_len >= 3;

        // Time format (local)
        let dt = Local.timestamp_opt(candle.epoch, 0).single().unwrap_or_default();
        let candletime_display = format!("{}", dt.format("%Y-%m-%d %H:%M:%S"));

        // Previous values (safe bounds)
        let prev_idx = if i > 0 { i - 1 } else { 0 };
        let previous_ema_short = ema_short[prev_idx];
        let previous_ema_medium = ema_medium[prev_idx];
        let previous_ema_long = ema_long[prev_idx];
        let previous_macd_12 = macd_val[prev_idx];
        let previous_macd_23 = macd_signal[prev_idx];

        // EMA Directions
        let ema_short_dir = if ema_short[i] >= previous_ema_short { "Up" } else { "Down" };
        let ema_medium_dir = if ema_medium[i] >= previous_ema_medium { "Up" } else { "Down" };
        let ema_long_dir = if ema_long[i] >= previous_ema_long { "Up" } else { "Down" };
        let ema_short_turn = if i > 1 {
            let prev2_ema = ema_short[i - 2];
            if previous_ema_short < prev2_ema && ema_short[i] > previous_ema_short { "TurnUp" }
            else if previous_ema_short > prev2_ema && ema_short[i] < previous_ema_short { "TurnDown" }
            else { "-" }
        } else { "-" };

        let ema_medium_turn = if i > 1 {
            let prev2_ema = ema_medium[i - 2];
            if previous_ema_medium < prev2_ema && ema_medium[i] > previous_ema_medium { "TurnUp" }
            else if previous_ema_medium > prev2_ema && ema_medium[i] < previous_ema_medium { "TurnDown" }
            else { "-" }
        } else { "-" };

        let ema_long_turn = if i > 1 {
            let prev2_ema = ema_long[i - 2];
            if previous_ema_long < prev2_ema && ema_long[i] > previous_ema_long { "TurnUp" }
            else if previous_ema_long > prev2_ema && ema_long[i] < previous_ema_long { "TurnDown" }
            else { "-" }
        } else { "-" };

        let ema_above = if ema_short[i] >= ema_medium[i] { "ShortAbove" } else { "MediumAbove" };
        let ema_long_above = if ema_medium[i] >= ema_long[i] { "MediumAbove" } else { "LongAbove" };

        let ema_conv = if (ema_short[i] - ema_medium[i]).abs() < (previous_ema_short - previous_ema_medium).abs() { "convergence" } else { "divergence" };
        let ema_long_conv = if (ema_medium[i] - ema_long[i]).abs() < (previous_ema_medium - previous_ema_long).abs() { "C" } else { "D" };

        let ema_cut_short_medium = if previous_ema_short < previous_ema_medium && ema_short[i] > ema_medium[i] { "CrossUp" }
                                   else if previous_ema_short > previous_ema_medium && ema_short[i] < ema_medium[i] { "CrossDown" }
                                   else { "-" };
                                   
        let ema_cut_medium_long = if previous_ema_medium < previous_ema_long && ema_medium[i] > ema_long[i] { "CrossUp" }
                                  else if previous_ema_medium > previous_ema_long && ema_medium[i] < ema_long[i] { "CrossDown" }
                                  else { "-" };
                                  
        let ema_cut_short_long = if previous_ema_short < previous_ema_long && ema_short[i] > ema_long[i] { "CrossUp" }
                                 else if previous_ema_short > previous_ema_long && ema_short[i] < ema_long[i] { "CrossDown" }
                                 else { "-" };
                                 
        let ema_cut_all = if ema_short[i] > ema_medium[i] && ema_medium[i] > ema_long[i] && 
                             !(previous_ema_short > previous_ema_medium && previous_ema_medium > previous_ema_long) {
            "AllCrossUp"
        } else if ema_short[i] < ema_medium[i] && ema_medium[i] < ema_long[i] && 
                  !(previous_ema_short < previous_ema_medium && previous_ema_medium < previous_ema_long) {
            "AllCrossDown"
        } else {
            "-"
        };

        // Update last crossover indices and types for ageCutCandleCode12
        if ema_cut_short_medium == "CrossUp" || ema_cut_short_medium == "CrossDown" {
            last_cut12_idx = i as i64;
            last_cut12_type = ema_cut_short_medium.to_string();
        }
        // Update last crossover indices and types for ageCutCandleCode123
        if ema_cut_all == "AllCrossUp" || ema_cut_all == "AllCrossDown" {
            last_cut123_idx = i as i64;
            last_cut123_type = if ema_cut_all == "AllCrossUp" { "CrossUp".to_string() } else { "CrossDown".to_string() };
        }

        let age12 = if last_cut12_idx != -1 { i as i64 - last_cut12_idx } else { i as i64 };
        let age12_type = if last_cut12_idx != -1 { last_cut12_type.clone() } else { "-".to_string() };
        let age_cut_candle_code12 = format!("{}-{}-{}:{}:{}", age12, age12_type, ema_short_dir, ema_medium_dir, ema_long_dir);

        let age123 = if last_cut123_idx != -1 { i as i64 - last_cut123_idx } else { i as i64 };
        let age123_type = if last_cut123_idx != -1 { last_cut123_type.clone() } else { "-".to_string() };
        let age_cut_candle_code123 = format!("{}-{}-{}:{}:{}", age123, age123_type, ema_short_dir, ema_medium_dir, ema_long_dir);

        let bb_position = if candle.close > bb_vals[i].upper { "AboveUpper" }
                          else if candle.close > bb_vals[i].middle { "NearUpper" }
                          else if candle.close < bb_vals[i].lower { "BelowLower" }
                          else { "NearLower" };

        // Abnormal candle/ATR check
        let is_abnormal_candle = range > atr_vals[i] * atr_multi; 
        let is_abnormal_atr = atr_vals[i] > (if i > 5 { atr_vals[i-5] } else { atr_vals[i] }) * 2.0;
        let is_atr = atr_vals[i] > 0.0 && range > atr_vals[i] * atr_multi;


        let ema_short_pos = get_candle_position(ema_short[i], candle.open, candle.high, candle.low, candle.close);
        let ema_medium_pos = get_candle_position(ema_medium[i], candle.open, candle.high, candle.low, candle.close);
        let ema_long_pos = get_candle_position(ema_long[i], candle.open, candle.high, candle.low, candle.close);

        let bb_line_upper_cut_pos = get_cut_pos_code(bb_vals[i].upper, candle.open, candle.high, candle.low, candle.close);
        let bb_line_middle_cut_pos = get_cut_pos_code(bb_vals[i].middle, candle.open, candle.high, candle.low, candle.close);
        let bb_line_low_cut_pos = get_cut_pos_code(bb_vals[i].lower, candle.open, candle.high, candle.low, candle.close);

        let big_idx = if choppy_cfg.group_size > 0 { i / choppy_cfg.group_size } else { 0 };
        let (kama_er, candle_body_ratio, color_switch_count, is_choppy_micro, macro_chop, is_choppy_macro, is_choppy_combined) =
            if big_idx < big_candles.len() {
                let b = &big_candles[big_idx];
                (b.kama_er, b.body_ratio, b.switch_count, b.is_choppy_micro, b.macro_chop, b.is_choppy_macro, b.is_choppy_combined)
            } else {
                (0.0, 0.0, 0, false, None, None, false)
            };

        let bb_bandwidth = bb_bandwidths[i];
        let is_bb_squeeze = squeeze_flags[i];

        // SMC Dummy Calculation (can be upgraded later)
        let smc = SmcData {
            structures: vec![],
            swing_points: vec![],
            order_blocks: vec![],
            fair_value_gaps: vec![],
            equal_highs_lows: vec![],
            premium_discount_zone: SmcPremiumDiscountZone {
                start_time: if i >= 20 { ohlcvs[i-20].timestamp } else { candle.epoch },
                end_time: candle.epoch,
                premium_top: candle.high + atr_vals[i],
                premium_bottom: candle.close,
                equilibrium: (candle.high + candle.low) / 2.0,
                discount_top: candle.close,
                discount_bottom: candle.low - atr_vals[i]
            },
            strong_weak_levels: vec![],
            swing_trend: if ema_medium_dir == "Up" { "bullish".to_string() } else { "bearish".to_string() },
            internal_trend: if ema_short_dir == "Up" { "bullish".to_string() } else { "bearish".to_string() },
        };

        let short_slope_abs = (ema_short[i] - previous_ema_short).abs();
        let medium_slope_abs = (ema_medium[i] - previous_ema_medium).abs();
        let long_slope_abs = (ema_long[i] - previous_ema_long).abs();

        let pk_trend = crate::pkDetectTrend_v5::detect_trend(&trend_candles, i, None);

        let res = FullAnalysisResult {
            index: i,
            asset_code: "".to_string(),
            candletime: candle.epoch,
            candletime_display,
            open: candle.open,
            high: candle.high,
            low: candle.low,
            close: candle.close,
            color: color.to_string(),
            next_color: None,
            pip_size,
            
            ema_short_value: ema_short[i],
            ema_short_direction: ema_short_dir.to_string(),
            ema_short_turn_type: ema_short_turn.to_string(),
            ema_short_slope_value: ema_short[i] - previous_ema_short,
            emaslope_thereshold: flat_threshold,
            diff: short_slope_abs - flat_threshold,
            ema_short_flat: if short_slope_abs <= flat_threshold { "y".to_string() } else { "n".to_string() },
            ema_medium_value: ema_medium[i],
            ema_medium_direction: ema_medium_dir.to_string(),
            ema_medium_turn_type: ema_medium_turn.to_string(),
            ema_medium_slope_value: ema_medium[i] - previous_ema_medium,
            ema_medium_flat: if medium_slope_abs <= flat_threshold { "y".to_string() } else { "n".to_string() },
            ema_long_value: ema_long[i],
            ema_long_direction: ema_long_dir.to_string(),
            ema_long_turn_type: ema_long_turn.to_string(),
            ema_long_slope_value: ema_long[i] - previous_ema_long,
            ema_long_flat: if long_slope_abs <= flat_threshold { "y".to_string() } else { "n".to_string() },
            
            short_medium_gap_value: (ema_short[i] - ema_medium[i]).abs(),
            is_short_medium_gap_occur: if i > 0 && ema_short[i] != 0.0 && ema_medium[i] != 0.0 && (ema_short[i] - ema_medium[i]).abs() <= macd_gap_value { "y".to_string() } else { "n".to_string() },
            medium_long_gap_value: (ema_medium[i] - ema_long[i]).abs(),
            is_medium_long_gap_occur: if i > 0 && ema_medium[i] != 0.0 && ema_long[i] != 0.0 && (ema_medium[i] - ema_long[i]).abs() <= macd_gap_value { "y".to_string() } else { "n".to_string() },
            
            ema_above: ema_above.to_string(),
            ema_long_above: ema_long_above.to_string(),
            
            macd_12: macd_val[i],
            macd_23: macd_signal[i],
            
            previous_ema_short_value: previous_ema_short,
            previous_ema_medium_value: previous_ema_medium,
            previous_ema_long_value: previous_ema_long,
            previous_macd_12,
            previous_macd_23,
            
            ema_convergence_type: ema_conv.to_string(),
            ema_long_convergence_type: ema_long_conv.to_string(),
            
            choppy_indicator: chop_vals[i],
            adx_value: adx_vals[i],
            rsi_value: rsi_vals[i],
            
            bb_values: bb_vals[i].clone(),
            bb_position: bb_position.to_string(),
            
            atr_value: atr_vals[i],
            is_abnormal_candle,
            is_abnormal_atr,
            is_atr,
            
            u_wick,
            u_wick_percent,
            body,
            body_percent,
            l_wick,
            l_wick_percent,
            
            ema_cut_position: ema_cut_short_medium.to_string(),
            ema_cut_long_type: ema_cut_medium_long.to_string(),
            ema_cut_short_long_type: ema_cut_short_long.to_string(),
            ema_cut_all_type: ema_cut_all.to_string(),
            candles_since_ema_cut: 0,
            age_cut_candle_code12,
            age_cut_candle_code123,
            
            ema_short_pos,
            ema_medium_pos,
            ema_long_pos,
            bb_bandwidth,
            is_bb_squeeze,
            is_bb_flat_choppy: flat_choppy_flags[i],
            bb_middle_slope: middle_slopes[i],
            bb_line_upper_cut_pos,
            bb_line_middle_cut_pos,
            bb_line_low_cut_pos,
            
            kama_er,
            candle_body_ratio,
            color_switch_count,
            is_choppy_micro,
            macro_chop,
            is_choppy_macro,
            is_choppy_combined,
            
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
            
            smc,
            
            tick_volatility: TickVolatility {
                tick_count: 0, buy_tick_count: 0, sell_tick_count: 0, buy_sell_ratio: 0.5,
                avg_tick_move: 0.0, max_tick_move: 0.0, sum_tick_move: 0.0, volatility_clustering: 0.0, volatility_level: "Low".to_string()
            },
            range_detector: range_detector_vals[i].clone(),
            pk_trend,
            is_alternating_pattern,
            alternating_sequence_length: alt_seq_len,
            is_alternating_trigger,
            is_alternating_spike,
        };
        results.push(res);
    }
    
    results
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn run_analysis_wasm(json_payload: &str, config_json: &str) -> String {
    let payload: Vec<RawCandleInput> = serde_json::from_str(json_payload).unwrap_or_default();
    let config: Option<AppConfigPayload> = serde_json::from_str(config_json).ok();
    
    let results = perform_analysis(&payload, config, None);
    serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string())
}

// ────────────────────────────────────────────────────────────────────────────
// API Handler
// ────────────────────────────────────────────────────────────────────────────

/// Compute full technical analysis including PK Trend Detection v5 for raw candle input
#[utoipa::path(
    post,
    path = "/getAnalysisdata",
    request_body = Vec<RawCandleInput>,
    responses(
        (status = 200, description = "Array of full analysis records per candle")
    ),
    tag = "Analysis Engine"
)]
pub async fn handle_post_analysis_data(
    State(_state): State<AppState>,
    Json(payload): Json<Vec<RawCandleInput>>,
) -> Json<Vec<FullAnalysisResult>> {
    let n = payload.len();
    if n == 0 {
        return Json(vec![]);
    }

    // โหลดการตั้งค่าจาก setup.json
    let config: Option<AppConfigPayload> = fs::read_to_string("setup/setup.json")
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok());

    let results = perform_analysis(&payload, config, None);
    
    Json(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cut_pos_code_bullish() {
        // Bullish candle: open = 100.0, close = 110.0, high = 115.0, low = 95.0
        let open = 100.0;
        let close = 110.0;
        let high = 115.0;
        let low = 95.0;

        assert_eq!(get_cut_pos_code(120.0, open, high, low, close), "abx");   // above max
        assert_eq!(get_cut_pos_code(112.0, open, high, low, close), "abxc");  // between max-close
        assert_eq!(get_cut_pos_code(115.0, open, high, low, close), "abxc");  // at high -> upper wick
        assert_eq!(get_cut_pos_code(108.0, open, high, low, close), "inb");   // inside body
        assert_eq!(get_cut_pos_code(110.0, open, high, low, close), "inb");   // at close -> body
        assert_eq!(get_cut_pos_code(100.0, open, high, low, close), "inb");   // at open -> body
        assert_eq!(get_cut_pos_code(98.0, open, high, low, close), "abno");   // between min-open
        assert_eq!(get_cut_pos_code(95.0, open, high, low, close), "abno");   // at low -> lower wick
        assert_eq!(get_cut_pos_code(90.0, open, high, low, close), "bln");    // below min
    }

    #[test]
    fn test_get_cut_pos_code_bearish() {
        // Bearish candle: open = 110.0, close = 100.0, high = 115.0, low = 95.0
        let open = 110.0;
        let close = 100.0;
        let high = 115.0;
        let low = 95.0;

        assert_eq!(get_cut_pos_code(120.0, open, high, low, close), "abx");   // above max
        assert_eq!(get_cut_pos_code(112.0, open, high, low, close), "abxo");  // between max-open
        assert_eq!(get_cut_pos_code(115.0, open, high, low, close), "abxo");  // at high -> upper wick
        assert_eq!(get_cut_pos_code(105.0, open, high, low, close), "inb");   // inside body
        assert_eq!(get_cut_pos_code(110.0, open, high, low, close), "inb");   // at open -> body
        assert_eq!(get_cut_pos_code(100.0, open, high, low, close), "inb");   // at close -> body
        assert_eq!(get_cut_pos_code(98.0, open, high, low, close), "abnc");   // between min-close
        assert_eq!(get_cut_pos_code(95.0, open, high, low, close), "abnc");   // at low -> lower wick
        assert_eq!(get_cut_pos_code(90.0, open, high, low, close), "bln");    // below min
    }

    #[test]
    fn test_get_cut_pos_code_uninitialized() {
        assert_eq!(get_cut_pos_code(0.0, 100.0, 115.0, 95.0, 110.0), "");
        assert_eq!(get_cut_pos_code(-1.0, 100.0, 115.0, 95.0, 110.0), "");
    }

    #[test]
    fn test_bb_cut_pos_json_field_names() {
        let mut raw_candles = Vec::new();
        for i in 0..25 {
            raw_candles.push(RawCandleInput {
                epoch: 1700000000 + i * 60,
                open: 100.0 + (i as f64) * 0.1,
                high: 102.0 + (i as f64) * 0.1,
                low: 99.0 + (i as f64) * 0.1,
                close: 101.0 + (i as f64) * 0.1,
            });
        }
        let results = perform_analysis(&raw_candles, None, None);
        let last_res = &results[results.len() - 1];
        let json_str = serde_json::to_string(last_res).expect("serialization failed");
        
        assert!(json_str.contains("\"BBLineupper_CutPos\":"), "Should contain BBLineupper_CutPos: {}", json_str);
        assert!(json_str.contains("\"BBLineMiddle_CutPos\":"), "Should contain BBLineMiddle_CutPos: {}", json_str);
        assert!(json_str.contains("\"BBLineLow_CutPos\":"), "Should contain BBLineLow_CutPos: {}", json_str);
    }

    #[test]
    fn test_compute_micro_kama_metrics_trending() {
        // กลุ่มแท่งเทียนที่วิ่งทางเดียวชัดเจน (Strong Trend Up)
        let group: Vec<OHLCV> = (0..15).map(|i| {
            OHLCV {
                open: 100.0 + i as f64,
                high: 101.0 + i as f64,
                low: 99.8 + i as f64,
                close: 100.9 + i as f64,
                volume: 0.0,
                timestamp: 1700000000 + i as i64 * 60,
            }
        }).collect();

        let (er, body_ratio, switches, is_micro) = compute_micro_kama_metrics(&group, 0.30, 0.35, 3);
        assert!(er > 0.8, "Trending should have high ER, got: {}", er);
        assert!(body_ratio > 0.5, "Trending should have high body ratio, got: {}", body_ratio);
        assert_eq!(switches, 0, "No color switches in monotonic trend");
        assert!(!is_micro, "Trending must not be choppy micro");
    }

    #[test]
    fn test_compute_micro_kama_metrics_choppy() {
        // กลุ่มแท่งเทียนสลับสีไปมา ไร้ทิศทาง (Choppy / Noise)
        let mut group = Vec::new();
        for i in 0..15 {
            let is_even = i % 2 == 0;
            group.push(OHLCV {
                open: if is_even { 100.0 } else { 101.0 },
                high: 101.5,
                low: 99.5,
                close: if is_even { 101.0 } else { 100.0 },
                volume: 0.0,
                timestamp: 1700000000 + i as i64 * 60,
            });
        }

        let (er, _body_ratio, switches, is_micro) = compute_micro_kama_metrics(&group, 0.50, 0.35, 3);
        assert!(er < 0.2, "Choppy zigzag should have low ER, got: {}", er);
        assert!(switches >= 10, "Should have many switches, got: {}", switches);
        assert!(is_micro, "Should be flagged as is_choppy_micro");
    }

    #[test]
    fn test_load_choppy_combined_config_from_settings() {
        let cfg = load_choppy_combined_config();
        assert_eq!(cfg.group_size, 15);
        assert_eq!(cfg.mode, "or");
        assert!((cfg.chop_threshold - 61.8).abs() < 1e-4);
        assert!((cfg.kama_er_threshold - 0.35).abs() < 1e-4);
    }
}

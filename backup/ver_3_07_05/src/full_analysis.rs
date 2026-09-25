use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use crate::{AppState, AppConfigPayload};
use turbo_indicators::OHLCV;
use turbo_indicators::{MACD, RSI, ChoppinessIndex, ATR, HMA, simd};
use chrono::{TimeZone, Local};
use std::fs;

// โครงสร้างรับข้อมูลดิบจาก Frontend
#[derive(Debug, Deserialize, Clone)]
pub struct RawCandleInput {
    pub epoch: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
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
    
    pub ema_medium_value: f64,
    pub ema_medium_direction: String,
    
    pub ema_long_value: f64,
    pub ema_long_direction: String,
    
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
    
    pub u_wick: f64,
    pub u_wick_percent: f64,
    pub body: f64,
    pub body_percent: f64,
    pub l_wick: f64,
    pub l_wick_percent: f64,
    
    pub ema_cut_position: String,
    pub ema_cut_long_type: String,
    pub candles_since_ema_cut: i32,
    
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

    if ma_type.to_lowercase() == "sma" {
        for i in period - 1..prices.len() {
            let sum: f64 = prices[i + 1 - period..=i].iter().sum();
            ma_values[i] = sum / period as f64;
        }
    } else if ma_type.to_lowercase() == "hma" {
        let result = HMA::new(period).compute(prices);
        for i in 0..prices.len() {
            if i < result.values.len() {
                // right align
                let offset = prices.len() - result.values.len();
                ma_values[i] = if i >= offset { result.values[i - offset] } else { 0.0 };
            }
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

fn compute_bb(prices: &[f64], period: usize, std_dev: f64) -> Vec<BollingerBands> {
    let mut bbs = vec![BollingerBands { upper: 0.0, middle: 0.0, lower: 0.0 }; prices.len()];
    if prices.len() < period || period == 0 {
        return bbs;
    }
    for i in period - 1..prices.len() {
        let window = &prices[i + 1 - period..=i];
        let mean = window.iter().sum::<f64>() / period as f64;
        let variance = window.iter().map(|value| {
            let diff = mean - *value;
            diff * diff
        }).sum::<f64>() / period as f64;
        let std = variance.sqrt();
        bbs[i] = BollingerBands {
            middle: mean,
            upper: mean + std_dev * std,
            lower: mean - std_dev * std,
        };
    }
    bbs
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

// ────────────────────────────────────────────────────────────────────────────
// API Handler
// ────────────────────────────────────────────────────────────────────────────

pub async fn handle_post_analysis_data(
    State(_state): State<AppState>,
    Json(payload): Json<Vec<RawCandleInput>>,
) -> Json<Vec<FullAnalysisResult>> {
    let n = payload.len();
    if n == 0 {
        return Json(vec![]);
    }

    // โหลดการตั้งค่าจาก setup.json
    let config: Option<AppConfigPayload> = fs::read_to_string("setup.json")
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok());

    // กำหนดค่า parameter จาก config หรือใช้ค่า default
    let ema_s_period = config.as_ref().map(|c| c.ema.short.period as usize).unwrap_or(9);
    let ema_m_period = config.as_ref().map(|c| c.ema.medium.period as usize).unwrap_or(21);
    let ema_l_period = config.as_ref().map(|c| c.ema.long.period as usize).unwrap_or(50);
    
    let ema_s_type = config.as_ref().map(|c| c.ema.short.ema_type.clone()).unwrap_or_else(|| "ema".to_string());
    let ema_m_type = config.as_ref().map(|c| c.ema.medium.ema_type.clone()).unwrap_or_else(|| "ema".to_string());
    let ema_l_type = config.as_ref().map(|c| c.ema.long.ema_type.clone()).unwrap_or_else(|| "ema".to_string());

    let adx_period = config.as_ref().map(|c| c.indicators.adx_period as usize).unwrap_or(14);
    let atr_period = config.as_ref().map(|c| c.indicators.atr_period as usize).unwrap_or(14);
    let atr_multi = config.as_ref().map(|c| c.indicators.atr_multi).unwrap_or(2.0);
    let bb_period = config.as_ref().map(|c| c.indicators.bb_period as usize).unwrap_or(20);
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

    // -- Indicator Setup --
    let ema_short = compute_ma(&closes, ema_s_period, &ema_s_type);
    let ema_medium = compute_ma(&closes, ema_m_period, &ema_m_type);
    let ema_long = compute_ma(&closes, ema_l_period, &ema_l_type);

    // คำนวณ MACD (12, 26, 9)
    let (macd_line, signal_line, _hist) = MACD::new(12, 26, 9).compute(&closes);
    let macd_val = right_align(macd_line, n);
    let macd_signal = right_align(signal_line, n);

    // คำนวณ RSI (ใช้ smc_period เป็น RSI ตามตัวอย่างในระบบเดิม หรือจะแยกก็ได้)
    let rsi_vals = right_align(RSI::new(smc_period).compute(&closes), n);

    // คำนวณ Choppiness Index
    let chop_vals = right_align(ChoppinessIndex::new(ci_period).compute(&ohlcvs), n);

    // คำนวณ ATR
    let atr_vals = right_align(ATR::new(atr_period).compute(&ohlcvs), n);

    // คำนวณ Bollinger Bands
    let bb_vals = compute_bb(&closes, bb_period, 2.0);

    // คำนวณ ADX
    let adx_vals = compute_adx_simple(&ohlcvs, adx_period);

    let mut results = Vec::with_capacity(n);
    
    for i in 0..n {
        let candle = &payload[i];
        
        let pip_size = (candle.open - candle.close).abs();
        let color = if candle.close > candle.open { "Green" } else if candle.close < candle.open { "Red" } else { "Equal" };
        
        // Anatomy
        let range = candle.high - candle.low;
        let body = (candle.open - candle.close).abs();
        let u_wick = candle.high - candle.open.max(candle.close);
        let l_wick = candle.open.min(candle.close) - candle.low;
        
        let u_wick_percent = if range > 0.0 { (u_wick / range) * 100.0 } else { 0.0 };
        let body_percent = if range > 0.0 { (body / range) * 100.0 } else { 0.0 };
        let l_wick_percent = if range > 0.0 { (l_wick / range) * 100.0 } else { 0.0 };

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

        let ema_above = if ema_short[i] >= ema_medium[i] { "ShortAbove" } else { "MediumAbove" };
        let ema_long_above = if ema_medium[i] >= ema_long[i] { "MediumAbove" } else { "LongAbove" };

        let ema_conv = if (ema_short[i] - ema_medium[i]).abs() < (previous_ema_short - previous_ema_medium).abs() { "convergence" } else { "divergence" };
        let ema_long_conv = if (ema_medium[i] - ema_long[i]).abs() < (previous_ema_medium - previous_ema_long).abs() { "C" } else { "D" };

        let bb_position = if candle.close > bb_vals[i].upper { "AboveUpper" }
                          else if candle.close > bb_vals[i].middle { "NearUpper" }
                          else if candle.close < bb_vals[i].lower { "BelowLower" }
                          else { "NearLower" };

        // Abnormal candle/ATR check
        let is_abnormal_candle = range > atr_vals[i] * atr_multi; 
        let is_abnormal_atr = atr_vals[i] > (if i > 5 { atr_vals[i-5] } else { atr_vals[i] }) * 2.0;

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
            ema_medium_value: ema_medium[i],
            ema_medium_direction: ema_medium_dir.to_string(),
            ema_long_value: ema_long[i],
            ema_long_direction: ema_long_dir.to_string(),
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
            
            u_wick,
            u_wick_percent,
            body,
            body_percent,
            l_wick,
            l_wick_percent,
            
            ema_cut_position: "-".to_string(),
            ema_cut_long_type: "-".to_string(),
            candles_since_ema_cut: 0,
            
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
            }
        };
        results.push(res);
    }
    
    Json(results)
}

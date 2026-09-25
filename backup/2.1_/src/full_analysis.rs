use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use crate::AppState;
use turbo_indicators::OHLCV;

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
    pub order_blocks: Vec<String>, // Placeholder
    pub fair_value_gaps: Vec<String>, // Placeholder
    pub equal_highs_lows: Vec<String>, // Placeholder
    pub premium_discount_zone: SmcPremiumDiscountZone,
    pub strong_weak_levels: Vec<String>, // Placeholder
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

// API Handler
pub async fn handle_post_analysis_data(
    State(_state): State<AppState>,
    Json(payload): Json<Vec<RawCandleInput>>,
) -> Json<Vec<FullAnalysisResult>> {
    let mut results = Vec::new();
    
    // (จำลองการคำนวณเบื้องต้น โครงสร้างพร้อมแล้ว เดี๋ยวเราจะใส่ลอจิกเต็มๆ ลงไป)
    for (i, candle) in payload.iter().enumerate() {
        let pip_size = (candle.open - candle.close).abs();
        let color = if candle.close > candle.open { "Green" } else if candle.close < candle.open { "Red" } else { "Equal" };
        
        let range = candle.high - candle.low;
        let body = (candle.open - candle.close).abs();
        let u_wick = candle.high - candle.open.max(candle.close);
        let l_wick = candle.open.min(candle.close) - candle.low;
        
        let u_wick_percent = if range > 0.0 { (u_wick / range) * 100.0 } else { 0.0 };
        let body_percent = if range > 0.0 { (body / range) * 100.0 } else { 0.0 };
        let l_wick_percent = if range > 0.0 { (l_wick / range) * 100.0 } else { 0.0 };
        
        let dt = chrono::Local::now(); // Placeholder
        let candletime_display = format!("{}", dt.format("%Y-%m-%d %H:%M:%S"));

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
            
            // Dummy for now, we will add full logic
            ema_short_value: candle.close,
            ema_short_direction: "Flat".to_string(),
            ema_short_turn_type: "-".to_string(),
            ema_medium_value: candle.close,
            ema_medium_direction: "Flat".to_string(),
            ema_long_value: candle.close,
            ema_long_direction: "Flat".to_string(),
            ema_above: "ShortAbove".to_string(),
            ema_long_above: "MediumAbove".to_string(),
            
            macd_12: 0.0,
            macd_23: 0.0,
            
            previous_ema_short_value: candle.close,
            previous_ema_medium_value: candle.close,
            previous_ema_long_value: candle.close,
            previous_macd_12: 0.0,
            previous_macd_23: 0.0,
            
            ema_convergence_type: "neutral".to_string(),
            ema_long_convergence_type: "N".to_string(),
            
            choppy_indicator: 0.0,
            adx_value: 0.0,
            rsi_value: 0.0,
            
            bb_values: BollingerBands { upper: candle.close, middle: candle.close, lower: candle.close },
            bb_position: "Middle".to_string(),
            
            atr_value: 0.0,
            is_abnormal_candle: false,
            is_abnormal_atr: false,
            
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
            
            smc: SmcData {
                structures: vec![],
                swing_points: vec![],
                order_blocks: vec![],
                fair_value_gaps: vec![],
                equal_highs_lows: vec![],
                premium_discount_zone: SmcPremiumDiscountZone {
                    start_time: 0, end_time: 0, premium_top: 0.0, premium_bottom: 0.0, equilibrium: 0.0, discount_top: 0.0, discount_bottom: 0.0
                },
                strong_weak_levels: vec![],
                swing_trend: "neutral".to_string(),
                internal_trend: "neutral".to_string(),
            },
            
            tick_volatility: TickVolatility {
                tick_count: 0, buy_tick_count: 0, sell_tick_count: 0, buy_sell_ratio: 0.5,
                avg_tick_move: 0.0, max_tick_move: 0.0, sum_tick_move: 0.0, volatility_clustering: 0.0, volatility_level: "Low".to_string()
            }
        };
        results.push(res);
    }
    
    Json(results)
}

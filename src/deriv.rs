use chrono::{Datelike, Local};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use turbo_indicators::OHLCV;

use crate::get_action::{
    get_suggest_color_by_strategy_with_reason, analyze_loss_factor, StrategyDecision, SuggestStrategy, TradeAction,
    ActionFixObj, MethodList, MethodAction,
};
use crate::AppConfigPayload;

fn json_to_f64(val: &serde_json::Value) -> Option<f64> {
    val.as_f64().or_else(|| val.as_str()?.parse::<f64>().ok())
}

fn json_to_i64(val: &serde_json::Value) -> Option<i64> {
    val.as_i64()
       .or_else(|| val.as_u64().map(|v| v as i64))
       .or_else(|| val.as_f64().map(|v| v as i64))
       .or_else(|| val.as_str()?.parse::<i64>().ok())
}

fn parse_candle(data: &serde_json::Value) -> Option<OHLCV> {
    let timestamp = data
        .get("open_time")
        .and_then(|v| json_to_i64(v))
        .or_else(|| data.get("epoch").and_then(|v| json_to_i64(v)))?;

    Some(OHLCV {
        open: json_to_f64(data.get("open")?)?,
        high: json_to_f64(data.get("high")?)?,
        low: json_to_f64(data.get("low")?)?,
        close: json_to_f64(data.get("close")?)?,
        volume: 0.0,
        timestamp,
    })
}

pub fn map_asset_to_symbol(asset: &str) -> String {
    match asset {
        "vol10" | "R_10" => "1HZ10V",
        "vol10_1s" => "1HZ10V",
        "vol15_1s" => "1HZ15V",
        "vol25" | "R_25" => "1HZ25V",
        "vol25_1s" => "1HZ25V",
        "vol30_1s" => "1HZ30V",
        "vol50" | "R_50" => "1HZ50V",
        "vol50_1s" => "1HZ50V",
        "vol75" | "R_75" => "1HZ75V",
        "vol75_1s" => "1HZ75V",
        "vol90_1s" => "1HZ90V",
        "vol100" | "R_100" => "1HZ100V",
        "vol100_1s" => "1HZ100V",
        _ => asset,
    }
    .to_string()
}

fn get_duration_params(granularity: i32) -> (i32, String) {
    if granularity == 2 {
        (2, "t".to_string()) // 2 Ticks for 2s
    } else {
        // เมื่อไม่เป็น 0 (และไม่ใช่ tick) ให้ใช้ durationUnit = second
        (granularity, "s".to_string())
    }
}

pub async fn get_deriv_balance(
    api_token: &str,
) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
    let (app_id, _, account_id) = crate::get_active_credentials(None);

    if account_id.is_empty() {
        return Err("DERIV_ACCOUNT_ID is not set".into());
    }

    let otp_url = format!("https://api.derivws.com/trading/v1/options/accounts/{}/otp", account_id);
    
    let client = reqwest::Client::new();
    let res = client.post(&otp_url)
        .header("Authorization", format!("Bearer {}", api_token))
        .header("Deriv-App-ID", app_id)
        .header("Content-Type", "application/json")
        .send()
        .await?;
        
    if !res.status().is_success() {
        return Err(format!("Failed to get OTP: {}", res.status()).into());
    }
    
    let otp_data: serde_json::Value = res.json().await?;
    let ws_url = otp_data.get("data").and_then(|d| d.get("url")).and_then(|u| u.as_str())
        .ok_or_else(|| "Failed to parse OTP WebSocket URL")?;

    let (mut ws_stream, _) = connect_async(ws_url).await?;
    let req = serde_json::json!({ "balance": 1, "subscribe": 1, "req_id": 1 });
    ws_stream.send(Message::Text(req.to_string())).await?;

    while let Some(msg_result) = ws_stream.next().await {
        let msg = msg_result?;
        if let Message::Text(text) = msg {
            let parsed: serde_json::Value = serde_json::from_str(&text)?;
            if let Some(error) = parsed.get("error") {
                return Err(error.get("message").and_then(|v| v.as_str()).unwrap_or("Deriv Error").into());
            }
            if let Some(bal_data) = parsed.get("balance") {
                if let Some(bal) = json_to_f64(bal_data.get("balance").unwrap_or(&serde_json::json!(0.0))) {
                    return Ok(bal);
                }
            }
        }
    }
    Err("Failed to get balance from Deriv".into())
}

pub async fn fetch_historical_candles(
    asset: &str,
    start: i64,
    end: i64,
    granularity: i64,
) -> Result<Vec<OHLCV>, Box<dyn std::error::Error + Send + Sync>> {
    let ws_app_id = env::var("DERIV_WS_APP_ID").unwrap_or_else(|_| "36544".to_string()).trim().to_string();
    let ws_url = format!("wss://ws.derivws.com/websockets/v3?app_id={}", ws_app_id);
    let (mut ws_stream, _) = connect_async(ws_url).await?;
    
    let symbol = map_asset_to_symbol(asset);
    
    // ใช้ granularity ที่ส่งมา (เช่น 60=1M, 300=5M, 900=15M, 1800=30M, 3600=1H)
    let request = json!({
        "ticks_history": symbol,
        "start": start,
        "end": end,
        "style": "candles",
        "granularity": granularity,
        "count": 5000,
        "req_id": 999
    });

    ws_stream.send(Message::Text(request.to_string())).await?;

    while let Some(msg_result) = ws_stream.next().await {
        let msg = msg_result?;
        if let Message::Text(text) = msg {
            let parsed: serde_json::Value = serde_json::from_str(&text)?;
            
            if let Some(error) = parsed.get("error") {
                return Err(error.get("message").and_then(|v| v.as_str()).unwrap_or("Deriv Error").into());
            }
            
            if let Some(candles) = parsed.get("candles") {
                let mut history = Vec::new();
                if let Some(arr) = candles.as_array() {
                    for c in arr {
                        if let Some(ohlcv) = parse_candle(c) {
                            history.push(ohlcv);
                        }
                    }
                }
                return Ok(history);
            }
        }
    }
    
    Err("Failed to fetch historical candles".into())
}

// ไม่ใช้ hash แล้ว — ใช้ sequential req_id แทน (ดูใน start_deriv_bot_multiplexed)

// ═══ TradeControlItem — แต่ละไม้เทรดใน TradeControl ═══
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[allow(non_snake_case)]
pub struct TradeControlItem {
    pub tradeno: u32,
    pub currentIndex: u32,
    pub candleSymbol: String,   // A, B, C, ...
    pub candleColor: String,    // green, red
    pub action: String,         // CALL, PUT, IDLE
    pub winStatus: String,      // Win, Loss, -, Skipped
    pub lossCon: u32,
    pub whipsawZone: bool,      // true = 🎯
}

/// ตรวจจับ Whipsaw Zone จากประวัติสี 4 แท่งล่าสุด
/// คืน true เมื่อพบแพทเทิร์น G-R-G-R หรือ R-G-R-G
pub fn detect_whipsaw_zone(color_list: &[String]) -> bool {
    if color_list.len() < 4 {
        return false;
    }
    let len = color_list.len();
    let last4 = &color_list[len - 4..];
    // ตรวจว่าสลับสีทุกแท่ง: แท่ง n ≠ แท่ง n+1 สำหรับทั้ง 3 คู่
    last4[0] != last4[1] && last4[1] != last4[2] && last4[2] != last4[3]
}

pub struct AssetState {
    pub asset: String,
    pub deriv_symbol: String,
    pub duration: i32,
    pub duration_unit: String,
    pub req_id: i64,

    pub current_history: Vec<OHLCV>,
    pub loss_con: u32,
    pub win_con: u32,
    pub max_win_con: u32,
    pub max_loss_con: u32,
    pub is_trading: bool,
    pub current_balance: f64,

    pub trade_no: u32,
    pub sub_trade_no: u32,
    pub last_trade_details: Option<serde_json::Value>,
    pub last_trade_epoch: i64,
    pub in_martingale: bool,
    pub overtime_mode: bool,

    pub last_tick_analysis: Option<crate::full_analysis_ver2::FullAnalysisResult>,

    pub trade_buy_instant: Option<std::time::Instant>,
    pub active_contract_id: Option<i64>,
    pub last_processed_contract_id: Option<String>,
    pub pending_proposal_call: Option<serde_json::Value>,
    pub pending_proposal_put: Option<serde_json::Value>,
    pub target_trade_action: Option<String>,
    pub last_spot_call: Option<f64>,
    pub last_spot_put: Option<f64>,

    // ═══ Strategy Comparison Tracking ═══
    pub strategy_loss_con_v1: u32,
    pub strategy_loss_con_v2: u32,
    pub strategy_loss_con_v3a: u32,
    pub strategy_loss_con_v3b: u32,
    pub strategy_loss_con_v3c: u32,
    pub strategy_loss_con_fta: u32,
    pub strategy_loss_con_ftb: u32,
    pub strategy_loss_con_pktrend: u32,
    pub strategy_loss_con_pktrend_v5: u32,
    pub strategy_loss_con_forecast: u32,
    pub strategy_loss_con_pktrend_case_code: u32,
    pub strategy_runno: u32,
    pub pending_strategy_entry: Option<serde_json::Value>,
    pub pending_analysis_obj: Option<crate::full_analysis_ver2::FullAnalysisResult>,

    // ═══ Whipsaw Zone (Trade Control) ═══
    pub trade_control_items: Vec<TradeControlItem>,
    pub trade_control_color_list: Vec<String>,  // ประวัติสีแท่งเทียนที่เข้าเทรด
    pub whipsaw_zone_active: bool,              // true = กำลังอยู่ใน Whipsaw Zone
    pub whipsaw_frozen_loss_con: u32,            // ค่า lossCon ที่ถูกแช่แข็งตอนเข้า Whipsaw

    // ═══ Track Order History ═══
    pub track_order_list: Vec<serde_json::Value>,
}

impl AssetState {
    pub fn new(asset: String, granularity: i32) -> Self {
        let deriv_symbol = map_asset_to_symbol(&asset);
        let (duration, duration_unit) = get_duration_params(granularity);
        let req_id = 0; // จะถูกตั้งค่าใน start_deriv_bot_multiplexed
        Self {
            asset,
            deriv_symbol,
            duration,
            duration_unit,
            req_id,
            current_history: Vec::new(),
            loss_con: 0,
            win_con: 0,
            max_win_con: 0,
            max_loss_con: 0,
            is_trading: false,
            current_balance: 0.0,
            trade_no: 1,
            sub_trade_no: 1,
            last_trade_details: None,
            last_trade_epoch: 0,
            in_martingale: false,
            overtime_mode: false,
            last_tick_analysis: None,
            trade_buy_instant: None,
            active_contract_id: None,
            last_processed_contract_id: None,
            pending_proposal_call: None,
            pending_proposal_put: None,
            target_trade_action: None,
            last_spot_call: None,
            last_spot_put: None,
            // Strategy Comparison
            strategy_loss_con_v1: 0,
            strategy_loss_con_v2: 0,
            strategy_loss_con_v3a: 0,
            strategy_loss_con_v3b: 0,
            strategy_loss_con_v3c: 0,
            strategy_loss_con_fta: 0,
            strategy_loss_con_ftb: 0,
            strategy_loss_con_pktrend: 0,
            strategy_loss_con_pktrend_v5: 0,
            strategy_loss_con_forecast: 0,
            strategy_loss_con_pktrend_case_code: 0,
            strategy_runno: 0,
            pending_strategy_entry: None,
            pending_analysis_obj: None,
            // Whipsaw Zone
            trade_control_items: Vec::new(),
            trade_control_color_list: Vec::new(),
            whipsaw_zone_active: false,
            whipsaw_frozen_loss_con: 0,
            track_order_list: Vec::new(),
        }
    }
}

fn find_asset_by_symbol(states: &HashMap<String, AssetState>, symbol: &str) -> String {
    states
        .iter()
        .find(|(_, s)| s.deriv_symbol == symbol)
        .map(|(k, _)| k.clone())
        .unwrap_or_default()
}

fn find_asset_by_contract_id(states: &HashMap<String, AssetState>, contract_id: i64) -> String {
    states
        .iter()
        .find(|(_, s)| s.active_contract_id == Some(contract_id))
        .map(|(k, _)| k.clone())
        .unwrap_or_default()
}

type WsSink = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    Message,
>;
type WsStream = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
>;

async fn reconnect_public_ws(
    api_token: &str,
    app_id: &str,
    account_id: &str,
) -> Result<(WsSink, WsStream), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let otp_url = format!("https://api.derivws.com/trading/v1/options/accounts/{}/otp", account_id);
    let res = client.post(&otp_url)
        .header("Authorization", format!("Bearer {}", api_token.trim()))
        .header("Deriv-App-ID", app_id.trim())
        .header("Content-Type", "application/json")
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(format!("Failed to get OTP for candle reconnect: {}", res.status()).into());
    }

    let otp_data: serde_json::Value = res.json().await?;
    let otp_ws_url = otp_data.get("data").and_then(|d| d.get("url")).and_then(|u| u.as_str())
        .ok_or_else(|| "Failed to parse OTP WebSocket URL for candle reconnect")?;

    let (ws, _) = tokio_tungstenite::connect_async(otp_ws_url).await?;
    Ok(ws.split())
}

async fn reconnect_private_ws(
    api_token: &str,
    app_id: &str,
    account_id: &str,
) -> Result<(WsSink, WsStream), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let otp_url = format!("https://api.derivws.com/trading/v1/options/accounts/{}/otp", account_id);
    let res = client.post(&otp_url)
        .header("Authorization", format!("Bearer {}", api_token.trim()))
        .header("Deriv-App-ID", app_id.trim())
        .header("Content-Type", "application/json")
        .send()
        .await?;
    
    if !res.status().is_success() {
        return Err(format!("OTP failed: {}", res.status()).into());
    }
    
    let otp_data: serde_json::Value = res.json().await?;
    let ws_url = otp_data.get("data").and_then(|d| d.get("url")).and_then(|u| u.as_str())
        .ok_or_else(|| "Failed to parse OTP URL")?;
    
    let (ws, _) = connect_async(ws_url).await?;
    let (mut write, read) = ws.split();
    
    // Re-subscribe to balance and transactions
    let bal_req = serde_json::json!({ "balance": 1, "subscribe": 1, "req_id": 1 });
    write.send(Message::Text(bal_req.to_string())).await?;
    let tx_sub = serde_json::json!({ "transaction": 1, "subscribe": 1, "req_id": 11111 });
    write.send(Message::Text(tx_sub.to_string())).await?;
    
    Ok((write, read))
}

pub async fn start_deriv_bot_multiplexed(
    app_id: String,
    api_token: String,
    account_id: String,
    config: AppConfigPayload,
    assets: Vec<String>,
    tx: Arc<broadcast::Sender<serde_json::Value>>,
    mut cmd_rx: broadcast::Receiver<serde_json::Value>,
    overtime_signals: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use futures_util::StreamExt;
    
    println!(
        "\n🔌 [Multiplex] กำลังเชื่อมต่อ Deriv OTP WebSocket สำหรับ {} assets...",
        assets.len()
    );

    let client = reqwest::Client::new();
    let otp_url = format!("https://api.derivws.com/trading/v1/options/accounts/{}/otp", account_id);
    
    println!("🔑 [Multiplex] OTP Request:");
    println!("   URL: {}", otp_url);
    println!("   Token (first 20): {}...", &api_token[..20.min(api_token.len())]);
    println!("   App-ID: '{}'", app_id);
    println!("   Account-ID: '{}'", account_id);
    println!("   Token len: {}, App-ID len: {}, Account-ID len: {}", api_token.len(), app_id.len(), account_id.len());
    
    let res = client.post(&otp_url)
        .header("Authorization", format!("Bearer {}", api_token.trim()))
        .header("Deriv-App-ID", app_id.trim())
        .header("Content-Type", "application/json")
        .send()
        .await?;
    
    println!("🔑 [Multiplex] OTP Response status: {}", res.status());
        
    if !res.status().is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("Failed to get OTP: {} | body: {}", body, otp_url).into());
    }
    
    let otp_data: serde_json::Value = res.json().await?;
    let private_ws_url = otp_data.get("data").and_then(|d| d.get("url")).and_then(|u| u.as_str())
        .ok_or_else(|| "Failed to parse OTP WebSocket URL")?;

    // ขอ OTP อีกครั้งสำหรับ candle data WebSocket (Deriv ต้องการ auth สำหรับ Synthetic Indices)
    let res2 = client.post(&otp_url)
        .header("Authorization", format!("Bearer {}", api_token.trim()))
        .header("Deriv-App-ID", app_id.trim())
        .header("Content-Type", "application/json")
        .send()
        .await?;

    if !res2.status().is_success() {
        return Err(format!("Failed to get second OTP for candle data: {}", res2.status()).into());
    }

    let otp_data2: serde_json::Value = res2.json().await?;
    let candle_ws_url = otp_data2.get("data").and_then(|d| d.get("url")).and_then(|u| u.as_str())
        .ok_or_else(|| "Failed to parse second OTP WebSocket URL for candle data")?;

    let (public_ws_stream, _) = connect_async(candle_ws_url).await?;
    let (private_ws_stream, _) = connect_async(private_ws_url).await?;
    
    let (mut public_write, mut public_read) = public_ws_stream.split();
    let (mut private_write, mut private_read) = private_ws_stream.split();

    println!("✅ [Multiplex] เชื่อมต่อ WebSocket สำเร็จ (Candle OTP & Private OTP)!");

    let mut asset_states: HashMap<String, AssetState> = HashMap::new();
    let mut req_id_to_asset: HashMap<i64, String> = HashMap::new();
    for (idx, asset) in assets.iter().enumerate() {
        let base_req_id = (idx as i64 + 1) * 1000;
        let mut state = AssetState::new(asset.clone(), config.granularity_settings);
        state.req_id = base_req_id;
        println!(
            "📋 [Multiplex] Asset: {} → symbol: {} | req_id_base: {} (ohlc:{}, proposal:{}, buy:{}, track:{})",
            asset, state.deriv_symbol, state.req_id,
            base_req_id, base_req_id + 1, base_req_id + 2, base_req_id + 3
        );
        req_id_to_asset.insert(base_req_id, asset.clone());
        req_id_to_asset.insert(base_req_id + 1, asset.clone());
        req_id_to_asset.insert(base_req_id + 2, asset.clone());
        req_id_to_asset.insert(base_req_id + 3, asset.clone());
        req_id_to_asset.insert(base_req_id + 4, asset.clone());
        asset_states.insert(asset.clone(), state);
    }

    let req = serde_json::json!({ "balance": 1, "subscribe": 1, "req_id": 1 });
    private_write.send(Message::Text(req.to_string())).await?;
    let tx_sub = serde_json::json!({ "transaction": 1, "subscribe": 1, "req_id": 11111 });
    private_write.send(Message::Text(tx_sub.to_string())).await?;

    let mut _global_balance = 0.0;
    let candle_count: i32 = std::env::var("CANDLE_COUNT")
        .unwrap_or_else(|_| "1000".to_string())
        .parse()
        .unwrap_or(1000);

    for (asset, state) in asset_states.iter_mut() {
        let fetch_granularity = if config.granularity < 60 { 60 } else { config.granularity };
        let ticks_request = serde_json::json!({
            "ticks_history": state.deriv_symbol,
            "end": "latest",
            "style": "candles",
            "granularity": fetch_granularity,
            "count": candle_count,
            "subscribe": 1,
            "req_id": state.req_id
        });
        println!("📦 [{}] ร้องขอข้อมูลแท่งเทียน (Public)", asset);
        public_write.send(Message::Text(ticks_request.to_string())).await?;
    }

    let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(30));

    loop {
        let msg_data = tokio::select! {
            _ = ping_interval.tick() => {
                let _ = public_write.send(Message::Ping(vec![])).await;
                let _ = private_write.send(Message::Ping(vec![])).await;
                continue;
            }
            cmd_result = cmd_rx.recv() => {
                match cmd_result {
                    Ok(cmd) => {
                        if cmd["command"] == "sell" {
                            if let Some(cid) = cmd["contract_id"].as_i64() {
                                let sell_req = serde_json::json!({ "sell": cid, "price": 0 });
                                let _ = private_write.send(Message::Text(sell_req.to_string())).await;
                                println!("📤 [Multiplex] ส่งคำสั่งขาย Contract ID: {}", cid);
                            }
                        } else if cmd["command"] == "reload_history" {
                            println!("\n🔄 [Multiplex] ได้รับคำสั่ง reload_history. กำลังคำนวณและอัปเดตประวัติแท่งเทียน...");
                            let mut latest_config = config.clone();
                            if let Ok(content) = fs::read_to_string("setup/setup.json") {
                                if let Ok(parsed_config) = serde_json::from_str::<AppConfigPayload>(&content) {
                                    latest_config = parsed_config;
                                }
                            }
                            for (_asset, state) in asset_states.iter_mut() {
                                if !state.current_history.is_empty() {
                                    let mapped_history: Vec<crate::full_analysis_ver2::RawCandleInput> = state.current_history.iter().map(|c| crate::full_analysis_ver2::RawCandleInput {
                                        epoch: c.timestamp,
                                        open: c.open,
                                        high: c.high,
                                        low: c.low,
                                        close: c.close,
                                    }).collect();
                                     let config_for_analysis = match serde_json::from_str::<crate::full_analysis_ver2::AppConfigPayload>(&serde_json::to_string(&latest_config).unwrap()) {
                                         Ok(c) => Some(c),
                                         Err(e) => {
                                             eprintln!("❌ [Multiplex-reload_history] Failed to deserialize full_analysis_ver2::AppConfigPayload: {:?}", e);
                                             None
                                         }
                                     };
                                    let analysis_result = crate::full_analysis_ver2::perform_analysis(&mapped_history, config_for_analysis, Some(&state.asset));
                                    
                                    let _ = tx.send(json!({
                                        "type": "candles_history",
                                        "asset": state.asset.clone(),
                                        "data": analysis_result
                                    }));
                                }
                            }
                            println!("✅ [Multiplex] ส่งข้อมูล candles_history ที่คำนวณใหม่ให้ Client เรียบร้อย");
                        } else if cmd["command"] == "manual_trade" {
                            // ═══ Manual Trade: เปิด order แบบ manual ═══
                            let target_asset = cmd["asset"].as_str().unwrap_or("").to_string();
                            let contract_type = cmd["contract_type"].as_str().unwrap_or("CALL").to_string();
                            let amount = cmd["amount"].as_f64().unwrap_or(1.0);

                            println!("\n🖐️ [Multiplex] Manual Trade: {} {} ${:.2} on {}", contract_type, target_asset, amount, target_asset);

                            if let Some(state) = asset_states.get_mut(&target_asset) {
                                if state.is_trading {
                                    println!("⚠️ [{}] Manual Trade ไม่สามารถเปิดได้ — กำลังเทรดอยู่!", target_asset);
                                    let _ = tx.send(json!({
                                        "type": "bot_log",
                                        "asset": target_asset.clone(),
                                        "data": { "message": format!("⚠️ [{}] Manual Trade ล้มเหลว — กำลังเทรดอยู่!", target_asset) }
                                    }));
                                } else {
                                    // อัปเดต duration จาก setup.json
                                    let mut manual_config = config.clone();
                                    if let Ok(content) = fs::read_to_string("setup/setup.json") {
                                        if let Ok(parsed_config) = serde_json::from_str::<AppConfigPayload>(&content) {
                                            manual_config = parsed_config;
                                        }
                                    }
                                    let trade_granularity = manual_config.granularity_settings;
                                    let (new_duration, new_duration_unit) = get_duration_params(trade_granularity);
                                    state.duration = new_duration;
                                    state.duration_unit = new_duration_unit;

                                    state.is_trading = true;
                                    state.trade_buy_instant = Some(std::time::Instant::now());
                                    state.active_contract_id = None;
                                    state.last_processed_contract_id = None;

                                    let _ = tx.send(json!({
                                        "type": "trade_start",
                                        "asset": state.asset.clone(),
                                        "data": {
                                            "loss_con": state.loss_con,
                                            "manual": true
                                        }
                                    }));

                                    // ส่ง Direct Buy Request
                                    let buy_req_id = state.req_id + 2;
                                    let buy_request = json!({
                                        "buy": 1,
                                        "price": 100000,
                                        "parameters": {
                                            "amount": amount,
                                            "basis": "stake",
                                            "contract_type": contract_type,
                                            "currency": "USD",
                                            "duration": state.duration,
                                            "duration_unit": state.duration_unit,
                                            "underlying_symbol": state.deriv_symbol
                                        },
                                        "req_id": buy_req_id
                                    });

                                    println!(
                                        "🖐️ [{}] Manual Trade → {} ${:.2} | duration={}{}  | req_id={}",
                                        state.asset, contract_type, amount,
                                        state.duration, state.duration_unit, buy_req_id
                                    );

                                    let _ = private_write
                                        .send(Message::Text(buy_request.to_string()))
                                        .await;

                                    // FIX: Insert req_id mapping to track buy response properly
                                    req_id_to_asset.insert(buy_req_id, target_asset.clone());

                                    let _ = tx.send(json!({
                                        "type": "bot_log",
                                        "asset": target_asset.clone(),
                                        "data": { "message": format!("🖐️ [{}] Manual {} ${:.2} | duration={}{}", target_asset, contract_type, amount, state.duration, state.duration_unit) }
                                    }));
                                }
                            } else {
                                println!("⚠️ [Multiplex] Manual Trade: asset '{}' ไม่พบในรายการ", target_asset);
                                let _ = tx.send(json!({
                                    "type": "bot_log",
                                    "asset": "system",
                                    "data": { "message": format!("⚠️ Manual Trade: asset '{}' ไม่พบ — ต้อง Go Trade ก่อน", target_asset) }
                                }));
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        println!("⚠️ [Multiplex] Command channel closed. Exiting bot loop.");
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        println!("⚠️ [Multiplex] Command channel lagged by {} messages.", n);
                    }
                }
                continue;
            }
            public_msg = public_read.next() => {
                match public_msg {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        eprintln!("⚠️ [Multiplex] Public WS error: {}. Reconnecting...", e);
                        match reconnect_public_ws(&api_token, &app_id, &account_id).await {
                            Ok((new_write, new_read)) => {
                                public_write = new_write;
                                public_read = new_read;
                                println!("✅ [Multiplex] Candle WS reconnected!");
                                let candle_count: i32 = std::env::var("CANDLE_COUNT").unwrap_or_else(|_| "1000".to_string()).parse().unwrap_or(1000);
                                for (_asset, state) in asset_states.iter_mut() {
                                    let fetch_granularity = if config.granularity < 60 { 60 } else { config.granularity };
                                    let ticks_request = serde_json::json!({
                                        "ticks_history": state.deriv_symbol,
                                        "end": "latest",
                                        "style": "candles",
                                        "granularity": fetch_granularity,
                                        "count": candle_count,
                                        "subscribe": 1,
                                        "req_id": state.req_id
                                    });
                                    let _ = public_write.send(Message::Text(ticks_request.to_string())).await;
                                }
                            }
                            Err(e) => {
                                eprintln!("❌ [Multiplex] Public WS reconnect failed: {}. Continuing...", e);
                            }
                        }
                        continue;
                    }
                    None => {
                        eprintln!("⚠️ [Multiplex] Public WS disconnected. Reconnecting...");
                        match reconnect_public_ws(&api_token, &app_id, &account_id).await {
                            Ok((new_write, new_read)) => {
                                public_write = new_write;
                                public_read = new_read;
                                println!("✅ [Multiplex] Candle WS reconnected!");
                                let candle_count: i32 = std::env::var("CANDLE_COUNT").unwrap_or_else(|_| "1000".to_string()).parse().unwrap_or(1000);
                                for (_asset, state) in asset_states.iter_mut() {
                                    let fetch_granularity = if config.granularity < 60 { 60 } else { config.granularity };
                                    let ticks_request = serde_json::json!({
                                        "ticks_history": state.deriv_symbol,
                                        "end": "latest",
                                        "style": "candles",
                                        "granularity": fetch_granularity,
                                        "count": candle_count,
                                        "subscribe": 1,
                                        "req_id": state.req_id
                                    });
                                    let _ = public_write.send(Message::Text(ticks_request.to_string())).await;
                                }
                            }
                            Err(e) => {
                                eprintln!("❌ [Multiplex] Public WS reconnect failed: {}. Continuing...", e);
                            }
                        }
                        continue;
                    }
                }
            }
            private_msg = private_read.next() => {
                match private_msg {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        eprintln!("⚠️ [Multiplex] Private WS error: {}. Reconnecting...", e);
                        match reconnect_private_ws(&api_token, &app_id, &account_id).await {
                            Ok((new_write, new_read)) => {
                                private_write = new_write;
                                private_read = new_read;
                                println!("✅ [Multiplex] Private WS reconnected!");
                            }
                            Err(e) => {
                                eprintln!("❌ [Multiplex] Private WS reconnect failed: {}. Continuing with public only.", e);
                            }
                        }
                        continue;
                    }
                    None => {
                        eprintln!("⚠️ [Multiplex] Private WS disconnected. Reconnecting...");
                        match reconnect_private_ws(&api_token, &app_id, &account_id).await {
                            Ok((new_write, new_read)) => {
                                private_write = new_write;
                                private_read = new_read;
                                println!("✅ [Multiplex] Private WS reconnected!");
                            }
                            Err(e) => {
                                eprintln!("❌ [Multiplex] Private WS reconnect failed: {}. Continuing with public only.", e);
                            }
                        }
                        continue;
                    }
                }
            }
        };
let mut current_config = config.clone();
                if let Ok(content) = tokio::fs::read_to_string("setup/setup.json").await {
                    if let Ok(parsed_config) = serde_json::from_str::<AppConfigPayload>(&content) {
                        current_config = parsed_config;
                    }
                }

                if let Message::Text(text) = msg_data {
            let parsed: serde_json::Value = serde_json::from_str(&text)?;

            let msg_type = parsed
                .get("msg_type")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown");

            if let Some(auth_data) = parsed.get("authorize") {
                println!("\n🔐 [Multiplex] Authorize สำเร็จ!");

                let mut global_balance = 0.0;
                if let Some(bal) = json_to_f64(auth_data.get("balance").unwrap_or(&json!(0.0))) {
                    global_balance = bal;
                }

                let candle_count: i32 = env::var("CANDLE_COUNT")
                    .unwrap_or_else(|_| "1000".to_string())
                    .parse()
                    .unwrap_or(1000);

                for (asset, state) in asset_states.iter_mut() {
                    state.current_balance = global_balance;
                    let _ = tx.send(json!({
                        "type": "balance_update",
                        "asset": asset.clone(),
                        "data": { "balance": state.current_balance }
                    }));

                    let fetch_granularity = if config.granularity < 60 { 60 } else { config.granularity };
                    let ticks_request = json!({
                        "ticks_history": state.deriv_symbol,
                        "end": "latest",
                        "style": "candles",
                        "granularity": fetch_granularity,
                        "count": candle_count,
                        "subscribe": 1,
                        "req_id": state.req_id
                    });

                    println!("📦 [{}] ร้องขอข้อมูลแท่งเทียน", asset);
                    public_write
                        .send(Message::Text(ticks_request.to_string()))
                        .await?;
                }

                let tx_sub = json!({ "transaction": 1, "subscribe": 1, "req_id": 11111 });
                println!("📡 [Multiplex] Subscribe transaction stream");
                private_write.send(Message::Text(tx_sub.to_string())).await?;
            }

            if let Some(candles) = parsed.get("candles") {
                // ลอง route ด้วย req_id ก่อน
                let req_id = parsed
                    .get("echo_req")
                    .and_then(|r| r.get("req_id"))
                    .and_then(|r| r.as_i64())
                    .unwrap_or(0);
                let mut asset_key = req_id_to_asset.get(&req_id).cloned().unwrap_or_default();

                // Fallback: ถ้า req_id ไม่ match → ใช้ echo_req.ticks_history (symbol) แทน
                if asset_key.is_empty() {
                    let symbol = parsed
                        .get("echo_req")
                        .and_then(|r| r.get("ticks_history"))
                        .and_then(|s| s.as_str())
                        .unwrap_or("");
                    asset_key = find_asset_by_symbol(&asset_states, symbol);
                    println!(
                        "🔄 [Multiplex] req_id={} ไม่ match → fallback ใช้ symbol='{}' → asset='{}'",
                        req_id, symbol, asset_key
                    );
                }

                if asset_key.is_empty() {
                    println!("⚠️ [Multiplex] candles response ไม่สามารถ route ได้! req_id={}, echo_req={:?}", req_id, parsed.get("echo_req"));
                } else if let Some(state) = asset_states.get_mut(&asset_key) {
                    if let Some(arr) = candles.as_array() {
                        for c in arr {
                            if let Some(ohlcv) = parse_candle(c) {
                                state.current_history.push(ohlcv);
                            }
                        }
                    }

                    println!(
                        "📊 [{}] ได้รับแท่งเทียนย้อนหลัง {} แท่ง",
                        state.asset,
                        state.current_history.len()
                    );

                    let mapped_history: Vec<crate::full_analysis_ver2::RawCandleInput> = state.current_history.iter().map(|c| crate::full_analysis_ver2::RawCandleInput {
                        epoch: c.timestamp,
                        open: c.open,
                        high: c.high,
                        low: c.low,
                        close: c.close,
                    }).collect();
                    let config_for_analysis = match serde_json::from_str::<crate::full_analysis_ver2::AppConfigPayload>(&serde_json::to_string(&current_config).unwrap()) {
                        Ok(c) => Some(c),
                        Err(e) => {
                            eprintln!("❌ [Multiplex-candles] Failed to deserialize full_analysis_ver2::AppConfigPayload: {:?}", e);
                            None
                        }
                    };
                    let analysis_result = crate::full_analysis_ver2::perform_analysis(&mapped_history, config_for_analysis, Some(&state.asset));

                    let _ = tx.send(json!({
                        "type": "candles_history",
                        "asset": state.asset.clone(),
                        "data": analysis_result
                    }));
                }
            }

            if let Some(ohlc) = parsed.get("ohlc") {
                let symbol = ohlc.get("symbol").and_then(|s| s.as_str()).unwrap_or("");
                let asset_key = find_asset_by_symbol(&asset_states, symbol);

                if asset_key.is_empty() {
                    // สำคัญ: ถ้า symbol ไม่ match → log ให้รู้
                    println!(
                        "⚠️ [Multiplex] ohlc symbol='{}' ไม่ match กับ asset ใดเลย",
                        symbol
                    );
                } else if let Some(state) = asset_states.get_mut(&asset_key) {
                    if let Some(new_tick) = parse_candle(ohlc) {
                        let is_new_candle;
                        if let Some(last) = state.current_history.last_mut() {
                            if last.timestamp == new_tick.timestamp {
                                *last = new_tick;
                                is_new_candle = false;
                            } else {
                                state.current_history.push(new_tick);
                                is_new_candle = true;
                            }
                        } else {
                            state.current_history.push(new_tick);
                            is_new_candle = true;
                        }

                        print!(
                            "\r⏱ [{}] close={:.4} | {} | {} แท่ง          ",
                            state.asset,
                            new_tick.close,
                            if is_new_candle {
                                "🆕 New"
                            } else {
                                "📝 Upd"
                            },
                            state.current_history.len()
                        );
                        let _ = io::stdout().flush();

                        let mapped_history: Vec<crate::full_analysis_ver2::RawCandleInput> = state.current_history.iter().map(|c| crate::full_analysis_ver2::RawCandleInput {
                            epoch: c.timestamp,
                            open: c.open,
                            high: c.high,
                            low: c.low,
                            close: c.close,
                        }).collect();
                        let config_for_analysis = match serde_json::from_str::<crate::full_analysis_ver2::AppConfigPayload>(&serde_json::to_string(&current_config).unwrap()) {
                            Ok(c) => Some(c),
                            Err(e) => {
                                eprintln!("❌ [Multiplex-ohlc] Failed to deserialize full_analysis_ver2::AppConfigPayload: {:?}", e);
                                None
                            }
                        };
                        let analysis_results = crate::full_analysis_ver2::perform_analysis(
                            &mapped_history,
                            config_for_analysis,
                            Some(&state.asset),
                        );
                        if let Some(analysis_obj) = analysis_results.last() {
                            let latest_analysis = analysis_obj;
                            let _ = tx.send(json!({
                                "type": "ohlc_update",
                                "asset": state.asset.clone(),
                                "data": latest_analysis
                            }));

                            if is_new_candle {
                                if let Some(ref prev_analysis) = state.last_tick_analysis {
                                    let now = Local::now();
                                    let month_folder =
                                        format!("{:02}-{:04}", now.month(), now.year() as i32 + 543);
                                    let day_folder = format!(
                                        "{:02}-{:02}-{:04}",
                                        now.day(),
                                        now.month(),
                                        now.year() as i32 + 543
                                    );
                                    let dir_path_asset = format!(
                                        "tradeData/{}/{}/{}",
                                        month_folder, day_folder, state.deriv_symbol
                                    );
                                    let _ = fs::create_dir_all(&dir_path_asset);
                                    let log_asset_file = format!("{}/asset.json", dir_path_asset);
                                    let mut asset_history: Vec<serde_json::Value> = vec![];
                                    if let Ok(content) = fs::read_to_string(&log_asset_file) {
                                        if let Ok(arr) = serde_json::from_str(&content) {
                                            asset_history = arr;
                                        }
                                    }
                                    if let Ok(json_val) = serde_json::to_value(prev_analysis) {
                                        asset_history.push(json_val);
                                        if let Ok(json_str) = serde_json::to_string_pretty(&asset_history) {
                                            let _ = fs::write(&log_asset_file, json_str);
                                        }
                                    }
                                }
                            }

                            if state.is_trading {
                                if let Some(ref buy_time) = state.trade_buy_instant {
                                    let elapsed = buy_time.elapsed().as_secs();
                                    let max_wait = (state.duration as u64) + 15;
                                    if elapsed > max_wait {
                                        println!("\n⏰ [{}] TIMEOUT! is_trading=true มานาน {}s (max={}s)", state.asset, elapsed, max_wait);

                                        // ═══ Auto Sell When Timeout ═══
                                        let auto_sell_enabled = current_config.trade.auto_sell_timeout;
                                        let auto_sell_secs = current_config.trade.auto_sell_timeout_seconds as u64;
                                        if auto_sell_enabled && elapsed >= auto_sell_secs {
                                            if let Some(cid) = state.active_contract_id {
                                                let sell_req = json!({ "sell": cid, "price": 0 });
                                                private_write.send(Message::Text(sell_req.to_string())).await?;
                                                let sell_log = format!(
                                                    "🔴 [{}] AUTO SELL TIMEOUT! elapsed={}s (limit={}s) → ส่งคำสั่งขาย Contract ID: {}",
                                                    state.asset, elapsed, auto_sell_secs, cid
                                                );
                                                println!("{}", sell_log);
                                                let _ = tx.send(json!({
                                                    "type": "bot_log",
                                                    "asset": state.asset.clone(),
                                                    "data": { "message": sell_log }
                                                }));
                                            } else {
                                                state.is_trading = false;
                                                state.trade_buy_instant = None;
                                            }
                                        } else if let Some(cid) = state.active_contract_id {
                                            let track_req_id = state.req_id + 3;
                                            let query = json!({
                                                "proposal_open_contract": 1,
                                                "contract_id": cid,
                                                "req_id": track_req_id
                                            });
                                            private_write
                                                .send(Message::Text(query.to_string()))
                                                .await?;
                                        } else {
                                            state.is_trading = false;
                                            state.trade_buy_instant = None;
                                        }
                                    }
                                }
                            }

                            // Overtime signal detection ถูกย้ายไปตรวจในระดับ global message loop แล้ว (ด้านล่าง)
                            // เพื่อให้ทุก asset ได้รับ signal แม้ไม่มี ohlc tick เข้ามา

                            if is_new_candle && !state.is_trading {
                                if state.overtime_mode && !state.in_martingale {
                                    let skip_log = format!(
                                        "⏭️ [{}] OVERTIME — Win แล้ว ไม่เปิดออเดอร์ใหม่",
                                        state.asset
                                    );
                                    // Only print once or handle properly. But since this ticks every second, let's limit logs
                                    // Actually, just let it be, but we should not send skip log every tick.
                                    // So we only log when is_new_candle is true. Since it's is_new_candle, it's fine.
                                    println!("\n{}", skip_log);
                                    let _ = tx.send(json!({
                                        "type": "bot_log",
                                        "asset": state.asset.clone(),
                                        "data": { "message": skip_log }
                                    }));
                                } else if let Some(ref prev) = state.last_tick_analysis {
                                    // ═══ MANUAL MODE — ข้ามการเทรดอัตโนมัติ รอ user กด Call/Put ═══
                                    if current_config.trade.suggest_strategy == "MANUAL" {
                                        let manual_log = format!(
                                            "🖐️ [{}] Manual Mode — ข้ามเทรดอัตโนมัติ รอคำสั่ง Call/Put จาก User",
                                            state.asset
                                        );
                                        println!("\n{}", manual_log);
                                        let _ = tx.send(json!({
                                            "type": "bot_log",
                                            "asset": state.asset.clone(),
                                            "data": { "message": manual_log }
                                        }));
                                        state.last_tick_analysis = Some(prev.clone());
                                        // ไม่เข้า should_trade block — continue loop
                                    } else {
                                    let should_trade = if state.in_martingale {
                                        true
                                    } else if current_config.trade.suggest_strategy == "FTA" || current_config.trade.suggest_strategy == "FTB" || current_config.trade.suggest_strategy == "PKTrend" || current_config.trade.suggest_strategy == "PKTrendV5" || current_config.trade.suggest_strategy == "ForecastSequence" || current_config.trade.suggest_strategy == "PKTrendSelectCaseCode" {
                                        true
                                    } else {
                                        prev.is_atr
                                    };

                                    if should_trade {
                                        // ═══ Global Pre-Filters (Karma Choppy / BB Flat Choppy / BB Squeeze) ═══
                                        // ตรวจก่อนทุก Strategy — ถ้าเปิด toggle + ค่า = true → IDLE ทันที
                                        let bb_block_reason: Option<String> = if current_config.trade.check_karma_choppy && prev.is_choppy_combined {
                                            Some(format!(
                                                "🛑 [{}] 🔮♎ Karma Choppy Combined detected (is_choppy_combined=true) → IDLE ห้ามเทรดเด็ดขาด",
                                                state.asset
                                            ))
                                        } else if current_config.trade.check_bb_flat_choppy && prev.is_bb_flat_choppy {
                                            Some(format!(
                                                "🛑 [{}] BB Flat/Choppy Zone detected (is_bb_flat_choppy=true) → IDLE ห้ามเทรดเด็ดขาด",
                                                state.asset
                                            ))
                                        } else if current_config.trade.check_bb_squeeze && prev.is_bb_squeeze {
                                            Some(format!(
                                                "🛑 [{}] BB Squeeze detected (is_bb_squeeze=true) → IDLE ห้ามเทรดเด็ดขาด",
                                                state.asset
                                            ))
                                        } else {
                                            None
                                        };

                                        if let Some(bb_log) = bb_block_reason {
                                            // — Log to console + bot_log
                                            println!("\n{}", bb_log);
                                            let _ = tx.send(json!({
                                                "type": "bot_log",
                                                "asset": state.asset.clone(),
                                                "data": { "message": bb_log.clone() }
                                            }));

                                            // — Log to trades.json
                                            let bb_action_label = if current_config.trade.check_karma_choppy && prev.is_choppy_combined {
                                                "Idle(KarmaChoppy)"
                                            } else if prev.is_bb_flat_choppy {
                                                "Idle(BBFlatChoppy)"
                                            } else {
                                                "Idle(BBSqueeze)"
                                            };
                                            let now = chrono::Local::now();
                                            let month_folder = format!("{:02}-{:04}", now.month(), now.year() as i32 + 543);
                                            let day_folder = format!("{:02}-{:02}-{:04}", now.day(), now.month(), now.year() as i32 + 543);
                                            let dir_path = format!("tradeData/{}/{}/{}", month_folder, day_folder, state.deriv_symbol);
                                            let _ = std::fs::create_dir_all(&dir_path);
                                            let log_file = format!("{}/trades.json", dir_path);

                                            let mut trade_history: Vec<serde_json::Value> = vec![];
                                            if let Ok(content) = std::fs::read_to_string(&log_file) {
                                                if let Ok(arr) = serde_json::from_str(&content) {
                                                    trade_history = arr;
                                                }
                                            }
                                            trade_history.push(json!({
                                                "serverCode": crate::trade_head::get_server_code(),
                                                "tradeRoundNo": crate::get_or_create_trade_control().total_trade,
                                                "contractId": 0,
                                                "buyId": 0,
                                                "sellId": 0,
                                                "assetCode": state.deriv_symbol.clone(),
                                                "entrySpot": 0.0,
                                                "exitSpot": 0.0,
                                                "DiffSpot": 0.0,
                                                "purchaseTime": 0,
                                                "purchaseTimeDisplay": "-",
                                                "scheduleTradeNo": crate::get_or_create_trade_control().total_trade as i32,
                                                "tradeNo": state.trade_no,
                                                "subTradeno": state.sub_trade_no,
                                                "timeCandle": prev.candletime,
                                                "timeCandleDisplay": prev.candletime_display.clone(),
                                                "sellTime": prev.candletime,
                                                "sellTimeDisplay": "-",
                                                "actualDuration": 0,
                                                "isAnomaly": false,
                                                "thisColor": prev.color.clone(),
                                                "thisAction": bb_action_label,
                                                "targetColor": "-",
                                                "emaShortDirection": prev.ema_short_direction.clone(),
                                                "emaMediumDirection": prev.ema_medium_direction.clone(),
                                                "MoneyTrade": 0.0,
                                                "WinStatus": "Skipped",
                                                "lossCon": state.loss_con,
                                                "ThisProfit": 0.0,
                                                "GrandBalance": state.current_balance,
                                                "gaveUp": false,
                                                "maxLossCon": current_config.trade.max_loss_con,
                                                "tradeStrategy": current_config.trade.suggest_strategy.clone(),
                                                "codeStrategy": bb_action_label,
                                                "is_bb_flat_choppy": prev.is_bb_flat_choppy,
                                                "is_bb_squeeze": prev.is_bb_squeeze,
                                                "bb_bandwidth": prev.bb_bandwidth,
                                                "is_choppy_combined": prev.is_choppy_combined
                                            }));
                                            if let Ok(json_str) = serde_json::to_string_pretty(&trade_history) {
                                                let _ = std::fs::write(&log_file, json_str);
                                            }
                                        } else {
                                        // ═══ Strategy Dispatcher (Step 7-8) ═══
                                        // อ่านค่า suggestStrategy จาก setup.json (default = V1)
                                        let strategy = match current_config.trade.suggest_strategy.as_str() {
                                            "V2"  => SuggestStrategy::V2,
                                            "V3A" => SuggestStrategy::V3A,
                                            "V3B" => SuggestStrategy::V3B,
                                            "V3C" => SuggestStrategy::V3C,
                                            "FTA" => SuggestStrategy::FTA,
                                            "FTB" => SuggestStrategy::FTB,
                                            "PKTrend" => SuggestStrategy::PKTrend,
                                            "PKTrendV5" => SuggestStrategy::PKTrendV5,
                                            "ForecastSequence" | "Forecast" => SuggestStrategy::ForecastSequence,
                                            "PKTrendSelectCaseCode" | "SelectCaseCode" => SuggestStrategy::PKTrendSelectCaseCode,
                                            _     => SuggestStrategy::V1,
                                        };
                                        // ═══ Borrow Signal (Whipsaw) Support ═══
                                        let action_fix_obj = if current_config.trade.borrow_signal {
                                            Some(ActionFixObj {
                                                assetCode: state.deriv_symbol.clone(),
                                                TradeNo: state.trade_no as u32,
                                                methodList: MethodList {
                                                    V1: MethodAction { action: "Idle".to_string(), winstatus: "-".to_string(), winCon: 0, lossCon: 0, BorrowSignalFrom: "-".to_string() },
                                                    V2: MethodAction { action: "Idle".to_string(), winstatus: "-".to_string(), winCon: 0, lossCon: state.loss_con, BorrowSignalFrom: "V2WhipSaw".to_string() },
                                                    V2WhipSaw: MethodAction { action: "Idle".to_string(), winstatus: "-".to_string(), winCon: 0, lossCon: 0, BorrowSignalFrom: "-".to_string() },
                                                    V3A: MethodAction { action: "Idle".to_string(), winstatus: "-".to_string(), winCon: 0, lossCon: 0, BorrowSignalFrom: "-".to_string() },
                                                    V3B: MethodAction { action: "Idle".to_string(), winstatus: "-".to_string(), winCon: 0, lossCon: 0, BorrowSignalFrom: "-".to_string() },
                                                    V3C: MethodAction { action: "Idle".to_string(), winstatus: "-".to_string(), winCon: 0, lossCon: 0, BorrowSignalFrom: "-".to_string() },
                                                    FTA: MethodAction { action: "Idle".to_string(), winstatus: "-".to_string(), winCon: 0, lossCon: 0, BorrowSignalFrom: "-".to_string() },
                                                },
                                            })
                                        } else {
                                            None
                                        };
                                        let suggest_decision =
                                            get_suggest_color_by_strategy_with_reason(&strategy, prev, state.loss_con, action_fix_obj.as_ref());
                                        let suggest = suggest_decision.suggest_color.clone();
                                        let trade_action = match suggest.as_str() {
                                            "green" => TradeAction::Call,
                                            "red" => TradeAction::Put,
                                            _ => TradeAction::Idle,
                                        };

                                        // ═══ Noise Filter Check ═══
                                        let mut noise_config = crate::filter_noise::NoiseFilterConfig {
                                            is_check_noise: "no".to_string(),
                                            alt_color: "no".to_string(),
                                            flat: "no".to_string(),
                                            flat_case: "Case1".to_string(),
                                            gap: "no".to_string(),
                                            // ═══ New Noise Filters (Loss Streak Fix) ═══
                                            check_adx: "no".to_string(),
                                            adx_threshold: 25.0,
                                            check_choppy: "no".to_string(),
                                            choppy_threshold: 50.0,
                                            check_range: "no".to_string(),
                                            check_bb_squeeze: "no".to_string(),
                                            bb_bandwidth_threshold: 0.5,
                                        };
                                        if let Ok(content) = std::fs::read_to_string("setup/thereshold.json") {
                                            if let Ok(root) = serde_json::from_str::<serde_json::Value>(&content) {
                                                if let Some(nc) = root.get("noiseConfig") {
                                                    noise_config.is_check_noise = nc.get("isCheckNoise").and_then(|v| v.as_str()).unwrap_or("no").to_string();
                                                    
                                                    let mut alt = "no";
                                                    let mut flat = "no";
                                                    let mut gap = "no";
                                                    
                                                    if let Some(arr) = nc.get("noiseTypeCheck").and_then(|v| v.as_array()) {
                                                        for v in arr {
                                                            if let Some(s) = v.as_str() {
                                                                if s == "Alternate" { alt = "yes"; }
                                                                if s == "Flat" { flat = "yes"; }
                                                                if s == "Gap" { gap = "yes"; }
                                                            }
                                                        }
                                                    }
                                                    
                                                    noise_config.alt_color = alt.to_string();
                                                    noise_config.flat = flat.to_string();
                                                    noise_config.gap = gap.to_string();
                                                    
                                                    if let Some(fc) = nc.get("flatCondition").and_then(|v| v.as_str()) {
                                                        let fc_capitalized = if fc.len() > 0 {
                                                            let mut c = fc.chars();
                                                            match c.next() {
                                                                None => String::new(),
                                                                Some(f) => f.to_uppercase().chain(c).collect(),
                                                            }
                                                        } else {
                                                            "Case1".to_string()
                                                        };
                                                        noise_config.flat_case = fc_capitalized;
                                                    }

                                                    // ═══ Read New Noise Filters from thereshold.json ═══
                                                    noise_config.check_adx = nc.get("checkAdx").and_then(|v| v.as_str()).unwrap_or("no").to_string();
                                                    noise_config.adx_threshold = nc.get("adxThreshold").and_then(|v| v.as_f64()).unwrap_or(25.0);
                                                    noise_config.check_choppy = nc.get("checkChoppy").and_then(|v| v.as_str()).unwrap_or("no").to_string();
                                                    noise_config.choppy_threshold = nc.get("choppyThreshold").and_then(|v| v.as_f64()).unwrap_or(50.0);
                                                    noise_config.check_range = nc.get("checkRange").and_then(|v| v.as_str()).unwrap_or("no").to_string();
                                                    noise_config.check_bb_squeeze = nc.get("checkBbSqueeze").and_then(|v| v.as_str()).unwrap_or("no").to_string();
                                                    noise_config.bb_bandwidth_threshold = nc.get("bbBandwidthThreshold").and_then(|v| v.as_f64()).unwrap_or(0.5);
                                                }
                                            }
                                        }
                                        let (is_idle, noise_code) = crate::filter_noise::check_noise_filter(prev, &noise_config);

                                        if is_idle {
                                            // ═══ Log Skipped Trade to trades.json ═══
                                            let now = chrono::Local::now();
                                            use chrono::Datelike;
                                            let month_folder = format!("{:02}-{:04}", now.month(), now.year() as i32 + 543);
                                            let day_folder = format!("{:02}-{:02}-{:04}", now.day(), now.month(), now.year() as i32 + 543);
                                            let dir_path = format!("tradeData/{}/{}/{}", month_folder, day_folder, state.deriv_symbol);
                                            let _ = std::fs::create_dir_all(&dir_path);
                                            let log_file = format!("{}/trades.json", dir_path);
                                            
                                            let mut trade_history: Vec<serde_json::Value> = vec![];
                                            if let Ok(content) = std::fs::read_to_string(&log_file) {
                                                if let Ok(arr) = serde_json::from_str(&content) {
                                                    trade_history = arr;
                                                }
                                            }
                                            trade_history.push(json!({
                                                "serverCode": crate::trade_head::get_server_code(),
                                                "tradeRoundNo": crate::get_or_create_trade_control().total_trade,
                                                "contractId": 0,
                                                "buyId": 0,
                                                "sellId": 0,
                                                "assetCode": state.deriv_symbol.clone(),
                                                "entrySpot": 0.0,
                                                "exitSpot": 0.0,
                                                "DiffSpot": 0.0,
                                                "purchaseTime": 0,
                                                "purchaseTimeDisplay": "-",
                                                "scheduleTradeNo": crate::get_or_create_trade_control().total_trade as i32,
                                                "tradeNo": state.trade_no,
                                                "subTradeno": state.sub_trade_no,
                                                "timeCandle": prev.candletime,
                                                "timeCandleDisplay": prev.candletime_display,
                                                "sellTime": prev.candletime,
                                                "sellTimeDisplay": "-",
                                                "actualDuration": 0,
                                                "isAnomaly": false,
                                                "thisColor": prev.color,
                                                "thisAction": "Idle(Noise)",
                                                "targetColor": suggest,
                                                "emaShortDirection": prev.ema_short_direction,
                                                "emaMediumDirection": prev.ema_medium_direction,
                                                "MoneyTrade": 0.0,
                                                "WinStatus": "Skipped",
                                                "lossCon": state.loss_con,
                                                "ThisProfit": 0.0,
                                                "GrandBalance": state.current_balance,
                                                "gaveUp": false,
                                                "maxLossCon": current_config.trade.max_loss_con,
                                                "tradeStrategy": current_config.trade.suggest_strategy,
                                                "codeStrategy": suggest_decision.code.clone(),
                                                "noiseCode": noise_code.clone()
                                            }));
                                            if let Ok(json_str) = serde_json::to_string_pretty(&trade_history) {
                                                let _ = std::fs::write(&log_file, json_str);
                                            }

                                            // ═══ Noise Blocked — แจ้ง Frontend & Log ═══
                                            let noise_log = format!(
                                                "🛑 [{}] NOISE BLOCKED — noiseCode={} | ไม่เปิด order (trade_action=Idle)",
                                                state.asset, noise_code
                                            );
                                            println!("\n{}", noise_log);
                                            let _ = tx.send(json!({
                                                "type": "bot_log",
                                                "asset": state.asset.clone(),
                                                "data": { "message": noise_log }
                                            }));
                                        } else {
                                        // ═══ ไม่มี Noise — ตรวจสอบ Whipsaw Zone ก่อนเทรดปกติ ═══

                                        // เพิ่มสีแท่งเทียนล่าสุดเข้า color_list
                                        state.trade_control_color_list.push(prev.color.clone());

                                        // สร้าง candle symbol (A, B, C, ...)
                                        let candle_idx = state.trade_control_items.len() as u32;
                                        let candle_symbol = (b'A' + (candle_idx % 26) as u8) as char;

                                        // ═══ Whipsaw Zone Detection: เช็คเฉพาะเมื่อ toggle เปิด AND lossCon >= 4 ═══
                                        let is_in_whipsaw = if current_config.trade.whipsaw_zone && state.loss_con >= 4 {
                                            detect_whipsaw_zone(&state.trade_control_color_list)
                                        } else {
                                            false
                                        };

                                        // อัปเดตสถานะ Whipsaw Zone
                                        if is_in_whipsaw && !state.whipsaw_zone_active {
                                            state.whipsaw_zone_active = true;
                                            state.whipsaw_frozen_loss_con = state.loss_con;
                                            println!("\n🎯 [{}] ENTERED Whipsaw Zone! lossCon frozen at {}", state.asset, state.loss_con);
                                        } else if !is_in_whipsaw && state.whipsaw_zone_active {
                                            state.whipsaw_zone_active = false;
                                            println!("\n✅ [{}] EXITED Whipsaw Zone! Resuming trade. lossCon={}", state.asset, state.loss_con);
                                        }

                                        if state.whipsaw_zone_active {
                                            // ═══ Whipsaw IDLE — ข้ามการเทรดในตานี้ ═══
                                            let idle_action_str = if suggest.as_str() == "green" { "CALL" } else if suggest.as_str() == "red" { "PUT" } else { "IDLE" };

                                            // บันทึก TradeControlItem
                                            state.trade_control_items.push(TradeControlItem {
                                                tradeno: state.trade_control_items.len() as u32 + 1,
                                                currentIndex: candle_idx,
                                                candleSymbol: candle_symbol.to_string(),
                                                candleColor: prev.color.clone(),
                                                action: "IDLE".to_string(),
                                                winStatus: "-".to_string(),
                                                lossCon: state.loss_con,
                                                whipsawZone: true,
                                            });

                                            // Log Whipsaw IDLE trade to trades.json
                                            let now_ws = chrono::Local::now();
                                            let mf_ws = format!("{:02}-{:04}", now_ws.month(), now_ws.year() as i32 + 543);
                                            let df_ws = format!("{:02}-{:02}-{:04}", now_ws.day(), now_ws.month(), now_ws.year() as i32 + 543);
                                            let dir_ws = format!("tradeData/{}/{}/{}", mf_ws, df_ws, state.deriv_symbol);
                                            let _ = std::fs::create_dir_all(&dir_ws);
                                            let log_ws = format!("{}/trades.json", dir_ws);

                                            let mut th_ws: Vec<serde_json::Value> = vec![];
                                            if let Ok(content) = std::fs::read_to_string(&log_ws) {
                                                if let Ok(arr) = serde_json::from_str(&content) {
                                                    th_ws = arr;
                                                }
                                            }
                                            th_ws.push(json!({
                                                "serverCode": crate::trade_head::get_server_code(),
                                                "tradeRoundNo": crate::get_or_create_trade_control().total_trade,
                                                "contractId": 0, "buyId": 0, "sellId": 0,
                                                "assetCode": state.deriv_symbol.clone(),
                                                "entrySpot": 0.0, "exitSpot": 0.0, "DiffSpot": 0.0,
                                                "purchaseTime": 0, "purchaseTimeDisplay": "-",
                                                "scheduleTradeNo": crate::get_or_create_trade_control().total_trade as i32,
                                                "tradeNo": state.trade_no, "subTradeno": state.sub_trade_no,
                                                "timeCandle": prev.candletime,
                                                "timeCandleDisplay": prev.candletime_display,
                                                "sellTime": prev.candletime, "sellTimeDisplay": "-",
                                                "actualDuration": 0, "isAnomaly": false,
                                                "thisColor": prev.color,
                                                "thisAction": format!("IDLE(Whipsaw-{})", idle_action_str),
                                                "targetColor": suggest.clone(),
                                                "emaShortDirection": prev.ema_short_direction,
                                                "emaMediumDirection": prev.ema_medium_direction,
                                                "MoneyTrade": 0.0, "WinStatus": "-",
                                                "lossCon": state.loss_con, "ThisProfit": 0.0,
                                                "GrandBalance": state.current_balance,
                                                "gaveUp": false, "maxLossCon": current_config.trade.max_loss_con,
                                                "tradeStrategy": current_config.trade.suggest_strategy,
                                                "codeStrategy": suggest_decision.code.clone(),
                                                "noiseCode": "whipsaw_idle", "whipsawZone": true
                                            }));
                                            if let Ok(json_str) = serde_json::to_string_pretty(&th_ws) {
                                                let _ = std::fs::write(&log_ws, json_str);
                                            }

                                            // แจ้ง Frontend
                                            let color_display: String = state.trade_control_color_list.iter().rev().take(4).collect::<Vec<_>>().into_iter().rev().map(|s| if s == "green" { "G" } else { "R" }).collect::<Vec<_>>().join("-");
                                            let ws_log = format!(
                                                "🎯 [{}] WHIPSAW IDLE — lossCon={} (frozen) | สี: {} | ข้ามเทรดตานี้",
                                                state.asset, state.loss_con, color_display
                                            );
                                            println!("\n{}", ws_log);
                                            let _ = tx.send(json!({
                                                "type": "bot_log",
                                                "asset": state.asset.clone(),
                                                "data": { "message": ws_log }
                                            }));

                                            // Broadcast trade_control_update
                                            let color_list_str: String = state.trade_control_color_list.iter().map(|c| if c == "green" { "G" } else { "R" }).collect::<Vec<_>>().join("-");
                                            let _ = tx.send(json!({
                                                "type": "trade_control_update",
                                                "asset": state.asset.clone(),
                                                "data": {
                                                    "assetCode": state.deriv_symbol.clone(),
                                                    "whipsawZoneActive": true,
                                                    "lossCon": state.loss_con,
                                                    "colorList": color_list_str,
                                                    "tradeList": state.trade_control_items.clone()
                                                }
                                            }));

                                        } else {
                                        // ═══ ไม่อยู่ใน Whipsaw Zone — ดำเนินการเทรดปกติ ═══

                                        // บันทึก TradeControlItem (ไม้ปกติ)
                                        state.trade_control_items.push(TradeControlItem {
                                            tradeno: state.trade_control_items.len() as u32 + 1,
                                            currentIndex: candle_idx,
                                            candleSymbol: candle_symbol.to_string(),
                                            candleColor: prev.color.clone(),
                                            action: if trade_action == TradeAction::Call { "CALL".to_string() } else if trade_action == TradeAction::Put { "PUT".to_string() } else { "IDLE".to_string() },
                                            winStatus: "".to_string(),
                                            lossCon: state.loss_con,
                                            whipsawZone: false,
                                        });

                                        // Broadcast trade_control_update
                                        {
                                            let cl_str: String = state.trade_control_color_list.iter().map(|c| if c == "green" { "G" } else { "R" }).collect::<Vec<_>>().join("-");
                                            let _ = tx.send(json!({
                                                "type": "trade_control_update",
                                                "asset": state.asset.clone(),
                                                "data": {
                                                    "assetCode": state.deriv_symbol.clone(),
                                                    "whipsawZoneActive": state.whipsaw_zone_active,
                                                    "lossCon": state.loss_con,
                                                    "colorList": cl_str,
                                                    "tradeList": state.trade_control_items.clone()
                                                }
                                            }));
                                        }


                                        // ═══ Strategy Comparison: คำนวณ suggest จากทุก strategy ═══
                                        {
                                            state.strategy_runno += 1;
                                            let s_v1_dec  = get_suggest_color_by_strategy_with_reason(&SuggestStrategy::V1,  prev, state.strategy_loss_con_v1, None);
                                            let s_v2_dec  = get_suggest_color_by_strategy_with_reason(&SuggestStrategy::V2,  prev, state.strategy_loss_con_v2, None);
                                            let s_v3a_dec = get_suggest_color_by_strategy_with_reason(&SuggestStrategy::V3A, prev, state.strategy_loss_con_v3a, None);
                                            let s_v3b_dec = get_suggest_color_by_strategy_with_reason(&SuggestStrategy::V3B, prev, state.strategy_loss_con_v3b, None);
                                            let s_v3c_dec = get_suggest_color_by_strategy_with_reason(&SuggestStrategy::V3C, prev, state.strategy_loss_con_v3c, None);
                                            let s_fta_dec = get_suggest_color_by_strategy_with_reason(&SuggestStrategy::FTA, prev, state.strategy_loss_con_fta, None);
                                            let s_ftb_dec = get_suggest_color_by_strategy_with_reason(&SuggestStrategy::FTB, prev, state.strategy_loss_con_ftb, None);
                                            let s_pktrend_dec = get_suggest_color_by_strategy_with_reason(&SuggestStrategy::PKTrend, prev, state.strategy_loss_con_pktrend, None);
                                            let s_pktrend_v5_dec = get_suggest_color_by_strategy_with_reason(&SuggestStrategy::PKTrendV5, prev, state.strategy_loss_con_pktrend_v5, None);
                                            let s_forecast_dec = get_suggest_color_by_strategy_with_reason(&SuggestStrategy::ForecastSequence, prev, state.strategy_loss_con_forecast, None);
                                            let s_pktrend_case_code_dec = get_suggest_color_by_strategy_with_reason(&SuggestStrategy::PKTrendSelectCaseCode, prev, state.strategy_loss_con_pktrend_case_code, None);

                                            let s_v1 = s_v1_dec.suggest_color.clone();
                                            let s_v2 = s_v2_dec.suggest_color.clone();
                                            let s_v3a = s_v3a_dec.suggest_color.clone();
                                            let s_v3b = s_v3b_dec.suggest_color.clone();
                                            let s_v3c = s_v3c_dec.suggest_color.clone();
                                            let s_fta = s_fta_dec.suggest_color.clone();
                                            let s_ftb = s_ftb_dec.suggest_color.clone();
                                            let s_pktrend = s_pktrend_dec.suggest_color.clone();
                                            let s_pktrend_v5 = s_pktrend_v5_dec.suggest_color.clone();
                                            let s_forecast = s_forecast_dec.suggest_color.clone();
                                            let s_pktrend_case_code = s_pktrend_case_code_dec.suggest_color.clone();

                                            let to_action = |s: &str| if s == "green" { "CALL" } else if s == "red" { "PUT" } else { "IDLE" };

                                            state.pending_strategy_entry = Some(json!({
                                                "runno": state.strategy_runno,
                                                "assetCode": state.deriv_symbol.clone(),
                                                "candleTimestamp": prev.candletime,
                                                "candleTimeDisp": prev.candletime_display.clone(),
                                                "thisColor": prev.color.clone(),
                                                "emaShortDir": prev.ema_short_direction.clone(),
                                                "emaMediumDir": prev.ema_medium_direction.clone(),
                                                "emaLongDir": prev.ema_long_direction.clone(),
                                                "isEmaShortCutType": prev.ema_cut_position.clone(),
                                                "activeStrategy": current_config.trade.suggest_strategy.clone(),
                                                "suggestActionByV1": to_action(&s_v1),
                                                "suggestColorByV1": s_v1,
                                                "v1Reason": s_v1_dec.reason.clone(),
                                                "v1Code": s_v1_dec.code.clone(),
                                                "lossConByV1": state.strategy_loss_con_v1,
                                                "suggestActionByV2": to_action(&s_v2),
                                                "suggestColorByV2": s_v2,
                                                "v2Reason": s_v2_dec.reason.clone(),
                                                "v2Code": s_v2_dec.code.clone(),
                                                "lossConByV2": state.strategy_loss_con_v2,
                                                "suggestActionByV3A": to_action(&s_v3a),
                                                "suggestColorByV3A": s_v3a,
                                                "v3aReason": s_v3a_dec.reason.clone(),
                                                "v3aCode": s_v3a_dec.code.clone(),
                                                "lossConByV3A": state.strategy_loss_con_v3a,
                                                "suggestActionByV3B": to_action(&s_v3b),
                                                "suggestColorByV3B": s_v3b,
                                                "v3bReason": s_v3b_dec.reason.clone(),
                                                "v3bCode": s_v3b_dec.code.clone(),
                                                "lossConByV3B": state.strategy_loss_con_v3b,
                                                "suggestActionByV3C": to_action(&s_v3c),
                                                "suggestColorByV3C": s_v3c,
                                                "v3cReason": s_v3c_dec.reason.clone(),
                                                "v3cCode": s_v3c_dec.code.clone(),
                                                "lossConByV3C": state.strategy_loss_con_v3c,
                                                "suggestActionByFTA": to_action(&s_fta),
                                                "suggestColorByFTA": s_fta,
                                                "ftaReason": s_fta_dec.reason.clone(),
                                                "ftaCode": s_fta_dec.code.clone(),
                                                "lossConByFTA": state.strategy_loss_con_fta,
                                                "suggestActionByFTB": to_action(&s_ftb),
                                                "suggestColorByFTB": s_ftb,
                                                "ftbReason": s_ftb_dec.reason.clone(),
                                                "ftbCode": s_ftb_dec.code.clone(),
                                                "lossConByFTB": state.strategy_loss_con_ftb,
                                                "suggestActionByPKTrend": to_action(&s_pktrend),
                                                "suggestColorByPKTrend": s_pktrend,
                                                "pktrendReason": s_pktrend_dec.reason.clone(),
                                                "pktrendCode": s_pktrend_dec.code.clone(),
                                                "lossConByPKTrend": state.strategy_loss_con_pktrend,
                                                "suggestActionByPKTrendV5": to_action(&s_pktrend_v5),
                                                "suggestColorByPKTrendV5": s_pktrend_v5,
                                                "pktrendV5Reason": s_pktrend_v5_dec.reason.clone(),
                                                "pktrendV5Code": s_pktrend_v5_dec.code.clone(),
                                                "lossConByPKTrendV5": state.strategy_loss_con_pktrend_v5,
                                                "suggestActionByForecast": to_action(&s_forecast),
                                                "suggestColorByForecast": s_forecast,
                                                "forecastReason": s_forecast_dec.reason.clone(),
                                                "forecastCode": s_forecast_dec.code.clone(),
                                                "lossConByForecast": state.strategy_loss_con_forecast,
                                                "suggestActionByPKTrendCaseCode": to_action(&s_pktrend_case_code),
                                                "suggestColorByPKTrendCaseCode": s_pktrend_case_code,
                                                "pktrendCaseCodeReason": s_pktrend_case_code_dec.reason.clone(),
                                                "pktrendCaseCodeCode": s_pktrend_case_code_dec.code.clone(),
                                                "lossConByPKTrendCaseCode": state.strategy_loss_con_pktrend_case_code
                                            }));
                                            state.pending_analysis_obj = Some(latest_analysis.clone());
                                        }

                                        // ═══ Guard: ถ้า Idle → ข้ามการยิงออเดอร์ (Freeze Martingale Step) ═══
                                        if trade_action == TradeAction::Idle {
                                            // บันทึกลง trades.json ว่าข้ามเทรด
                                            let now_idle = chrono::Local::now();
                                            use chrono::Datelike;
                                            let mf_idle = format!("{:02}-{:04}", now_idle.month(), now_idle.year() as i32 + 543);
                                            let df_idle = format!("{:02}-{:02}-{:04}", now_idle.day(), now_idle.month(), now_idle.year() as i32 + 543);
                                            let dir_idle = format!("tradeData/{}/{}/{}", mf_idle, df_idle, state.deriv_symbol);
                                            let _ = std::fs::create_dir_all(&dir_idle);
                                            let log_idle = format!("{}/trades.json", dir_idle);

                                            let mut th_idle: Vec<serde_json::Value> = vec![];
                                            if let Ok(content) = std::fs::read_to_string(&log_idle) {
                                                if let Ok(arr) = serde_json::from_str(&content) {
                                                    th_idle = arr;
                                                }
                                            }
                                            th_idle.push(json!({
                                                "serverCode": crate::trade_head::get_server_code(),
                                                "tradeRoundNo": crate::get_or_create_trade_control().total_trade,
                                                "contractId": 0, "buyId": 0, "sellId": 0,
                                                "assetCode": state.deriv_symbol.clone(),
                                                "entrySpot": 0.0, "exitSpot": 0.0, "DiffSpot": 0.0,
                                                "purchaseTime": 0, "purchaseTimeDisplay": "-",
                                                "scheduleTradeNo": crate::get_or_create_trade_control().total_trade as i32,
                                                "tradeNo": state.trade_no, "subTradeno": state.sub_trade_no,
                                                "timeCandle": prev.candletime,
                                                "timeCandleDisplay": prev.candletime_display,
                                                "sellTime": prev.candletime, "sellTimeDisplay": "-",
                                                "actualDuration": 0, "isAnomaly": false,
                                                "thisColor": prev.color,
                                                "thisAction": "Idle",
                                                "targetColor": suggest.clone(),
                                                "emaShortDirection": prev.ema_short_direction,
                                                "emaMediumDirection": prev.ema_medium_direction,
                                                "MoneyTrade": 0.0, "WinStatus": "Skipped",
                                                "lossCon": state.loss_con, "ThisProfit": 0.0,
                                                "GrandBalance": state.current_balance,
                                                "gaveUp": false, "maxLossCon": current_config.trade.max_loss_con,
                                                "tradeStrategy": current_config.trade.suggest_strategy,
                                                "codeStrategy": suggest_decision.code.clone(),
                                                "noiseCode": "idle_signal"
                                            }));
                                            if let Ok(json_str) = serde_json::to_string_pretty(&th_idle) {
                                                let _ = std::fs::write(&log_idle, json_str);
                                            }

                                            // แจ้ง Frontend + Log
                                            let idle_log = format!(
                                                "⏸️ [{}] IDLE — สัญญาณไม่ชัดเจน ข้ามเทรดตานี้ | lossCon={} (Freeze Martingale)",
                                                state.asset, state.loss_con
                                            );
                                            println!("\n{}", idle_log);
                                            let _ = tx.send(json!({
                                                "type": "bot_log",
                                                "asset": state.asset.clone(),
                                                "data": { "message": idle_log }
                                            }));

                                            // ไม่เปลี่ยน loss_con → Freeze Martingale Step
                                            // ไม่ set is_trading = true → รอสัญญาณแท่งถัดไป
                                        } else {
                                        // ═══ สัญญาณ CALL/PUT ชัดเจน — ดำเนินการเทรดปกติ ═══
                                        println!(
                                            "\n📈 [{}] SIGNAL: {:?} | LossCon: {}",
                                            state.asset, trade_action, state.loss_con
                                        );
                                        state.is_trading = true;
                                        state.last_trade_epoch = latest_analysis.candletime;
                                        state.trade_buy_instant = Some(std::time::Instant::now());
                                        state.active_contract_id = None;
                                        state.last_processed_contract_id = None;

                                        let _ = tx.send(json!({
                                            "type": "trade_start",
                                            "asset": state.asset.clone(),
                                            "data": {
                                                "loss_con": state.loss_con
                                            }
                                        }));

                                        let mut amount = current_config.trade.target_lot;
                                        if current_config.trade.martingale.martingale_type
                                            == "martingale"
                                        {
                                            if let Some(m_amount) = current_config
                                                .trade
                                                .martingale
                                                .list
                                                .get(state.loss_con as usize)
                                            {
                                                amount = *m_amount;
                                            } else if let Some(last_m) =
                                                current_config.trade.martingale.list.last()
                                            {
                                                amount = *last_m;
                                            }
                                        }

                                        let contract_type = match trade_action {
                                            TradeAction::Call => "CALL",
                                            TradeAction::Put => "PUT",
                                            TradeAction::Idle => unreachable!("Idle should be handled before reaching this point"),
                                        };

                                        state.last_trade_details = Some(json!({
                                            "timeCandle": latest_analysis.candletime,
                                            "timeCandleDisplay": chrono::DateTime::from_timestamp(latest_analysis.candletime, 0).unwrap().with_timezone(&Local).format("%d/%m/%Y %H:%M:%S").to_string(),
                                            "thisColor": prev.color.clone(),
                                            "thisAction": format!("{:?}", trade_action),
                                            "targetColor": match trade_action { TradeAction::Call => "green", TradeAction::Put => "red", TradeAction::Idle => unreachable!() },
                                            "emaShortDirection": prev.ema_short_direction,
                                            "emaMediumDirection": prev.ema_medium_direction,
                                            "MoneyTrade": amount,
                                            "codeStrategy": suggest_decision.code.clone(),
                                            "noiseCode": noise_code,
                                        }));

                                        // ═══ อัปเดต duration จาก granularitySettings (Settings Modal) ═══
                                        let trade_granularity = current_config.granularity_settings;
                                        let (new_duration, new_duration_unit) = get_duration_params(trade_granularity);
                                        if state.duration != new_duration || state.duration_unit != new_duration_unit {
                                            println!(
                                                "🔄 [{}] อัปเดต duration: {}{}  →  {}{} (granularitySettings={})",
                                                state.asset, state.duration, state.duration_unit,
                                                new_duration, new_duration_unit, trade_granularity
                                            );
                                            state.duration = new_duration;
                                            state.duration_unit = new_duration_unit;
                                        }

                                        // ═══ Buy Method: เลือกระหว่าง direct vs proposal ═══
                                        let buy_method = current_config.trade.buy_method.as_str();

                                        if buy_method == "proposal" {
                                            // ── Method 2: Dual Proposal → Buy ──
                                            // ส่ง proposal request ทั้ง CALL และ PUT
                                            state.target_trade_action = Some(contract_type.to_string());
                                            
                                            // clear previous pending if any
                                            state.pending_proposal_call = None;
                                            state.pending_proposal_put = None;

                                            // ใช้ req_id+1 สำหรับ proposal CALL
                                            let proposal_req_id_call = state.req_id + 1;
                                            let proposal_request_call = json!({
                                                "proposal": 1,
                                                "amount": amount,
                                                "basis": "stake",
                                                "contract_type": "CALL",
                                                "currency": "USD",
                                                "duration": state.duration,
                                                "duration_unit": state.duration_unit,
                                                "underlying_symbol": state.deriv_symbol,
                                                "req_id": proposal_req_id_call
                                            });

                                            // ใช้ req_id+4 สำหรับ proposal PUT
                                            let proposal_req_id_put = state.req_id + 4;
                                            let proposal_request_put = json!({
                                                "proposal": 1,
                                                "amount": amount,
                                                "basis": "stake",
                                                "contract_type": "PUT",
                                                "currency": "USD",
                                                "duration": state.duration,
                                                "duration_unit": state.duration_unit,
                                                "underlying_symbol": state.deriv_symbol,
                                                "req_id": proposal_req_id_put
                                            });

                                            println!(
                                                "📋 [{}] [Method 2] ส่ง Dual Proposal requests (req_id CALL={}, PUT={})",
                                                state.asset, proposal_req_id_call, proposal_req_id_put
                                            );
                                            private_write
                                                .send(Message::Text(proposal_request_call.to_string()))
                                                .await?;
                                            private_write
                                                .send(Message::Text(proposal_request_put.to_string()))
                                                .await?;
                                        } else {
                                            // ── Method 1: Direct Buy (วิธีเดิม) ──
                                            // ใช้ req_id+2 สำหรับ buy (แยกจาก ohlc)
                                            let buy_req_id = state.req_id + 2;
                                            let buy_request = json!({
                                                "buy": 1,
                                                "price": 100000,
                                                "parameters": {
                                                    "amount": amount,
                                                    "basis": "stake",
                                                    "contract_type": contract_type,
                                                    "currency": "USD",
                                                    "duration": state.duration,
                                                    "duration_unit": state.duration_unit,
                                                    "underlying_symbol": state.deriv_symbol
                                                },
                                                "req_id": buy_req_id
                                            });

                                            println!(
                                                "🚀 [{}] [Method 1] ยิงคำสั่งเทรด Direct Buy (req_id={}): {:?}",
                                                state.asset, buy_req_id, buy_request
                                            );
                                            private_write
                                                .send(Message::Text(buy_request.to_string()))
                                                .await?;
                                        }
                                        } // ← ปิด else ของ whipsaw_zone_active
                                        } // ← ปิด else ของ if trade_action == Idle
                                        } // ← ปิด else ของ if is_idle (Noise Filter)
                                        } // ← ปิด else ของ BB Pre-Filter (bb_block_reason)
                                    }
                                    } // ← ปิด else ของ MANUAL check
                                }
                            }

                            state.last_tick_analysis = Some(analysis_obj.clone());
                        }
                    }
                }
            }

            // ═══ Handle Proposal Response (Method 2: Dual Proposal → Buy) ═══
            if msg_type == "proposal" {
                if let Some(proposal_data) = parsed.get("proposal") {
                    let req_id = parsed
                        .get("echo_req")
                        .and_then(|r| r.get("req_id"))
                        .and_then(|r| r.as_i64())
                        .unwrap_or(0);
                    let asset_key = req_id_to_asset.get(&req_id).cloned().unwrap_or_default();

                    let is_call_req = req_id % 1000 == 1;
                    let is_put_req = req_id % 1000 == 4;

                    println!(
                        "📬 [Proposal] req_id={} → asset_key='{}' | CALL: {}, PUT: {}",
                        req_id, asset_key, is_call_req, is_put_req
                    );

                    if let Some(state) = asset_states.get_mut(&asset_key) {
                        // เช็คว่ากำลังอยู่ในช่วง trade และยังมี target action ค้างอยู่
                        if state.is_trading && state.target_trade_action.is_some() {
                            let proposal_id = proposal_data
                                .get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let ask_price = proposal_data
                                .get("ask_price")
                                .and_then(|v| json_to_f64(v))
                                .unwrap_or(0.0);
                            let payout = proposal_data
                                .get("payout")
                                .and_then(|v| json_to_f64(v))
                                .unwrap_or(0.0);
                            let spot = proposal_data
                                .get("spot")
                                .and_then(|v| json_to_f64(v))
                                .unwrap_or(0.0);

                            println!(
                                "\n📋 [{}] [Method 2] ได้ Proposal (req={})! id={} | ask={:.2} | payout={:.2} | spot={:.5}",
                                state.asset, req_id, proposal_id, ask_price, payout, spot
                            );

                            if proposal_id.is_empty() {
                                eprintln!("❌ [{}] Proposal ID ว่าง — ยกเลิกการ buy", state.asset);
                                state.is_trading = false;
                                state.trade_buy_instant = None;
                                state.pending_proposal_call = None;
                                state.pending_proposal_put = None;
                                state.target_trade_action = None;
                                state.last_trade_details = None;
                            } else {
                                // เก็บลง pending ที่ตรงกัน
                                if is_call_req {
                                    state.pending_proposal_call = Some(proposal_data.clone());
                                    state.last_spot_call = Some(spot);
                                } else if is_put_req {
                                    state.pending_proposal_put = Some(proposal_data.clone());
                                    state.last_spot_put = Some(spot);
                                }

                                // ═══ ตรวจสอบว่าได้ครบทั้ง 2 proposal หรือยัง ═══
                                if state.pending_proposal_call.is_some() && state.pending_proposal_put.is_some() {
                                    if let Some(target_action) = state.target_trade_action.clone() {
                                        // เลือก proposal ที่ตรงกับ action
                                        let selected_proposal = if target_action == "CALL" {
                                            state.pending_proposal_call.as_ref().unwrap()
                                        } else {
                                            state.pending_proposal_put.as_ref().unwrap()
                                        };

                                        let sel_proposal_id = selected_proposal.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                        let sel_ask_price = selected_proposal.get("ask_price").and_then(|v| json_to_f64(v)).unwrap_or(0.0);

                                        // เคลียร์ pending ทันที
                                        state.pending_proposal_call = None;
                                        state.pending_proposal_put = None;

                                        let buy_req_id = state.req_id + 2;
                                        let buy_request = json!({
                                            "buy": sel_proposal_id,
                                            "price": sel_ask_price,
                                            "req_id": buy_req_id
                                        });

                                        println!(
                                            "🚀 [{}] [Method 2] ส่ง Buy (Action={}) ด้วย Proposal ID: {} (req_id={})",
                                            state.asset, target_action, sel_proposal_id, buy_req_id
                                        );
                                        private_write
                                            .send(Message::Text(buy_request.to_string()))
                                            .await?;
                                    }
                                }
                            }
                        } else if state.is_trading && state.target_trade_action.is_none() {
                            // Proposal streaming update มาหลัง buy — skip
                            println!(
                                "⏭️ [{}] [Method 2] Skip proposal update (buy ถูกส่งไปแล้ว หรือไม่ใช่ request คู่)",
                                state.asset
                            );
                        }
                    }
                }
            }

            // ═══ Handle Buy Response ═══
            // เหมือน JS: case "buy" → trackOrder(data.buy.contract_id)
            if let Some(buy_response) = parsed.get("buy") {
                let req_id = parsed
                    .get("echo_req")
                    .and_then(|r| r.get("req_id"))
                    .and_then(|r| r.as_i64())
                    .unwrap_or(0);
                let asset_key = req_id_to_asset.get(&req_id).cloned().unwrap_or_default();
                println!(
                    "📬 [Buy] req_id={} → asset_key='{}'",
                    req_id, asset_key
                );

                if let Some(state) = asset_states.get_mut(&asset_key) {
                    if let Some(contract_id) = buy_response.get("contract_id") {
                        let cid_num = json_to_i64(contract_id).unwrap_or(0);
                        state.active_contract_id = Some(cid_num);
                        println!(
                            "\n✅ [{}] เข้าออเดอร์สำเร็จ Contract ID: {}",
                            state.asset, cid_num
                        );

                        // ═══ Log ORDER OPENED to tracker file ═══
                        {
                            let now = Local::now();
                            let mf = format!("{:02}-{:04}", now.month(), now.year() as i32 + 543);
                            let df = format!("{:02}-{:02}-{:04}", now.day(), now.month(), now.year() as i32 + 543);
                            let dir = format!("tradeData/{}/{}", mf, df);
                            let _ = fs::create_dir_all(&dir);
                            let log_path = format!("{}/order_tracker.log", dir);
                            let buy_price = buy_response.get("buy_price").and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                            let payout_val = buy_response.get("payout").and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                            // ดึง contract_type: ถ้า Method 1 จะอยู่ใน echo_req.parameters, ถ้า Method 2 จะอยู่ใน target_trade_action
                            let ctype = parsed.get("echo_req")
                                .and_then(|r| r.get("parameters"))
                                .and_then(|p| p.get("contract_type"))
                                .and_then(|c| c.as_str())
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| {
                                    state.target_trade_action.clone()
                                        .unwrap_or("?".to_string())
                                });
                            // Clear target_trade_action หลัง buy สำเร็จ
                            state.target_trade_action = None;
                            let line = format!(
                                "[{}] 🟢 OPENED | {:>8} | CID: {:>12} | {:>4} | Stake: ${:.2} | Payout: ${:.2} | LossCon: {}\n",
                                now.format("%H:%M:%S"), state.deriv_symbol, cid_num, ctype, buy_price, payout_val, state.loss_con
                            );
                            if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&log_path) {
                                let _ = write!(f, "{}", line);
                            }
                        }

                        if let Some(bal) = buy_response
                            .get("balance_after")
                            .and_then(|b| json_to_f64(b))
                        {
                            state.current_balance = bal;
                        }

                        // ═══ Track Order: เหมือน JS: trackOrder(contractId) ═══
                        // ใช้ req_id+3 สำหรับ proposal_open_contract (track)
                        let track_req_id = state.req_id + 3;
                        let sub_proposal = json!({
                            "proposal_open_contract": 1,
                            "contract_id": cid_num,
                            "subscribe": 1,
                            "req_id": track_req_id
                        });
                        private_write
                            .send(Message::Text(sub_proposal.to_string()))
                            .await?;
                    } else {
                        state.is_trading = false;
                        state.trade_buy_instant = None;
                    }
                }
            }

            if let Some(tx_data) = parsed.get("transaction") {
                let tx_balance = tx_data
                    .get("balance")
                    .and_then(|v| json_to_f64(v))
                    .unwrap_or(0.0);

                if tx_balance > 0.0 {
                    for (_, s) in asset_states.iter_mut() {
                        s.current_balance = tx_balance;
                    }
                    let _ = tx.send(json!({
                        "type": "balance_update",
                        "asset": "system",
                        "data": { "balance": tx_balance }
                    }));
                }

                if let Some(action) = tx_data.get("action").and_then(|a| a.as_str()) {
                    let tx_contract_id = tx_data
                        .get("contract_id")
                        .and_then(|v| json_to_i64(v))
                        .unwrap_or(0);

                    if action == "sell" {
                        let asset_key = find_asset_by_contract_id(&asset_states, tx_contract_id);
                        if let Some(state) = asset_states.get_mut(&asset_key) {
                            println!(
                                "\n🔔 [{}] TRANSACTION SELL detected! contract_id={}",
                                state.asset, tx_contract_id
                            );
                            if state.is_trading {
                                let track_req_id = state.req_id + 3;
                                let query = json!({
                                    "proposal_open_contract": 1,
                                    "contract_id": tx_contract_id,
                                    "req_id": track_req_id
                                });
                                private_write.send(Message::Text(query.to_string())).await?;
                            }
                        }
                    }
                }
            }

            if let Some(proposal) = parsed.get("proposal_open_contract") {
                if proposal.is_object() {
                    let req_id = parsed
                        .get("echo_req")
                        .and_then(|r| r.get("req_id"))
                        .and_then(|r| r.as_i64())
                        .unwrap_or(0);
                    let mut asset_key = req_id_to_asset.get(&req_id).cloned().unwrap_or_default();

                    // Fallback: ถ้า req_id ไม่ match → ใช้ contract_id หา asset
                    if asset_key.is_empty() {
                        let cid = proposal
                            .get("contract_id")
                            .and_then(|v| json_to_i64(v))
                            .unwrap_or(0);
                        asset_key = find_asset_by_contract_id(&asset_states, cid);
                    }

                    if let Some(state) = asset_states.get_mut(&asset_key) {
                        state.track_order_list.push(proposal.clone());
                        let is_sold_val = match proposal.get("is_sold") {
                            Some(v) => {
                                if let Some(i) = json_to_i64(v) {
                                    i
                                } else if let Some(b) = v.as_bool() {
                                    if b { 1 } else { 0 }
                                } else {
                                    0
                                }
                            }
                            None => 0,
                        };
                        let current_profit = proposal
                            .get("profit")
                            .and_then(|v| json_to_f64(v))
                            .unwrap_or(0.0);
                        let contract_status = proposal
                            .get("status")
                            .and_then(|s| s.as_str())
                            .unwrap_or("unknown");

                        let is_contract_ended = is_sold_val == 1
                            || contract_status == "won"
                            || contract_status == "lost"
                            || contract_status == "sold";

                        // ═══ Broadcast order_update ให้ Frontend (Order Tracker Tab) ═══
                        {
                            let oc_cid = proposal.get("contract_id").and_then(|v| json_to_i64(v)).unwrap_or(0);
                            let oc_buy = proposal.get("buy_price").and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                            let oc_sell = proposal.get("sell_price").and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                            let oc_payout = proposal.get("payout").and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                            let oc_ctype = proposal.get("contract_type").and_then(|v| v.as_str()).unwrap_or("");
                            let oc_ptime = proposal.get("purchase_time").and_then(|v| json_to_i64(v)).unwrap_or(0);
                            let oc_expiry = proposal.get("date_expiry").and_then(|v| json_to_i64(v)).unwrap_or(0);
                            let oc_entry_spot = proposal.get("entry_spot").or_else(|| proposal.get("entry_tick")).and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                            
                            let _ = tx.send(json!({
                                "type": "order_update",
                                "asset": state.asset.clone(),
                                "data": {
                                    "contract_id": oc_cid,
                                    "underlying_symbol": state.deriv_symbol.clone(),
                                    "asset": state.asset.clone(),
                                    "contract_type": oc_ctype,
                                    "buy_price": oc_buy,
                                    "sell_price": oc_sell,
                                    "profit": current_profit,
                                    "payout": oc_payout,
                                    "status": contract_status,
                                    "is_sold": is_sold_val,
                                    "purchase_time": oc_ptime,
                                    "date_expiry": oc_expiry,
                                    "entry_spot": oc_entry_spot
                                }
                            }));
                        }

                        if is_contract_ended {
                            let contract_id_str = proposal
                                .get("contract_id")
                                .map(|v| v.to_string())
                                .unwrap_or_default();

                            if state.last_processed_contract_id.as_deref()
                                == Some(contract_id_str.as_str())
                            {
                                continue;
                            }
                            state.last_processed_contract_id = Some(contract_id_str);

                            let profit = current_profit;
                            println!(
                                "\n🏁 [{}] ไม้จบ! สถานะ: {} | Profit: {:.2}",
                                state.asset, contract_status, profit
                            );

                            let (
                                time_candle,
                                time_candle_disp,
                                this_col,
                                this_act,
                                tar_col,
                                e_s_dir,
                                e_m_dir,
                                money_tr,
                                code_strategy,
                                noise_code,
                            ) = if let Some(ref details) = state.last_trade_details {
                                (
                                    details["timeCandle"].as_i64().unwrap_or(0),
                                    details["timeCandleDisplay"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_string(),
                                    details["thisColor"].as_str().unwrap_or("").to_string(),
                                    details["thisAction"].as_str().unwrap_or("").to_string(),
                                    details["targetColor"].as_str().unwrap_or("").to_string(),
                                    details["emaShortDirection"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_string(),
                                    details["emaMediumDirection"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_string(),
                                    details["MoneyTrade"].as_f64().unwrap_or(0.0),
                                    details["codeStrategy"].as_str().unwrap_or("").to_string(),
                                    details["noiseCode"].as_str().unwrap_or("none").to_string(),
                                )
                            } else {
                                (
                                    0,
                                    String::new(),
                                    String::new(),
                                    String::new(),
                                    String::new(),
                                    String::new(),
                                    String::new(),
                                    0.0,
                                    String::new(),
                                    String::new(),
                                )
                            };

                            let purchase_time = proposal
                                .get("purchase_time")
                                .and_then(|v| json_to_i64(v))
                                .unwrap_or(0);
                            let sell_time = proposal
                                .get("sell_time")
                                .and_then(|v| json_to_i64(v))
                                .unwrap_or(0);
                            let sell_time_display = if sell_time > 0 {
                                chrono::DateTime::from_timestamp(sell_time, 0)
                                    .unwrap()
                                    .with_timezone(&Local)
                                    .format("%d/%m/%Y %H:%M:%S")
                                    .to_string()
                            } else {
                                String::new()
                            };

                            let expected_duration_sec = match state.duration_unit.as_str() {
                                "s" => state.duration as i64,
                                "m" => (state.duration * 60) as i64,
                                "h" => (state.duration * 3600) as i64,
                                "d" => (state.duration * 86400) as i64,
                                _ => (state.duration * 60) as i64,
                            };

                            let mut is_anomaly = false;
                            let mut actual_duration = 0;
                            if purchase_time > 0 && sell_time > 0 {
                                actual_duration = sell_time - purchase_time;
                                if actual_duration > expected_duration_sec + 30 {
                                    is_anomaly = true;
                                }
                            }

                            let win_status = if profit > 0.0 { "Win" } else { "Loss" };

                            // ═══ Pre-compute gaveUp: ตรวจว่าไม้นี้จะทำให้ถึง MaxLossCon หรือไม่ ═══
                            let max_lc = current_config.trade.max_loss_con;
                            let gave_up = profit <= 0.0 && max_lc > 0 && (state.loss_con + 1) >= max_lc;

                            let oc_cid = proposal.get("contract_id").and_then(|v| json_to_i64(v)).unwrap_or(0);
                            let entry_spot = proposal.get("entry_spot").or_else(|| proposal.get("entry_tick")).and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                            let exit_spot = proposal.get("sell_spot").or_else(|| proposal.get("exit_tick")).or_else(|| proposal.get("current_spot")).and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                            let diff_spot = exit_spot - entry_spot;
                            
                            let purchase_time_display = if purchase_time > 0 {
                                chrono::DateTime::from_timestamp(purchase_time, 0)
                                    .unwrap()
                                    .with_timezone(&Local)
                                    .format("%d/%m/%Y %H:%M:%S")
                                    .to_string()
                            } else {
                                String::new()
                            };

                            let buy_id = proposal.get("transaction_ids").and_then(|t| t.get("buy")).and_then(|v| json_to_i64(v)).unwrap_or(0);
                            let sell_id = proposal.get("transaction_ids").and_then(|t| t.get("sell")).and_then(|v| json_to_i64(v)).unwrap_or(0);

                            let prev_candle = state.current_history.iter().rev().nth(1);
                            let (spot_call_code, spot_put_code) = if let Some(c) = prev_candle {
                                let c_price = state.last_spot_call.unwrap_or(0.0);
                                let p_price = state.last_spot_put.unwrap_or(0.0);
                                (
                                    get_spot_position_code(c_price, c.open, c.high, c.low, c.close),
                                    get_spot_position_code(p_price, c.open, c.high, c.low, c.close)
                                )
                            } else {
                                ("".to_string(), "".to_string())
                            };

                            let mut trade_record = json!({
                                "serverCode": crate::trade_head::get_server_code(),
                                "tradeRoundNo": crate::get_or_create_trade_control().total_trade,
                                "contractId": oc_cid,
                                "buyId": buy_id,
                                "sellId": sell_id,
                                "assetCode": state.deriv_symbol.clone(),
                                "entrySpot": entry_spot,
                                "exitSpot": exit_spot,
                                "DiffSpot": diff_spot,
                                "purchaseTime": purchase_time,
                                "purchaseTimeDisplay": purchase_time_display,
                                "scheduleTradeNo": crate::get_or_create_trade_control().total_trade as i32,
                                "tradeNo": state.trade_no,
                                "subTradeno": state.sub_trade_no,
                                "timeCandle": time_candle,
                                "timeCandleDisplay": time_candle_disp,
                                "sellTime": sell_time,
                                "sellTimeDisplay": sell_time_display,
                                "actualDuration": actual_duration,
                                "isAnomaly": is_anomaly,
                                "thisColor": this_col,
                                "thisAction": this_act,
                                "targetColor": tar_col,
                                "emaShortDirection": e_s_dir,
                                "emaMediumDirection": e_m_dir,
                                "MoneyTrade": money_tr,
                                "WinStatus": win_status,
                                "lossCon": state.loss_con,
                                "ThisProfit": profit,
                                "GrandBalance": state.current_balance + profit,
                                "gaveUp": gave_up,
                                "maxLossCon": max_lc,
                                "tradeStrategy": current_config.trade.suggest_strategy,
                                "codeStrategy": code_strategy,
                                "noiseCode": noise_code,
                                "spotPriceCall": state.last_spot_call.unwrap_or(0.0),
                                "spotPricePut": state.last_spot_put.unwrap_or(0.0),
                                "spotCallPositionCode": spot_call_code,
                                "spotPutPositionCode": spot_put_code
                            });

                            if current_config.trade.save_track_order {
                                if let Some(obj) = trade_record.as_object_mut() {
                                    obj.insert("trackOrderList".to_string(), json!(state.track_order_list));
                                }
                            }
                            // Always clear it after trade finishes
                            state.track_order_list.clear();

                            let now = Local::now();
                            let month_folder =
                                format!("{:02}-{:04}", now.month(), now.year() as i32 + 543);
                            let day_folder = format!(
                                "{:02}-{:02}-{:04}",
                                now.day(),
                                now.month(),
                                now.year() as i32 + 543
                            );
                            let dir_path = format!(
                                "tradeData/{}/{}/{}",
                                month_folder, day_folder, state.deriv_symbol
                            );
                            let _ = fs::create_dir_all(&dir_path);

                            let log_file = format!("{}/trades.json", dir_path);
                            let mut trade_history: Vec<serde_json::Value> = vec![];
                            if let Ok(content) = fs::read_to_string(&log_file) {
                                if let Ok(arr) = serde_json::from_str(&content) {
                                    trade_history = arr;
                                }
                            }
                            trade_history.push(trade_record);
                            if let Ok(json_str) = serde_json::to_string_pretty(&trade_history) {
                                let _ = fs::write(&log_file, json_str);
                            }

                            // ═══ Strategy Comparison: บันทึกผลลัพธ์เปรียบเทียบทุก strategy ═══
                            if let Some(mut entry) = state.pending_strategy_entry.take() {
                                // หาสีแท่งเทียนจริงจาก contract result
                                // CALL + Win = green, CALL + Loss = red
                                // PUT + Win = red, PUT + Loss = green
                                let actual_contract_type = proposal
                                    .get("contract_type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let actual_next_color = match (actual_contract_type, profit > 0.0) {
                                    ("CALL", true)  | ("PUT", false) => "green",
                                    ("PUT", true)   | ("CALL", false) => "red",
                                    _ => "unknown",
                                };

                                // ตรวจ Win/Loss สำหรับแต่ละ strategy
                                let check_win = |suggest_color: &str| -> &str {
                                    if suggest_color == "idle" { return "Idle"; }
                                    if actual_next_color == "unknown" { return "Unknown"; }
                                    if suggest_color == actual_next_color { "Win" } else { "Loss" }
                                };

                                let sc_v1  = entry["suggestColorByV1"].as_str().unwrap_or("").to_string();
                                let sc_v2  = entry["suggestColorByV2"].as_str().unwrap_or("").to_string();
                                let sc_v3a = entry["suggestColorByV3A"].as_str().unwrap_or("").to_string();
                                let sc_v3b = entry["suggestColorByV3B"].as_str().unwrap_or("").to_string();
                                let sc_v3c = entry["suggestColorByV3C"].as_str().unwrap_or("").to_string();
                                let sc_fta = entry["suggestColorByFTA"].as_str().unwrap_or("").to_string();
                                let sc_ftb = entry["suggestColorByFTB"].as_str().unwrap_or("").to_string();
                                let sc_pktrend = entry["suggestColorByPKTrend"].as_str().unwrap_or("").to_string();
                                let sc_pktrend_v5 = entry["suggestColorByPKTrendV5"].as_str().unwrap_or("").to_string();
                                let sc_forecast = entry["suggestColorByForecast"].as_str().unwrap_or("").to_string();
                                let sc_pktrend_case_code = entry["suggestColorByPKTrendCaseCode"].as_str().unwrap_or("").to_string();

                                let win_v1  = check_win(&sc_v1);
                                let win_v2  = check_win(&sc_v2);
                                let win_v3a = check_win(&sc_v3a);
                                let win_v3b = check_win(&sc_v3b);
                                let win_v3c = check_win(&sc_v3c);
                                let win_fta = check_win(&sc_fta);
                                let win_ftb = check_win(&sc_ftb);
                                let win_pktrend = check_win(&sc_pktrend);
                                let win_pktrend_v5 = check_win(&sc_pktrend_v5);
                                let win_forecast = check_win(&sc_forecast);
                                let win_pktrend_case_code = check_win(&sc_pktrend_case_code);

                                // อัปเดต per-strategy loss_con
                                if win_v1 == "Win" { state.strategy_loss_con_v1 = 0; } else if win_v1 == "Loss" { state.strategy_loss_con_v1 += 1; }
                                if win_v2 == "Win" { state.strategy_loss_con_v2 = 0; } else if win_v2 == "Loss" { state.strategy_loss_con_v2 += 1; }
                                if win_v3a == "Win" { state.strategy_loss_con_v3a = 0; } else if win_v3a == "Loss" { state.strategy_loss_con_v3a += 1; }
                                if win_v3b == "Win" { state.strategy_loss_con_v3b = 0; } else if win_v3b == "Loss" { state.strategy_loss_con_v3b += 1; }
                                if win_v3c == "Win" { state.strategy_loss_con_v3c = 0; } else if win_v3c == "Loss" { state.strategy_loss_con_v3c += 1; }
                                if win_fta == "Win" { state.strategy_loss_con_fta = 0; } else if win_fta == "Loss" { state.strategy_loss_con_fta += 1; }
                                // FTA "Idle" → lossCon คงเดิม (ไม่เปลี่ยน)
                                if win_ftb == "Win" { state.strategy_loss_con_ftb = 0; } else if win_ftb == "Loss" { state.strategy_loss_con_ftb += 1; }
                                // FTB "Idle" → lossCon คงเดิม (ไม่เปลี่ยน)
                                if win_pktrend == "Win" { state.strategy_loss_con_pktrend = 0; } else if win_pktrend == "Loss" { state.strategy_loss_con_pktrend += 1; }
                                if win_pktrend_v5 == "Win" { state.strategy_loss_con_pktrend_v5 = 0; } else if win_pktrend_v5 == "Loss" { state.strategy_loss_con_pktrend_v5 += 1; }
                                if win_forecast == "Win" { state.strategy_loss_con_forecast = 0; } else if win_forecast == "Loss" { state.strategy_loss_con_forecast += 1; }
                                if win_pktrend_case_code == "Win" { state.strategy_loss_con_pktrend_case_code = 0; } else if win_pktrend_case_code == "Loss" { state.strategy_loss_con_pktrend_case_code += 1; }

                                // ═══ Post-Trade Diagnostic ═══
                                let fa_ref = state.pending_analysis_obj.as_ref();
                                let v1_factor = if win_v1 == "Loss" { analyze_loss_factor("V1", &StrategyDecision { suggest_color: sc_v1.clone(), reason: "".to_string(), conditions_matched: vec![], code: "".to_string() }, fa_ref, actual_next_color) } else { "".to_string() };
                                let v2_factor = if win_v2 == "Loss" { analyze_loss_factor("V2", &StrategyDecision { suggest_color: sc_v2.clone(), reason: "".to_string(), conditions_matched: vec![], code: "".to_string() }, fa_ref, actual_next_color) } else { "".to_string() };
                                let v3a_factor = if win_v3a == "Loss" { analyze_loss_factor("V3A", &StrategyDecision { suggest_color: sc_v3a.clone(), reason: "".to_string(), conditions_matched: vec![], code: "".to_string() }, fa_ref, actual_next_color) } else { "".to_string() };
                                let v3b_factor = if win_v3b == "Loss" { analyze_loss_factor("V3B", &StrategyDecision { suggest_color: sc_v3b.clone(), reason: "".to_string(), conditions_matched: vec![], code: "".to_string() }, fa_ref, actual_next_color) } else { "".to_string() };
                                let v3c_factor = if win_v3c == "Loss" { analyze_loss_factor("V3C", &StrategyDecision { suggest_color: sc_v3c.clone(), reason: "".to_string(), conditions_matched: vec![], code: "".to_string() }, fa_ref, actual_next_color) } else { "".to_string() };
                                let fta_factor = if win_fta == "Loss" { analyze_loss_factor("FTA", &StrategyDecision { suggest_color: sc_fta.clone(), reason: "".to_string(), conditions_matched: vec![], code: "".to_string() }, fa_ref, actual_next_color) } else { "".to_string() };
                                let ftb_factor = if win_ftb == "Loss" { analyze_loss_factor("FTB", &StrategyDecision { suggest_color: sc_ftb.clone(), reason: "".to_string(), conditions_matched: vec![], code: "".to_string() }, fa_ref, actual_next_color) } else { "".to_string() };
                                let pktrend_factor = if win_pktrend == "Loss" { analyze_loss_factor("PKTrend", &StrategyDecision { suggest_color: sc_pktrend.clone(), reason: "".to_string(), conditions_matched: vec![], code: "".to_string() }, fa_ref, actual_next_color) } else { "".to_string() };
                                let pktrend_v5_factor = if win_pktrend_v5 == "Loss" { analyze_loss_factor("PKTrendV5", &StrategyDecision { suggest_color: sc_pktrend_v5.clone(), reason: "".to_string(), conditions_matched: vec![], code: "".to_string() }, fa_ref, actual_next_color) } else { "".to_string() };
                                let forecast_factor = if win_forecast == "Loss" { analyze_loss_factor("Forecast", &StrategyDecision { suggest_color: sc_forecast.clone(), reason: "".to_string(), conditions_matched: vec![], code: "".to_string() }, fa_ref, actual_next_color) } else { "".to_string() };
                                let pktrend_case_code_factor = if win_pktrend_case_code == "Loss" { analyze_loss_factor("PKTrendCaseCode", &StrategyDecision { suggest_color: sc_pktrend_case_code.clone(), reason: "".to_string(), conditions_matched: vec![], code: "".to_string() }, fa_ref, actual_next_color) } else { "".to_string() };
                                state.pending_analysis_obj = None; // clear

                                // เพิ่มผลลัพธ์เข้า entry
                                if let Some(obj) = entry.as_object_mut() {
                                    obj.insert("actualNextColor".to_string(), json!(actual_next_color));
                                    obj.insert("actualContractType".to_string(), json!(actual_contract_type));
                                    obj.insert("actualProfit".to_string(), json!(profit));
                                    obj.insert("winStatusByV1".to_string(), json!(win_v1));
                                    obj.insert("winStatusByV2".to_string(), json!(win_v2));
                                    obj.insert("winStatusByV3A".to_string(), json!(win_v3a));
                                    obj.insert("winStatusByV3B".to_string(), json!(win_v3b));
                                    obj.insert("winStatusByV3C".to_string(), json!(win_v3c));
                                    obj.insert("winStatusByFTA".to_string(), json!(win_fta));
                                    obj.insert("winStatusByFTB".to_string(), json!(win_ftb));
                                    obj.insert("winStatusByPKTrend".to_string(), json!(win_pktrend));
                                    obj.insert("winStatusByPKTrendV5".to_string(), json!(win_pktrend_v5));
                                    obj.insert("winStatusByForecast".to_string(), json!(win_forecast));
                                    obj.insert("winStatusByPKTrendCaseCode".to_string(), json!(win_pktrend_case_code));
                                    obj.insert("v1LossFactor".to_string(), json!(v1_factor));
                                    obj.insert("v2LossFactor".to_string(), json!(v2_factor));
                                    obj.insert("v3aLossFactor".to_string(), json!(v3a_factor));
                                    obj.insert("v3bLossFactor".to_string(), json!(v3b_factor));
                                    obj.insert("v3cLossFactor".to_string(), json!(v3c_factor));
                                    obj.insert("ftaLossFactor".to_string(), json!(fta_factor));
                                    obj.insert("ftbLossFactor".to_string(), json!(ftb_factor));
                                    obj.insert("pktrendLossFactor".to_string(), json!(pktrend_factor));
                                    obj.insert("pktrendV5LossFactor".to_string(), json!(pktrend_v5_factor));
                                    obj.insert("forecastLossFactor".to_string(), json!(forecast_factor));
                                    obj.insert("pktrendCaseCodeLossFactor".to_string(), json!(pktrend_case_code_factor));
                                    obj.insert("lossConAfterV1".to_string(), json!(state.strategy_loss_con_v1));
                                    obj.insert("lossConAfterV2".to_string(), json!(state.strategy_loss_con_v2));
                                    obj.insert("lossConAfterV3A".to_string(), json!(state.strategy_loss_con_v3a));
                                    obj.insert("lossConAfterV3B".to_string(), json!(state.strategy_loss_con_v3b));
                                    obj.insert("lossConAfterV3C".to_string(), json!(state.strategy_loss_con_v3c));
                                    obj.insert("lossConAfterFTA".to_string(), json!(state.strategy_loss_con_fta));
                                    obj.insert("lossConAfterFTB".to_string(), json!(state.strategy_loss_con_ftb));
                                    obj.insert("lossConAfterPKTrend".to_string(), json!(state.strategy_loss_con_pktrend));
                                    obj.insert("lossConAfterPKTrendV5".to_string(), json!(state.strategy_loss_con_pktrend_v5));
                                    obj.insert("lossConAfterForecast".to_string(), json!(state.strategy_loss_con_forecast));
                                }

                                // บันทึกไฟล์ strategy_comparison.json (ในระดับ day — ไม่แยก asset)
                                let comp_dir = format!("tradeData/{}/{}", month_folder, day_folder);
                                let _ = fs::create_dir_all(&comp_dir);
                                let comp_file = format!("{}/strategy_comparison.json", comp_dir);
                                let mut comp_history: Vec<serde_json::Value> = vec![];
                                if let Ok(content) = fs::read_to_string(&comp_file) {
                                    if let Ok(arr) = serde_json::from_str(&content) {
                                        comp_history = arr;
                                    }
                                }
                                comp_history.push(entry);
                                if let Ok(json_str) = serde_json::to_string_pretty(&comp_history) {
                                    let _ = fs::write(&comp_file, json_str);
                                }

                                println!(
                                    "📊 [{}] Strategy Comparison #{}: V1={} V2={} V3A={} V3B={} V3C={} FTA={} FTB={} | nextColor={}",
                                    state.asset, state.strategy_runno, win_v1, win_v2, win_v3a, win_v3b, win_v3c, win_fta, win_ftb, actual_next_color
                                );
                            }

                            // ═══ Log ORDER CLOSED to tracker file ═══
                            {
                                let close_now = Local::now();
                                let mf = format!("{:02}-{:04}", close_now.month(), close_now.year() as i32 + 543);
                                let df = format!("{:02}-{:02}-{:04}", close_now.day(), close_now.month(), close_now.year() as i32 + 543);
                                let dir = format!("tradeData/{}/{}", mf, df);
                                let _ = fs::create_dir_all(&dir);
                                let log_path = format!("{}/order_tracker.log", dir);
                                let profit_sign = if profit >= 0.0 { "+" } else { "" };
                                let result_icon = if profit > 0.0 { "✅" } else { "❌" };
                                let close_cid = proposal.get("contract_id").and_then(|v| json_to_i64(v)).unwrap_or(0);
                                let close_ctype = proposal.get("contract_type").and_then(|v| v.as_str()).unwrap_or("?");
                                let line = format!(
                                    "[{}] 🏁 CLOSED {} | {:>8} | CID: {:>12} | {:>4} | Profit: {}${:.2} | Balance: ${:.2} | Duration: {}s | LossCon: {}\n",
                                    close_now.format("%H:%M:%S"), result_icon, state.deriv_symbol, close_cid, close_ctype,
                                    profit_sign, profit, state.current_balance + profit, actual_duration, state.loss_con
                                );
                                if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&log_path) {
                                    let _ = write!(f, "{}", line);
                                }
                            }

                            state.last_trade_details = None;

                            // ═══ อัปเดต TradeControlItem ล่าสุดด้วยผลเทรด ═══
                            if let Some(last_item) = state.trade_control_items.last_mut() {
                                last_item.winStatus = if profit > 0.0 { "Win".to_string() } else { "Loss".to_string() };
                            }

                            if profit > 0.0 {
                                println!("🎉 [{}] ชนะ! Reset LossCon = 0", state.asset);
                                state.win_con += 1;
                                if state.win_con > state.max_win_con {
                                    state.max_win_con = state.win_con;
                                }
                                state.loss_con = 0;
                                state.trade_no += 1;
                                state.sub_trade_no = 1;
                                state.in_martingale = false;

                                // ═══ Whipsaw Zone: Win → Reset ═══
                                state.whipsaw_zone_active = false;
                                state.whipsaw_frozen_loss_con = 0;
                                state.trade_control_items.clear();
                                state.trade_control_color_list.clear();

                                if current_config.trade.notify_telegram {
                                    let bot_token =
                                        env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
                                    let chat_id = env::var("TELEGRAM_CHAT_ID").unwrap_or_default();
                                    if !bot_token.is_empty() && !chat_id.is_empty() {
                                        let current_schedule_no = crate::get_or_create_trade_control().total_trade as i32;
                                        let thb_rate = current_config.trade.thb_rate;
                                        
                                        let mut schedule_profit = 0.0;
                                        let mut today_profit = 0.0;
                                        
                                        let now = chrono::Local::now();
                                        use chrono::Datelike;
                                        let month_folder = format!("{:02}-{:04}", now.month(), now.year() as i32 + 543);
                                        let day_folder = format!("{:02}-{:02}-{:04}", now.day(), now.month(), now.year() as i32 + 543);
                                        let base_dir = format!("tradeData/{}/{}", month_folder, day_folder);
                                        
                                        if let Ok(entries) = std::fs::read_dir(&base_dir) {
                                            for entry in entries.flatten() {
                                                if let Ok(ft) = entry.file_type() {
                                                    if ft.is_dir() {
                                                        let json_path = entry.path().join("trades.json");
                                                        if let Ok(content) = std::fs::read_to_string(&json_path) {
                                                            if let Ok(trades) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                                                                for t in trades {
                                                                    let p = t.get("ThisProfit").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                                                    today_profit += p;
                                                                    let s_no = t.get("scheduleTradeNo").and_then(|v| v.as_i64()).unwrap_or(1);
                                                                    if s_no as i32 == current_schedule_no {
                                                                        schedule_profit += p;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        let format_money = |amount: f64, prefix: &str| -> String {
                                            if amount >= 0.0 {
                                                format!("+{}{:.2}", prefix, amount)
                                            } else {
                                                format!("-{}{:.2}", prefix, amount.abs())
                                            }
                                        };

                                        let msg = format!("กำไรรอบนี้ ($) #{}\n{}\n\nกำไรรอบนี้ (฿) #{}\n{}\n\nกำไรวันนี้ ($)\n{}\n\nกำไรวันนี้ (฿)\n{}\n\n(เรท {:.2} ฿/$)",
                                            current_schedule_no, format_money(schedule_profit, "$"),
                                            current_schedule_no, format_money(schedule_profit * thb_rate, "฿"),
                                            format_money(today_profit, "$"),
                                            format_money(today_profit * thb_rate, "฿"),
                                            thb_rate
                                        );

                                        let url = format!(
                                            "https://api.telegram.org/bot{}/sendMessage",
                                            bot_token
                                        );
                                        let payload = serde_json::json!({ "chat_id": chat_id, "text": msg });
                                        tokio::spawn(async move {
                                            let _ = reqwest::Client::new()
                                                .post(&url)
                                                .json(&payload)
                                                .send()
                                                .await;
                                        });
                                    }
                                }
                            } else {
                                state.loss_con += 1;
                                if state.loss_con > state.max_loss_con {
                                    state.max_loss_con = state.loss_con;
                                }
                                state.win_con = 0;
                                state.in_martingale = true;
                                println!(
                                    "😢 [{}] แพ้! Martingale → LossCon = {}",
                                    state.asset, state.loss_con
                                );
                                state.sub_trade_no += 1;

                                if current_config.trade.notify_telegram {
                                    let bot_token =
                                        env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
                                    let chat_id = env::var("TELEGRAM_CHAT_ID").unwrap_or_default();
                                    if !bot_token.is_empty() && !chat_id.is_empty() {
                                        let msg = format!("😢 LOSS Trade!\nAsset: {}\nProfit: {:.2}\nLossCon: {}\nBalance: {:.2}", state.deriv_symbol, profit, state.loss_con, state.current_balance + profit);
                                        let url = format!(
                                            "https://api.telegram.org/bot{}/sendMessage",
                                            bot_token
                                        );
                                        let payload = json!({ "chat_id": chat_id, "text": msg });
                                        tokio::spawn(async move {
                                            let _ = reqwest::Client::new()
                                                .post(&url)
                                                .json(&payload)
                                                .send()
                                                .await;
                                        });
                                    }
                                }

                                // ═══ MaxLossCon Check: ถ้า loss_con ถึง max_loss_con → ยอมแพ้รอบนี้ ═══
                                if max_lc > 0 && state.loss_con >= max_lc {
                                    // ═══ Overtime Mode: ถ้าอยู่ในโหมด Overtime ต้องเทรดต่อจน Win จริง ═══
                                    if state.overtime_mode {
                                        let ot_keep_log = format!(
                                            "⏰ [{}] MAX LOSS CON REACHED (LossCon={} >= MaxLossCon={}) แต่อยู่ใน OVERTIME MODE → reset loss_con แล้วเทรดต่อจน Win",
                                            state.asset, state.loss_con, max_lc
                                        );
                                        println!("\n{}", ot_keep_log);
                                        let _ = tx.send(json!({
                                            "type": "bot_log",
                                            "asset": state.asset.clone(),
                                            "data": { "message": ot_keep_log }
                                        }));

                                        // Reset loss_con เพื่อเริ่ม martingale ใหม่ แต่ยังคง in_martingale = true
                                        state.loss_con = 0;
                                        state.sub_trade_no = 1;
                                        // in_martingale ยังเป็น true → จะไม่ถูกนับว่า "จบ" ใน overtime check
                                    } else {
                                        let give_up_log = format!(
                                            "🏳️ [{}] MAX LOSS CON REACHED! LossCon={} >= MaxLossCon={} → ยอมแพ้รอบนี้ Reset Martingale",
                                            state.asset, state.loss_con, max_lc
                                        );
                                        println!("\n{}", give_up_log);
                                        let _ = tx.send(json!({
                                            "type": "bot_log",
                                            "asset": state.asset.clone(),
                                            "data": { "message": give_up_log }
                                        }));

                                        // Reset เหมือนชนะ แต่เป็นการยอมแพ้
                                        state.loss_con = 0;
                                        state.in_martingale = false;
                                        state.trade_no += 1;
                                        state.sub_trade_no = 1;

                                        // ═══ Whipsaw Zone: Give Up → Reset ═══
                                        state.whipsaw_zone_active = false;
                                        state.whipsaw_frozen_loss_con = 0;
                                        state.trade_control_items.clear();
                                        state.trade_control_color_list.clear();
                                    }

                                    // แจ้ง Telegram
                                    if current_config.trade.notify_telegram {
                                        let bot_token =
                                            env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
                                        let chat_id = env::var("TELEGRAM_CHAT_ID").unwrap_or_default();
                                        if !bot_token.is_empty() && !chat_id.is_empty() {
                                            let msg = format!(
                                                "🏳️ GIVE UP! MaxLossCon Reached!\nAsset: {}\nLossCon: {} >= Max: {}\nBalance: {:.2}\n→ ยอมแพ้รอบนี้ เริ่มรอบใหม่",
                                                state.deriv_symbol, max_lc, max_lc, state.current_balance + profit
                                            );
                                            let url = format!(
                                                "https://api.telegram.org/bot{}/sendMessage",
                                                bot_token
                                            );
                                            let payload = json!({ "chat_id": chat_id, "text": msg });
                                            tokio::spawn(async move {
                                                let _ = reqwest::Client::new()
                                                    .post(&url)
                                                    .json(&payload)
                                                    .send()
                                                    .await;
                                            });
                                        }
                                    }
                                }
                            }

                            let current_round_no = crate::get_or_create_trade_control().total_trade;
                            crate::trade_head::on_trade_update(current_round_no, &state.deriv_symbol, state.max_win_con, state.max_loss_con);

                            state.current_balance += profit;
                            state.is_trading = false;
                            state.trade_buy_instant = None;
                            state.active_contract_id = None;

                            let _ = tx.send(json!({
                                "type": "trade_result",
                                "asset": state.asset.clone(),
                                "data": {
                                    "asset": state.asset.clone(),
                                    "win_status": win_status,
                                    "profit": profit,
                                    "loss_con": state.loss_con,
                                    "balance": state.current_balance
                                }
                            }));

                            // ═══ Overtime Check: ถ้าทุก asset Win ครบ → จบ loop ═══
                            if state.overtime_mode && !state.in_martingale {
                                let ot_log = format!(
                                    "✅ [{}] OVERTIME WIN — Asset นี้ Win แล้ว ไม่ต้องเทรดต่อ",
                                    state.asset
                                );
                                println!("\n{}", ot_log);
                                let _ = tx.send(json!({
                                    "type": "bot_log",
                                    "asset": state.asset.clone(),
                                    "data": { "message": ot_log }
                                }));
                            }
                        }
                    }
                }
            }

            if let Some(error) = parsed.get("error") {
                let req_id = parsed
                    .get("echo_req")
                    .and_then(|r| r.get("req_id"))
                    .and_then(|r| r.as_i64())
                    .unwrap_or(0);
                let asset_key = req_id_to_asset.get(&req_id).cloned().unwrap_or_default();

                let error_code = error
                    .get("code")
                    .and_then(|c| c.as_str())
                    .unwrap_or("unknown");
                let error_msg = error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown");

                if let Some(state) = asset_states.get_mut(&asset_key) {
                    eprintln!(
                        "\n❌ [{}] Deriv API Error: code={} | msg={}",
                        state.asset, error_code, error_msg
                    );
                    if state.is_trading {
                        state.is_trading = false;
                        state.trade_buy_instant = None;
                        state.active_contract_id = None;
                        state.last_processed_contract_id = None;
                        state.last_trade_details = None;
                    }
                } else {
                    eprintln!(
                        "\n❌ [Multiplex Global] Deriv API Error: code={} | msg={}",
                        error_code, error_msg
                    );
                }
            }

            // ═══ Overtime Signal Detection: ตรวจ signal สำหรับทุก asset ในทุก message ═══
            // (ย้ายมาจากใน ohlc block เพื่อให้ทุก asset ได้รับ signal ทันที)
            {
                let sigs = overtime_signals.lock().await;
                for (asset_key, state) in asset_states.iter_mut() {
                    if !state.overtime_mode {
                        if let Some(sig) = sigs.get(asset_key) {
                            if sig.load(Ordering::Relaxed) {
                                state.overtime_mode = true;
                                let ot_log = format!("⏰ [{}] OVERTIME MODE — ถึง StopDate แล้ว | Martingale: {} | LossCon: {} | is_trading: {}", state.asset, state.in_martingale, state.loss_con, state.is_trading);
                                println!("\n{}", ot_log);
                                let _ = tx.send(json!({
                                    "type": "bot_log",
                                    "asset": state.asset.clone(),
                                    "data": { "message": ot_log }
                                }));

                                if !state.in_martingale && !state.is_trading {
                                    let stop_log = format!("✅ [{}] OVERTIME STOP — ไม่มี Martingale ค้างและไม่ได้เข้าออเดอร์ → หยุดเทรด", state.asset);
                                    println!("{}", stop_log);
                                    let _ = tx.send(json!({
                                        "type": "bot_log",
                                        "asset": state.asset.clone(),
                                        "data": { "message": stop_log }
                                    }));
                                }
                            }
                        }
                    }
                }
                drop(sigs);
            }

            // ═══ Overtime Completion Check: ถ้าทุก asset Win ครบ → break loop ═══
            {
                // ต้องตรวจว่า ทุก asset เข้า overtime mode แล้ว (ALL ไม่ใช่ ANY)
                let all_in_overtime = asset_states.values().all(|s| s.overtime_mode);
                if all_in_overtime && !asset_states.is_empty() {
                    let all_done = asset_states.values().all(|s| {
                        // ทุก asset ต้อง: ไม่อยู่ระหว่าง martingale loss recovery AND ไม่กำลังเทรด
                        !s.in_martingale && !s.is_trading
                    });

                    // Debug log ทุก 50 messages เพื่อ monitor สถานะ
                    let debug_entries: Vec<String> = asset_states.values().map(|s| {
                        format!("{}(ot:{},mart:{},trading:{},lc:{})", s.asset, s.overtime_mode, s.in_martingale, s.is_trading, s.loss_con)
                    }).collect();
                    println!("\r⏳ OT CHECK: all_done={} | {}", all_done, debug_entries.join(" | "));

                    if all_done {
                        let done_log = format!(
                            "🏆 [Multiplex] OVERTIME COMPLETED — ทุก asset Win ครบ! จบการเทรด | {}",
                            debug_entries.join(" | ")
                        );
                        println!("\n{}", done_log);
                        let _ = tx.send(json!({
                            "type": "bot_log",
                            "asset": "system",
                            "data": { "message": done_log }
                        }));
                        break;
                    }
                }
            }
        }
    }

    eprintln!("\n⚠️ [Multiplex] WebSocket loop ended.");
    Ok(())
}

fn get_spot_position_code(spot_price: f64, open: f64, high: f64, low: f64, close: f64) -> String {
    if spot_price > high {
        return format!("{}-A", if open >= close { "R" } else { "G" });
    }
    if spot_price < low {
        return format!("{}-E", if open >= close { "R" } else { "G" });
    }

    if open >= close {
        if spot_price >= open {
            "R-B".to_string()
        } else if spot_price <= close {
            "R-D".to_string()
        } else {
            "R-C".to_string()
        }
    } else {
        if spot_price >= close {
            "G-B".to_string()
        } else if spot_price <= open {
            "G-D".to_string()
        } else {
            "G-C".to_string()
        }
    }
}

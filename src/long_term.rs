use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::full_analysis_ver2;

fn json_to_f64(val: &serde_json::Value) -> Option<f64> {
    val.as_f64().or_else(|| val.as_str()?.parse::<f64>().ok())
}

fn json_to_i64(val: &serde_json::Value) -> Option<i64> {
    val.as_i64()
        .or_else(|| val.as_u64().map(|v| v as i64))
        .or_else(|| val.as_f64().map(|v| v as i64))
        .or_else(|| val.as_str()?.parse::<i64>().ok())
}

fn parse_candle(data: &serde_json::Value) -> Option<full_analysis_ver2::RawCandleInput> {
    let epoch = data
        .get("open_time")
        .and_then(|v| json_to_i64(v))
        .or_else(|| data.get("epoch").and_then(|v| json_to_i64(v)))?;

    Some(full_analysis_ver2::RawCandleInput {
        epoch,
        open: json_to_f64(data.get("open")?)?,
        high: json_to_f64(data.get("high")?)?,
        low: json_to_f64(data.get("low")?)?,
        close: json_to_f64(data.get("close")?)?,
    })
}

fn map_asset_to_symbol(asset: &str) -> String {
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

pub struct LtAssetState {
    pub asset: String,
    pub deriv_symbol: String,
    pub req_id: i64,
    pub current_history: Vec<full_analysis_ver2::RawCandleInput>,
    pub last_trade_epoch: Option<i64>,
    pub last_order_time: Option<std::time::Instant>,  // เพิ่ม cooldown tracking
}

pub async fn start_longterm_bot(
    assets: Vec<String>,
    granularity: i32,
    tx: Arc<broadcast::Sender<serde_json::Value>>,
    lt_cmd_tx: Arc<broadcast::Sender<serde_json::Value>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // ═══ Spawn Private Bot with Auto-Reconnect ═══
    let tx_clone = tx.clone();
    let lt_cmd_tx_clone = lt_cmd_tx.clone();
    tokio::spawn(async move {
        let mut retry_count: u64 = 0;
        loop {
            let lt_cmd_rx = lt_cmd_tx_clone.subscribe();
            match start_longterm_private_bot(tx_clone.clone(), lt_cmd_rx).await {
                Ok(_) => {
                    println!("🛑 [LongTerm Private] exited normally.");
                    break;
                }
                Err(e) => {
                    retry_count += 1;
                    let delay = std::cmp::min(5 * retry_count, 30);
                    eprintln!("⚠️ [LongTerm Private] crashed: {}. Retry #{} in {}s...", e, retry_count, delay);
                    let _ = tx_clone.send(json!({
                        "type": "lt_log",
                        "data": { "message": format!("🔄 [Private Bot] Reconnecting... (retry #{})", retry_count) }
                    }));
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                }
            }
        }
    });

    // ═══ Candle Bot with Auto-Reconnect ═══
    let mut candle_retry_count: u64 = 0;

    'candle_reconnect: loop {
    // Broadcast connecting status
    let _ = tx.send(json!({ "type": "lt_ws_status", "target": "candle", "status": if candle_retry_count > 0 { "reconnecting" } else { "connecting" } }));

    // Get fresh credentials each reconnect attempt
    let (app_id, api_token, account_id) = crate::get_active_credentials(None);
    println!("\n📈 [LongTerm] กำลังเชื่อมต่อ Deriv OTP WebSocket สำหรับ {} assets...{}", assets.len(),
        if candle_retry_count > 0 { format!(" (retry #{})", candle_retry_count) } else { String::new() });

    let client = reqwest::Client::new();
    let otp_url = format!("https://api.derivws.com/trading/v1/options/accounts/{}/otp", account_id);
    let res = match client.post(&otp_url)
        .header("Authorization", format!("Bearer {}", api_token.trim()))
        .header("Deriv-App-ID", app_id.trim())
        .header("Content-Type", "application/json")
        .send()
        .await {
            Ok(r) => r,
            Err(e) => {
                candle_retry_count += 1;
                let _ = tx.send(json!({ "type": "lt_ws_status", "target": "candle", "status": "reconnecting" }));
                let delay = std::cmp::min(5 * candle_retry_count, 30);
                eprintln!("⚠️ [LongTerm] OTP request failed: {}. Retry in {}s...", e, delay);
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                continue 'candle_reconnect;
            }
        };

    if !res.status().is_success() {
        candle_retry_count += 1;
        let _ = tx.send(json!({ "type": "lt_ws_status", "target": "candle", "status": "reconnecting" }));
        let delay = std::cmp::min(5 * candle_retry_count, 30);
        eprintln!("❌ [LongTerm] Failed to get OTP: {}. Retry in {}s...", res.status(), delay);
        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
        continue 'candle_reconnect;
    }

    let otp_data: serde_json::Value = match res.json().await {
        Ok(d) => d,
        Err(e) => {
            candle_retry_count += 1;
            let _ = tx.send(json!({ "type": "lt_ws_status", "target": "candle", "status": "reconnecting" }));
            let delay = std::cmp::min(5 * candle_retry_count, 30);
            eprintln!("⚠️ [LongTerm] OTP parse failed: {}. Retry in {}s...", e, delay);
            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            continue 'candle_reconnect;
        }
    };
    let otp_ws_url = match otp_data.get("data").and_then(|d| d.get("url")).and_then(|u| u.as_str()) {
        Some(url) => url.to_string(),
        None => {
            candle_retry_count += 1;
            let _ = tx.send(json!({ "type": "lt_ws_status", "target": "candle", "status": "reconnecting" }));
            let delay = std::cmp::min(5 * candle_retry_count, 30);
            eprintln!("⚠️ [LongTerm] Failed to parse OTP URL. Retry in {}s...", delay);
            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            continue 'candle_reconnect;
        }
    };

    let (ws_stream, _) = match connect_async(&otp_ws_url).await {
        Ok(ws) => ws,
        Err(e) => {
            candle_retry_count += 1;
            let _ = tx.send(json!({ "type": "lt_ws_status", "target": "candle", "status": "reconnecting" }));
            let delay = std::cmp::min(5 * candle_retry_count, 30);
            eprintln!("⚠️ [LongTerm] WS connect failed: {}. Retry in {}s...", e, delay);
            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            continue 'candle_reconnect;
        }
    };
    let (mut ws_write, mut ws_read) = ws_stream.split();

    let _ = tx.send(json!({ "type": "lt_ws_status", "target": "candle", "status": "connected" }));

    if candle_retry_count > 0 {
        println!("🔄 [LongTerm] Reconnected successfully! (after {} retries)", candle_retry_count);
        let _ = tx.send(json!({
            "type": "lt_log",
            "data": { "message": format!("🔄 [Candle Bot] Reconnected to Deriv! (retry #{})", candle_retry_count) }
        }));
    } else {
        println!("✅ [LongTerm] เชื่อมต่อ OTP WebSocket สำเร็จ (สำหรับข้อมูลแท่งเทียน)!");
    }
    candle_retry_count = 0;

    let mut asset_states: HashMap<String, LtAssetState> = HashMap::new();
    let mut req_id_to_asset: HashMap<i64, String> = HashMap::new();

    for (idx, asset) in assets.iter().enumerate() {
        let req_id = (idx as i64 + 1) * 2000;
        let deriv_symbol = map_asset_to_symbol(asset);
        
        let state = LtAssetState {
            asset: asset.clone(),
            deriv_symbol: deriv_symbol.clone(),
            req_id,
            current_history: Vec::new(),
            last_trade_epoch: None,
            last_order_time: None,
        };
        
        req_id_to_asset.insert(req_id, asset.clone());
        asset_states.insert(asset.clone(), state);

        let ticks_request = json!({
            "ticks_history": deriv_symbol,
            "end": "latest",
            "style": "candles",
            "granularity": granularity,
            "count": 1000,
            "subscribe": 1,
            "req_id": req_id
        });
        
        println!("📦 [LongTerm] [{}] ร้องขอข้อมูลแท่งเทียน granularity: {}", asset, granularity);
        ws_write.send(Message::Text(ticks_request.to_string())).await?;
    }

    let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(30));

    loop {
        tokio::select! {
            _ = ping_interval.tick() => {
                let _ = ws_write.send(Message::Ping(vec![])).await;
                continue;
            }
            msg_opt = ws_read.next() => {
                let msg = match msg_opt {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        eprintln!("⚠️ [LongTerm] WS error: {}. Exiting loop.", e);
                        break;
                    }
                    None => {
                        eprintln!("⚠️ [LongTerm] WS disconnected. Exiting loop.");
                        break;
                    }
                };

                if let Message::Text(text) = msg {
                    let parsed: serde_json::Value = match serde_json::from_str(&text) {
                        Ok(p) => p,
                        Err(_) => continue,
                    };


                    if let Some(error) = parsed.get("error") {
                        eprintln!("❌ [LongTerm] Deriv Error: {:?}", error);
                        continue;
                    }

                    // Handle candles history response
                    if let Some(candles) = parsed.get("candles") {
                        let req_id = parsed.get("echo_req").and_then(|r| r.get("req_id")).and_then(|r| r.as_i64()).unwrap_or(0);
                        if let Some(asset) = req_id_to_asset.get(&req_id) {
                            if let Some(state) = asset_states.get_mut(asset) {
                                if let Some(arr) = candles.as_array() {
                                    state.current_history.clear();
                                    for c in arr {
                                        if let Some(raw) = parse_candle(c) {
                                            state.current_history.push(raw);
                                        }
                                    }
                                    
                                    let config_opt = get_longterm_analysis_config();

                                    let analysis = full_analysis_ver2::perform_analysis(&state.current_history, config_opt, Some(asset));
                                    
                                    let _ = tx.send(json!({
                                        "type": "lt_candles_history",
                                        "asset": asset,
                                        "data": analysis
                                    }));
                                    println!("📊 [LongTerm] ส่ง lt_candles_history สำหรับ {} ({} แท่ง)", asset, analysis.len());
                                    
                                    // ⭐ เพิ่ม: วิเคราะห์เงื่อนไขทันทีหลังโหลด historical candles
                                    if let Some(last_result) = analysis.last() {
                                        let setup = crate::get_or_create_lt_setup();
                                        println!("🔍 [LongTerm] Initial analysis for {}: ema_cut_position={}, ema_cut_all_type={}, auto_trade={}", 
                                            asset, last_result.ema_cut_position, last_result.ema_cut_all_type, setup.auto_trade);
                                        
                                        if setup.auto_trade {
                                            println!("✅ [LongTerm] Auto Trade is ENABLED - checking conditions...");
                                            let cond_sm = setup.conditions.contains(&"condShortMedium".to_string());
                                            let cond_all = setup.conditions.contains(&"condLongCross".to_string());
                                            let cond_sl = setup.conditions.contains(&"condShortLong".to_string());
                                            println!("📋 [LongTerm] Conditions enabled: SM={}, SML={}, SL={}", cond_sm, cond_all, cond_sl);

                                            let mut trigger_type = None;
                                            let mut entry_signal_str = String::new();
                                            let mut trade_code_str = String::new();

                                            if cond_sm {
                                                if last_result.ema_cut_position == "CrossUp" { 
                                                    trigger_type = Some("CALL"); 
                                                    entry_signal_str = "CrossUp".to_string();
                                                    trade_code_str = "SM".to_string();
                                                }
                                                if last_result.ema_cut_position == "CrossDown" { 
                                                    trigger_type = Some("PUT"); 
                                                    entry_signal_str = "CrossDown".to_string();
                                                    trade_code_str = "SM".to_string();
                                                }
                                            }
                                            if cond_all && trigger_type.is_none() {
                                                if last_result.ema_cut_all_type == "AllCrossUp" { 
                                                    trigger_type = Some("CALL"); 
                                                    entry_signal_str = "AllCrossUp".to_string();
                                                    trade_code_str = "SML".to_string();
                                                }
                                                if last_result.ema_cut_all_type == "AllCrossDown" { 
                                                    trigger_type = Some("PUT"); 
                                                    entry_signal_str = "AllCrossDown".to_string();
                                                    trade_code_str = "SML".to_string();
                                                }
                                            }
                                            if cond_sl && trigger_type.is_none() {
                                                if last_result.ema_cut_short_long_type == "CrossUp" { 
                                                    trigger_type = Some("CALL"); 
                                                    entry_signal_str = "CrossUp_SL".to_string();
                                                    trade_code_str = "SL".to_string();
                                                }
                                                if last_result.ema_cut_short_long_type == "CrossDown" { 
                                                    trigger_type = Some("PUT"); 
                                                    entry_signal_str = "CrossDown_SL".to_string();
                                                    trade_code_str = "SL".to_string();
                                                }
                                            }

                                            if let Some(c_type) = trigger_type {
                                                let now = std::time::Instant::now();
                                                let can_trade = if let Some(last_time) = state.last_order_time {
                                                    now.duration_since(last_time).as_secs() >= 5
                                                } else {
                                                    true
                                                };

                                                if !can_trade {
                                                    println!("⏳ [LongTerm INITIAL] Cooldown active (5s) - ข้าม order (asset: {})", asset);
                                                } else if state.last_trade_epoch != Some(last_result.candletime) {
                                                    state.last_trade_epoch = Some(last_result.candletime);
                                                    state.last_order_time = Some(now);
                                                    println!("🎯 [LongTerm INITIAL] เงื่อนไขตรง! ส่งคำสั่งซื้อ {} {} (Entry: {}, Code: {})", c_type, asset, entry_signal_str, trade_code_str);
                                                    let _ = lt_cmd_tx.send(json!({
                                                        "command": "lt_manual_trade",
                                                        "asset": asset,
                                                        "contract_type": c_type,
                                                        "amount": setup.amount,
                                                        "target_profit": setup.target_profit,
                                                        "duration": setup.duration,
                                                        "check_duplicate_asset": true,
                                                        "exit_strategy": setup.exit_strategy,
                                                        "entry_signal": entry_signal_str,
                                                        "trade_code": trade_code_str
                                                    }));
                                                } else {
                                                    println!("⏹️ [LongTerm INITIAL] Already traded on candle {} - skipping (asset: {})", last_result.candletime, asset);
                                                }
                                            } else {
                                                println!("⏹️ [LongTerm] No matching condition on initial analysis for {}", asset);
                                            }
                                        } else {
                                            println!("⏸️ [LongTerm] Auto Trade is DISABLED - skipping initial analysis");
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Handle tick update
                    if let Some(ohlc) = parsed.get("ohlc") {
                        let req_id = parsed.get("echo_req").and_then(|r| r.get("req_id")).and_then(|r| r.as_i64()).unwrap_or(0);
                        if let Some(asset) = req_id_to_asset.get(&req_id) {
                            if let Some(state) = asset_states.get_mut(asset) {
                                if let Some(new_raw) = parse_candle(ohlc) {
                                    let mut is_new_candle = false;
                                    if let Some(last) = state.current_history.last_mut() {
                                        if last.epoch == new_raw.epoch {
                                            *last = new_raw.clone();
                                        } else if new_raw.epoch > last.epoch {
                                            state.current_history.push(new_raw.clone());
                                            is_new_candle = true;
                                        }
                                    } else {
                                        state.current_history.push(new_raw.clone());
                                        is_new_candle = true;
                                    }

                                    let config_opt = get_longterm_analysis_config();
                                    let analysis = full_analysis_ver2::perform_analysis(&state.current_history, config_opt, Some(asset));
                                    
                                    if let Some(last_result) = analysis.last() {
                                        let setup = crate::get_or_create_lt_setup();
                                        
                                        // ⭐ Evaluate entry signals ONLY on CLOSED candles (when a new candle opens)
                                        if is_new_candle && analysis.len() >= 2 {
                                            let closed_candle_result = &analysis[analysis.len() - 2];
                                            
                                            let cond_sm = setup.conditions.contains(&"condShortMedium".to_string());
                                            let cond_all = setup.conditions.contains(&"condLongCross".to_string());
                                            let cond_sl = setup.conditions.contains(&"condShortLong".to_string());

                                            let mut trigger_type = None;
                                            let mut entry_signal_str = String::new();
                                            let mut trade_code_str = String::new();

                                            if cond_sm {
                                                if closed_candle_result.ema_cut_position == "CrossUp" { 
                                                    trigger_type = Some("CALL"); 
                                                    entry_signal_str = "CrossUp".to_string();
                                                    trade_code_str = "SM".to_string();
                                                }
                                                if closed_candle_result.ema_cut_position == "CrossDown" { 
                                                    trigger_type = Some("PUT"); 
                                                    entry_signal_str = "CrossDown".to_string();
                                                    trade_code_str = "SM".to_string();
                                                }
                                            }
                                            if cond_all && trigger_type.is_none() {
                                                if closed_candle_result.ema_cut_all_type == "AllCrossUp" { 
                                                    trigger_type = Some("CALL"); 
                                                    entry_signal_str = "AllCrossUp".to_string();
                                                    trade_code_str = "SML".to_string();
                                                }
                                                if closed_candle_result.ema_cut_all_type == "AllCrossDown" { 
                                                    trigger_type = Some("PUT"); 
                                                    entry_signal_str = "AllCrossDown".to_string();
                                                    trade_code_str = "SML".to_string();
                                                }
                                            }
                                            if cond_sl && trigger_type.is_none() {
                                                if closed_candle_result.ema_cut_short_long_type == "CrossUp" { 
                                                    trigger_type = Some("CALL"); 
                                                    entry_signal_str = "CrossUp_SL".to_string();
                                                    trade_code_str = "SL".to_string();
                                                }
                                                if closed_candle_result.ema_cut_short_long_type == "CrossDown" { 
                                                    trigger_type = Some("PUT"); 
                                                    entry_signal_str = "CrossDown_SL".to_string();
                                                    trade_code_str = "SL".to_string();
                                                }
                                            }

                                            if let Some(c_type) = trigger_type {
                                                if setup.auto_trade {
                                                    // ตรวจสอบ cooldown (ห้ามยิง order ซ้ำภายใน 5 วินาที)
                                                    let now = std::time::Instant::now();
                                                    let can_trade = if let Some(last_time) = state.last_order_time {
                                                        now.duration_since(last_time).as_secs() >= 5
                                                    } else {
                                                        true
                                                    };
                                                    
                                                    if !can_trade {
                                                        println!("⏳ [LongTerm] Cooldown active - ข้าม order (asset: {})", asset);
                                                    } else if state.last_trade_epoch != Some(closed_candle_result.candletime) {
                                                        state.last_trade_epoch = Some(closed_candle_result.candletime);
                                                        state.last_order_time = Some(now);
                                                        println!("🎯 [LongTerm CLOSED CANDLE] เงื่อนไขตรงบนแท่งปิด! ส่งคำสั่งซื้อ {} {} (Candle: {}, Entry: {}, Code: {})", c_type, asset, closed_candle_result.candletime, entry_signal_str, trade_code_str);
                                                        let _ = lt_cmd_tx.send(json!({
                                                            "command": "lt_manual_trade",
                                                            "asset": asset,
                                                            "contract_type": c_type,
                                                            "amount": setup.amount,
                                                            "target_profit": setup.target_profit,
                                                            "duration": setup.duration,
                                                            "check_duplicate_asset": true,
                                                            "exit_strategy": setup.exit_strategy,
                                                            "entry_signal": entry_signal_str,
                                                            "trade_code": trade_code_str
                                                        }));
                                                    }
                                                } else {
                                                    println!("⏸️ [LongTerm] Auto Trade DISABLED - condition matched on closed candle but skipping (asset: {})", asset);
                                                }
                                            }
                                        }
                                        
                                        // ส่ง EMA cross signal เพื่อเช็คว่าต้องขาย order ที่มีอยู่หรือไม่
                                        let _ = lt_cmd_tx.send(json!({
                                            "command": "lt_check_exit",
                                            "asset": asset,
                                            "ema_cut_position": last_result.ema_cut_position,
                                            "ema_cut_all_type": last_result.ema_cut_all_type,
                                            "ema_cut_short_long_type": last_result.ema_cut_short_long_type
                                        }));

                                        let _ = tx.send(json!({
                                            "type": "lt_candle_update",
                                            "asset": asset,
                                            "data": last_result
                                        }));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Inner loop broke (WebSocket disconnected) — auto-reconnect
    candle_retry_count += 1;
    let delay = std::cmp::min(5 * candle_retry_count, 30);
    eprintln!("⚠️ [LongTerm] WebSocket disconnected. Reconnecting in {}s... (retry #{})", delay, candle_retry_count);
    let _ = tx.send(json!({
        "type": "lt_ws_status",
        "target": "candle",
        "status": "reconnecting"
    }));
    let _ = tx.send(json!({
        "type": "lt_log",
        "data": { "message": format!("🔄 [Candle Bot] Disconnected. Reconnecting in {}s...", delay) }
    }));
    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
    } // end 'candle_reconnect loop
}

/// One-shot fetch: ดึงข้อมูลแท่งเทียนจาก Deriv WS สำหรับแสดงกราฟที่ granularity ใดๆ
pub async fn fetch_lt_chart_candles(
    asset: &str,
    granularity: i32,
) -> Result<Vec<crate::full_analysis_ver2::FullAnalysisResult>, Box<dyn std::error::Error + Send + Sync>> {
    // ใช้ OTP WebSocket แทน Public WebSocket
    let (app_id_str, api_token, account_id) = crate::get_active_credentials(None);

    let client = reqwest::Client::new();
    let otp_url = format!("https://api.derivws.com/trading/v1/options/accounts/{}/otp", account_id);
    let res = client.post(&otp_url)
        .header("Authorization", format!("Bearer {}", api_token.trim()))
        .header("Deriv-App-ID", app_id_str.trim())
        .header("Content-Type", "application/json")
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(format!("Failed to get OTP for LT-Chart: {}", res.status()).into());
    }

    let otp_data: serde_json::Value = res.json().await?;
    let otp_ws_url = otp_data.get("data").and_then(|d| d.get("url")).and_then(|u| u.as_str())
        .ok_or_else(|| "Failed to parse OTP WebSocket URL for LT-Chart")?;

    let (ws_stream, _) = tokio_tungstenite::connect_async(otp_ws_url).await?;
    let (mut ws_write, mut ws_read) = ws_stream.split();
    
    let deriv_symbol = map_asset_to_symbol(asset);
    
    let request = serde_json::json!({
        "ticks_history": deriv_symbol,
        "end": "latest",
        "style": "candles",
        "granularity": granularity,
        "count": 1000,
        "req_id": 9999
    });
    
    println!("📊 [LT-Chart] Fetching {} candles for {} (gran={})", 1000, asset, granularity);
    ws_write.send(Message::Text(request.to_string())).await?;
    
    while let Some(msg_result) = ws_read.next().await {
        let msg = msg_result?;
        if let Message::Text(text) = msg {
            let parsed: serde_json::Value = match serde_json::from_str(&text) {
                Ok(p) => p,
                Err(_) => continue,
            };
            
            if let Some(error) = parsed.get("error") {
                return Err(format!("Deriv Error: {:?}", error).into());
            }
            
            if let Some(candles) = parsed.get("candles") {
                let mut raw_history = Vec::new();
                if let Some(arr) = candles.as_array() {
                    for c in arr {
                        if let Some(raw) = parse_candle(c) {
                            raw_history.push(raw);
                        }
                    }
                }
                
                let config_opt = get_longterm_analysis_config();
                
                let analysis = full_analysis_ver2::perform_analysis(&raw_history, config_opt, Some(asset));
                println!("📊 [LT-Chart] Got {} analyzed candles for {} (gran={})", analysis.len(), asset, granularity);
                
                // Close WS connection
                let _ = ws_write.close().await;
                
                return Ok(analysis);
            }
        }
    }
    
    Err("Failed to fetch chart candles".into())
}

use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LtOrder {
    pub contract_id: i64,
    pub buy_price: f64,
    pub payout: f64,
    pub time: String,
    pub target_profit: f64,
    pub symbol: String,
    pub contract_type: String,
    pub min_profit: f64,
    pub max_profit: f64,
    pub profit: f64,
    pub current_spot: f64,
    pub entry_spot: f64,
    pub is_selling: bool,
    #[serde(default)]
    pub underlying: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub purchase_time: i64,
    #[serde(default)]
    pub date_start: i64,
    #[serde(default)]
    pub date_expiry: i64,
    #[serde(default)]
    pub exit_strategy: String,  // "targetProfit", "nextCross", "durationExpiry"
    #[serde(default)]
    pub entry_signal: String,   // "CrossUp", "CrossDown", "AllCrossUp", "AllCrossDown", etc.
    #[serde(default)]
    pub trade_code: String,     // "SM", "SML", "SL", "SM+SL", "MANUAL", etc.
    #[serde(default)]
    pub asset: String,
    #[serde(default)]
    pub ema_periods: String,
}

fn map_symbol_to_asset(symbol: &str) -> String {
    match symbol {
        "1HZ10V" => "vol10".to_string(),
        "1HZ25V" => "vol25".to_string(),
        "1HZ50V" => "vol50".to_string(),
        "1HZ75V" => "vol75".to_string(),
        "1HZ100V" => "vol100".to_string(),
        _ => symbol.to_string(),
    }
}

fn get_current_ema_periods_string() -> String {
    if let Some(cfg) = get_longterm_analysis_config() {
        format!("{}-{}-{}", cfg.ema.short.period, cfg.ema.medium.period, cfg.ema.long.period)
    } else {
        "9-21-50".to_string()
    }
}

/// บันทึกข้อมูลเทรด Long Term ลง tradeData/{MM-YYYY}/{DD-MM-YYYY}/{ASSET}/trades.json
fn save_lt_trade_to_history(order: &LtOrder, result_data: &serde_json::Value, loss_con: u32) {
    use chrono::{Local, Datelike, Timelike};
    
    let now = Local::now();
    let month_folder = format!("{:02}-{:04}", now.month(), now.year() as i32 + 543);
    let day_folder = format!("{:02}-{:02}-{:04}", now.day(), now.month(), now.year() as i32 + 543);
    
    // ใช้ underlying ถ้ามี ไม่มีใช้ symbol
    let asset_code = if !order.underlying.is_empty() {
        &order.underlying
    } else if !order.symbol.is_empty() {
        &order.symbol
    } else {
        &order.display_name
    };
    
    let dir_path = format!("tradeData/{}/{}/{}", month_folder, day_folder, asset_code);
    let _ = std::fs::create_dir_all(&dir_path);
    let log_file = format!("{}/trades.json", dir_path);
    
    // อ่านข้อมูลเก่า
    let mut trade_history: Vec<serde_json::Value> = vec![];
    if let Ok(content) = std::fs::read_to_string(&log_file) {
        if let Ok(arr) = serde_json::from_str(&content) {
            trade_history = arr;
        }
    }
    
    // คำนวณข้อมูล
    let profit = result_data.get("profit").and_then(|v| json_to_f64(v)).unwrap_or(order.profit);
    let _sell_price = result_data.get("sell_price").and_then(|v| json_to_f64(v)).unwrap_or_else(|| order.buy_price + profit);
    let buy_time_i64 = order.purchase_time;
    let sell_time_i64 = result_data.get("sell_time").and_then(|v| v.as_i64()).unwrap_or_else(|| Local::now().timestamp());
    let duration = sell_time_i64 - buy_time_i64;
    
    // Format timestamp เป็น Thai format
    let buy_dt = chrono::DateTime::from_timestamp(buy_time_i64, 0)
        .map(|dt| dt.with_timezone(&chrono::FixedOffset::east_opt(7 * 3600).unwrap()))
        .unwrap_or_else(|| Local::now().with_timezone(&chrono::FixedOffset::east_opt(7 * 3600).unwrap()));
    let sell_dt = chrono::DateTime::from_timestamp(sell_time_i64, 0)
        .map(|dt| dt.with_timezone(&chrono::FixedOffset::east_opt(7 * 3600).unwrap()))
        .unwrap_or_else(|| Local::now().with_timezone(&chrono::FixedOffset::east_opt(7 * 3600).unwrap()));
    
    let buy_time_display = format!("{}/{}/{} {:02}:{:02}:{:02}", 
        buy_dt.day(), buy_dt.month(), buy_dt.year() as i32 + 543,
        buy_dt.hour(), buy_dt.minute(), buy_dt.second());
    let sell_time_display = format!("{}/{}/{} {:02}:{:02}:{:02}", 
        sell_dt.day(), sell_dt.month(), sell_dt.year() as i32 + 543,
        sell_dt.hour(), sell_dt.minute(), sell_dt.second());
    
    let win_status = if profit > 0.0 { "Win" } else { "Loss" };
    
    // สร้าง trade record (เหมือนบอทหลัก)
    let trade_record = json!({
        "contractId": order.contract_id,
        "buyId": 0,  // Long Term ไม่มี buyId แยก
        "sellId": 0, // Long Term ไม่มี sellId แยก
        "assetCode": asset_code,
        "entrySpot": order.entry_spot,
        "exitSpot": order.current_spot,
        "DiffSpot": order.current_spot - order.entry_spot,
        "purchaseTime": buy_time_i64,
        "purchaseTimeDisplay": buy_time_display,
        "scheduleTradeNo": trade_history.len() + 1,
        "tradeNo": trade_history.len() + 1,
        "subTradeno": 1,
        "timeCandle": 0,
        "timeCandleDisplay": "",
        "sellTime": sell_time_i64,
        "sellTimeDisplay": sell_time_display,
        "actualDuration": duration,
        "isAnomaly": false,
        "thisColor": if order.contract_type == "CALL" { "green" } else { "red" },
        "thisAction": order.contract_type,
        "targetColor": "",
        "emaShortDirection": "",
        "emaMediumDirection": "",
        "MoneyTrade": order.buy_price,
        "WinStatus": win_status,
        "lossCon": loss_con,
        "ThisProfit": profit,
        "GrandBalance": 0.0,  // Long Term ไม่ track balance
        "gaveUp": false,
        "maxLossCon": 0,
        "tradeStrategy": "LONGTERM",
        "codeStrategy": "LT",
        "noiseCode": "",
        "spotCallPositionCode": "",
        "spotPutPositionCode": "",
        "spotPriceCall": 0.0,
        "spotPricePut": 0.0,
        // Long Term specific fields
        "targetProfit": order.target_profit,
        "minProfit": order.min_profit,
        "maxProfit": order.max_profit,
        "payout": order.payout,
        "dateStart": order.date_start,
        "dateExpiry": order.date_expiry,
        "exitStrategy": order.exit_strategy,
        "entrySignal": order.entry_signal,
        "entryConditions": order.entry_signal,  // Alias for frontend
        "tradeCode": order.trade_code,
        "trade_code": order.trade_code,  // Alias for frontend (snake_case)
        "emaPeriods": order.ema_periods,
        "ema_periods": order.ema_periods
    });
    
    trade_history.push(trade_record);
    
    // บันทึกกลับไปยังไฟล์
    if let Ok(json_str) = serde_json::to_string_pretty(&trade_history) {
        let _ = std::fs::write(&log_file, json_str);
        println!("💾 [LongTerm] บันทึกประวัติเทรดไปยัง: {} | Profit: ${:.2} | {}", log_file, profit, win_status);
    }
}

pub async fn start_longterm_private_bot(
    tx: Arc<broadcast::Sender<serde_json::Value>>,
    mut lt_cmd_rx: broadcast::Receiver<serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    {
        let (_, api_token, _) = crate::get_active_credentials(None);
        if api_token.is_empty() || api_token == "YOUR_API_TOKEN_HERE" {
            eprintln!("⚠️ [LongTerm Private] No API Token. Cannot start private tracking.");
            return Ok(());
        }
    }

    // State preserved across reconnections
    let mut orders: Vec<LtOrder> = Vec::new();
    let mut is_portfolio_loaded = false;
    let mut loss_con_map: HashMap<String, u32> = HashMap::new();
    let mut last_buy_attempts: HashMap<String, std::time::Instant> = HashMap::new();
    let tracking_file = "setup/longterm_tracking.json";
    let mut private_retry_count: u64 = 0;

    'private_reconnect: loop {
    // Broadcast connecting status
    let _ = tx.send(json!({ "type": "lt_ws_status", "target": "private", "status": if private_retry_count > 0 { "reconnecting" } else { "connecting" } }));

    // Load/reload persisted tracking data
    let mut saved_tracking: HashMap<i64, LtOrder> = HashMap::new();
    if let Ok(content) = fs::read_to_string(tracking_file) {
        if let Ok(saved) = serde_json::from_str::<Vec<LtOrder>>(&content) {
            for o in saved {
                saved_tracking.insert(o.contract_id, o);
            }
        }
    }

    // Get fresh credentials
    let (app_id, api_token, account_id) = crate::get_active_credentials(None);
    println!("🔐 [LongTerm Private] กำลังขอ OTP และเชื่อมต่อ Deriv WebSocket...{}",
        if private_retry_count > 0 { format!(" (retry #{})", private_retry_count) } else { String::new() });

    let client = reqwest::Client::new();
    let otp_url = format!("https://api.derivws.com/trading/v1/options/accounts/{}/otp", account_id);
    let res = match client.post(&otp_url)
        .header("Authorization", format!("Bearer {}", api_token.trim()))
        .header("Deriv-App-ID", app_id.trim())
        .header("Content-Type", "application/json")
        .send()
        .await {
            Ok(r) => r,
            Err(e) => {
                private_retry_count += 1;
                let _ = tx.send(json!({ "type": "lt_ws_status", "target": "private", "status": "reconnecting" }));
                let delay = std::cmp::min(5 * private_retry_count, 30);
                eprintln!("⚠️ [LongTerm Private] OTP request failed: {}. Retry in {}s...", e, delay);
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                continue 'private_reconnect;
            }
        };
    
    if !res.status().is_success() {
        private_retry_count += 1;
        let _ = tx.send(json!({ "type": "lt_ws_status", "target": "private", "status": "reconnecting" }));
        let delay = std::cmp::min(5 * private_retry_count, 30);
        eprintln!("❌ [LongTerm Private] Failed to get OTP: {}. Retry in {}s...", res.status(), delay);
        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
        continue 'private_reconnect;
    }
    
    let otp_data: serde_json::Value = match res.json().await {
        Ok(d) => d,
        Err(e) => {
            private_retry_count += 1;
            let _ = tx.send(json!({ "type": "lt_ws_status", "target": "private", "status": "reconnecting" }));
            let delay = std::cmp::min(5 * private_retry_count, 30);
            eprintln!("⚠️ [LongTerm Private] OTP parse failed: {}. Retry in {}s...", e, delay);
            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            continue 'private_reconnect;
        }
    };
    let private_ws_url = match otp_data.get("data").and_then(|d| d.get("url")).and_then(|u| u.as_str()) {
        Some(url) => url.to_string(),
        None => {
            private_retry_count += 1;
            let _ = tx.send(json!({ "type": "lt_ws_status", "target": "private", "status": "reconnecting" }));
            let delay = std::cmp::min(5 * private_retry_count, 30);
            eprintln!("⚠️ [LongTerm Private] Failed to parse OTP URL. Retry in {}s...", delay);
            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            continue 'private_reconnect;
        }
    };

    let (ws_stream, _) = match tokio_tungstenite::connect_async(&private_ws_url).await {
        Ok(ws) => ws,
        Err(e) => {
            private_retry_count += 1;
            let _ = tx.send(json!({ "type": "lt_ws_status", "target": "private", "status": "reconnecting" }));
            let delay = std::cmp::min(5 * private_retry_count, 30);
            eprintln!("⚠️ [LongTerm Private] WS connect failed: {}. Retry in {}s...", e, delay);
            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            continue 'private_reconnect;
        }
    };
    let (mut ws_write, mut ws_read) = ws_stream.split();

    let _ = tx.send(json!({ "type": "lt_ws_status", "target": "private", "status": "connected" }));

    if private_retry_count > 0 {
        println!("🔄 [LongTerm Private] Reconnected successfully! (after {} retries)", private_retry_count);
        let _ = tx.send(json!({
            "type": "lt_log",
            "data": { "message": format!("🔄 [Private Bot] Reconnected to Deriv! (retry #{})", private_retry_count) }
        }));
    } else {
        println!("✅ [LongTerm Private] OTP Authorize Success! Requesting portfolio...");
    }
    private_retry_count = 0;
    is_portfolio_loaded = false;

    // Request portfolio to sync open orders
    let port_req = json!({ "portfolio": 1 });
    let _ = ws_write.send(Message::Text(port_req.to_string())).await;

    let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(30));

    loop {
        tokio::select! {
            _ = ping_interval.tick() => {
                let _ = ws_write.send(Message::Ping(vec![])).await;
            }
            cmd_opt = lt_cmd_rx.recv() => {
                if let Ok(cmd) = cmd_opt {
                    let c_type = cmd.get("command").and_then(|v| v.as_str()).unwrap_or("");
                    if c_type == "lt_manual_trade" {
                        if !is_portfolio_loaded {
                            println!("⚠️ [LongTerm] ข้ามการเปิดออเดอร์ เนื่องจากกำลังโหลดข้อมูล Portfolio...");
                            continue;
                        }
                        
                        let max_orders = crate::get_or_create_lt_setup().max_orders;
                        if orders.len() >= max_orders as usize {
                            println!("⚠️ [LongTerm] ข้ามการเปิดออเดอร์ (โควต้าเต็ม max_orders={})", max_orders);
                            continue;
                        }
                        
                        let asset = cmd.get("asset").and_then(|v| v.as_str()).unwrap_or("");
                        if asset.is_empty() {
                            continue;
                        }
                        
                        // Debounce Lock: Check minimum 5,000ms (5 seconds) interval between buy attempts per asset
                        let now_inst = std::time::Instant::now();
                        if let Some(&last_time) = last_buy_attempts.get(asset) {
                            let elapsed_ms = now_inst.duration_since(last_time).as_millis();
                            if elapsed_ms < 5000 {
                                println!("⏳ [LongTerm Backend] Asset {} เพิ่งยิง order ไปเมื่อ {:.1}s ที่แล้ว (< 5s lock) — ข้ามการเปิดซ้ำ", asset, elapsed_ms as f64 / 1000.0);
                                continue;
                            }
                        }
                        
                        // Check duplicate asset (if check_duplicate_asset is enabled)
                        let check_duplicate = cmd.get("check_duplicate_asset").and_then(|v| v.as_bool()).unwrap_or(true);
                        if check_duplicate {
                            let target_symbol = map_asset_to_symbol(asset);
                            let target_asset = map_symbol_to_asset(asset);

                            let has_duplicate = orders.iter().any(|o| {
                                let o_sym = map_asset_to_symbol(if !o.asset.is_empty() { &o.asset } else if !o.underlying.is_empty() { &o.underlying } else { &o.symbol });
                                let o_ast = map_symbol_to_asset(if !o.asset.is_empty() { &o.asset } else if !o.symbol.is_empty() { &o.symbol } else { &o.underlying });
                                o_sym == target_symbol || o_ast == target_asset || o.asset == asset || o.symbol == asset || o.underlying == asset || o.symbol == target_symbol
                            });
                            
                            if has_duplicate {
                                println!("⚠️ [LongTerm Backend] Asset {} (Symbol: {}) มี Order เปิดอยู่แล้ว — ข้ามการเปิดซ้ำ", asset, target_symbol);
                                continue;
                            }
                        }
                        
                        // Record buy attempt timestamp to lock out duplicate requests within 5000ms
                        last_buy_attempts.insert(asset.to_string(), now_inst);
                        
                        let contract_type = cmd.get("contract_type").and_then(|v| v.as_str()).unwrap_or("");
                        let mut amount = cmd.get("amount").and_then(|v| v.as_f64()).unwrap_or(10.0);
                        let target_percent = cmd.get("target_profit").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        
                        let exit_strategy = cmd.get("exit_strategy").and_then(|v| v.as_str()).unwrap_or("targetProfit");
                        let entry_signal = cmd.get("entry_signal").and_then(|v| v.as_str()).unwrap_or("");
                        let trade_code = cmd.get("trade_code").and_then(|v| v.as_str()).unwrap_or("MANUAL");
                        
                        // Apply martingale stake selection
                        {
                            let setup = crate::get_or_create_lt_setup();
                            if setup.martingale.martingale_type == "martingale" {
                                let loss_con = *loss_con_map.get(&asset.to_string()).unwrap_or(&0);
                                if let Some(&mart_amount) = setup.martingale.list.get(loss_con as usize) {
                                    amount = mart_amount;
                                    println!("📊 [LongTerm Martingale] Asset: {} | loss_con: {} | Selecting Martingale stake: ${:.2}", asset, loss_con, amount);
                                } else if let Some(&last_amount) = setup.martingale.list.last() {
                                    amount = last_amount;
                                    println!("📊 [LongTerm Martingale] Asset: {} | loss_con: {} (out of bounds) | Selecting last Martingale stake: ${:.2}", asset, loss_con, amount);
                                }
                            }
                        }

                        let target_profit = (amount * target_percent) / 100.0;
                        println!("📥 [Rust Backend] lt_manual_trade - target: {}%, exit: {}, entry: {}, code: {}, final stake: ${:.2}", target_percent, exit_strategy, entry_signal, trade_code, amount);
                        let duration = cmd.get("duration").and_then(|v| v.as_i64()).unwrap_or(3600);
                        
                        let deriv_symbol = map_asset_to_symbol(asset);
                        println!("📤 [LongTerm] Placing buy order for {} (Mapped: {})", asset, deriv_symbol);
                        
                        let buy_req = json!({
                            "buy": 1,
                            "price": 100000,
                            "parameters": {
                                "amount": amount,
                                "basis": "stake",
                                "contract_type": contract_type,
                                "currency": "USD",
                                "duration": duration,
                                "duration_unit": "s",
                                "underlying_symbol": deriv_symbol
                            },
                            "passthrough": {
                                "target_profit": target_profit,
                                "exit_strategy": exit_strategy,
                                "entry_signal": entry_signal,
                                "trade_code": trade_code,
                                "asset": asset
                            }
                        });
                        let _ = ws_write.send(Message::Text(buy_req.to_string())).await;
                    } else if c_type == "lt_sell" {
                        let contract_id = cmd.get("contract_id").and_then(|v| v.as_i64()).unwrap_or(0);
                        
                        // ตรวจสอบว่า order ยังอยู่ใน list หรือไม่
                        if let Some(o) = orders.iter_mut().find(|o| o.contract_id == contract_id) {
                            // ตรวจสอบว่ากำลัง selling อยู่แล้วหรือไม่
                            if o.is_selling {
                                println!("⚠️ [LongTerm] Contract {} กำลังขายอยู่แล้ว - ข้ามการส่ง sell ซ้ำ", contract_id);
                                continue;
                            }
                            o.is_selling = true;
                            let sell_req = json!({ "sell": contract_id, "price": 0 });
                            let _ = ws_write.send(Message::Text(sell_req.to_string())).await;
                            println!("📤 [LongTerm] ส่งคำสั่งขาย Contract {}", contract_id);
                        } else {
                            println!("⚠️ [LongTerm] Contract {} ไม่อยู่ใน open orders - อาจถูกขายไปแล้ว", contract_id);
                        }
                    } else if c_type == "lt_set_target" {
                        let contract_id = cmd.get("contract_id").and_then(|v| v.as_i64()).unwrap_or(0);
                        let target_percent = cmd.get("target_profit").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        println!("📥 [Rust Backend] lt_set_target - Received target_percent: {}% for Contract: {}", target_percent, contract_id);
                        if let Some(o) = orders.iter_mut().find(|o| o.contract_id == contract_id) {
                            o.target_profit = (o.buy_price * target_percent) / 100.0;
                        }
                        let _ = fs::write(tracking_file, serde_json::to_string_pretty(&orders).unwrap_or_default());
                        let _ = tx.send(json!({ "type": "lt_orders_update", "data": orders }));
                    } else if c_type == "lt_check_exit" {
                        // เช็คว่ามี order ไหนที่ใช้ nextCross exit strategy และเกิด cross ตรงข้าม
                        let asset = cmd.get("asset").and_then(|v| v.as_str()).unwrap_or("");
                        let ema_cut_position = cmd.get("ema_cut_position").and_then(|v| v.as_str()).unwrap_or("");
                        let ema_cut_all_type = cmd.get("ema_cut_all_type").and_then(|v| v.as_str()).unwrap_or("");
                        let _ema_cut_short_long_type = cmd.get("ema_cut_short_long_type").and_then(|v| v.as_str()).unwrap_or("");
                        
                        for o in orders.iter_mut() {
                            // เช็คว่า order นี้ใช่ asset ที่ได้รับ signal มาหรือไม่
                            let order_asset = if !o.underlying.is_empty() {
                                &o.underlying
                            } else if !o.symbol.is_empty() {
                                &o.symbol
                            } else {
                                &o.display_name
                            };
                            
                            if order_asset != asset {
                                continue;
                            }
                            
                            // เช็คว่าใช้ exit strategy แบบ nextCross หรือไม่
                            if o.exit_strategy != "nextCross" || o.is_selling {
                                continue;
                            }
                            
                            // เช็คว่าเกิด cross ตรงข้ามกับ entry signal หรือไม่
                            let mut should_sell = false;
                            let mut exit_reason = String::new();
                            
                            // Entry: CrossUp (CALL) → Exit: CrossDown (PUT signal)
                            if o.entry_signal.contains("CrossUp") {
                                if ema_cut_position == "CrossDown" {
                                    should_sell = true;
                                    exit_reason = "EMA CrossDown (opposite of entry)".to_string();
                                } else if o.entry_signal == "AllCrossUp" && ema_cut_all_type == "AllCrossDown" {
                                    should_sell = true;
                                    exit_reason = "EMA AllCrossDown (opposite of entry)".to_string();
                                }
                            }
                            // Entry: CrossDown (PUT) → Exit: CrossUp (CALL signal)
                            else if o.entry_signal.contains("CrossDown") {
                                if ema_cut_position == "CrossUp" {
                                    should_sell = true;
                                    exit_reason = "EMA CrossUp (opposite of entry)".to_string();
                                } else if o.entry_signal == "AllCrossDown" && ema_cut_all_type == "AllCrossUp" {
                                    should_sell = true;
                                    exit_reason = "EMA AllCrossUp (opposite of entry)".to_string();
                                }
                            }
                            
                            if should_sell {
                                o.is_selling = true;
                                println!("🔀 [LongTerm] Sell on Next Cross! Contract {} - Entry: {}, Exit: {}", o.contract_id, o.entry_signal, exit_reason);
                                let sell_req = json!({ "sell": o.contract_id, "price": 0 });
                                let _ = ws_write.send(Message::Text(sell_req.to_string())).await;
                                let _ = tx.send(json!({ 
                                    "type": "lt_log",
                                    "data": { 
                                        "message": format!("🔀 Sell on Next Cross - Contract {} | {}", o.contract_id, exit_reason)
                                    }
                                }));
                            }
                        }
                        let _ = fs::write(tracking_file, serde_json::to_string_pretty(&orders).unwrap_or_default());
                        let _ = tx.send(json!({ "type": "lt_orders_update", "data": orders }));
                    }
                }
            }
            msg_opt = ws_read.next() => {
                let msg = match msg_opt {
                    Some(Ok(m)) => m,
                    _ => break,
                };
                if let Message::Text(text) = msg {
                    let parsed: serde_json::Value = match serde_json::from_str(&text) {
                        Ok(p) => p,
                        Err(_) => continue,
                    };
                    
                    if let Some(error) = parsed.get("error") {
                        eprintln!("❌ [LongTerm Private] Deriv Error: {:?}", error);
                        if let Some(echo) = parsed.get("echo_req") {
                            if let Some(sell_cid) = echo.get("sell").and_then(|v| json_to_i64(v)) {
                                if let Some(o) = orders.iter_mut().find(|o| o.contract_id == sell_cid) {
                                    o.is_selling = false;
                                }
                            }
                        }
                        continue;
                    }

                    let msg_type = parsed.get("msg_type").and_then(|v| v.as_str()).unwrap_or("");
                    match msg_type {
                        "authorize" => {
                            // We use OTP so explicit authorize msg shouldn't be received here, 
                            // but if it is, we already handle portfolio earlier.
                        },
                        "portfolio" => {
                            if let Some(port) = parsed.get("portfolio").and_then(|p| p.get("contracts")).and_then(|c| c.as_array()) {
                                is_portfolio_loaded = true;
                                let old_orders = orders.clone();
                                orders.clear();
                                for c in port {
                                    let cid = c.get("contract_id").and_then(|v| v.as_i64()).unwrap_or(0);
                                    let buy_price = c.get("buy_price").and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                                    let payout = c.get("payout").and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                                    let symbol = c.get("symbol").and_then(|v| v.as_str())
                                        .or_else(|| c.get("underlying_symbol").and_then(|v| v.as_str()))
                                        .unwrap_or("-").to_string();
                                    let contract_type = c.get("contract_type").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    
                                    let mut target = 0.0;
                                    let mut min_p = 0.0;
                                    let mut max_p = 0.0;
                                    let mut exit_strat = String::new();
                                    let mut entry_sig = String::new();
                                    let mut trade_code_val = String::from("MANUAL");
                                    let mut asset_val = String::new();
                                    if let Some(saved) = saved_tracking.get(&cid) {
                                        target = saved.target_profit;
                                        min_p = saved.min_profit;
                                        max_p = saved.max_profit;
                                        exit_strat = saved.exit_strategy.clone();
                                        entry_sig = saved.entry_signal.clone();
                                        trade_code_val = saved.trade_code.clone();
                                        asset_val = saved.asset.clone();
                                    }
                                    if asset_val.is_empty() {
                                        asset_val = map_symbol_to_asset(&symbol);
                                    }
                                    
                                    let mut is_selling = false;
                                    if let Some(old) = old_orders.iter().find(|o| o.contract_id == cid) {
                                        is_selling = old.is_selling;
                                    }
                                    
                                    let purchase_time = c.get("purchase_time").and_then(|v| v.as_i64()).unwrap_or(0);
                                    let date_start = c.get("date_start").and_then(|v| v.as_i64()).unwrap_or(0);
                                    let date_expiry = c.get("expiry_time").and_then(|v| v.as_i64()).unwrap_or(0);
                                    
                                    orders.push(LtOrder {
                                        contract_id: cid,
                                        buy_price,
                                        payout,
                                        time: chrono::Local::now().to_rfc3339(),
                                        target_profit: target,
                                        symbol: symbol,
                                        contract_type: contract_type,
                                        min_profit: min_p,
                                        max_profit: max_p,
                                        profit: 0.0,
                                        current_spot: 0.0,
                                        entry_spot: 0.0,
                                        is_selling,
                                        underlying: "".to_string(),
                                        display_name: "".to_string(),
                                        purchase_time,
                                        date_start,
                                        date_expiry,
                                        exit_strategy: exit_strat,
                                        entry_signal: entry_sig,
                                        trade_code: trade_code_val,
                                        asset: asset_val,
                                        ema_periods: saved_tracking.get(&cid).map(|s| s.ema_periods.clone()).unwrap_or_else(get_current_ema_periods_string),
                                    });
                                    
                                    // subscribe to proposal_open_contract
                                    let sub_req = json!({ "proposal_open_contract": 1, "contract_id": cid, "subscribe": 1 });
                                    let _ = ws_write.send(Message::Text(sub_req.to_string())).await;
                                }
                                let _ = fs::write(tracking_file, serde_json::to_string_pretty(&orders).unwrap_or_default());
                                let _ = tx.send(json!({ "type": "lt_orders_update", "data": orders }));
                            }
                        },
                        "buy" => {
                            if let Some(buy) = parsed.get("buy") {
                                let cid = buy.get("contract_id").and_then(|v| v.as_i64()).unwrap_or(0);
                                let buy_price = buy.get("buy_price").and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                                let payout = buy.get("payout").and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                                
                                let req_params = parsed.get("echo_req").and_then(|r| r.get("parameters"));
                                let symbol = req_params.and_then(|p| p.get("symbol")).and_then(|v| v.as_str())
                                    .or_else(|| req_params.and_then(|p| p.get("underlying_symbol")).and_then(|v| v.as_str()))
                                    .unwrap_or("-").to_string();
                                let contract_type = req_params.and_then(|p| p.get("contract_type")).and_then(|v| v.as_str()).unwrap_or("-").to_string();
                                
                                let req_passthrough = parsed.get("passthrough").or_else(|| parsed.get("echo_req").and_then(|r| r.get("passthrough")));
                                let mut target_profit = req_passthrough.and_then(|p| p.get("target_profit")).and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                                let exit_strategy = req_passthrough.and_then(|p| p.get("exit_strategy")).and_then(|v| v.as_str()).unwrap_or("targetProfit").to_string();
                                let entry_signal = req_passthrough.and_then(|p| p.get("entry_signal")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let trade_code = req_passthrough.and_then(|p| p.get("trade_code")).and_then(|v| v.as_str()).unwrap_or("MANUAL").to_string();
                                let asset_code_parsed = req_passthrough.and_then(|p| p.get("asset")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                                
                                if target_profit <= 0.0 {
                                    // Auto set target profit to +30% as default if not provided
                                    target_profit = buy_price * 0.3;
                                }
                                
                                let purchase_time = buy.get("purchase_time").and_then(|v| v.as_i64()).unwrap_or(0);
                                let date_start = buy.get("start_time").and_then(|v| v.as_i64()).unwrap_or(0);
                                let duration = req_params.and_then(|p| p.get("duration")).and_then(|v| v.as_i64()).unwrap_or(0);
                                
                                println!("🟢 [LongTerm] เปิดเทรดสำเร็จ! | Asset: {} | Type: {} | Money: ${:.2} | Duration: {}s", symbol, contract_type, buy_price, duration);
                                println!("🎯 [LongTerm] New order - Exit Strategy: {}, Entry Signal: {}, Trade Code: {}, Asset: {}", exit_strategy, entry_signal, trade_code, asset_code_parsed);
                                
                                orders.push(LtOrder {
                                    contract_id: cid,
                                    buy_price,
                                    payout,
                                    time: chrono::Local::now().to_rfc3339(),
                                    target_profit,
                                    symbol: symbol,
                                    contract_type: contract_type,
                                    min_profit: 0.0,
                                    max_profit: 0.0,
                                    profit: 0.0,
                                    current_spot: 0.0,
                                    entry_spot: 0.0,
                                    is_selling: false,
                                    underlying: "".to_string(),
                                    display_name: "".to_string(),
                                    purchase_time,
                                    date_start,
                                    date_expiry: 0,
                                    exit_strategy,
                                    entry_signal,
                                    trade_code,
                                    asset: asset_code_parsed,
                                    ema_periods: get_current_ema_periods_string(),
                                });
                                
                                // subscribe
                                let sub_req = json!({ "proposal_open_contract": 1, "contract_id": cid, "subscribe": 1 });
                                let _ = ws_write.send(Message::Text(sub_req.to_string())).await;
                                let _ = fs::write(tracking_file, serde_json::to_string_pretty(&orders).unwrap_or_default());
                                let _ = tx.send(json!({ "type": "lt_orders_update", "data": orders }));
                            }
                        },
                        "proposal_open_contract" => {
                            if let Some(poc) = parsed.get("proposal_open_contract") {
                                let cid = poc.get("contract_id").and_then(|v| v.as_i64()).unwrap_or(0);
                                let is_sold = poc.get("is_sold").and_then(|v| v.as_i64()).unwrap_or(0) == 1;
                                
                                if is_sold {
                                    // บันทึกข้อมูลก่อนลบ order
                                    if let Some(order) = orders.iter().find(|o| o.contract_id == cid) {
                                        // Update martingale loss_con for all trades
                                        let mut loss_con_val = 0u32;
                                        {
                                            let asset_name = if order.asset.is_empty() {
                                                map_symbol_to_asset(&order.symbol)
                                            } else {
                                                order.asset.clone()
                                            };
                                            
                                            let profit = poc.get("profit").and_then(|v| json_to_f64(v)).unwrap_or(order.profit);
                                            let is_win = profit > 0.0;
                                            
                                            let setup = crate::get_or_create_lt_setup();
                                            let max_lc = setup.max_loss_con;
                                            
                                            let loss_con = loss_con_map.entry(asset_name.clone()).or_insert(0);
                                            if is_win {
                                                *loss_con = 0;
                                                println!("🎉 [LongTerm Martingale] Win on {} - resetting loss_con to 0", asset_name);
                                            } else {
                                                *loss_con += 1;
                                                println!("😢 [LongTerm Martingale] Loss on {} - loss_con is now {}", asset_name, *loss_con);
                                                
                                                if max_lc > 0 && *loss_con >= max_lc {
                                                    *loss_con = 0;
                                                    println!("🏳️ [LongTerm Martingale] Max loss con reached ({} >= {}) on {} - resetting martingale", *loss_con, max_lc, asset_name);
                                                }
                                            }
                                            loss_con_val = *loss_con;
                                        }
                                        
                                        save_lt_trade_to_history(order, poc, loss_con_val);
                                    }
                                    
                                    orders.retain(|o| o.contract_id != cid);
                                    let _ = fs::write(tracking_file, serde_json::to_string_pretty(&orders).unwrap_or_default());
                                    let _ = tx.send(json!({ "type": "lt_orders_update", "data": orders }));
                                    continue;
                                }
                                
                                let profit = poc.get("profit").and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                                let current_spot = poc.get("current_spot").and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                                let entry_spot = poc.get("entry_spot").and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                                let underlying = poc.get("underlying").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let display_name = poc.get("display_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let purchase_time = poc.get("purchase_time").and_then(|v| v.as_i64()).unwrap_or(0);
                                let date_start = poc.get("date_start").and_then(|v| v.as_i64()).unwrap_or(0);
                                let date_expiry = poc.get("date_expiry").and_then(|v| v.as_i64()).unwrap_or(0);
                                
                                let mut updated = false;
                                let mut needs_sell = false;
                                let mut sell_reason = String::new();
                                
                                if let Some(o) = orders.iter_mut().find(|o| o.contract_id == cid) {
                                    o.profit = profit;
                                    o.current_spot = current_spot;
                                    
                                    if o.entry_spot == 0.0 && entry_spot > 0.0 {
                                        o.min_profit = profit;
                                        o.max_profit = profit;
                                    } else if entry_spot > 0.0 {
                                        if profit < o.min_profit { o.min_profit = profit; }
                                        if profit > o.max_profit { o.max_profit = profit; }
                                    }

                                    o.entry_spot = entry_spot;
                                    if !underlying.is_empty() { o.underlying = underlying.clone(); }
                                    if !display_name.is_empty() { o.display_name = display_name.clone(); }
                                    if purchase_time > 0 { o.purchase_time = purchase_time; }
                                    if date_start > 0 { o.date_start = date_start; }
                                    if date_expiry > 0 { o.date_expiry = date_expiry; }
                                    
                                    // เช็คว่าควรขายหรือไม่ (ถ้ายังไม่ได้ขายอยู่)
                                    if !o.is_selling && entry_spot > 0.0 {
                                        // Strategy 1: Target Profit
                                        if o.exit_strategy == "targetProfit" && profit > 0.0 && profit >= o.target_profit {
                                            needs_sell = true;
                                            sell_reason = format!("Target Profit (${:.2})", profit);
                                        }
                                        // Strategy 2: Sell on Next Cross
                                        else if o.exit_strategy == "nextCross" && !o.entry_signal.is_empty() {
                                            // ต้องรอให้มี EMA analysis data
                                            // เราจะเช็คจาก asset_states ที่อยู่ในฝั่ง public bot
                                            // ตอนนี้ใช้วิธีดึงจาก global state ผ่าน underlying
                                            // หรือส่ง analysis data มาจาก frontend
                                            // แต่เพื่อความง่าย เราจะ subscribe analysis data ผ่าน WebSocket
                                            
                                            // สำหรับตอนนี้ ให้ส่ง request ขอ analysis ล่าสุด
                                            // (ต้องเพิ่ม logic ดึง EMA analysis จาก public bot)
                                        }
                                    }
                                    
                                    if needs_sell {
                                        o.is_selling = true;
                                        println!("🎯 [LongTerm] Auto-sell Contract {} - Reason: {}", cid, sell_reason);
                                    }
                                    updated = true;
                                }
                                
                                if needs_sell {
                                    let sell_req = json!({ "sell": cid, "price": 0 });
                                    let _ = ws_write.send(Message::Text(sell_req.to_string())).await;
                                }
                                
                                if updated {
                                    let _ = fs::write(tracking_file, serde_json::to_string_pretty(&orders).unwrap_or_default());
                                    let _ = tx.send(json!({ "type": "lt_orders_update", "data": orders }));
                                }
                            }
                        },
                        "sell" => {
                            if let Some(sell) = parsed.get("sell") {
                                let cid = sell.get("contract_id").and_then(|v| v.as_i64()).unwrap_or(0);
                                
                                // บันทึกข้อมูลก่อนลบ order
                                if let Some(order) = orders.iter().find(|o| o.contract_id == cid) {
                                    // Update martingale loss_con for all trades
                                    let mut loss_con_val = 0u32;
                                    {
                                        let asset_name = if order.asset.is_empty() {
                                            map_symbol_to_asset(&order.symbol)
                                        } else {
                                            order.asset.clone()
                                        };
                                        
                                        let profit = sell.get("profit").and_then(|v| json_to_f64(v)).unwrap_or(order.profit);
                                        let is_win = profit > 0.0;
                                        
                                        let setup = crate::get_or_create_lt_setup();
                                        let max_lc = setup.max_loss_con;
                                        
                                        let loss_con = loss_con_map.entry(asset_name.clone()).or_insert(0);
                                        if is_win {
                                            *loss_con = 0;
                                            println!("🎉 [LongTerm Martingale] Win on {} - resetting loss_con to 0", asset_name);
                                        } else {
                                            *loss_con += 1;
                                            println!("😢 [LongTerm Martingale] Loss on {} - loss_con is now {}", asset_name, *loss_con);
                                            
                                            if max_lc > 0 && *loss_con >= max_lc {
                                                *loss_con = 0;
                                                println!("🏳️ [LongTerm Martingale] Max loss con reached ({} >= {}) on {} - resetting martingale", *loss_con, max_lc, asset_name);
                                            }
                                        }
                                        loss_con_val = *loss_con;
                                    }
                                    
                                    save_lt_trade_to_history(order, sell, loss_con_val);
                                }
                                
                                orders.retain(|o| o.contract_id != cid);
                                let _ = fs::write(tracking_file, serde_json::to_string_pretty(&orders).unwrap_or_default());
                                let _ = tx.send(json!({ "type": "lt_orders_update", "data": orders }));
                            }
                        },
                        _ => {}
                    }
                }
            }
        }
    }
    // Inner loop broke (WebSocket disconnected) — auto-reconnect
    private_retry_count += 1;
    let delay = std::cmp::min(5 * private_retry_count, 30);
    eprintln!("⚠️ [LongTerm Private] WebSocket disconnected. Reconnecting in {}s... (retry #{})", delay, private_retry_count);
    let _ = tx.send(json!({
        "type": "lt_ws_status",
        "target": "private",
        "status": "reconnecting"
    }));
    let _ = tx.send(json!({
        "type": "lt_log",
        "data": { "message": format!("🔄 [Private Bot] Disconnected. Reconnecting in {}s...", delay) }
    }));
    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
    } // end 'private_reconnect loop
}

fn load_longterm_config() -> Option<full_analysis_ver2::AppConfigPayload> {
    let content = std::fs::read_to_string("setup/longterm_settings.json").ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    
    let ema_json = json.get("ema")?;
    let short_period = ema_json.get("shortPeriod").and_then(|v| v.as_u64()).unwrap_or(9) as u32;
    let short_type = ema_json.get("shortType").and_then(|v| v.as_str()).unwrap_or("ema").to_string();
    let medium_period = ema_json.get("mediumPeriod").and_then(|v| v.as_u64()).unwrap_or(21) as u32;
    let medium_type = ema_json.get("mediumType").and_then(|v| v.as_str()).unwrap_or("ema").to_string();
    let long_period = ema_json.get("longPeriod").and_then(|v| v.as_u64()).unwrap_or(50) as u32;
    let long_type = ema_json.get("longType").and_then(|v| v.as_str()).unwrap_or("ema").to_string();
    
    let indicators_json = json.get("indicators")?;
    
    let config = full_analysis_ver2::AppConfigPayload {
        ema: full_analysis_ver2::EmaConfigGroup {
            short: full_analysis_ver2::EmaConfig {
                period: short_period,
                ema_type: short_type,
            },
            medium: full_analysis_ver2::EmaConfig {
                period: medium_period,
                ema_type: medium_type,
            },
            long: full_analysis_ver2::EmaConfig {
                period: long_period,
                ema_type: long_type,
            },
        },
        indicators: serde_json::from_value(indicators_json.clone()).ok()?,
    };
    
    Some(config)
}

fn get_longterm_analysis_config() -> Option<full_analysis_ver2::AppConfigPayload> {
    if let Some(cfg) = load_longterm_config() {
        return Some(cfg);
    }
    // Fallback to setup.json
    if let Ok(content) = std::fs::read_to_string("setup/setup.json") {
        if let Ok(c) = serde_json::from_str::<full_analysis_ver2::AppConfigPayload>(&content) {
            return Some(c);
        }
    }
    None
}

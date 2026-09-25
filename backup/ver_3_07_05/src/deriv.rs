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

use crate::analysis::AnalysisEngine;
use crate::get_action::{get_suggest_color, AnalysisObject, TradeAction};
use crate::AppConfigPayload;

fn json_to_f64(val: &serde_json::Value) -> Option<f64> {
    val.as_f64().or_else(|| val.as_str()?.parse::<f64>().ok())
}

fn json_to_i64(val: &serde_json::Value) -> Option<i64> {
    val.as_i64().or_else(|| val.as_str()?.parse::<i64>().ok())
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

fn map_asset_to_symbol(asset: &str) -> String {
    match asset {
        "vol10" => "R_10",
        "vol10_1s" => "1HZ10V",
        "vol15_1s" => "1HZ15V",
        "vol25" => "R_25",
        "vol25_1s" => "1HZ25V",
        "vol30_1s" => "1HZ30V",
        "vol50" => "R_50",
        "vol50_1s" => "1HZ50V",
        "vol75" => "R_75",
        "vol75_1s" => "1HZ75V",
        "vol90_1s" => "1HZ90V",
        "vol100" => "R_100",
        "vol100_1s" => "1HZ100V",
        _ => asset,
    }
    .to_string()
}

fn get_duration_params(granularity: i32) -> (i32, String) {
    let duration_seconds = (granularity - 5).max(1);
    (duration_seconds, "s".to_string())
}

pub async fn get_deriv_balance(
    api_token: &str,
) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
    let ws_url = "wss://ws.binaryws.com/websockets/v3?app_id=1089";
    let (mut ws_stream, _) = connect_async(ws_url).await?;
    let auth_request = json!({ "authorize": api_token });
    ws_stream
        .send(Message::Text(auth_request.to_string()))
        .await?;

    while let Some(msg_result) = ws_stream.next().await {
        let msg = msg_result?;
        if let Message::Text(text) = msg {
            let parsed: serde_json::Value = serde_json::from_str(&text)?;
            if let Some(auth_data) = parsed.get("authorize") {
                if let Some(bal) = json_to_f64(auth_data.get("balance").unwrap_or(&json!(0.0))) {
                    return Ok(bal);
                }
            } else if let Some(error) = parsed.get("error") {
                return Err(error
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Deriv Auth Error")
                    .into());
            }
        }
    }
    Err("Failed to get balance from Deriv".into())
}

// ไม่ใช้ hash แล้ว — ใช้ sequential req_id แทน (ดูใน start_deriv_bot_multiplexed)

pub struct AssetState {
    pub asset: String,
    pub deriv_symbol: String,
    pub duration: i32,
    pub duration_unit: String,
    pub req_id: i64,

    pub current_history: Vec<OHLCV>,
    pub loss_con: u32,
    pub is_trading: bool,
    pub current_balance: f64,

    pub trade_no: u32,
    pub sub_trade_no: u32,
    pub last_trade_details: Option<serde_json::Value>,
    pub last_trade_epoch: i64,
    pub in_martingale: bool,
    pub overtime_mode: bool,

    pub analysis_engine: Option<AnalysisEngine>,
    pub last_tick_analysis: Option<AnalysisObject>,

    pub trade_buy_instant: Option<std::time::Instant>,
    pub active_contract_id: Option<i64>,
    pub last_processed_contract_id: Option<String>,
    pub pending_proposal_data: Option<serde_json::Value>, // เก็บข้อมูล contract_type สำหรับ Method 2 (Proposal→Buy)
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
            is_trading: false,
            current_balance: 0.0,
            trade_no: 1,
            sub_trade_no: 1,
            last_trade_details: None,
            last_trade_epoch: 0,
            in_martingale: false,
            overtime_mode: false,
            analysis_engine: None,
            last_tick_analysis: None,
            trade_buy_instant: None,
            active_contract_id: None,
            last_processed_contract_id: None,
            pending_proposal_data: None,
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

pub async fn start_deriv_bot_multiplexed(
    app_id: String,
    api_token: String,
    config: AppConfigPayload,
    assets: Vec<String>,
    tx: Arc<broadcast::Sender<serde_json::Value>>,
    mut cmd_rx: broadcast::Receiver<serde_json::Value>,
    overtime_signals: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_url = format!("wss://ws.binaryws.com/websockets/v3?app_id={}", app_id);
    println!(
        "\n🔌 [Multiplex] กำลังเชื่อมต่อ Deriv WebSocket สำหรับ {} assets...",
        assets.len()
    );

    let (mut ws_stream, _) = connect_async(&ws_url).await?;
    println!("✅ [Multiplex] เชื่อมต่อ WebSocket สำเร็จ!");

    let mut asset_states: HashMap<String, AssetState> = HashMap::new();
    let mut req_id_to_asset: HashMap<i64, String> = HashMap::new();
    for (idx, asset) in assets.iter().enumerate() {
        let base_req_id = (idx as i64 + 1) * 1000; // 1000, 2000, 3000...
        let mut state = AssetState::new(asset.clone(), config.granularity);
        state.req_id = base_req_id;
        println!(
            "📋 [Multiplex] Asset: {} → symbol: {} | req_id_base: {} (ohlc:{}, proposal:{}, buy:{}, track:{})",
            asset, state.deriv_symbol, state.req_id,
            base_req_id, base_req_id + 1, base_req_id + 2, base_req_id + 3
        );
        // Register ALL req_id offsets for this asset (เหมือน JS ที่ใช้ reqId++ แยกกัน)
        req_id_to_asset.insert(base_req_id, asset.clone());      // +0 = ohlc/ticks_history
        req_id_to_asset.insert(base_req_id + 1, asset.clone());  // +1 = proposal
        req_id_to_asset.insert(base_req_id + 2, asset.clone());  // +2 = buy
        req_id_to_asset.insert(base_req_id + 3, asset.clone());  // +3 = proposal_open_contract (track)
        asset_states.insert(asset.clone(), state);
    }

    let auth_request = json!({ "authorize": api_token });
    ws_stream
        .send(Message::Text(auth_request.to_string()))
        .await?;

    loop {
        tokio::select! {
            cmd_result = cmd_rx.recv() => {
                match cmd_result {
                    Ok(cmd) => {
                        if cmd["command"] == "sell" {
                            if let Some(cid) = cmd["contract_id"].as_i64() {
                                let sell_req = json!({ "sell": cid, "price": 0 });
                                let _ = ws_stream.send(Message::Text(sell_req.to_string())).await;
                                println!("📤 [Multiplex] ส่งคำสั่งขาย Contract ID: {}", cid);
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
            }
            msg_result = ws_stream.next() => {
                let msg_data = match msg_result {
                    Some(Ok(m)) => m,
                    _ => break,
                };
                let mut current_config = config.clone();
                if let Ok(content) = tokio::fs::read_to_string("setup.json").await {
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
            if msg_type != "ohlc" && msg_type != "tick" {
                println!("\n📨 [Multiplex] Deriv msg_type: {}", msg_type);
            }

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

                    let ticks_request = json!({
                        "ticks_history": state.deriv_symbol,
                        "end": "latest",
                        "style": "candles",
                        "granularity": config.granularity,
                        "count": candle_count,
                        "subscribe": 1,
                        "req_id": state.req_id
                    });

                    println!("📦 [{}] ร้องขอข้อมูลแท่งเทียน", asset);
                    ws_stream
                        .send(Message::Text(ticks_request.to_string()))
                        .await?;
                }

                let tx_sub = json!({ "transaction": 1, "subscribe": 1, "req_id": 11111 });
                println!("📡 [Multiplex] Subscribe transaction stream");
                ws_stream.send(Message::Text(tx_sub.to_string())).await?;
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

                    let mut engine = AnalysisEngine::new(&current_config);
                    let analysis_result =
                        engine.process_history(&state.current_history, &current_config, &state.asset);
                    state.analysis_engine = Some(engine);

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

                        if let Some(ref mut engine) = state.analysis_engine {
                            let analysis_obj = engine.update_tick(
                                state.current_history.last().unwrap(),
                                is_new_candle,
                                current_config.indicators.atr_multi,
                                &state.asset,
                            );
                            let latest_analysis = &analysis_obj;
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
                                    let log_asset_file = format!("{}/asset.jsonl", dir_path_asset);
                                    if let Ok(mut file) = fs::OpenOptions::new()
                                        .create(true)
                                        .append(true)
                                        .open(&log_asset_file)
                                    {
                                        if let Ok(json_str) = serde_json::to_string(prev_analysis) {
                                            let _ = writeln!(file, "{}", json_str);
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
                                                ws_stream.send(Message::Text(sell_req.to_string())).await?;
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
                                            ws_stream
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
                                    let should_trade = if state.in_martingale {
                                        true
                                    } else {
                                        prev.is_atr
                                    };

                                    if should_trade {
                                        let suggest =
                                            get_suggest_color(&prev.this_color, state.loss_con);
                                        let trade_action = if suggest == "green" {
                                            TradeAction::Call
                                        } else {
                                            TradeAction::Put
                                        };

                                        println!(
                                            "\n📈 [{}] SIGNAL: {:?} | LossCon: {}",
                                            state.asset, trade_action, state.loss_con
                                        );
                                        state.is_trading = true;
                                        state.last_trade_epoch = latest_analysis.epoch;
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

                                        let contract_type = if trade_action == TradeAction::Call {
                                            "CALL"
                                        } else {
                                            "PUT"
                                        };

                                        state.last_trade_details = Some(json!({
                                            "timeCandle": latest_analysis.epoch,
                                            "timeCandleDisplay": chrono::DateTime::from_timestamp(latest_analysis.epoch, 0).unwrap().with_timezone(&Local).format("%d/%m/%Y %H:%M:%S").to_string(),
                                            "thisColor": prev.this_color,
                                            "thisAction": format!("{:?}", trade_action),
                                            "targetColor": if trade_action == TradeAction::Call { "green" } else { "red" },
                                            "emaShortDirection": prev.ema_short_direction,
                                            "emaMediumDirection": prev.ema_medium_direction,
                                            "MoneyTrade": amount,
                                        }));

                                        // ═══ Buy Method: เลือกระหว่าง direct vs proposal ═══
                                        let buy_method = current_config.trade.buy_method.as_str();

                                        if buy_method == "proposal" {
                                            // ── Method 2: Proposal → Buy ── (เหมือน JS: sendProposal → buyContract)
                                            // ส่ง proposal request ก่อน เมื่อได้ proposal_id จึง buy
                                            state.pending_proposal_data = Some(json!({
                                                "contract_type": contract_type,
                                                "amount": amount
                                            }));

                                            // ใช้ req_id+1 สำหรับ proposal (แยกจาก ohlc ที่ใช้ req_id+0)
                                            let proposal_req_id = state.req_id + 1;
                                            let proposal_request = json!({
                                                "proposal": 1,
                                                "amount": amount,
                                                "basis": "stake",
                                                "contract_type": contract_type,
                                                "currency": "USD",
                                                "duration": state.duration,
                                                "duration_unit": state.duration_unit,
                                                "symbol": state.deriv_symbol,
                                                "req_id": proposal_req_id
                                            });

                                            println!(
                                                "📋 [{}] [Method 2] ส่ง Proposal request (req_id={}): {:?}",
                                                state.asset, proposal_req_id, proposal_request
                                            );
                                            ws_stream
                                                .send(Message::Text(proposal_request.to_string()))
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
                                                    "symbol": state.deriv_symbol
                                                },
                                                "req_id": buy_req_id
                                            });

                                            println!(
                                                "🚀 [{}] [Method 1] ยิงคำสั่งเทรด Direct Buy (req_id={}): {:?}",
                                                state.asset, buy_req_id, buy_request
                                            );
                                            ws_stream
                                                .send(Message::Text(buy_request.to_string()))
                                                .await?;
                                        }
                                    }
                                }
                            }

                            state.last_tick_analysis = Some(analysis_obj.clone());
                        }
                    }
                }
            }

            // ═══ Handle Proposal Response (Method 2: Proposal → Buy) ═══
            // เหมือน JS: case "proposal" → buyContract(data.proposal.id, data.proposal.ask_price)
            if msg_type == "proposal" {
                if let Some(proposal_data) = parsed.get("proposal") {
                    let req_id = parsed
                        .get("echo_req")
                        .and_then(|r| r.get("req_id"))
                        .and_then(|r| r.as_i64())
                        .unwrap_or(0);
                    let asset_key = req_id_to_asset.get(&req_id).cloned().unwrap_or_default();

                    println!(
                        "📬 [Proposal] req_id={} → asset_key='{}' | is proposal req_id: {}",
                        req_id, asset_key, req_id % 1000 == 1
                    );

                    if let Some(state) = asset_states.get_mut(&asset_key) {
                        // ═══ สำคัญ: ต้องเช็ค pending_proposal_data ก่อน ═══
                        // ถ้า pending_proposal_data == None แปลว่า buy ถูกส่งไปแล้ว
                        // Deriv จะ stream proposal updates ต่อเนื่อง ต้อง skip ที่ซ้ำ
                        if state.is_trading && state.pending_proposal_data.is_some() {
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

                            println!(
                                "\n📋 [{}] [Method 2] ได้ Proposal! id={} | ask_price={:.2} | payout={:.2}",
                                state.asset, proposal_id, ask_price, payout
                            );

                            if proposal_id.is_empty() {
                                eprintln!("❌ [{}] Proposal ID ว่าง — ยกเลิกการ buy", state.asset);
                                state.is_trading = false;
                                state.trade_buy_instant = None;
                                state.pending_proposal_data = None;
                                state.last_trade_details = None;
                            } else {
                                // ═══ Step 1: Clear pending_proposal_data ทันที ═══
                                // เพื่อป้องกัน proposal streaming update ซ้ำ ส่ง buy ซ้ำอีกรอบ
                                state.pending_proposal_data = None;

                                // ═══ Step 2: ส่งคำสั่ง Buy ด้วย proposal_id ก่อน ═══
                                // ⚠️ ต้องส่ง buy ก่อน forget_all!
                                // เพราะ forget_all จะยกเลิก proposal_id ทำให้ buy ไม่ได้
                                // (เหมือน JS: buyContract(proposalId, askPrice) → แล้วค่อย forget)
                                let buy_req_id = state.req_id + 2;
                                let buy_request = json!({
                                    "buy": proposal_id,
                                    "price": ask_price,
                                    "req_id": buy_req_id
                                });

                                println!(
                                    "🚀 [{}] [Method 2] ส่ง Buy ด้วย Proposal ID: {} (req_id={})",
                                    state.asset, proposal_id, buy_req_id
                                );
                                ws_stream
                                    .send(Message::Text(buy_request.to_string()))
                                    .await?;

                                // ═══ Step 3: ส่ง forget_all หลัง buy ═══
                                // (หยุด proposal streaming หลังจาก buy ส่งไปแล้ว)
                                let forget_req = json!({
                                    "forget_all": "proposal"
                                });
                                ws_stream
                                    .send(Message::Text(forget_req.to_string()))
                                    .await?;
                                println!(
                                    "🧹 [{}] [Method 2] ส่ง forget_all proposal หลัง buy",
                                    state.asset
                                );
                            }
                        } else if state.is_trading && state.pending_proposal_data.is_none() {
                            // Proposal streaming update มาหลัง buy — skip
                            println!(
                                "⏭️ [{}] [Method 2] Skip proposal update (buy ถูกส่งไปแล้ว)",
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
                            // ดึง contract_type: ถ้า Method 1 จะอยู่ใน echo_req.parameters, ถ้า Method 2 จะอยู่ใน pending_proposal_data
                            let ctype = parsed.get("echo_req")
                                .and_then(|r| r.get("parameters"))
                                .and_then(|p| p.get("contract_type"))
                                .and_then(|c| c.as_str())
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| {
                                    state.pending_proposal_data.as_ref()
                                        .and_then(|d| d.get("contract_type"))
                                        .and_then(|c| c.as_str())
                                        .unwrap_or("?")
                                        .to_string()
                                });
                            // Clear pending proposal data หลัง buy สำเร็จ
                            state.pending_proposal_data = None;
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
                        println!(
                            "📡 [{}] Track Order: contract_id={} (req_id={})",
                            state.asset, cid_num, track_req_id
                        );
                        ws_stream
                            .send(Message::Text(sub_proposal.to_string()))
                            .await?;
                    } else {
                        state.is_trading = false;
                        state.trade_buy_instant = None;
                    }
                }
            }

            if let Some(tx_data) = parsed.get("transaction") {
                if let Some(action) = tx_data.get("action").and_then(|a| a.as_str()) {
                    let tx_contract_id = tx_data
                        .get("contract_id")
                        .and_then(|v| json_to_i64(v))
                        .unwrap_or(0);
                    let tx_balance = tx_data
                        .get("balance")
                        .and_then(|v| json_to_f64(v))
                        .unwrap_or(0.0);

                    if action == "sell" {
                        let asset_key = find_asset_by_contract_id(&asset_states, tx_contract_id);
                        if let Some(state) = asset_states.get_mut(&asset_key) {
                            if state.is_trading {
                                println!(
                                    "\n🔔 [{}] TRANSACTION SELL detected! contract_id={}",
                                    state.asset, tx_contract_id
                                );
                                state.current_balance = tx_balance;
                                let _ = tx.send(json!({
                                    "type": "balance_update",
                                    "asset": state.asset.clone(),
                                    "data": { "balance": state.current_balance }
                                }));

                                let track_req_id = state.req_id + 3;
                                let query = json!({
                                    "proposal_open_contract": 1,
                                    "contract_id": tx_contract_id,
                                    "req_id": track_req_id
                                });
                                ws_stream.send(Message::Text(query.to_string())).await?;
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
                        let is_sold_val = match proposal.get("is_sold") {
                            Some(v) if v.is_i64() => v.as_i64().unwrap_or(0),
                            Some(v) if v.is_boolean() => {
                                if v.as_bool().unwrap_or(false) {
                                    1
                                } else {
                                    0
                                }
                            }
                            Some(v) if v.is_string() => {
                                let s = v.as_str().unwrap_or("0");
                                if s == "1" || s == "true" {
                                    1
                                } else {
                                    0
                                }
                            }
                            Some(v) if v.is_f64() => v.as_f64().unwrap_or(0.0) as i64,
                            _ => 0,
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
                            || contract_status == "lost";

                        // ═══ Broadcast order_update ให้ Frontend (Order Tracker Tab) ═══
                        {
                            let oc_cid = proposal.get("contract_id").and_then(|v| json_to_i64(v)).unwrap_or(0);
                            let oc_buy = proposal.get("buy_price").and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                            let oc_sell = proposal.get("sell_price").and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                            let oc_payout = proposal.get("payout").and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                            let oc_ctype = proposal.get("contract_type").and_then(|v| v.as_str()).unwrap_or("");
                            let oc_ptime = proposal.get("purchase_time").and_then(|v| json_to_i64(v)).unwrap_or(0);
                            let oc_expiry = proposal.get("date_expiry").and_then(|v| json_to_i64(v)).unwrap_or(0);
                            let _ = tx.send(json!({
                                "type": "order_update",
                                "asset": state.asset.clone(),
                                "data": {
                                    "contract_id": oc_cid,
                                    "symbol": state.deriv_symbol.clone(),
                                    "asset": state.asset.clone(),
                                    "contract_type": oc_ctype,
                                    "buy_price": oc_buy,
                                    "sell_price": oc_sell,
                                    "profit": current_profit,
                                    "payout": oc_payout,
                                    "status": contract_status,
                                    "is_sold": is_sold_val,
                                    "purchase_time": oc_ptime,
                                    "date_expiry": oc_expiry
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
                            let trade_record = json!({
                                "scheduleTradeNo": current_config.date_range.schedule_trade_no,
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
                                "GrandBalance": state.current_balance + profit
                            });

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

                            if profit > 0.0 {
                                println!("🎉 [{}] ชนะ! Reset LossCon = 0", state.asset);
                                state.loss_con = 0;
                                state.trade_no += 1;
                                state.sub_trade_no = 1;
                                state.in_martingale = false;

                                if current_config.trade.notify_telegram {
                                    let bot_token =
                                        env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
                                    let chat_id = env::var("TELEGRAM_CHAT_ID").unwrap_or_default();
                                    if !bot_token.is_empty() && !chat_id.is_empty() {
                                        let msg = format!("🎉 WIN Trade!\nAsset: {}\nProfit: +{:.2}\nBalance: {:.2}", state.deriv_symbol, profit, state.current_balance + profit);
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
                            } else {
                                state.loss_con += 1;
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
                            }

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
}
    }

    eprintln!("\n⚠️ [Multiplex] WebSocket loop ended.");
    Ok(())
}

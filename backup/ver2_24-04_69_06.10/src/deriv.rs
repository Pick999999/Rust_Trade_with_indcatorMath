use chrono::{Datelike, Local};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use turbo_indicators::OHLCV;

use crate::analysis::AnalysisEngine;
use crate::get_action::{get_suggest_color, AnalysisObject, TradeAction};
use crate::AppConfigPayload;

// Helper: อ่านค่า f64 จาก JSON ได้ทั้งแบบ Number และ String
fn json_to_f64(val: &serde_json::Value) -> Option<f64> {
    val.as_f64().or_else(|| val.as_str()?.parse::<f64>().ok())
}

fn json_to_i64(val: &serde_json::Value) -> Option<i64> {
    val.as_i64().or_else(|| val.as_str()?.parse::<i64>().ok())
}

// แปลง JSON ให้เป็น OHLCV (รองรับทั้ง historical candles และ live ohlc)
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

// ฟังก์ชันแปลง Asset ให้ตรงกับ Deriv API
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

// แปลงเลข granularity เป็น duration ในหน่วยวินาที
fn get_duration_params(granularity: i32) -> (i32, String) {
    let duration_seconds = (granularity - 3).max(1);
    (duration_seconds, "s".to_string())
}

/// ฟังก์ชันหลักสำหรับรัน WebSocket กับ Deriv.com
pub async fn start_deriv_bot(
    app_id: String,
    api_token: String,
    config: AppConfigPayload,
    asset: String,
    tx: Arc<broadcast::Sender<serde_json::Value>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_url = format!("wss://ws.binaryws.com/websockets/v3?app_id={}", app_id);
    println!("\n🔌 [{}] กำลังเชื่อมต่อ Deriv WebSocket...", asset);

    let (mut ws_stream, _) = connect_async(&ws_url).await?;
    println!("✅ [{}] เชื่อมต่อ WebSocket สำเร็จ!", asset);

    // 1. Authorize
    let auth_request = json!({ "authorize": api_token });
    ws_stream
        .send(Message::Text(auth_request.to_string()))
        .await?;

    let mut current_history: Vec<OHLCV> = Vec::new();
    let mut loss_con: u32 = 0;
    let mut is_trading: bool = false;
    let mut current_balance: f64 = 0.0;

    let mut trade_no: u32 = 1;
    let mut sub_trade_no: u32 = 1;
    let mut last_trade_details: Option<serde_json::Value> = None;
    let mut last_trade_epoch: i64 = 0;
    let mut in_martingale: bool = false;
    let mut analysis_engine: Option<AnalysisEngine> = None;
    let mut last_tick_analysis: Option<AnalysisObject> = None;

    // ── FIX: ตัวแปรสำหรับ timeout safety net ──
    let mut trade_buy_instant: Option<std::time::Instant> = None;
    let mut active_contract_id: Option<i64> = None;

    // ── FIX 1: Guard ป้องกัน double-process contract เดิม ──
    // Deriv ส่ง proposal_open_contract ซ้ำได้หลายครั้งหลัง is_sold=1
    let mut last_processed_contract_id: Option<String> = None;

    let deriv_symbol = map_asset_to_symbol(&asset);
    let (duration, duration_unit) = get_duration_params(config.granularity);

    // 2. Main loop
    while let Some(msg_result) = ws_stream.next().await {
        let mut current_config = config.clone();
        if let Ok(content) = fs::read_to_string("setup.json") {
            if let Ok(parsed_config) = serde_json::from_str::<AppConfigPayload>(&content) {
                current_config = parsed_config;
            }
        }

        let msg = msg_result?;
        if let Message::Text(text) = msg {
            let parsed: serde_json::Value = serde_json::from_str(&text)?;

            let msg_type = parsed
                .get("msg_type")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown");
            if msg_type != "ohlc" && msg_type != "tick" {
                println!(
                    "\n📨 [{}] Deriv msg_type: {} | is_trading: {}",
                    asset, msg_type, is_trading
                );
            }

            // ── Authorize สำเร็จ → ขอ candle history ──
            if let Some(auth_data) = parsed.get("authorize") {
                println!("\n🔐 [{}] Authorize สำเร็จ!", asset);

                if let Some(bal) = json_to_f64(auth_data.get("balance").unwrap_or(&json!(0.0))) {
                    current_balance = bal;
                    let _ = tx.send(json!({
                        "type": "balance_update",
                        "asset": asset,
                        "data": { "balance": current_balance }
                    }));
                }

                let candle_count: i32 = env::var("CANDLE_COUNT")
                    .unwrap_or_else(|_| "1000".to_string())
                    .parse()
                    .unwrap_or(1000);

                let ticks_request = json!({
                    "ticks_history": deriv_symbol,
                    "end": "latest",
                    "style": "candles",
                    "granularity": config.granularity,
                    "count": candle_count,
                    "subscribe": 1
                });

                println!("📦 [{}] ร้องขอข้อมูลแท่งเทียน: {}", asset, deriv_symbol);
                ws_stream
                    .send(Message::Text(ticks_request.to_string()))
                    .await?;

                // ── FIX: Subscribe transaction stream เป็น backup สำหรับตรวจจับ sell ──
                let tx_sub = json!({ "transaction": 1, "subscribe": 1 });
                println!("📡 [{}] Subscribe transaction stream (backup sell detection)", asset);
                ws_stream.send(Message::Text(tx_sub.to_string())).await?;
            }

            // ── Historical candles ──
            if let Some(candles) = parsed.get("candles") {
                if let Some(arr) = candles.as_array() {
                    for c in arr {
                        if let Some(ohlcv) = parse_candle(c) {
                            current_history.push(ohlcv);
                        }
                    }
                }

                println!(
                    "📊 [{}] ได้รับแท่งเทียนย้อนหลัง {} แท่ง (SIMD+StatefulATR)",
                    asset,
                    current_history.len()
                );

                let mut engine = AnalysisEngine::new(&current_config);
                let analysis_result = engine.process_history(&current_history, &current_config);
                analysis_engine = Some(engine);

                let _ = tx.send(json!({
                    "type": "candles_history",
                    "asset": asset,
                    "data": analysis_result
                }));
            }

            // ── Realtime OHLC update ──
            if let Some(ohlc) = parsed.get("ohlc") {
                if let Some(new_tick) = parse_candle(ohlc) {
                    let is_new_candle;
                    if let Some(last) = current_history.last_mut() {
                        if last.timestamp == new_tick.timestamp {
                            *last = new_tick;
                            is_new_candle = false;
                        } else {
                            current_history.push(new_tick);
                            is_new_candle = true;
                        }
                    } else {
                        current_history.push(new_tick);
                        is_new_candle = true;
                    }

                    print!(
                        "\r⏱ [{}] close={:.4} | {} | {} แท่ง          ",
                        asset,
                        new_tick.close,
                        if is_new_candle {
                            "🆕 New"
                        } else {
                            "📝 Upd"
                        },
                        current_history.len()
                    );
                    let _ = io::stdout().flush();

                    if let Some(ref mut engine) = analysis_engine {
                        let analysis_obj = engine.update_tick(
                            current_history.last().unwrap(),
                            is_new_candle,
                            current_config.indicators.atr_multi,
                        );
                        let latest_analysis = &analysis_obj;
                        let _ = tx.send(json!({
                            "type": "ohlc_update",
                            "asset": asset,
                            "data": latest_analysis
                        }));

                        // Log asset ราย tick
                        let now = Local::now();
                        let month_folder =
                            format!("{:02}-{:04}", now.month(), now.year() as i32 + 543);
                        let day_folder = format!(
                            "{:02}-{:02}-{:04}",
                            now.day(),
                            now.month(),
                            now.year() as i32 + 543
                        );
                        let dir_path_asset =
                            format!("tradeData/{}/{}/{}", month_folder, day_folder, deriv_symbol);
                        let _ = fs::create_dir_all(&dir_path_asset);
                        let log_asset_file = format!("{}/asset.jsonl", dir_path_asset);
                        if let Ok(mut file) = fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&log_asset_file)
                        {
                            if let Ok(json_str) = serde_json::to_string(latest_analysis) {
                                let _ = writeln!(file, "{}", json_str);
                            }
                        }

                        // ── FIX: Timeout safety net — ถ้า is_trading นานเกินไป ให้ query contract ──
                        if is_trading {
                            if let Some(ref buy_time) = trade_buy_instant {
                                let elapsed = buy_time.elapsed().as_secs();
                                let max_wait = (duration as u64) + 15;
                                if elapsed > max_wait {
                                    println!("\n⏰ [{}] TIMEOUT! is_trading=true มานาน {}s (max={}s) → force query contract", asset, elapsed, max_wait);
                                    if let Some(cid) = active_contract_id {
                                        let query = json!({
                                            "proposal_open_contract": 1,
                                            "contract_id": cid,
                                            "req_id": 88
                                        });
                                        println!("🔍 [{}] Force query contract_id={}", asset, cid);
                                        ws_stream.send(Message::Text(query.to_string())).await?;
                                    } else {
                                        println!("⚠️ [{}] TIMEOUT แต่ไม่มี contract_id → force reset is_trading", asset);
                                        is_trading = false;
                                        trade_buy_instant = None;
                                    }
                                }
                            }
                        }

                        // ══ Phase 4: ตัดสินใจเข้าเทรดที่ candle boundary ══
                        if is_new_candle && !is_trading {
                            if let Some(ref prev) = last_tick_analysis {
                                let should_trade = if in_martingale { true } else { prev.is_atr };

                                let candle_time =
                                    chrono::DateTime::from_timestamp(latest_analysis.epoch, 0)
                                        .map(|dt| {
                                            dt.with_timezone(&Local).format("%H:%M:%S").to_string()
                                        })
                                        .unwrap_or_default();

                                let log_msg = if should_trade {
                                    format!("🆕 [{}] {} | Spike: ✅ | สี: {} | Martingale: {} | LossCon: {} → 🚀 กำลังเข้าเทรด!",
                                        asset, candle_time, prev.this_color, in_martingale, loss_con)
                                } else {
                                    format!("🆕 [{}] {} | Spike: ❌ | สี: {} | Martingale: {} | LossCon: {} → ⏳ รอ Spike...",
                                        asset, candle_time, prev.this_color, in_martingale, loss_con)
                                };
                                println!("{}", log_msg);
                                let _ = tx.send(json!({
                                    "type": "bot_log",
                                    "asset": asset,
                                    "data": {
                                        "message": log_msg,
                                        "is_atr": prev.is_atr,
                                        "this_color": prev.this_color,
                                        "in_martingale": in_martingale,
                                        "loss_con": loss_con,
                                        "should_trade": should_trade,
                                        "epoch": latest_analysis.epoch
                                    }
                                }));

                                if should_trade {
                                    let suggest = get_suggest_color(&prev.this_color, loss_con);
                                    let trade_action = if suggest == "green" {
                                        TradeAction::Call
                                    } else {
                                        TradeAction::Put
                                    };

                                    println!("\n📈 [{}] SIGNAL (candle boundary :00): {:?} | PrevSpike: {} | PrevColor: {} | Martingale: {} | LossCon: {}",
                                        asset, trade_action, prev.is_atr, prev.this_color, in_martingale, loss_con);
                                    is_trading = true;
                                    last_trade_epoch = latest_analysis.epoch;
                                    trade_buy_instant = Some(std::time::Instant::now());
                                    active_contract_id = None;

                                    // ── FIX 2: Reset contract guard ทุกครั้งที่เริ่มไม้ใหม่ ──
                                    last_processed_contract_id = None;

                                    let mut amount = current_config.trade.target_lot;
                                    if current_config.trade.martingale.martingale_type
                                        == "martingale"
                                    {
                                        if let Some(m_amount) = current_config
                                            .trade
                                            .martingale
                                            .list
                                            .get(loss_con as usize)
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

                                    last_trade_details = Some(json!({
                                        "timeCandle": latest_analysis.epoch,
                                        "timeCandleDisplay": chrono::DateTime::from_timestamp(latest_analysis.epoch, 0)
                                            .unwrap()
                                            .with_timezone(&Local)
                                            .format("%d/%m/%Y %H:%M:%S")
                                            .to_string(),
                                        "thisColor": prev.this_color,
                                        "thisAction": format!("{:?}", trade_action),
                                        "targetColor": if trade_action == TradeAction::Call { "green" } else { "red" },
                                        "emaShortDirection": prev.ema_short_direction,
                                        "emaMediumDirection": prev.ema_medium_direction,
                                        "MoneyTrade": amount,
                                    }));

                                    let buy_request = json!({
                                        "buy": 1,
                                        "price": 100000,
                                        "parameters": {
                                            "amount": amount,
                                            "basis": "stake",
                                            "contract_type": contract_type,
                                            "currency": "USD",
                                            "duration": duration,
                                            "duration_unit": duration_unit,
                                            "symbol": deriv_symbol
                                        },
                                        "req_id": 99
                                    });

                                    let trade_log = format!(
                                        "🚀 [{}] {} | เข้าเทรด {:?} | Amount: ${} | Duration: {}{}",
                                        asset,
                                        candle_time,
                                        trade_action,
                                        amount,
                                        duration,
                                        duration_unit
                                    );
                                    let _ = tx.send(json!({
                                        "type": "bot_log",
                                        "asset": asset,
                                        "data": { "message": trade_log, "epoch": latest_analysis.epoch }
                                    }));

                                    println!(
                                        "🚀 [{}] ยิงคำสั่งเทรด (duration={}{}): {:?}",
                                        asset, duration, duration_unit, buy_request
                                    );
                                    ws_stream
                                        .send(Message::Text(buy_request.to_string()))
                                        .await?;
                                }
                            }
                        }

                        last_tick_analysis = Some(analysis_obj.clone());
                    }
                }
            }

            // ── Buy response → subscribe contract ──
            if let Some(buy_response) = parsed.get("buy") {
                if let Some(contract_id) = buy_response.get("contract_id") {
                    let cid_num = json_to_i64(contract_id).unwrap_or(0);
                    active_contract_id = Some(cid_num);
                    println!(
                        "\n✅ [{}] เข้าออเดอร์สำเร็จ Contract ID: {}",
                        asset, cid_num
                    );

                    if let Some(bal) = buy_response
                        .get("balance_after")
                        .and_then(|b| json_to_f64(b))
                    {
                        current_balance = bal;
                    }

                    // ใช้ตัวเลข contract_id เสมอ (ป้องกัน type mismatch)
                    let sub_proposal = json!({
                        "proposal_open_contract": 1,
                        "contract_id": cid_num,
                        "subscribe": 1
                    });
                    println!(
                        "📡 [{}] Subscribe proposal_open_contract: contract_id={}",
                        asset, cid_num
                    );
                    ws_stream
                        .send(Message::Text(sub_proposal.to_string()))
                        .await?;
                } else {
                    eprintln!(
                        "⚠️ [{}] Buy response ไม่มี contract_id: {:?}",
                        asset, buy_response
                    );
                    is_trading = false;
                    trade_buy_instant = None;
                }
            }

            // ── FIX: Transaction stream handler (backup sell detection) ──
            if let Some(tx_data) = parsed.get("transaction") {
                if let Some(action) = tx_data.get("action").and_then(|a| a.as_str()) {
                    let tx_contract_id = tx_data.get("contract_id").and_then(|v| json_to_i64(v)).unwrap_or(0);
                    let tx_amount = tx_data.get("amount").and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                    let tx_balance = tx_data.get("balance").and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                    println!("\n💰 [{}] Transaction: action={} | contract_id={} | amount={:.2} | balance={:.2}",
                        asset, action, tx_contract_id, tx_amount, tx_balance);

                    if action == "sell" && is_trading {
                        // เช็คว่าเป็น contract ที่เรากำลังเทรดอยู่
                        let is_our_contract = match active_contract_id {
                            Some(cid) => cid == tx_contract_id,
                            None => true, // ถ้าไม่มี contract_id ให้ถือว่าใช่
                        };
                        if is_our_contract {
                            println!("\n🔔 [{}] TRANSACTION SELL detected! contract_id={} | amount={:.2}", asset, tx_contract_id, tx_amount);
                            // อัพเดท balance จาก transaction (reliable source)
                            current_balance = tx_balance;
                            let _ = tx.send(json!({
                                "type": "balance_update",
                                "asset": asset,
                                "data": { "balance": current_balance }
                            }));
                            // ส่ง one-shot query เพื่อดึง profit/status
                            if tx_contract_id > 0 {
                                let query = json!({
                                    "proposal_open_contract": 1,
                                    "contract_id": tx_contract_id,
                                    "req_id": 77
                                });
                                println!("🔍 [{}] Query contract details after sell: contract_id={}", asset, tx_contract_id);
                                ws_stream.send(Message::Text(query.to_string())).await?;
                            }
                        }
                    }
                }
            }

            // ── ติดตามผลกำไร/ขาดทุน ──
            if let Some(proposal) = parsed.get("proposal_open_contract") {
                // Guard: ถ้า proposal เป็น null หรือไม่ใช่ object → ข้าม
                if !proposal.is_object() {
                    if is_trading {
                        println!("\n⚠️ [{}] proposal_open_contract is NOT object: {}", asset, proposal);
                    }
                    continue;
                }

                // ── FIX: รองรับ is_sold ทั้ง number, boolean, string ──
                let is_sold_val = match proposal.get("is_sold") {
                    Some(v) if v.is_i64() => v.as_i64().unwrap_or(0),
                    Some(v) if v.is_boolean() => if v.as_bool().unwrap_or(false) { 1 } else { 0 },
                    Some(v) if v.is_string() => {
                        let s = v.as_str().unwrap_or("0");
                        if s == "1" || s == "true" { 1 } else { 0 }
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

                // ── FIX: Raw JSON log เมื่อ is_trading (เพื่อ debug) ──
                if is_trading && (is_sold_val == 1 || contract_status == "won" || contract_status == "lost") {
                    println!("\n🔬 [{}] RAW proposal_open_contract (SOLD/FINAL): {}", asset, proposal);
                }

                print!(
                    "\r📋 [{}] Contract: is_sold={} | status={} | profit={:.2}          ",
                    asset, is_sold_val, contract_status, current_profit
                );
                let _ = io::stdout().flush();

                // ── FIX: ถ้า status เป็น won/lost แม้ is_sold ไม่เป็น 1 ก็ถือว่าจบ ──
                let is_contract_ended = is_sold_val == 1
                    || contract_status == "won"
                    || contract_status == "lost";

                if is_contract_ended {
                    // ── FIX 3: Guard ป้องกัน Deriv ส่ง is_sold=1 ซ้ำหลายรอบ ──
                    // ถ้า contract_id นี้เคย process ไปแล้ว → ข้ามทันที ไม่ทำซ้ำ
                    let contract_id_str = proposal
                        .get("contract_id")
                        .map(|v| v.to_string())
                        .unwrap_or_default();

                    if last_processed_contract_id.as_deref() == Some(contract_id_str.as_str()) {
                        // เคย process ไปแล้ว → skip (แต่ terminal log ยังแสดงอยู่ข้างบน)
                        continue;
                    }
                    // บันทึกว่า process แล้ว
                    last_processed_contract_id = Some(contract_id_str);

                    // ══ สัญญาปิดแล้ว — ประมวลผลกำไร/ขาดทุน ══
                    let profit = proposal
                        .get("profit")
                        .and_then(|p| json_to_f64(p))
                        .unwrap_or(0.0);
                    let status = contract_status;

                    println!(
                        "\n🏁 [{}] ไม้จบ! สถานะ: {} | Profit: {:.2}",
                        asset, status, profit
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
                    ) = if let Some(ref details) = last_trade_details {
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

                    let win_status = if profit > 0.0 { "Win" } else { "Loss" };

                    let trade_record = json!({
                        "tradeNo": trade_no,
                        "subTradeno": sub_trade_no,
                        "timeCandle": time_candle,
                        "timeCandleDisplay": time_candle_disp,
                        "thisColor": this_col,
                        "thisAction": this_act,
                        "targetColor": tar_col,
                        "emaShortDirection": e_s_dir,
                        "emaMediumDirection": e_m_dir,
                        "MoneyTrade": money_tr,
                        "WinStatus": win_status,
                        "lossCon": loss_con,
                        "ThisProfit": profit,
                        "GrandBalance": current_balance + profit
                    });

                    // บันทึกลงไฟล์ JSON
                    let now = Local::now();
                    let month_folder = format!("{:02}-{:04}", now.month(), now.year() as i32 + 543);
                    let day_folder = format!(
                        "{:02}-{:02}-{:04}",
                        now.day(),
                        now.month(),
                        now.year() as i32 + 543
                    );
                    let asset_str = deriv_symbol.clone();

                    let base_path = "tradeData";
                    let dir_path = format!(
                        "{}/{}/{}/{}",
                        base_path, month_folder, day_folder, asset_str
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

                    // ── FIX 4: Clear last_trade_details หลัง process เสร็จ ──
                    // ป้องกัน duplicate message อ่านข้อมูลไม้เก่าไปบันทึกซ้ำ
                    last_trade_details = None;

                    if profit > 0.0 {
                        println!("🎉 [{}] ชนะ! Reset LossCon = 0", asset);
                        loss_con = 0;
                        trade_no += 1;
                        sub_trade_no = 1;
                        in_martingale = false;

                        let win_log = format!(
                            "🎉 [{}] ชนะ! Profit: +{:.2} | Balance: {:.2} | → กลับไปรอ Spike ใหม่",
                            asset,
                            profit,
                            current_balance + profit
                        );
                        let _ = tx.send(json!({
                            "type": "bot_log",
                            "asset": asset,
                            "data": { "message": win_log }
                        }));

                        // Telegram notify (Win)
                        if current_config.trade.notify_telegram {
                            let bot_token = env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
                            let chat_id = env::var("TELEGRAM_CHAT_ID").unwrap_or_default();
                            if !bot_token.is_empty() && !chat_id.is_empty() {
                                let message = format!(
                                    "🎉 WIN Trade!\nAsset: {}\nProfit: +{:.2}\nBalance: {:.2}",
                                    asset_str,
                                    profit,
                                    current_balance + profit
                                );
                                let telegram_url = format!(
                                    "https://api.telegram.org/bot{}/sendMessage",
                                    bot_token
                                );
                                let payload = json!({ "chat_id": chat_id, "text": message });
                                tokio::spawn(async move {
                                    let client = reqwest::Client::new();
                                    let _ = client.post(&telegram_url).json(&payload).send().await;
                                });
                            }
                        }
                    } else {
                        loss_con += 1;
                        in_martingale = true;
                        println!("😢 [{}] แพ้! Martingale → LossCon = {}", asset, loss_con);
                        sub_trade_no += 1;

                        let loss_log = format!(
                            "😢 [{}] แพ้! Profit: {:.2} | LossCon: {} | → Martingale เข้าเทรดแท่งถัดไปทันที",
                            asset, profit, loss_con
                        );
                        let _ = tx.send(json!({
                            "type": "bot_log",
                            "asset": asset,
                            "data": { "message": loss_log }
                        }));

                        // ── FIX 5: Telegram notify (Loss) ── เดิมไม่มี!
                        if current_config.trade.notify_telegram {
                            let bot_token = env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
                            let chat_id = env::var("TELEGRAM_CHAT_ID").unwrap_or_default();
                            if !bot_token.is_empty() && !chat_id.is_empty() {
                                let message = format!(
                                    "😢 LOSS Trade!\nAsset: {}\nProfit: {:.2}\nLossCon: {}\nBalance: {:.2}",
                                    asset_str, profit, loss_con, current_balance + profit
                                );
                                let telegram_url = format!(
                                    "https://api.telegram.org/bot{}/sendMessage",
                                    bot_token
                                );
                                let payload = json!({ "chat_id": chat_id, "text": message });
                                tokio::spawn(async move {
                                    let client = reqwest::Client::new();
                                    let _ = client.post(&telegram_url).json(&payload).send().await;
                                });
                            }
                        }
                    }

                    current_balance += profit;
                    is_trading = false;
                    trade_buy_instant = None;
                    active_contract_id = None;

                    let _ = tx.send(json!({
                        "type": "trade_result",
                        "asset": asset,
                        "data": {
                            "asset": asset,
                            "win_status": win_status,
                            "profit": profit,
                            "loss_con": loss_con,
                            "balance": current_balance
                        }
                    }));
                } // end is_contract_ended
            } // end proposal_open_contract

            // ── Deriv API Error handler ──
            if let Some(error) = parsed.get("error") {
                let error_code = error
                    .get("code")
                    .and_then(|c| c.as_str())
                    .unwrap_or("unknown");
                let error_msg = error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown");
                eprintln!(
                    "\n❌ [{}] Deriv API Error: code={} | msg={}",
                    asset, error_code, error_msg
                );
                eprintln!("   Full error: {:?}", error);

                if is_trading {
                    eprintln!(
                        "⚠️ [{}] Error while is_trading=true → resetting is_trading=false",
                        asset
                    );
                    is_trading = false;
                    trade_buy_instant = None;
                    active_contract_id = None;
                    // ── FIX 6: Reset contract guard ด้วยเมื่อเกิด error ──
                    last_processed_contract_id = None;
                    last_trade_details = None;

                    let err_log = format!(
                        "❌ [{}] Deriv Error: {} - {} | → reset สถานะเทรด",
                        asset, error_code, error_msg
                    );
                    let _ = tx.send(json!({
                        "type": "bot_log",
                        "asset": asset,
                        "data": { "message": err_log }
                    }));
                }
            }
        }
    }

    // WebSocket loop จบ
    eprintln!(
        "\n⚠️ [{}] WebSocket loop ended (connection closed). is_trading={}",
        asset, is_trading
    );
    let _ = tx.send(json!({
        "type": "bot_log",
        "asset": asset,
        "data": { "message": format!("⚠️ [{}] WebSocket connection closed", asset) }
    }));

    Ok(())
}

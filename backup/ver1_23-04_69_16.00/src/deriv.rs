use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use std::env;
use std::sync::Arc;
use tokio::sync::broadcast;
use turbo_indicators::OHLCV;
use chrono::{Datelike, Local};
use std::fs;
use std::io::{self, Write};

use crate::AppConfigPayload;
use crate::analysis::AnalysisEngine;
use crate::get_action::{get_suggest_color, TradeAction, AnalysisObject};

// Helper: อ่านค่า f64 จาก JSON ได้ทั้งแบบ Number และ String
// เพราะ Deriv API ส่ง historical candles เป็น Number แต่ live ohlc เป็น String
fn json_to_f64(val: &serde_json::Value) -> Option<f64> {
    val.as_f64().or_else(|| val.as_str()?.parse::<f64>().ok())
}

fn json_to_i64(val: &serde_json::Value) -> Option<i64> {
    val.as_i64().or_else(|| val.as_str()?.parse::<i64>().ok())
}

// แปลง JSON ให้เป็น OHLCV (รองรับทั้ง historical candles และ live ohlc)
fn parse_candle(data: &serde_json::Value) -> Option<OHLCV> {
    // Live ohlc มี open_time (จุดเริ่มต้นแท่ง) + epoch (tick ปัจจุบัน)
    // Historical candles มีแค่ epoch (จุดเริ่มต้นแท่ง)
    // ใช้ open_time ก่อน (ถ้ามี) แล้ว fallback เป็น epoch
    let timestamp = data.get("open_time")
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
        _ => asset, // Fallback
    }.to_string()
}

// ฟังก์ชันแปลงเลข granularity เป็น duration ในหน่วยวินาทีหรือนาที
fn get_duration_params(granularity: i32) -> (i32, String) {
    // Duration = granularity - 3 วินาที
    // ตัวอย่าง: granularity 60s → duration 57s
    // เข้าเทรดที่ second :00 → จบที่ second :57
    // เหลือ 3 วินาทีก่อนแท่งใหม่เพื่อรับผล win/loss
    let duration_seconds = (granularity - 3).max(1);
    (duration_seconds, "s".to_string())
}

/// ฟังก์ชันหลักสำหรับรัน WebSocket กับ Deriv.com
/// รับ Config ที่มาจากหน้าเว็บ (index.html) เข้ามาประมวลผลการเทรด
pub async fn start_deriv_bot(
    app_id: String, 
    api_token: String, 
    config: AppConfigPayload,
    asset: String,  // Asset ที่ bot นี้รับผิดชอบ (เช่น "vol10", "vol25")
    tx: Arc<broadcast::Sender<serde_json::Value>>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_url = format!("wss://ws.binaryws.com/websockets/v3?app_id={}", app_id);
    println!("\n🔌 [{}] กำลังเชื่อมต่อ Deriv WebSocket...", asset);

    let (mut ws_stream, _) = connect_async(&ws_url).await?;
    println!("✅ [{}] เชื่อมต่อ WebSocket สำเร็จ!", asset);

    // 1. กระบวนการ Authorize ด้วย API Token
    let auth_request = json!({
        "authorize": api_token
    });
    ws_stream.send(Message::Text(auth_request.to_string())).await?;

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
    let mut last_tick_analysis: Option<AnalysisObject> = None; // เก็บ analysis ล่าสุดของแท่งปัจจุบัน
    
    // กำหนด Asset สำหรับ bot นี้
    let deriv_symbol = map_asset_to_symbol(&asset);
    let (duration, duration_unit) = get_duration_params(config.granularity);

    // 2. ลูปรับข้อมูลตอบกลับ
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

            // ── Debug: แสดง msg_type ทุกข้อความ (ยกเว้น ohlc/tick เพื่อลด spam) ──
            let msg_type = parsed.get("msg_type").and_then(|t| t.as_str()).unwrap_or("unknown");
            if msg_type != "ohlc" && msg_type != "tick" {
                println!("\n📨 [{}] Deriv msg_type: {} | is_trading: {}", asset, msg_type, is_trading);
            }

            // ถ้า Authorize สำเร็จ ให้เรียกดูแท่งเทียน
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
                
                // อ่านจำนวนแท่งเทียนจาก .env (ค่า Default = 1000)
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
                    "subscribe": 1 // เปิด Real-time Subscribe
                });

                println!("📦 [{}] ร้องขอข้อมูลแท่งเทียน: {}", asset, deriv_symbol);
                ws_stream.send(Message::Text(ticks_request.to_string())).await?;
            }

            // ถ้าเป็นชุดข้อมูลย้อนหลัง
            if let Some(candles) = parsed.get("candles") {
                if let Some(arr) = candles.as_array() {
                    for c in arr {
                        if let Some(ohlcv) = parse_candle(c) {
                            current_history.push(ohlcv);
                        }
                    }
                }
                
                println!("📊 [{}] ได้รับแท่งเทียนย้อนหลัง {} แท่ง (SIMD+StatefulATR)", asset, current_history.len());
                
                // สร้าง Stateful Engine + batch process ด้วย SIMD EMA / StatefulATR
                let mut engine = AnalysisEngine::new(&current_config);
                let analysis_result = engine.process_history(&current_history, &current_config);
                analysis_engine = Some(engine);
                
                let _ = tx.send(json!({
                    "type": "candles_history",
                    "asset": asset,
                    "data": analysis_result
                }));
            }

            // ถ้าเป็นข้อมูลอัปเดตแบบ Realtime แท่งต่อแท่ง
            if let Some(ohlc) = parsed.get("ohlc") {
                if let Some(new_tick) = parse_candle(ohlc) {
                    
                    // อัปเดตแท่งล่าสุด (OHLC อาจจะอัปเดตแท่งเดิม หรือขึ้นแท่งใหม่)
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
                    
                    print!("\r⏱ [{}] close={:.4} | {} | {} แท่ง          ", 
                        asset,
                        new_tick.close,
                        if is_new_candle { "🆕 New" } else { "📝 Upd" },
                        current_history.len()
                    );
                    let _ = io::stdout().flush();

                    // O(1) Stateful Analysis — SIMD EMA + Wilder's ATR
                    if let Some(ref mut engine) = analysis_engine {
                        let analysis_obj = engine.update_tick(
                            current_history.last().unwrap(),
                            is_new_candle,
                            current_config.indicators.atr_multi
                        );
                        let latest_analysis = &analysis_obj;
                        let _ = tx.send(json!({
                            "type": "ohlc_update",
                            "asset": asset,
                            "data": latest_analysis
                        }));

                        // --- สร้าง Log ของ Asset ---
                        let now = Local::now();
                        let month_folder = format!("{:02}-{:04}", now.month(), now.year() as i32 + 543);
                        let day_folder = format!("{:02}-{:02}-{:04}", now.day(), now.month(), now.year() as i32 + 543);
                        let dir_path_asset = format!("tradeData/{}/{}/{}", month_folder, day_folder, deriv_symbol);
                        let _ = fs::create_dir_all(&dir_path_asset);
                        let log_asset_file = format!("{}/asset.jsonl", dir_path_asset);
                        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(&log_asset_file) {
                            if let Ok(json_str) = serde_json::to_string(latest_analysis) {
                                let _ = writeln!(file, "{}", json_str);
                            }
                        }

                        // ══════════════════════════════════════════
                        //  Phase 4: ตัดสินใจเข้าเทรด — ONLY at candle boundary
                        //  ดู spike ณ second :59 (แท่งที่จบ) → เทรดที่ second :00 (แท่งใหม่)
                        //  Duration = granularity - 3 seconds (จบที่ :57)
                        // ══════════════════════════════════════════
                        if is_new_candle && !is_trading {
                            // last_tick_analysis = สถานะสุดท้ายของแท่งเทียนที่เพิ่งจบ (ณ second :59)
                            if let Some(ref prev) = last_tick_analysis {
                                let should_trade = if in_martingale {
                                    // Martingale: เข้าเทรดทันทีที่แท่งใหม่เปิด
                                    // ใช้ suggestColor จากแท่งที่เพิ่งจบ
                                    true
                                } else {
                                    // ปกติ: ตรวจ Spike ของแท่งที่เพิ่งจบ (is_atr)
                                    prev.is_atr
                                };

                                // ══════ BOT LOG: แจ้งสถานะทุกแท่งใหม่ ══════
                                let candle_time = chrono::DateTime::from_timestamp(latest_analysis.epoch, 0)
                                    .map(|dt| dt.with_timezone(&Local).format("%H:%M:%S").to_string())
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
                                    // คำนวณ suggest color จากแท่งที่เพิ่งจบ
                                    let suggest = get_suggest_color(&prev.this_color, loss_con);
                                    let trade_action = if suggest == "green" { TradeAction::Call } else { TradeAction::Put };
                                    
                                    println!("\n📈 [{}] SIGNAL (candle boundary :00): {:?} | PrevSpike: {} | PrevColor: {} | Martingale: {} | LossCon: {}", 
                                        asset, trade_action, prev.is_atr, prev.this_color, in_martingale, loss_con);
                                    is_trading = true;
                                    last_trade_epoch = latest_analysis.epoch;

                                    // 1. คำนวณ Lot (Phase 5 - Money Management)
                                    let mut amount = current_config.trade.target_lot;
                                    if current_config.trade.martingale.martingale_type == "martingale" {
                                        if let Some(m_amount) = current_config.trade.martingale.list.get(loss_con as usize) {
                                            amount = *m_amount;
                                        } else if let Some(last_m) = current_config.trade.martingale.list.last() {
                                            amount = *last_m; // ถ้าลิสต์หมด ใช้ไม้สุดท้ายประคอง
                                        }
                                    }

                                    // 2. ออกแบบคำสั่ง Trade (CALL / PUT)
                                    let contract_type = if trade_action == TradeAction::Call { "CALL" } else { "PUT" };
                                    
                                    // Record: timeCandle = แท่งใหม่ (เทรดจริง), thisColor = แท่งที่จบ (spike/signal)
                                    last_trade_details = Some(json!({
                                        "timeCandle": latest_analysis.epoch,
                                        "timeCandleDisplay": chrono::DateTime::from_timestamp(latest_analysis.epoch, 0).unwrap().with_timezone(&Local).format("%d/%m/%Y %H:%M:%S").to_string(),
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

                                    // BOT LOG: แจ้งเข้าเทรด
                                    let trade_log = format!("🚀 [{}] {} | เข้าเทรด {:?} | Amount: ${} | Duration: {}{}s",
                                        asset, candle_time, trade_action, amount, duration, duration_unit);
                                    let _ = tx.send(json!({
                                        "type": "bot_log",
                                        "asset": asset,
                                        "data": { "message": trade_log, "epoch": latest_analysis.epoch }
                                    }));

                                    println!("🚀 [{}] ยิงคำสั่งเทรด (duration={}{}): {:?}", asset, duration, duration_unit, buy_request);
                                    ws_stream.send(Message::Text(buy_request.to_string())).await?;
                                }
                            }
                        }

                        // บันทึก analysis ล่าสุดของทุก tick (จะกลายเป็น "prev" เมื่อแท่งใหม่มา)
                        last_tick_analysis = Some(analysis_obj.clone());
                    }
                }
            }

            // รับผลของคำสั่ง Buy เพื่อเอา Contract ID ไปติดตามผลไม้ต่อไม้
            if let Some(buy_response) = parsed.get("buy") {
                if let Some(contract_id) = buy_response.get("contract_id") {
                    println!("\n✅ [{}] เข้าออเดอร์สำเร็จ Contract ID: {}", asset, contract_id);
                    
                    // แสดง balance_after ถ้ามี
                    if let Some(bal) = buy_response.get("balance_after").and_then(|b| json_to_f64(b)) {
                        current_balance = bal;
                    }
                    
                    let sub_proposal = json!({
                        "proposal_open_contract": 1,
                        "contract_id": contract_id,
                        "subscribe": 1
                    });
                    println!("📡 [{}] Subscribe proposal_open_contract: contract_id={}", asset, contract_id);
                    ws_stream.send(Message::Text(sub_proposal.to_string())).await?;
                } else {
                    // Buy สำเร็จแต่ไม่มี contract_id → ผิดปกติ
                    eprintln!("⚠️ [{}] Buy response ไม่มี contract_id: {:?}", asset, buy_response);
                    is_trading = false;
                }
            }

            // ติดตามผลกำไร/ขาดทุน
            if let Some(proposal) = parsed.get("proposal_open_contract") {
                // ── Debug: แสดงสถานะสัญญาทุกรอบ ──
                let is_sold_val = proposal.get("is_sold").and_then(|v| json_to_i64(v)).unwrap_or(0);
                let current_profit = proposal.get("profit").and_then(|v| json_to_f64(v)).unwrap_or(0.0);
                let contract_status = proposal.get("status").and_then(|s| s.as_str()).unwrap_or("unknown");
                
                print!("\r📋 [{}] Contract: is_sold={} | status={} | profit={:.2}          ",
                    asset, is_sold_val, contract_status, current_profit);
                let _ = io::stdout().flush();

                if is_sold_val == 1 {
                    // ══ สัญญาปิดแล้ว — ประมวลผลกำไร/ขาดทุน ══
                    let profit = proposal.get("profit").and_then(|p| json_to_f64(p)).unwrap_or(0.0);
                    let status = contract_status;
                        
                        println!("\n🏁 [{}] ไม้จบ! สถานะ: {} | Profit: {}", asset, status, profit);
                        
                        let (time_candle, time_candle_disp, this_col, this_act, tar_col, e_s_dir, e_m_dir, money_tr) = if let Some(ref details) = last_trade_details {
                             (
                                details["timeCandle"].as_i64().unwrap_or(0),
                                details["timeCandleDisplay"].as_str().unwrap_or("").to_string(),
                                details["thisColor"].as_str().unwrap_or("").to_string(),
                                details["thisAction"].as_str().unwrap_or("").to_string(),
                                details["targetColor"].as_str().unwrap_or("").to_string(),
                                details["emaShortDirection"].as_str().unwrap_or("").to_string(),
                                details["emaMediumDirection"].as_str().unwrap_or("").to_string(),
                                details["MoneyTrade"].as_f64().unwrap_or(0.0)
                             )
                        } else {
                            (0, String::new(), String::new(), String::new(), String::new(), String::new(), String::new(), 0.0)
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
                        let day_folder = format!("{:02}-{:02}-{:04}", now.day(), now.month(), now.year() as i32 + 543);
                        let asset_str = deriv_symbol.clone();
                        
                        let base_path = "tradeData";
                        let dir_path = format!("{}/{}/{}/{}", base_path, month_folder, day_folder, asset_str);
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

                        if profit > 0.0 {
                            println!("🎉 [{}] ชนะ! Reset LossCon = 0", asset);
                            loss_con = 0;
                            trade_no += 1;
                            sub_trade_no = 1;
                            in_martingale = false; // ชนะแล้ว → กลับไปรอ Spike ใหม่
                            
                            // BOT LOG: แจ้งผลชนะ
                            let win_log = format!("🎉 [{}] ชนะ! Profit: +{:.2} | Balance: {:.2} | → กลับไปรอ Spike ใหม่",
                                asset, profit, current_balance + profit);
                            let _ = tx.send(json!({
                                "type": "bot_log",
                                "asset": asset,
                                "data": { "message": win_log }
                            }));

                            if current_config.trade.notify_telegram {
                                let bot_token = env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
                                let chat_id = env::var("TELEGRAM_CHAT_ID").unwrap_or_default();
                                if !bot_token.is_empty() && !chat_id.is_empty() {
                                    let message = format!("🎉 WIN Trade!\nAsset: {}\nProfit: {}\nBalance: {:.2}", asset_str, profit, current_balance + profit);
                                    let telegram_url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
                                    let payload = json!({ "chat_id": chat_id, "text": message });
                                    tokio::spawn(async move {
                                        let client = reqwest::Client::new();
                                        let _ = client.post(&telegram_url).json(&payload).send().await;
                                    });
                                }
                            }
                        } else {
                            loss_con += 1;
                            in_martingale = true; // แพ้ → เข้าโหมด Martingale เทรดแท่งถัดไปทันที
                            println!("😢 [{}] แพ้! Martingale → LossCon = {}", asset, loss_con);
                            sub_trade_no += 1;

                            // BOT LOG: แจ้งผลแพ้
                            let loss_log = format!("😢 [{}] แพ้! Profit: {:.2} | LossCon: {} | → Martingale เข้าเทรดแท่งถัดไปทันที",
                                asset, profit, loss_con);
                            let _ = tx.send(json!({
                                "type": "bot_log",
                                "asset": asset,
                                "data": { "message": loss_log }
                            }));
                        }

                        current_balance += profit;
                        is_trading = false; // ปลดล็อกให้เทรดไม้ต่อไปได้

                        // อัปเดตบอก UI
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
                }  // end is_sold_val == 1
            }  // end proposal_open_contract

            // จัดการ Error ที่มาจาก Deriv
            if let Some(error) = parsed.get("error") {
                let error_code = error.get("code").and_then(|c| c.as_str()).unwrap_or("unknown");
                let error_msg = error.get("message").and_then(|m| m.as_str()).unwrap_or("unknown");
                eprintln!("\n❌ [{}] Deriv API Error: code={} | msg={}", asset, error_code, error_msg);
                eprintln!("   Full error: {:?}", error);
                
                // ถ้าเกิด Error ขณะกำลังเทรด → reset สถานะเพื่อไม่ให้ bot ค้าง
                if is_trading {
                    eprintln!("⚠️ [{}] Error while is_trading=true → resetting is_trading=false", asset);
                    is_trading = false;
                    // ส่ง bot_log แจ้ง UI
                    let err_log = format!("❌ [{}] Deriv Error: {} - {} | → reset สถานะเทรด", asset, error_code, error_msg);
                    let _ = tx.send(json!({
                        "type": "bot_log",
                        "asset": asset,
                        "data": { "message": err_log }
                    }));
                }
            }
        }
    }

    // ── WebSocket loop จบแล้ว ──
    eprintln!("\n⚠️ [{}] WebSocket loop ended (connection closed). is_trading={}", asset, is_trading);
    let _ = tx.send(json!({
        "type": "bot_log",
        "asset": asset,
        "data": { "message": format!("⚠️ [{}] WebSocket connection closed", asset) }
    }));

    Ok(())
}

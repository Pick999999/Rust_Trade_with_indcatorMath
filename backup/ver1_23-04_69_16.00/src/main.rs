use axum::{
    extract::ws::{Message, WebSocket},
    extract::{State, WebSocketUpgrade, Query},
    routing::{get, post},
    Json, Router,
};
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;
use tower_http::services::ServeDir;
use std::fs;
use std::path::Path;
use chrono::{Local, NaiveDateTime, TimeZone};

pub mod analysis;
pub mod deriv;
pub mod get_action;

// --- Data Models matching the JSON from index.html ---

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Meta {
    #[serde(rename = "generatedAt")]
    pub generated_at: String,
    pub theme: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DateRange {
    #[serde(rename = "startDate")]
    pub start_date: String,
    #[serde(rename = "stopDate")]
    pub stop_date: String,
    #[serde(rename = "useSchedule", default)]
    pub use_schedule: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Indicators {
    #[serde(rename = "atrPeriod")]
    pub atr_period: i32,
    #[serde(rename = "atrMulti")]
    pub atr_multi: f64,
    #[serde(rename = "ciPeriod")]
    pub ci_period: i32,
    #[serde(rename = "adxPeriod")]
    pub adx_period: i32,
    #[serde(rename = "bbPeriod")]
    pub bb_period: i32,
    #[serde(rename = "smcPeriod")]
    pub smc_period: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EmaSettings {
    pub enabled: bool,
    pub period: i32,
    #[serde(rename = "type")]
    pub ema_type: String,
    pub color: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EmaConfig {
    pub short: EmaSettings,
    pub medium: EmaSettings,
    pub long: EmaSettings,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Martingale {
    #[serde(rename = "type")]
    pub martingale_type: String,
    pub list: Vec<f64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TradeConfig {
    #[serde(rename = "conditionStopTrade")]
    pub condition_stop_trade: String,
    #[serde(rename = "targetMoney")]
    pub target_money: f64,
    #[serde(rename = "targetLot")]
    pub target_lot: f64,
    #[serde(rename = "notifyTelegram", default)]
    pub notify_telegram: bool,
    pub martingale: Martingale,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfigPayload {
    pub meta: Meta,
    #[serde(rename = "dateRange")]
    pub date_range: DateRange,
    pub granularity: i32,
    #[serde(rename = "granularitySettings")]
    pub granularity_settings: i32,
    pub assets: Vec<String>,
    pub indicators: Indicators,
    pub ema: EmaConfig,
    pub trade: TradeConfig,
}

// --- Shared App State ---
#[derive(Clone)]
pub struct AppState {
    pub tx: Arc<broadcast::Sender<serde_json::Value>>,
    pub active_bots: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    pub bot_logs: Arc<Mutex<Vec<serde_json::Value>>>,
    pub candle_data: Arc<Mutex<HashMap<String, serde_json::Value>>>,
}

// --- Handler Functions ---

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.tx.subscribe();
    println!("Web Client Connected to Rust WebSocket");

    // ═══ SNAPSHOT: ส่งข้อมูลย้อนหลังให้ Client ใหม่ (เครื่อง B) ═══
    // 1. ส่ง candle_data ที่เก็บไว้ (เพื่อให้หน้าเว็บวาดกราฟได้ทันที)
    {
        let candle_guard = state.candle_data.lock().await;
        for (_asset, payload) in candle_guard.iter() {
            let text = payload.to_string();
            if socket.send(Message::Text(text)).await.is_err() {
                println!("Client disconnected during snapshot");
                return;
            }
        }
    }

    // 2. ส่ง bot_logs ย้อนหลัง
    {
        let logs_guard = state.bot_logs.lock().await;
        for log_entry in logs_guard.iter() {
            let text = log_entry.to_string();
            if socket.send(Message::Text(text)).await.is_err() {
                println!("Client disconnected during snapshot");
                return;
            }
        }
    }

    println!("📦 Snapshot sent to new client ({} candle sets, {} logs)",
        state.candle_data.lock().await.len(),
        state.bot_logs.lock().await.len()
    );

    // Loop เพื่อส่งข้อมูลจาก Broadcast กลับไปให้ Browser (index.html) ทันที
    while let Ok(msg) = rx.recv().await {
        let text = msg.to_string();
        if socket.send(Message::Text(text)).await.is_err() {
            println!("Client disconnected");
            break;
        }
    }
}

async fn handle_post_trade(
    State(state): State<AppState>,
    Json(payload): Json<AppConfigPayload>,
) -> Json<serde_json::Value> {
    println!("Received Payload from Web UI.");

    let app_id = env::var("DERIV_APP_ID").unwrap_or_else(|_| "1089".to_string());
    let api_token = env::var("DERIV_API_TOKEN").unwrap_or_else(|_| "".to_string());

    if api_token.is_empty() || api_token == "YOUR_API_TOKEN_HERE" {
        return Json(serde_json::json!({
            "status": "error",
            "message": "กรุณาแก้ไฟล์ .env เพื่อใส่ DERIV_API_TOKEN ให้เรียบร้อย"
        }));
    }

    let mut bot_guard = state.active_bots.lock().await;
    
    // ปิดบอทตัวเก่าทั้งหมดก่อน (ถ้ามีรันอยู่)
    for (asset_name, handle) in bot_guard.drain() {
        handle.abort();
        println!("🛑 Aborted bot: {}", asset_name);
    }

    // Spawn bot แยกตัวต่อ Asset
    let mut spawned_assets = Vec::new();
    for asset in payload.assets.iter() {
        let tx = state.tx.clone();
        let app_id = app_id.clone();
        let api_token = api_token.clone();
        let config = payload.clone();
        let asset_clone = asset.clone();

        let handle = tokio::spawn(async move {
            if let Err(e) = deriv::start_deriv_bot(app_id, api_token, config, asset_clone.clone(), tx).await {
                eprintln!("Bot [{}] crashed: {}", asset_clone, e);
            }
        });

        spawned_assets.push(asset.clone());
        bot_guard.insert(asset.clone(), handle);
    }

    let assets_str = spawned_assets.join(", ");
    println!("🚀 Spawned {} bot(s): {:?}", spawned_assets.len(), spawned_assets);

    // ส่ง Telegram แจ้งเตือนเมื่อเริ่มเทรด
    let now_str = Local::now().format("%d/%m/%Y %H:%M:%S").to_string();
    let stop_display = if payload.date_range.stop_date.is_empty() {
        "ไม่ได้ตั้งค่า".to_string()
    } else {
        // แปลง format จาก 2026-04-22T15:00 เป็น 22/04/2026 15:00
        NaiveDateTime::parse_from_str(&payload.date_range.stop_date, "%Y-%m-%dT%H:%M")
            .map(|dt| dt.format("%d/%m/%Y %H:%M").to_string())
            .unwrap_or_else(|_| payload.date_range.stop_date.clone())
    };
    send_telegram_message(&format!(
        "🟢 เริ่มเทรด!\n📊 Assets: {}\n⏰ เริ่ม: {}\n🔴 สิ้นสุด: {}\n📋 Condition: {}",
        assets_str, now_str, stop_display, payload.trade.condition_stop_trade
    )).await;

    // ส่ง bot_log ไปยังหน้าเว็บ
    let start_log = format!("🟢 เริ่มเทรด! Assets: {} | สิ้นสุด: {} | Condition: {}",
        assets_str, stop_display, payload.trade.condition_stop_trade);
    let _ = state.tx.send(serde_json::json!({
        "type": "bot_log",
        "asset": "system",
        "data": { "message": start_log }
    }));

    Json(serde_json::json!({
        "status": "success",
        "message": format!("Spawned {} bot(s): {}. กำลังเชื่อมต่อ Deriv...", spawned_assets.len(), assets_str)
    }))
}

async fn handle_post_stop(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut bot_guard = state.active_bots.lock().await;
    let count = bot_guard.len();
    let mut stopped_assets = Vec::new();
    for (asset_name, handle) in bot_guard.drain() {
        handle.abort();
        stopped_assets.push(asset_name.clone());
        println!("🛑 หยุดบอท: {}", asset_name);
    }
    println!("🛑 หยุดบอททั้งหมด {} ตัว", count);

    // ส่ง Telegram แจ้งเตือนเมื่อหยุดเทรด
    let now_str = Local::now().format("%d/%m/%Y %H:%M:%S").to_string();
    send_telegram_message(&format!(
        "🔴 หยุดเทรด!\n📊 Assets: {}\n⏰ เวลา: {}\n📊 หยุดบอททั้งหมด {} ตัว",
        stopped_assets.join(", "), now_str, count
    )).await;

    Json(serde_json::json!({
        "status": "success",
        "message": format!("หยุดบอททั้งหมด {} ตัวแล้ว", count)
    }))
}

async fn handle_post_terminate(State(state): State<AppState>) -> Json<serde_json::Value> {
    // ปิดบอททั้งหมดก่อน
    let mut bot_guard = state.active_bots.lock().await;
    for (asset_name, handle) in bot_guard.drain() {
        handle.abort();
        println!("🛑 Terminate: หยุดบอท {}", asset_name);
    }

    println!("⚠️ ได้รับสัญญาณ Terminate: โปรแกรมกำลังจะปิดการทำงานทั้งหมดใน 1 วินาที");

    // ตั้งเวลาให้รอแบบ Async เพื่อให้สามารถตอบ HTTP Response กลับไปหาหน้าเว็บก่อนปิดตัวเอง
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        std::process::exit(0);
    });

    Json(serde_json::json!({
        "status": "success",
        "message": "ปิดระบบ Backend แล้ว! โปรแกรมจะออกสู่หน้าต่าง Prompt"
    }))
}

async fn get_setup() -> Json<serde_json::Value> {
    let path = Path::new("setup.json");
    if path.exists() {
        if let Ok(contents) = fs::read_to_string(path) {
            if let Ok(json) = serde_json::from_str(&contents) {
                return Json(json);
            }
        }
    }
    // Return empty payload if not exists
    Json(serde_json::json!({}))
}

// ══════════════════════════════════════════════════════════════
//  NOTIFY API — ให้ Frontend ส่ง Telegram message ได้
// ══════════════════════════════════════════════════════════════
#[derive(Debug, Deserialize)]
pub struct NotifyPayload {
    pub message: String,
}

async fn handle_post_notify(
    Json(payload): Json<NotifyPayload>,
) -> Json<serde_json::Value> {
    println!("📨 Notify request: {}", payload.message);
    send_telegram_message(&payload.message).await;
    Json(serde_json::json!({
        "status": "success",
        "message": "ส่ง Telegram เรียบร้อย"
    }))
}

async fn save_setup(Json(payload): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let path = Path::new("setup.json");
    if fs::write(path, serde_json::to_string_pretty(&payload).unwrap_or_default()).is_ok() {
        Json(serde_json::json!({"status": "success"}))
    } else {
        Json(serde_json::json!({"status": "error", "message": "Failed to save setup"}))
    }
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub date: String,
}

async fn get_history(Query(params): Query<HistoryQuery>) -> Json<Vec<serde_json::Value>> {
    let parts: Vec<&str> = params.date.split('-').collect();
    if parts.len() != 3 {
        return Json(vec![]);
    }
    let year: i32 = parts[0].parse().unwrap_or(0);
    let month = parts[1];
    let day = parts[2];
    
    let bei_year = year + 543;
    let month_folder = format!("{}-{}", month, bei_year);
    let day_folder = format!("{}-{}-{}", day, month, bei_year);
    
    let path_str = format!("tradeData/{}/{}", month_folder, day_folder);
    let path = Path::new(&path_str);
    
    let mut all_trades = Vec::new();
    
    if path.exists() && path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let json_path = entry.path().join("trades.json");
                        if let Ok(contents) = fs::read_to_string(json_path) {
                            if let Ok(mut trades) = serde_json::from_str::<Vec<serde_json::Value>>(&contents) {
                                let current_asset = entry.file_name().to_string_lossy().to_string();
                                for t in &mut trades {
                                    if let Some(obj) = t.as_object_mut() {
                                        obj.insert("Asset".to_string(), serde_json::json!(current_asset));
                                    }
                                }
                                all_trades.extend(trades);
                            }
                        }
                    }
                }
            }
        }
    }
    
    all_trades.sort_by(|a, b| {
        let ta = a.get("timeCandle").and_then(|t| t.as_i64()).unwrap_or(0);
        let tb = b.get("timeCandle").and_then(|t| t.as_i64()).unwrap_or(0);
        tb.cmp(&ta)
    });
    
    Json(all_trades)
}

// ══════════════════════════════════════════════════════════════
//  TELEGRAM HELPER — ส่งข้อความแจ้งเตือน Telegram
// ══════════════════════════════════════════════════════════════
async fn send_telegram_message(message: &str) {
    let bot_token = env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
    let chat_id = env::var("TELEGRAM_CHAT_ID").unwrap_or_default();
    if bot_token.is_empty() || chat_id.is_empty() || chat_id == "YOUR_CHAT_ID" {
        println!("⚠️ Telegram: ยังไม่ได้ตั้งค่า TELEGRAM_BOT_TOKEN หรือ TELEGRAM_CHAT_ID");
        return;
    }
    let telegram_url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
    let payload = serde_json::json!({ "chat_id": chat_id, "text": message });
    let client = reqwest::Client::new();
    match client.post(&telegram_url).json(&payload).send().await {
        Ok(_) => println!("📨 Telegram: ส่งข้อความสำเร็จ"),
        Err(e) => eprintln!("❌ Telegram: ส่งข้อความไม่สำเร็จ: {}", e),
    }
}

// ══════════════════════════════════════════════════════════════
//  AUTO SCHEDULER — เริ่ม/หยุดเทรดอัตโนมัติตาม startDate/stopDate
// ══════════════════════════════════════════════════════════════
async fn auto_schedule_trade(state: AppState) {
    // 1. อ่าน setup.json
    let config: AppConfigPayload = match fs::read_to_string("setup.json") {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("❌ Scheduler: ไม่สามารถ parse setup.json: {}", e);
                return;
            }
        },
        Err(e) => {
            eprintln!("❌ Scheduler: ไม่พบไฟล์ setup.json: {}", e);
            return;
        }
    };

    // ตรวจสอบว่ามีการตั้ง assets หรือไม่
    if config.assets.is_empty() {
        println!("⚠️ Scheduler: ไม่มี asset ที่ตั้งค่าไว้ใน setup.json → ข้ามการเทรดอัตโนมัติ");
        return;
    }

    // 2. Parse startDate
    let start_dt = match NaiveDateTime::parse_from_str(&config.date_range.start_date, "%Y-%m-%dT%H:%M") {
        Ok(dt) => dt,
        Err(e) => {
            eprintln!("❌ Scheduler: ไม่สามารถ parse startDate '{}': {}", config.date_range.start_date, e);
            return;
        }
    };

    let start_time = match Local.from_local_datetime(&start_dt).single() {
        Some(t) => t,
        None => {
            eprintln!("❌ Scheduler: ไม่สามารถแปลง startDate เป็น local time");
            return;
        }
    };

    let now = Local::now();
    println!("\n══════════════════════════════════════════");
    println!("  ⏰ AUTO SCHEDULER ACTIVATED");
    println!("  📊 Assets: {:?}", config.assets);
    println!("  🟢 Start: {}", start_time.format("%d/%m/%Y %H:%M:%S"));
    println!("  🔴 Stop:  {} (condition: {})", config.date_range.stop_date, config.trade.condition_stop_trade);
    println!("  🕐 Now:   {}", now.format("%d/%m/%Y %H:%M:%S"));
    println!("══════════════════════════════════════════\n");

    // 3. รอจนถึง startDate
    if start_time > now {
        let wait_duration = (start_time - now).to_std().unwrap_or_default();
        let wait_secs = wait_duration.as_secs();
        let hours = wait_secs / 3600;
        let mins = (wait_secs % 3600) / 60;
        let secs = wait_secs % 60;
        println!("⏳ Scheduler: รอจนถึง startDate... (อีก {}h {}m {}s)", hours, mins, secs);
        tokio::time::sleep(wait_duration).await;
    } else {
        println!("⚡ Scheduler: startDate เลยไปแล้ว → เริ่มเทรดทันที!");
    }

    // 4. อ่าน setup.json อีกครั้ง (อาจมีการแก้ไขระหว่างรอ)
    let config: AppConfigPayload = match fs::read_to_string("setup.json") {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("❌ Scheduler: ไม่สามารถ parse setup.json (retry): {}", e);
                return;
            }
        },
        Err(e) => {
            eprintln!("❌ Scheduler: ไม่พบ setup.json (retry): {}", e);
            return;
        }
    };

    // 5. ตรวจสอบ API Token
    let app_id = env::var("DERIV_APP_ID").unwrap_or_else(|_| "1089".to_string());
    let api_token = env::var("DERIV_API_TOKEN").unwrap_or_else(|_| "".to_string());

    if api_token.is_empty() || api_token == "YOUR_API_TOKEN_HERE" {
        eprintln!("❌ Scheduler: กรุณาตั้งค่า DERIV_API_TOKEN ใน .env");
        send_telegram_message("❌ ไม่สามารถเริ่มเทรดอัตโนมัติ: ยังไม่ได้ตั้งค่า DERIV_API_TOKEN").await;
        return;
    }

    // 6. ส่ง Telegram แจ้งเริ่มเทรด
    let assets_str = config.assets.join(", ");
    send_telegram_message(&format!(
        "🟢 เริ่มเทรดอัตโนมัติ!\n📊 Assets: {}\n⏰ เวลา: {}\n📋 Condition: {}",
        assets_str,
        Local::now().format("%d/%m/%Y %H:%M:%S"),
        config.trade.condition_stop_trade
    )).await;

    // 7. Spawn bots (เหมือน handle_post_trade)
    {
        let mut bot_guard = state.active_bots.lock().await;

        // ปิดบอทตัวเก่าทั้งหมดก่อน (ถ้ามีรันอยู่)
        for (asset_name, handle) in bot_guard.drain() {
            handle.abort();
            println!("🛑 Scheduler: Aborted old bot: {}", asset_name);
        }

        let mut spawned_assets = Vec::new();
        for asset in config.assets.iter() {
            let tx = state.tx.clone();
            let app_id = app_id.clone();
            let api_token = api_token.clone();
            let config_clone = config.clone();
            let asset_clone = asset.clone();

            let handle = tokio::spawn(async move {
                if let Err(e) = deriv::start_deriv_bot(app_id, api_token, config_clone, asset_clone.clone(), tx).await {
                    eprintln!("Bot [{}] crashed: {}", asset_clone, e);
                }
            });

            spawned_assets.push(asset.clone());
            bot_guard.insert(asset.clone(), handle);
        }

        println!("🚀 Scheduler: Spawned {} bot(s): {:?}", spawned_assets.len(), spawned_assets);
    }

    // 8. ถ้า conditionStopTrade == "stopDate" → รอจนถึง stopDate แล้วหยุดเทรดอัตโนมัติ
    if config.trade.condition_stop_trade == "stopDate" {
        let stop_dt = match NaiveDateTime::parse_from_str(&config.date_range.stop_date, "%Y-%m-%dT%H:%M") {
            Ok(dt) => dt,
            Err(e) => {
                eprintln!("⚠️ Scheduler: ไม่สามารถ parse stopDate '{}': {} → จะไม่หยุดอัตโนมัติ", config.date_range.stop_date, e);
                return;
            }
        };

        let stop_time = match Local.from_local_datetime(&stop_dt).single() {
            Some(t) => t,
            None => {
                eprintln!("⚠️ Scheduler: ไม่สามารถแปลง stopDate เป็น local time → จะไม่หยุดอัตโนมัติ");
                return;
            }
        };

        let now = Local::now();
        if stop_time > now {
            let wait_duration = (stop_time - now).to_std().unwrap_or_default();
            let wait_secs = wait_duration.as_secs();
            let hours = wait_secs / 3600;
            let mins = (wait_secs % 3600) / 60;
            let secs = wait_secs % 60;
            println!("⏳ Scheduler: จะหยุดเทรดอัตโนมัติเมื่อถึง {} (อีก {}h {}m {}s)",
                stop_time.format("%d/%m/%Y %H:%M:%S"), hours, mins, secs);
            tokio::time::sleep(wait_duration).await;

            // หยุดบอททั้งหมด
            let mut bot_guard = state.active_bots.lock().await;
            let count = bot_guard.len();
            for (asset_name, handle) in bot_guard.drain() {
                handle.abort();
                println!("🛑 Scheduler: หยุดบอท: {}", asset_name);
            }
            println!("\n══════════════════════════════════════════");
            println!("  🔴 AUTO STOP — ถึง StopDate แล้ว");
            println!("  ⏰ {}", Local::now().format("%d/%m/%Y %H:%M:%S"));
            println!("  📊 หยุดบอททั้งหมด {} ตัว", count);
            println!("══════════════════════════════════════════\n");

            // ส่ง Telegram แจ้งหยุดเทรด
            send_telegram_message(&format!(
                "🔴 หยุดเทรดอัตโนมัติ!\n⏰ ถึง StopDate: {}\n📊 หยุดบอททั้งหมด {} ตัว",
                stop_time.format("%d/%m/%Y %H:%M:%S"),
                count
            )).await;
        } else {
            println!("⚠️ Scheduler: stopDate เลยไปแล้ว → จะไม่หยุดอัตโนมัติ");
        }
    } else {
        println!("ℹ️ Scheduler: conditionStopTrade = '{}' → ไม่ใช่ stopDate จึงไม่ตั้งเวลาหยุดอัตโนมัติ", config.trade.condition_stop_trade);
    }
}

async fn background_scheduler_loop(state: AppState) {
    println!("🔄 Background Scheduler is now active and monitoring setup.json...");
    let app_id = env::var("DERIV_APP_ID").unwrap_or_else(|_| "1089".to_string());
    let mut is_auto_trading = false;
    let mut last_processed_start = String::new();

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let api_token = env::var("DERIV_API_TOKEN").unwrap_or_else(|_| "".to_string());
        if api_token.is_empty() || api_token == "YOUR_API_TOKEN_HERE" { continue; }

        let config: AppConfigPayload = match fs::read_to_string("setup.json") {
            Ok(content) => match serde_json::from_str(&content) { Ok(c) => c, Err(_) => continue },
            Err(_) => continue,
        };

        if !config.date_range.use_schedule || config.assets.is_empty() {
            if is_auto_trading {
                println!("🛑 Scheduler: Schedule is disabled in setup.json...");
                is_auto_trading = false;
            }
            continue;
        }

        let start_dt = match NaiveDateTime::parse_from_str(&config.date_range.start_date, "%Y-%m-%dT%H:%M") {
            Ok(dt) => match Local.from_local_datetime(&dt).single() { Some(t) => t, None => continue },
            Err(_) => continue,
        };

        let stop_dt = match NaiveDateTime::parse_from_str(&config.date_range.stop_date, "%Y-%m-%dT%H:%M") {
            Ok(dt) => match Local.from_local_datetime(&dt).single() { Some(t) => t, None => Local::now() + chrono::Duration::days(365) },
            Err(_) => Local::now() + chrono::Duration::days(365),
        };

        let now = Local::now();

        // 1. เช็คเวลา Start Date
        if now >= start_dt && now < stop_dt {
            if !is_auto_trading && last_processed_start != config.date_range.start_date {
                println!("⚡ Scheduler: Target time reached! Automatically starting trading bots.");
                is_auto_trading = true;
                last_processed_start = config.date_range.start_date.clone();

                let mut bot_guard = state.active_bots.lock().await;

                // Stop any old stray bots running manually
                for (asset_name, handle) in bot_guard.drain() {
                    handle.abort();
                }

                let mut spawned_assets = Vec::new();
                for asset in config.assets.iter() {
                    let tx = state.tx.clone();
                    let app_id_clone = app_id.clone();
                    let api_token_clone = api_token.clone();
                    let config_clone = config.clone();
                    let asset_clone = asset.clone();

                    let handle = tokio::spawn(async move {
                        if let Err(e) = deriv::start_deriv_bot(app_id_clone, api_token_clone, config_clone, asset_clone.clone(), tx).await {
                            eprintln!("Bot [{}] crashed: {}", asset_clone, e);
                        }
                    });

                    spawned_assets.push(asset.clone());
                    bot_guard.insert(asset.clone(), handle);
                }
                
                let assets_str = config.assets.join(", ");
                let stop_display = NaiveDateTime::parse_from_str(&config.date_range.stop_date, "%Y-%m-%dT%H:%M")
                    .map(|dt| dt.format("%d/%m/%Y %H:%M").to_string())
                    .unwrap_or_else(|_| config.date_range.stop_date.clone());
                send_telegram_message(&format!("🟢 เริ่มเทรดอัตโนมัติ (Background Scheduler)!\n📊 Assets: {}\n⏰ เวลาตั้ง: {}\n🔴 สิ้นสุด: {}\n📋 Condition: {}", assets_str, start_dt.format("%d/%m/%Y %H:%M:%S"), stop_display, config.trade.condition_stop_trade)).await;

                // ส่ง bot_log ไปยังหน้าเว็บ
                let start_log = format!("🟢 เริ่มเทรดอัตโนมัติ! Assets: {} | สิ้นสุด: {} | Condition: {}",
                    assets_str, stop_display, config.trade.condition_stop_trade);
                let _ = state.tx.send(serde_json::json!({
                    "type": "bot_log",
                    "asset": "system",
                    "data": { "message": start_log }
                }));
            }
        }

        // 2. เช็คเวลา Stop Date
        if now >= stop_dt && is_auto_trading && config.trade.condition_stop_trade == "stopDate" {
            println!("🛑 Scheduler: Target Stop time reached! Assuming auto stop.");
            is_auto_trading = false;

            let mut bot_guard = state.active_bots.lock().await;
            let count = bot_guard.len();
            for (asset_name, handle) in bot_guard.drain() {
                handle.abort();
            }
            
            // ยกเลิก Schedule ใน setup.json ออก เพื่อไม่ให้เริ่มใหม่และเด้งขึ้นอีกครั้ง
            let mut new_config = config.clone();
            new_config.date_range.use_schedule = false;
            let _ = fs::write("setup.json", serde_json::to_string_pretty(&new_config).unwrap_or_default());

            send_telegram_message(&format!("🔴 หยุดเทรดอัตโนมัติ (Background Scheduler)!\n⏰ ถึงเวลา Stop: {}\n📊 หยุดบอททั้งหมด {} ตัว", stop_dt.format("%d/%m/%Y %H:%M:%S"), count)).await;
        }
    }
}

async fn get_version() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "main_rs_version": "v1.3",
        "compiled_at": chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
    }))
}

// ══════════════════════════════════════════════════════════════
//  STATUS API — ให้ Frontend ตรวจสอบสถานะบอทปัจจุบัน
//  เครื่อง B เปิดมาจะได้รู้ว่าบอทกำลังทำงานอยู่หรือไม่
// ══════════════════════════════════════════════════════════════
async fn handle_get_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let bot_guard = state.active_bots.lock().await;
    let active_assets: Vec<String> = bot_guard.keys().cloned().collect();
    let is_trading = !active_assets.is_empty();
    let log_count = state.bot_logs.lock().await.len();
    let candle_assets: Vec<String> = state.candle_data.lock().await.keys().cloned().collect();

    Json(serde_json::json!({
        "is_trading": is_trading,
        "active_assets": active_assets,
        "bot_count": active_assets.len(),
        "log_count": log_count,
        "candle_assets": candle_assets
    }))
}

#[tokio::main]
async fn main() {
    // 1. Load .env file configurations
    dotenv().ok();

    let app_id = env::var("DERIV_APP_ID").unwrap_or_else(|_| "NOT_SET".to_string());
    let port_str = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let port: u16 = port_str.parse().unwrap_or(3000);

    println!("Starting server on port {}", port);
    println!("Deriv App ID configured: {}", app_id);

    // 2. Set up Axum Router with State
    let (tx, _rx) = broadcast::channel(100);
    let app_state = AppState {
        tx: Arc::new(tx),
        active_bots: Arc::new(Mutex::new(HashMap::new())),
        bot_logs: Arc::new(Mutex::new(Vec::new())),
        candle_data: Arc::new(Mutex::new(HashMap::new())),
    };

    // 3. เปิดใช้งาน Background Scheduler แบบ Active Polling
    let scheduler_state = app_state.clone();
    tokio::spawn(async move {
        background_scheduler_loop(scheduler_state).await;
    });

    // 3.5 Storage Subscriber — ดักจับข้อมูลจาก broadcast เพื่อเก็บไว้ให้ client ใหม่
    //     เครื่อง B เปิดมาจะได้รับ snapshot ของข้อมูลทั้งหมดที่สะสมไว้
    let storage_state = app_state.clone();
    let mut storage_rx = app_state.tx.subscribe();
    tokio::spawn(async move {
        const MAX_LOGS: usize = 500;
        while let Ok(msg) = storage_rx.recv().await {
            let msg_type = msg.get("type").and_then(|t| t.as_str()).unwrap_or("");
            match msg_type {
                "candles_history" => {
                    if let Some(asset) = msg.get("asset").and_then(|a| a.as_str()) {
                        let mut data = storage_state.candle_data.lock().await;
                        data.insert(asset.to_string(), msg);
                    }
                }
                "ohlc_update" => {
                    // อัปเดตข้อมูลแท่งเทียนล่าสุดใน candle_data เพื่อให้ snapshot ทันสมัย
                    if let Some(asset) = msg.get("asset").and_then(|a| a.as_str()) {
                        let mut data = storage_state.candle_data.lock().await;
                        if let Some(stored) = data.get_mut(asset) {
                            if let Some(new_data) = msg.get("data") {
                                if let Some(new_epoch) = new_data.get("epoch").and_then(|e| e.as_i64()) {
                                    if let Some(arr) = stored.get_mut("data").and_then(|d| d.as_array_mut()) {
                                        if let Some(last) = arr.last_mut() {
                                            let last_epoch = last.get("epoch").and_then(|e| e.as_i64()).unwrap_or(0);
                                            if last_epoch == new_epoch {
                                                *last = new_data.clone();
                                            } else if new_epoch > last_epoch {
                                                arr.push(new_data.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                "bot_log" => {
                    let mut logs = storage_state.bot_logs.lock().await;
                    logs.push(msg);
                    if logs.len() > MAX_LOGS {
                        let drain_count = logs.len() - MAX_LOGS;
                        logs.drain(0..drain_count);
                    }
                }
                "balance_update" | "trade_result" => {
                    // เก็บไว้ใน bot_logs ด้วยเพื่อให้ client ใหม่เห็นยอดเงินล่าสุด
                    let mut logs = storage_state.bot_logs.lock().await;
                    logs.push(msg);
                    if logs.len() > MAX_LOGS {
                        let drain_count = logs.len() - MAX_LOGS;
                        logs.drain(0..drain_count);
                    }
                }
                _ => {}
            }
        }
    });

    let app = Router::new()
        .nest_service("/", ServeDir::new("php"))
        .route("/api/trade", post(handle_post_trade))
        .route("/api/stop", post(handle_post_stop))
        .route("/api/terminate", post(handle_post_terminate))
        .route("/api/notify", post(handle_post_notify))
        .route("/api/setup", get(get_setup).post(save_setup))
        .route("/api/history", get(get_history))
        .route("/api/version", get(get_version))
        .route("/api/status", get(handle_get_status))
        .route("/ws", get(ws_handler))
        .with_state(app_state);

    // 4. Start Server
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    println!("Web Server is running at http://localhost:{}", port);
    println!("Serving UI from the 'php/' directory.");

    axum::serve(listener, app).await.unwrap();
}

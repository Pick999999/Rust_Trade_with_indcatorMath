#![recursion_limit = "512"]
use axum::{
    extract::ws::{Message, WebSocket},
    extract::{State, WebSocketUpgrade, Query, Request},
    routing::{get, post},
    response::{IntoResponse, Response},
    http::{header, StatusCode},
    middleware::{self, Next},
    Json, Router,
};
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::collections::HashMap;
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;
use tower_http::services::ServeDir;
use std::fs;
use std::path::Path;
use chrono::{Local, NaiveDateTime, TimeZone};
use utoipa::OpenApi;

pub mod deriv;
pub mod get_action;
pub mod full_analysis_ver2;
pub mod filter_noise;
pub mod long_term;
#[allow(non_snake_case)]
pub mod pkDetectTrend;
#[allow(non_snake_case)]
pub mod getActionByPKTrend;
#[allow(non_snake_case)]
pub mod getActionByPKTrendVerSelectCaseCode;
#[allow(non_snake_case)]
pub mod pkDetectTrend_v5;
pub mod predict_next_candle;
pub mod trade_head;
pub mod node_sync;

// --- Data Models matching the JSON from index.html ---

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Meta {
    #[serde(rename = "generatedAt")]
    pub generated_at: String,
    pub theme: String,
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

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
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
    #[serde(rename = "playSound", default)]
    pub play_sound: bool,
    pub martingale: Martingale,
    #[serde(rename = "autoSellTimeout", default)]
    pub auto_sell_timeout: bool,
    #[serde(rename = "autoSellTimeoutSeconds", default = "default_timeout_seconds")]
    pub auto_sell_timeout_seconds: i32,
    #[serde(rename = "buyMethod", default = "default_buy_method")]
    pub buy_method: String,
    #[serde(rename = "maxLossCon", default)]
    pub max_loss_con: u32,
    #[serde(rename = "suggestStrategy", default = "default_suggest_strategy")]
    pub suggest_strategy: String,
    #[serde(rename = "thbRate", default = "default_thb_rate")]
    pub thb_rate: f64,
    #[serde(rename = "borrowSignal", default)]
    pub borrow_signal: bool,
    #[serde(rename = "whipsawZone", default = "default_whipsaw_zone")]
    pub whipsaw_zone: bool,
    #[serde(rename = "saveTrackOrder", default)]
    pub save_track_order: bool,
    #[serde(rename = "checkBbFlatChoppy", default = "default_check_bb_flat_choppy")]
    pub check_bb_flat_choppy: bool,
    #[serde(rename = "checkBbSqueeze", default = "default_check_bb_squeeze")]
    pub check_bb_squeeze: bool,
    #[serde(rename = "checkKarmaChoppy", default = "default_check_karma_choppy")]
    pub check_karma_choppy: bool,
}

fn default_buy_method() -> String {
    "proposal".to_string()
}

fn default_timeout_seconds() -> i32 {
    90
}

fn default_suggest_strategy() -> String {
    "V1".to_string()
}

fn default_thb_rate() -> f64 {
    35.0
}

fn default_whipsaw_zone() -> bool {
    true
}

fn default_check_bb_flat_choppy() -> bool {
    true
}

fn default_check_bb_squeeze() -> bool {
    true
}

fn default_check_karma_choppy() -> bool {
    true
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfigPayload {
    pub meta: Meta,
    pub granularity: i32,
    #[serde(rename = "granularitySettings")]
    pub granularity_settings: i32,
    pub assets: Vec<String>,
    pub indicators: Indicators,
    pub ema: EmaConfig,
    pub trade: TradeConfig,
}

#[derive(Debug, Deserialize)]
pub struct FingerprintPayload {
    pub fingerprint: String,
    pub device_info: Option<String>,
    pub device_name: Option<String>,
}

// --- Deriv Account Management ---
pub fn get_active_credentials(_host_opt: Option<String>) -> (String, String, String) {
    if let Ok(accounts_str) = env::var("DERIV_ACCOUNTS") {
        let clean_str = accounts_str.trim().trim_matches('\'').trim_matches('"');
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(clean_str) {
            let target_obj = if val.is_object() {
                Some(val)
            } else if val.is_array() {
                val.as_array().and_then(|arr| arr.first()).cloned()
            } else {
                None
            };

            if let Some(acc) = target_obj {
                let app_id = acc.get("DERIV_APP_ID").and_then(|v| v.as_str()).unwrap_or("1089").trim().to_string();
                let api_token = acc.get("DERIV_API_TOKEN").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                let account_id = acc.get("DERIV_ACCOUNT_ID").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                return (app_id, api_token, account_id);
            }
        }
    }
    
    // Fallback if DERIV_ACCOUNTS doesn't exist
    (
        env::var("DERIV_APP_ID").unwrap_or_else(|_| "1089".to_string()).trim().to_string(),
        env::var("DERIV_API_TOKEN").unwrap_or_else(|_| "".to_string()).trim().to_string(),
        env::var("DERIV_ACCOUNT_ID").unwrap_or_else(|_| "".to_string()).trim().to_string(),
    )
}

pub fn get_active_account_name() -> String {
    if let Ok(accounts_str) = env::var("DERIV_ACCOUNTS") {
        let clean_str = accounts_str.trim().trim_matches('\'').trim_matches('"');
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(clean_str) {
            let target_obj = if val.is_object() {
                Some(val)
            } else if val.is_array() {
                val.as_array().and_then(|arr| arr.first()).cloned()
            } else {
                None
            };

            if let Some(acc) = target_obj {
                if let Some(name) = acc.get("name").and_then(|v| v.as_str()) {
                    return name.trim().to_string();
                }
            }
        }
    }
    "".to_string()
}

// --- Trade Control ---
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TradeControl {
    #[serde(rename = "DayTrade")]
    pub day_trade: String,
    #[serde(rename = "totalTrade")]
    pub total_trade: u32,
    #[serde(rename = "TradeStatus")]
    pub trade_status: String,
    #[serde(rename = "startTradeTime")]
    pub start_trade_time: String,
    #[serde(rename = "stopTradeTime")]
    pub stop_trade_time: String,
    #[serde(rename = "durationTrade")]
    pub duration_trade: String,
    #[serde(rename = "actionStop")]
    pub action_stop: String,
    #[serde(rename = "actualStartTime", default)]
    pub actual_start_time: String,
    #[serde(rename = "actualStopTime", default)]
    pub actual_stop_time: String,
    #[serde(rename = "useSchedule", default)]
    pub use_schedule: bool,
}

fn get_trade_control_path() -> String {
    let now = chrono::Local::now();
    use chrono::Datelike;
    let month_folder = format!("{:02}-{:04}", now.month(), now.year() as i32 + 543);
    let day_folder = format!("{:02}-{:02}-{:04}", now.day(), now.month(), now.year() as i32 + 543);
    let today_dir = format!("tradeData/{}/{}", month_folder, day_folder);
    let _ = std::fs::create_dir_all(&today_dir);
    format!("{}/tradeControl.json", today_dir)
}

pub fn get_or_create_trade_control() -> TradeControl {
    let path = get_trade_control_path();
    let now = chrono::Local::now();
    use chrono::Datelike;
    let day_trade = format!("{:02}-{:02}-{:04}", now.day(), now.month(), now.year() as i32 + 543);
    
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(tc) = serde_json::from_str::<TradeControl>(&content) {
            return tc;
        }
    }
    
    let date_prefix = format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day());
    let default_start = format!("{} 00:05:00", date_prefix);
    let default_stop = format!("{} 01:05:00", date_prefix);

    let tc = TradeControl {
        day_trade,
        total_trade: 0,
        trade_status: "ปิดเทรดอยู่".to_string(),
        start_trade_time: default_start,
        stop_trade_time: default_stop,
        duration_trade: "".to_string(),
        action_stop: "".to_string(),
        actual_start_time: "".to_string(),
        actual_stop_time: "".to_string(),
        use_schedule: false,
    };
    if let Ok(json_str) = serde_json::to_string_pretty(&tc) {
        let _ = std::fs::write(&path, json_str);
    }
    tc
}

fn update_trade_control(status: &str, increment_trade: bool, action_stop_opt: Option<&str>) -> TradeControl {
    let mut tc = get_or_create_trade_control();
    tc.trade_status = status.to_string();
    if increment_trade {
        tc.total_trade += 1;
    }

    let now_local = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    if status == "กำลังเทรด" {
        tc.actual_start_time = now_local;
        tc.actual_stop_time = "".to_string();
        tc.duration_trade = "".to_string();
        tc.action_stop = "".to_string();
    } else if status == "ปิดเทรดอยู่" {
        tc.actual_stop_time = now_local.clone();
        if let Ok(start) = chrono::NaiveDateTime::parse_from_str(&tc.actual_start_time, "%Y-%m-%d %H:%M:%S") {
            if let Ok(stop) = chrono::NaiveDateTime::parse_from_str(&now_local, "%Y-%m-%d %H:%M:%S") {
                let duration = stop.signed_duration_since(start);
                let secs = duration.num_seconds();
                if secs > 0 {
                    let mins = secs / 60;
                    let s = secs % 60;
                    let hrs = mins / 60;
                    let m = mins % 60;
                    if hrs > 0 {
                        tc.duration_trade = format!("{}h {}m {}s", hrs, m, s);
                    } else if mins > 0 {
                        tc.duration_trade = format!("{}m {}s", m, s);
                    } else {
                        tc.duration_trade = format!("{}s", s);
                    }
                } else {
                    tc.duration_trade = "0s".to_string();
                }
            }
        }
        if let Some(action) = action_stop_opt {
            tc.action_stop = action.to_string();
        }
    }

    if let Ok(json_str) = serde_json::to_string_pretty(&tc) {
        let _ = std::fs::write(get_trade_control_path(), json_str);
    }



    tc
}

// --- Long Term Config & Control ---
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LtTradeControl {
    pub use_schedule: bool,
    pub start_trade_time: String,
    pub stop_trade_time: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LongTermSetup {
    pub assets: Vec<String>,
    pub granularity: i32,
    pub duration: i32,
    pub exit_strategy: String,
    pub conditions: Vec<String>,
    pub amount: f64,
    pub target_profit: f64,
    #[serde(default = "default_true")]
    pub auto_trade: bool,
    #[serde(default = "default_max_orders")]
    pub max_orders: i32,
    #[serde(default)]
    pub martingale: Martingale,
    #[serde(rename = "maxLossCon", default)]
    pub max_loss_con: u32,
}

fn default_true() -> bool { true }
fn default_max_orders() -> i32 { 1 }

pub fn get_lt_trade_control_path() -> String {
    let now = chrono::Local::now();
    use chrono::Datelike;
    let month_folder = format!("{:02}-{:04}", now.month(), now.year() as i32 + 543);
    let day_folder = format!("{:02}-{:02}-{:04}", now.day(), now.month(), now.year() as i32 + 543);
    let today_dir = format!("tradeData/{}/{}", month_folder, day_folder);
    let _ = std::fs::create_dir_all(&today_dir);
    format!("{}/ltTradeControl.json", today_dir)
}

pub fn get_or_create_lt_trade_control() -> LtTradeControl {
    let path = get_lt_trade_control_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(tc) = serde_json::from_str::<LtTradeControl>(&content) {
            return tc;
        }
    }
    
    let now = chrono::Local::now();
    use chrono::Datelike;
    let date_prefix = format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day());
    let default_start = format!("{} 00:05:00", date_prefix);
    let default_stop = format!("{} 01:05:00", date_prefix);

    let tc = LtTradeControl {
        use_schedule: false,
        start_trade_time: default_start,
        stop_trade_time: default_stop,
    };
    if let Ok(json_str) = serde_json::to_string_pretty(&tc) {
        let _ = std::fs::write(&path, json_str);
    }
    tc
}

pub fn get_lt_setup_path() -> String {
    "setup/longTermSetup.json".to_string()
}

pub fn get_or_create_lt_setup() -> LongTermSetup {
    let path = get_lt_setup_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(setup) = serde_json::from_str::<LongTermSetup>(&content) {
            return setup;
        }
    }
    
    let setup = LongTermSetup {
        assets: vec![],
        granularity: 60,
        duration: 3600,
        exit_strategy: "profit".to_string(),
        conditions: vec![],
        amount: 10.0,
        target_profit: 30.0,
        auto_trade: true,
        max_orders: 1,
        martingale: Martingale::default(),
        max_loss_con: 0,
    };
    if let Ok(json_str) = serde_json::to_string_pretty(&setup) {
        let _ = std::fs::write(&path, json_str);
    }
    setup
}

async fn handle_get_trade_control() -> Json<serde_json::Value> {
    let tc = get_or_create_trade_control();
    Json(serde_json::json!({
        "status": "success",
        "data": tc
    }))
}

#[derive(Deserialize)]
pub struct UpdateTradeControlRequest {
    #[serde(rename = "startTradeTime")]
    pub start_trade_time: Option<String>,
    #[serde(rename = "stopTradeTime")]
    pub stop_trade_time: Option<String>,
    #[serde(rename = "useSchedule")]
    pub use_schedule: Option<bool>,
}

async fn handle_post_trade_control(Json(payload): Json<UpdateTradeControlRequest>) -> Json<serde_json::Value> {
    let mut tc = get_or_create_trade_control();
    if let Some(start) = payload.start_trade_time {
        tc.start_trade_time = start;
    }
    if let Some(stop) = payload.stop_trade_time {
        tc.stop_trade_time = stop;
    }
    if let Some(use_sch) = payload.use_schedule {
        tc.use_schedule = use_sch;
    }
    if let Ok(json_str) = serde_json::to_string_pretty(&tc) {
        let _ = std::fs::write(get_trade_control_path(), json_str);
    }
    Json(serde_json::json!({
        "status": "success",
        "data": tc
    }))
}

async fn handle_post_restart_service() -> Json<serde_json::Value> {
    println!("⚠️ [System] Received Restart Service request from Web UI / Dashboard");
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        println!("🔄 [System] Exiting process now. systemd will automatically restart the service...");
        std::process::exit(0);
    });
    Json(serde_json::json!({
        "status": "success",
        "message": "Service is restarting in 5 seconds via systemd..."
    }))
}



// --- Middleware ---
async fn auth_middleware(req: Request, next: Next) -> Result<Response, StatusCode> {
    let headers = req.headers();
    let mut host_str = String::new();
    
    if let Some(forwarded) = headers.get("x-forwarded-host").and_then(|h| h.to_str().ok()) {
        host_str = forwarded.to_string();
    } else if let Some(forwarded_server) = headers.get("x-forwarded-server").and_then(|h| h.to_str().ok()) {
        host_str = forwarded_server.to_string();
    } else if let Some(host) = headers.get("host").and_then(|h| h.to_str().ok()) {
        host_str = host.to_string();
    }
    
    if !host_str.is_empty() {
        let _ = std::fs::write("setup/active_host.txt", &host_str);
    }

    let expected_key = std::env::var("SERVER_API_KEY").unwrap_or_default();
    
    // Fallback for development if no key is configured
    if expected_key.is_empty() || expected_key == "YOUR_SECRET_KEY_HERE" {
        return Ok(next.run(req).await);
    }

    let token = req
        .headers()
        .get("X-API-Key")
        .and_then(|value| value.to_str().ok());

    let mut is_valid = false;

    if let Some(t) = token {
        if t == expected_key {
            is_valid = true;
        }
    }

    // Check query string for WebSocket or alternative
    if !is_valid {
        if let Some(query) = req.uri().query() {
            let params: std::collections::HashMap<String, String> = query
                .split('&')
                .filter_map(|kv| {
                    let mut parts = kv.splitn(2, '=');
                    Some((parts.next()?.to_string(), parts.next()?.to_string()))
                })
                .collect();
            if params.get("token") == Some(&expected_key) {
                is_valid = true;
            }
        }
    }

    if is_valid {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}


// --- Shared App State ---
#[derive(Clone)]
pub struct AppState {
    pub tx: Arc<broadcast::Sender<serde_json::Value>>,
    pub cmd_tx: Arc<broadcast::Sender<serde_json::Value>>,
    pub lt_cmd_tx: Arc<broadcast::Sender<serde_json::Value>>,
    pub active_bot: Arc<Mutex<Option<JoinHandle<()>>>>,
    pub long_term_bot: Arc<Mutex<Option<JoinHandle<()>>>>,
    pub overtime_signals: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    pub bot_logs: Arc<Mutex<Vec<serde_json::Value>>>,
    pub candle_data: Arc<Mutex<HashMap<String, serde_json::Value>>>,
    pub last_balance: Arc<Mutex<f64>>,
    pub lt_ws_status: Arc<Mutex<HashMap<String, String>>>,
    pub aggregator: Arc<node_sync::MultiNodeAggregator>,
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

    // 3. ส่ง lt_ws_status ย้อนหลังให้ Client ใหม่
    {
        let status_guard = state.lt_ws_status.lock().await;
        for (target, status) in status_guard.iter() {
            let status_msg = serde_json::json!({
                "type": "lt_ws_status",
                "target": target,
                "status": status
            });
            if socket.send(Message::Text(status_msg.to_string())).await.is_err() {
                println!("Client disconnected during status snapshot");
                return;
            }
        }
    }

    // 4. ส่ง multi_node_summary ย้อนหลังให้ Client ใหม่
    {
        let summary = state.aggregator.get_summary().await;
        let summary_msg = serde_json::json!({
            "type": "multi_node_summary",
            "data": summary
        });
        if socket.send(Message::Text(summary_msg.to_string())).await.is_err() {
            println!("Client disconnected during multi_node_summary snapshot");
            return;
        }
    }

    println!("📦 Snapshot sent to new client ({} candle sets, {} logs, {} ws statuses, multi-node summary)",
        state.candle_data.lock().await.len(),
        state.bot_logs.lock().await.len(),
        state.lt_ws_status.lock().await.len()
    );

    // Loop เพื่อส่งข้อมูลจาก Broadcast กลับไปให้ Browser (index.html) ทันที
    loop {
        match rx.recv().await {
            Ok(msg) => {
                let text = msg.to_string();
                if socket.send(Message::Text(text)).await.is_err() {
                    println!("Client disconnected");
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                println!("⚠️ WebSocket client lagged by {} messages. Continuing...", n);
                continue;
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                println!("🛑 Broadcast channel closed.");
                break;
            }
        }
    }
}

async fn handle_post_trade(
    State(state): State<AppState>,
    Json(payload): Json<AppConfigPayload>,
) -> Json<serde_json::Value> {
    println!("Received Payload from Web UI.");

    let (app_id, api_token, account_id) = crate::get_active_credentials(None);

    if api_token.is_empty() || api_token == "YOUR_API_TOKEN_HERE" || account_id.is_empty() {
        return Json(serde_json::json!({
            "status": "error",
            "message": "กรุณาแก้ไฟล์ .env เพื่อใส่ DERIV_API_TOKEN และ DERIV_ACCOUNT_ID ให้เรียบร้อย"
        }));
    }

    let mut bot_guard = state.active_bot.lock().await;
    let mut ot_guard = state.overtime_signals.lock().await;

    // ╔══════════════════════════════════════════════════════════════════╗
    // ║ ⚠️ CRITICAL BUG FIX — ห้ามลบหรือแก้ไขโค้ดส่วนนี้! ⚠️           ║
    // ║ ป้องกัน "Schedule Trade Round +1 Bug"                           ║
    // ║ ต้องตรวจ was_already_running ก่อน +1 totalTrade                  ║
    // ║ ถ้าลบส่วนนี้ → เปิดหน้าเว็บระหว่างเทรด → totalTrade +1 ซ้ำ     ║
    // ╚══════════════════════════════════════════════════════════════════╝
    let was_already_running = bot_guard.as_ref().map_or(false, |h| !h.is_finished());
    
    // ปิดบอทตัวเก่าทั้งหมดก่อน (ถ้ามีรันอยู่)
    if let Some(handle) = bot_guard.take() {
        handle.abort();
        println!("🛑 Aborted old multiplexed bot");
    }
    ot_guard.clear();

    let mut spawned_assets = Vec::new();
    for asset in payload.assets.iter() {
        let ot_signal = Arc::new(AtomicBool::new(false));
        ot_guard.insert(asset.clone(), ot_signal);
        spawned_assets.push(asset.clone());
    }

    let tx = state.tx.clone();
    let cmd_rx = state.cmd_tx.subscribe();
    let app_id = app_id.clone();
    let api_token = api_token.clone();
    let account_id = account_id.clone();
    let config = payload.clone();
    
    // ถ้าบอทรันอยู่แล้ว (was_already_running) ไม่ต้อง +1 totalTrade
    let should_increment = !was_already_running;
    if was_already_running {
        println!("⚠️ [GoTrade] Bot was already running — skipping totalTrade increment");
    }
    let tc = update_trade_control("กำลังเทรด", should_increment, None);
    trade_head::on_bot_start(tc.total_trade, &tc.actual_start_time, &spawned_assets, &payload.trade.suggest_strategy);
    
    let assets = spawned_assets.clone();
    let ot_signals = state.overtime_signals.clone();

    let handle = tokio::spawn(async move {
        if let Err(e) = deriv::start_deriv_bot_multiplexed(app_id, api_token, account_id, config, assets, tx.clone(), cmd_rx, ot_signals).await {
            eprintln!("Multiplexed bot crashed: {}", e);
        }

        let tc = update_trade_control("ปิดเทรดอยู่", false, Some("เทรดสำเร็จ"));
        trade_head::on_bot_stop(tc.total_trade, &tc.actual_stop_time);
        let _ = tx.send(serde_json::json!({
            "type": "bot_log",
            "asset": "system",
            "data": { "message": format!("🏁 [System] เทรดรอบที่ {} จบการทำงาน สถานะ: {}", tc.total_trade, tc.trade_status) }
        }));
    });

    *bot_guard = Some(handle);

    let assets_str = spawned_assets.join(", ");
    println!("🚀 Spawned 1 multiplexed bot for {} asset(s): {:?}", spawned_assets.len(), spawned_assets);

    // ส่ง Telegram แจ้งเตือนเมื่อเริ่มเทรด
    let now_str = Local::now().format("%d/%m/%Y %H:%M:%S").to_string();
    let stop_display = if tc.stop_trade_time.is_empty() {
        "ไม่ได้ตั้งค่า".to_string()
    } else {
        tc.stop_trade_time.clone()
    };
    let vps_name = get_vps_name();
    send_telegram_message(&format!(
        "🟢 [{}] เริ่มเทรด!\n📊 Assets: {}\n⏰ เริ่ม: {}\n🔴 สิ้นสุด: {}\n📋 Condition: {}",
        vps_name, assets_str, now_str, stop_display, payload.trade.condition_stop_trade
    )).await;

    // ส่ง bot_log ไปยังหน้าเว็บ
    let start_log = format!("🟢 [{}] เริ่มเทรด! Assets: {} | สิ้นสุด: {} | Condition: {}",
        vps_name, assets_str, stop_display, payload.trade.condition_stop_trade);
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
    let mut bot_guard = state.active_bot.lock().await;
    let count = if bot_guard.is_some() { 1 } else { 0 };
    let mut stopped_round = 0;
    if let Some(handle) = bot_guard.take() {
        handle.abort();
        println!("🛑 หยุดบอท Multiplexed");
        let tc = update_trade_control("ปิดเทรดอยู่", false, Some("กดหยุดเทรด"));
        stopped_round = tc.total_trade;
        trade_head::on_bot_stop(tc.total_trade, &tc.actual_stop_time);
    }
    println!("🛑 หยุดบอททั้งหมด {} ตัว", count);

    // ส่ง Telegram แจ้งเตือนเมื่อหยุดเทรด
    let vps_name = get_vps_name();
    let (max_loss, loss_assets) = trade_head::get_round_max_loss(stopped_round);
    let now_str = Local::now().format("%d/%m/%Y %H:%M:%S").to_string();
    send_telegram_message(&format!(
        "🔴 [{}] หยุดเทรด!\n⏰ เวลา: {}\n📊 หยุดบอททั้งหมด {} ตัว\n📉 MaxLossCon: {} (Asset: {})",
        vps_name, now_str, count, max_loss, loss_assets
    )).await;

    Json(serde_json::json!({
        "status": "success",
        "message": format!("หยุดบอททั้งหมด {} ตัวแล้ว", count)
    }))
}

async fn handle_post_terminate(State(state): State<AppState>) -> Json<serde_json::Value> {
    // ปิดบอททั้งหมดก่อน
    let mut bot_guard = state.active_bot.lock().await;
    if let Some(handle) = bot_guard.take() {
        handle.abort();
        println!("🛑 Terminate: หยุดบอท Multiplexed");
        let tc = update_trade_control("ปิดเทรดอยู่", false, Some("กด Terminate"));
        trade_head::on_bot_stop(tc.total_trade, &tc.actual_stop_time);
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

#[derive(Deserialize)]
pub struct LongTermStartRequest {
    pub assets: Vec<String>,
    pub granularity: i32,
}

async fn handle_post_longterm_start(
    State(state): State<AppState>,
    Json(payload): Json<LongTermStartRequest>,
) -> Json<serde_json::Value> {
    let mut bot_guard = state.long_term_bot.lock().await;
    if let Some(handle) = bot_guard.take() {
        handle.abort();
        println!("🛑 Aborted old Long Term bot");
    }

    let tx = state.tx.clone();
    let lt_cmd_tx_clone = state.lt_cmd_tx.clone();
    let assets = payload.assets.clone();
    let granularity = payload.granularity;

    let handle = tokio::spawn(async move {
        if let Err(e) = crate::long_term::start_longterm_bot(assets, granularity, tx, lt_cmd_tx_clone).await {
            eprintln!("Long Term bot crashed: {}", e);
        }
    });

    *bot_guard = Some(handle);

    Json(serde_json::json!({
        "status": "success",
        "message": format!("เริ่ม Long Term bot สำหรับ {} assets", payload.assets.len())
    }))
}

#[derive(Deserialize)]
pub struct LongTermReconnectWsRequest {
    pub target: String,
}

async fn handle_post_lt_reconnect_ws(
    State(state): State<AppState>,
    Json(payload): Json<LongTermReconnectWsRequest>,
) -> Json<serde_json::Value> {
    let setup = get_or_create_lt_setup();
    let assets = if setup.assets.is_empty() {
        vec!["vol50".to_string()]
    } else {
        setup.assets.clone()
    };
    let granularity = setup.granularity;

    println!("🔄 [Reconnect WS Request] Target: {}", payload.target);

    let mut bot_guard = state.long_term_bot.lock().await;
    if let Some(handle) = bot_guard.take() {
        handle.abort();
        println!("🛑 Aborted previous Long Term bot for reconnect ({})", payload.target);
    }

    let tx = state.tx.clone();
    let lt_cmd_tx_clone = state.lt_cmd_tx.clone();

    let _ = tx.send(serde_json::json!({
        "type": "lt_ws_status",
        "target": payload.target,
        "status": "connecting"
    }));

    let handle = tokio::spawn(async move {
        if let Err(e) = crate::long_term::start_longterm_bot(assets, granularity, tx, lt_cmd_tx_clone).await {
            eprintln!("Long Term bot crashed after reconnect: {}", e);
        }
    });

    *bot_guard = Some(handle);

    Json(serde_json::json!({
        "status": "success",
        "message": format!("Reconnecting {} WebSocket...", payload.target)
    }))
}

#[derive(Deserialize)]
pub struct LongTermScheduleRequest {
    pub assets: Option<Vec<String>>,
    pub granularity: Option<i32>,
    pub duration: Option<i32>,
    pub exit_strategy: Option<String>,
    pub conditions: Option<Vec<String>>,
    pub start_time: Option<String>,
    pub stop_time: Option<String>,
    pub use_schedule: Option<bool>,
    pub amount: Option<f64>,
    pub target_profit: Option<f64>,
    pub max_orders: Option<i32>,
    pub play_ema_cut_sound: Option<bool>,
    pub play_close_order_sound: Option<bool>,
    pub martingale: Option<Martingale>,
    #[serde(rename = "maxLossCon")]
    pub max_loss_con: Option<u32>,
}

async fn handle_get_longterm_schedule() -> Json<serde_json::Value> {
    let setup = get_or_create_lt_setup();
    let tc = get_or_create_lt_trade_control();
    
    let response = serde_json::json!({
        "status": "success",
        "data": {
            "assets": setup.assets,
            "granularity": setup.granularity,
            "duration": setup.duration,
            "exit_strategy": setup.exit_strategy,
            "conditions": setup.conditions,
            "amount": setup.amount,
            "target_profit": setup.target_profit,
            "max_orders": setup.max_orders,
            "use_schedule": tc.use_schedule,
            "start_time": tc.start_trade_time,
            "stop_time": tc.stop_trade_time,
            "martingale": setup.martingale,
            "maxLossCon": setup.max_loss_con,
        }
    });
    
    Json(response)
}

async fn handle_get_active_settings() -> Json<serde_json::Value> {
    let path = Path::new("setup/longterm_settings.json");
    let mut ema_config = serde_json::json!({
        "short": { "period": 9, "type": "ema", "color": "#06d6a0", "enabled": true },
        "medium": { "period": 21, "type": "ema", "color": "#f5a623", "enabled": true },
        "long": { "period": 50, "type": "ema", "color": "#e8304a", "enabled": true }
    });
    let mut indicators = serde_json::json!({});
    
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(ema_json) = json.get("ema") {
                let short_period = ema_json.get("shortPeriod").and_then(|v| v.as_u64()).unwrap_or(9);
                let short_type = ema_json.get("shortType").and_then(|v| v.as_str()).unwrap_or("ema");
                let medium_period = ema_json.get("mediumPeriod").and_then(|v| v.as_u64()).unwrap_or(21);
                let medium_type = ema_json.get("mediumType").and_then(|v| v.as_str()).unwrap_or("ema");
                let long_period = ema_json.get("longPeriod").and_then(|v| v.as_u64()).unwrap_or(50);
                let long_type = ema_json.get("longType").and_then(|v| v.as_str()).unwrap_or("ema");

                ema_config = serde_json::json!({
                    "short": { "period": short_period, "type": short_type, "color": "#06d6a0", "enabled": true },
                    "medium": { "period": medium_period, "type": medium_type, "color": "#f5a623", "enabled": true },
                    "long": { "period": long_period, "type": long_type, "color": "#e8304a", "enabled": true }
                });
            }
            if let Some(inds) = json.get("indicators") {
                indicators = inds.clone();
            }
        }
    }
    
    let setup = get_or_create_lt_setup();
    
    Json(serde_json::json!({
        "status": "success",
        "data": {
            "ema": ema_config,
            "indicators": indicators,
            "trade": {
                "assets": setup.assets,
                "granularity": setup.granularity,
                "duration": setup.duration,
                "exit_strategy": setup.exit_strategy,
                "conditions": setup.conditions,
                "amount": setup.amount,
                "target_profit": setup.target_profit,
                "auto_trade": setup.auto_trade,
                "max_orders": setup.max_orders,
                "martingale": setup.martingale,
                "maxLossCon": setup.max_loss_con,
            }
        }
    }))
}


async fn handle_post_longterm_schedule(
    State(state): State<AppState>,
    Json(payload): Json<LongTermScheduleRequest>,
) -> Json<serde_json::Value> {
    // 1. Save setup only if at least one setup parameter is provided
    let mut setup = get_or_create_lt_setup();
    let mut setup_changed = false;

    if let Some(assets) = payload.assets {
        setup.assets = assets;
        setup_changed = true;
    }
    if let Some(granularity) = payload.granularity {
        setup.granularity = granularity;
        setup_changed = true;
    }
    if let Some(duration) = payload.duration {
        setup.duration = duration;
        setup_changed = true;
    }
    if let Some(exit_strategy) = payload.exit_strategy {
        setup.exit_strategy = exit_strategy;
        setup_changed = true;
    }
    if let Some(conditions) = payload.conditions {
        setup.conditions = conditions;
        setup_changed = true;
    }
    if let Some(amount) = payload.amount {
        setup.amount = amount;
        setup_changed = true;
    }
    if let Some(target_profit) = payload.target_profit {
        setup.target_profit = target_profit;
        setup_changed = true;
    }
    if let Some(max_orders) = payload.max_orders {
        setup.max_orders = max_orders;
        setup_changed = true;
    }
    if let Some(martingale) = payload.martingale {
        setup.martingale = martingale;
        setup_changed = true;
    }
    if let Some(max_loss_con) = payload.max_loss_con {
        setup.max_loss_con = max_loss_con;
        setup_changed = true;
    }

    if setup_changed {
        if let Ok(json_str) = serde_json::to_string_pretty(&setup) {
            let _ = std::fs::write(get_lt_setup_path(), json_str);
        }
    }
    
    // 2. Save schedule
    let mut tc = get_or_create_lt_trade_control();
    let mut use_sched = false;
    if let Some(use_schedule) = payload.use_schedule {
        tc.use_schedule = use_schedule;
        use_sched = use_schedule;
    }
    
    let mut start_str = String::new();
    if let Some(start_time) = payload.start_time {
        if !start_time.is_empty() {
            tc.start_trade_time = start_time.clone();
            start_str = start_time;
        }
    }
    if let Some(stop_time) = payload.stop_time {
        if !stop_time.is_empty() {
            tc.stop_trade_time = stop_time;
        }
    }
    if let Ok(json_str) = serde_json::to_string_pretty(&tc) {
        let _ = std::fs::write(get_lt_trade_control_path(), json_str);
    }

    // 3. Save sound settings to setup/longterm_settings.json if provided
    if let (Some(play_ema), Some(play_close)) = (payload.play_ema_cut_sound, payload.play_close_order_sound) {
        let settings_file = "setup/longterm_settings.json";
        let mut settings_json = if let Ok(content) = fs::read_to_string(settings_file) {
            serde_json::from_str::<serde_json::Value>(&content).unwrap_or(serde_json::json!({}))
        } else {
            serde_json::json!({})
        };
        settings_json["playEmaCutSound"] = serde_json::json!(play_ema);
        settings_json["playCloseOrderSound"] = serde_json::json!(play_close);
        if let Ok(json_str) = serde_json::to_string_pretty(&settings_json) {
            let _ = fs::write(settings_file, json_str);
        }
    }
    
    let msg = if use_sched {
        format!("📅 บันทึกการตั้งเวลา Long Term เรียบร้อย (Start: {})", start_str)
    } else {
        "🛑 ปิดการใช้งานตั้งเวลา Long Term".to_string()
    };
    let _ = state.tx.send(serde_json::json!({
        "type": "bot_log",
        "asset": "system",
        "data": { "message": msg }
    }));

    Json(serde_json::json!({
        "status": "success",
        "message": "Long term schedule saved."
    }))
}

async fn handle_post_longterm_stop(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut bot_guard = state.long_term_bot.lock().await;
    let stopped = if let Some(handle) = bot_guard.take() {
        handle.abort();
        true
    } else {
        false
    };

    {
        let mut map = state.lt_ws_status.lock().await;
        map.insert("candle".to_string(), "disconnected".to_string());
        map.insert("private".to_string(), "disconnected".to_string());
    }

    let _ = state.tx.send(serde_json::json!({
        "type": "lt_ws_status",
        "target": "candle",
        "status": "disconnected"
    }));
    let _ = state.tx.send(serde_json::json!({
        "type": "lt_ws_status",
        "target": "private",
        "status": "disconnected"
    }));

    Json(serde_json::json!({
        "status": "success",
        "message": if stopped { "หยุด Long Term bot แล้ว" } else { "ไม่มี Long Term bot ทำงานอยู่" }
    }))
}

#[derive(Deserialize)]
pub struct LtToggleAutoRequest {
    pub enabled: bool,
}

async fn handle_post_longterm_toggle_auto(
    State(state): State<AppState>,
    Json(payload): Json<LtToggleAutoRequest>,
) -> Json<serde_json::Value> {
    let mut setup = get_or_create_lt_setup();
    setup.auto_trade = payload.enabled;
    if let Ok(json_str) = serde_json::to_string_pretty(&setup) {
        let _ = std::fs::write(get_lt_setup_path(), json_str);
    }
    
    let msg = if payload.enabled {
        "⚙️ ระบบ Auto Trade: เปิด (ON)".to_string()
    } else {
        "⚙️ ระบบ Auto Trade: ปิด (OFF)".to_string()
    };
    
    // Log to both console and WebSocket
    println!("🔄 [Backend] Auto Trade toggled: {}", if payload.enabled { "ON" } else { "OFF" });
    
    let _ = state.tx.send(serde_json::json!({
        "type": "bot_log",
        "asset": "system",
        "data": { "message": msg }
    }));
    
    Json(serde_json::json!({ "status": "success", "enabled": payload.enabled }))
}

#[derive(Deserialize)]
pub struct LtChartCandlesQuery {
    pub asset: String,
    pub granularity: i32,
}

async fn handle_get_lt_chart_candles(
    Query(params): Query<LtChartCandlesQuery>,
) -> Json<serde_json::Value> {
    match crate::long_term::fetch_lt_chart_candles(&params.asset, params.granularity).await {
        Ok(data) => {
            Json(serde_json::json!({
                "status": "success",
                "asset": params.asset,
                "granularity": params.granularity,
                "count": data.len(),
                "data": data
            }))
        }
        Err(e) => {
            Json(serde_json::json!({
                "status": "error",
                "message": e.to_string()
            }))
        }
    }
}

// ══════════════════════════════════════════════════════════════
//  LT SETTINGS API
// ══════════════════════════════════════════════════════════════
async fn handle_get_lt_settings() -> Json<serde_json::Value> {
    let settings_file = "setup/longterm_settings.json";
    
    match fs::read_to_string(settings_file) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(settings) => Json(settings),
                Err(_) => Json(serde_json::json!({
                    "status": "error",
                    "message": "Failed to parse settings"
                }))
            }
        }
        Err(_) => {
            // Return empty settings if file doesn't exist
            Json(serde_json::json!({
                "status": "error",
                "message": "No settings found"
            }))
        }
    }
}

async fn handle_post_lt_settings(
    axum::Json(settings): axum::Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let settings_file = "setup/longterm_settings.json";
    
    // Create setup directory if it doesn't exist
    let _ = fs::create_dir_all("setup");

    // Also sync to longTermSetup.json so long_term.rs reads the updated setup
    let mut lt_setup = get_or_create_lt_setup();
    if let Some(assets) = settings.get("assets").and_then(|v| v.as_array()) {
        lt_setup.assets = assets.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
    }
    if let Some(exit_strat) = settings.get("exitStrategy").and_then(|v| v.as_str()).or_else(|| settings.get("exit_strategy").and_then(|v| v.as_str())) {
        lt_setup.exit_strategy = exit_strat.to_string();
    }
    if let Some(cond_obj) = settings.get("conditions").and_then(|v| v.as_object()) {
        let mut cond_vec = Vec::new();
        if cond_obj.get("condShortMedium").and_then(|v| v.as_bool()).unwrap_or(false) {
            cond_vec.push("condShortMedium".to_string());
        }
        if cond_obj.get("condLongCross").and_then(|v| v.as_bool()).unwrap_or(false) {
            cond_vec.push("condLongCross".to_string());
        }
        if cond_obj.get("condShortLong").and_then(|v| v.as_bool()).unwrap_or(false) {
            cond_vec.push("condShortLong".to_string());
        }
        lt_setup.conditions = cond_vec;
    } else if let Some(cond_arr) = settings.get("conditions").and_then(|v| v.as_array()) {
        lt_setup.conditions = cond_arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
    }
    if let Some(stake) = settings.get("stakeAmount").and_then(|v| v.as_f64()) {
        lt_setup.amount = stake;
    }
    if let Some(target) = settings.get("targetProfitPercent").and_then(|v| v.as_f64()) {
        lt_setup.target_profit = target;
    }
    if let Some(dur) = settings.get("duration").and_then(|v| v.as_i64()) {
        lt_setup.duration = dur as i32;
    }
    if let Some(max_ord) = settings.get("maxOrders").and_then(|v| v.as_i64()) {
        lt_setup.max_orders = max_ord as i32;
    }
    if let Some(auto_tr) = settings.get("autoTrade").and_then(|v| v.as_bool()) {
        lt_setup.auto_trade = auto_tr;
    }
    if let Some(max_lc) = settings.get("maxLossCon").and_then(|v| v.as_u64()) {
        lt_setup.max_loss_con = max_lc as u32;
    }
    if let Some(mart) = settings.get("martingale") {
        if let Ok(mart_struct) = serde_json::from_value::<Martingale>(mart.clone()) {
            lt_setup.martingale = mart_struct;
        }
    }
    if let Ok(setup_json) = serde_json::to_string_pretty(&lt_setup) {
        let _ = fs::write(get_lt_setup_path(), setup_json);
        println!("💾 [LongTerm] Synced setup to {}", get_lt_setup_path());
    }
    
    match serde_json::to_string_pretty(&settings) {
        Ok(json_str) => {
            match fs::write(settings_file, json_str) {
                Ok(_) => {
                    println!("💾 [LongTerm] Settings saved to {}", settings_file);
                    Json(serde_json::json!({
                        "status": "success",
                        "message": "Settings saved successfully"
                    }))
                }
                Err(e) => {
                    Json(serde_json::json!({
                        "status": "error",
                        "message": format!("Failed to write settings: {}", e)
                    }))
                }
            }
        }
        Err(e) => {
            Json(serde_json::json!({
                "status": "error",
                "message": format!("Failed to serialize settings: {}", e)
            }))
        }
    }
}

#[derive(Deserialize)]
pub struct GenerateAnalysisRequest {
    pub asset: String,
    pub start_date: String,
    pub stop_date: String,
}

async fn handle_post_generate_analysis(
    Json(payload): Json<GenerateAnalysisRequest>,
) -> Json<serde_json::Value> {
    let start_dt = match NaiveDateTime::parse_from_str(&payload.start_date, "%Y-%m-%dT%H:%M") {
        Ok(dt) => dt,
        Err(_) => return Json(serde_json::json!({"status": "error", "message": "Invalid start_date format"}))
    };
    
    let stop_dt = match NaiveDateTime::parse_from_str(&payload.stop_date, "%Y-%m-%dT%H:%M") {
        Ok(dt) => dt,
        Err(_) => return Json(serde_json::json!({"status": "error", "message": "Invalid stop_date format"}))
    };
    
    let start_epoch = Local.from_local_datetime(&start_dt).single().map(|dt| dt.timestamp()).unwrap_or(start_dt.and_utc().timestamp());
    let stop_epoch = Local.from_local_datetime(&stop_dt).single().map(|dt| dt.timestamp()).unwrap_or(stop_dt.and_utc().timestamp());
    
    match deriv::fetch_historical_candles(&payload.asset, start_epoch, stop_epoch, 60).await {
        Ok(candles) => {
            let config_content = fs::read_to_string("setup/setup.json").unwrap_or_default();
            if let Ok(_config) = serde_json::from_str::<AppConfigPayload>(&config_content) {
                let raw_candles: Vec<full_analysis_ver2::RawCandleInput> = candles.into_iter().map(|c| full_analysis_ver2::RawCandleInput {
                    epoch: c.timestamp,
                    open: c.open,
                    high: c.high,
                    low: c.low,
                    close: c.close,
                }).collect();
                
                use chrono::Datelike;
                let now = Local::now();
                let month_folder = format!("{:02}-{:04}", now.month(), now.year() as i32 + 543);
                let day_folder = format!("{:02}-{:02}-{:04}", now.day(), now.month(), now.year() as i32 + 543);
                
                let symbol = match payload.asset.as_str() {
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
                    _ => &payload.asset,
                };

                let v2_config = match serde_json::from_str::<full_analysis_ver2::AppConfigPayload>(&config_content) {
                    Ok(c) => Some(c),
                    Err(e) => {
                        eprintln!("❌ [handle_post_generate_analysis] Failed to deserialize full_analysis_ver2::AppConfigPayload: {:?}", e);
                        None
                    }
                };
                let mut results = full_analysis_ver2::perform_analysis(&raw_candles, v2_config, Some(&payload.asset));
                
                for r in &mut results {
                    r.asset_code = symbol.to_string();
                }
                
                let dir_path = format!("analysisData/{}/{}/{}", month_folder, day_folder, symbol);
                let _ = fs::create_dir_all(&dir_path);
                
                let safe_start = payload.start_date.replace(":", "-");
                let safe_stop = payload.stop_date.replace(":", "-");
                let file_path = format!("{}/analysis_{}_to_{}.json", dir_path, safe_start, safe_stop);
                let setup_path = format!("{}/setup_{}_to_{}.json", dir_path, safe_start, safe_stop);
                
                if let Ok(json_str) = serde_json::to_string_pretty(&results) {
                    let _ = fs::write(&file_path, json_str);
                }
                
                // Save setup.json alongside analysis data
                let _ = fs::write(&setup_path, &config_content);
                
                Json(serde_json::json!({
                    "status": "success",
                    "asset": payload.asset,
                    "count": results.len(),
                    "start": payload.start_date,
                    "stop": payload.stop_date,
                    "file_saved": file_path,
                    "setup_saved": setup_path,
                    "data": results
                }))
            } else {
                Json(serde_json::json!({"status": "error", "message": "Failed to load setup.json"}))
            }
        },
        Err(e) => {
            Json(serde_json::json!({"status": "error", "message": e.to_string()}))
        }
    }
}

async fn get_url_ajax_post() -> Json<serde_json::Value> {
    let url = env::var("URLAJAXPOST_FullAnalysis").unwrap_or_else(|_| "".to_string());
    Json(serde_json::json!({ "url": url }))
}

async fn handle_get_deriv_config() -> Json<serde_json::Value> {
    let (app_id, api_token, account_id) = crate::get_active_credentials(None);
    let ws_app_id = env::var("DERIV_WS_APP_ID").unwrap_or_else(|_| "36544".to_string()).trim().to_string();
    let account_name = crate::get_active_account_name();
    Json(serde_json::json!({
        "app_id": app_id,
        "ws_app_id": ws_app_id,
        "api_token": api_token,
        "account_id": account_id,
        "account_name": account_name
    }))
}

async fn handle_get_deriv_otp() -> Json<serde_json::Value> {
    let (app_id, api_token, account_id) = crate::get_active_credentials(None);
    
    if api_token.is_empty() || account_id.is_empty() {
        return Json(serde_json::json!({
            "status": "error",
            "message": "DERIV_API_TOKEN or DERIV_ACCOUNT_ID is not set in .env"
        }));
    }

    let otp_url = format!("https://api.derivws.com/trading/v1/options/accounts/{}/otp", account_id);
    let client = reqwest::Client::new();
    
    match client.post(&otp_url)
        .header("Authorization", format!("Bearer {}", api_token.trim()))
        .header("Deriv-App-ID", app_id.trim())
        .header("Content-Type", "application/json")
        .send()
        .await
    {
        Ok(res) => {
            if !res.status().is_success() {
                let status = res.status();
                let body = res.text().await.unwrap_or_default();
                return Json(serde_json::json!({
                    "status": "error",
                    "message": format!("OTP request failed: {} | {}", status, body)
                }));
            }
            match res.json::<serde_json::Value>().await {
                Ok(otp_data) => {
                    if let Some(ws_url) = otp_data.get("data").and_then(|d| d.get("url")).and_then(|u| u.as_str()) {
                        Json(serde_json::json!({
                            "status": "success",
                            "ws_url": ws_url,
                            "account_id": account_id
                        }))
                    } else {
                        Json(serde_json::json!({
                            "status": "error",
                            "message": "Failed to parse OTP WebSocket URL from Deriv response"
                        }))
                    }
                }
                Err(e) => {
                    Json(serde_json::json!({
                        "status": "error",
                        "message": format!("Failed to parse OTP response: {}", e)
                    }))
                }
            }
        }
        Err(e) => {
            Json(serde_json::json!({
                "status": "error",
                "message": format!("OTP request error: {}", e)
            }))
        }
    }
}

/// Get current bot and indicator configuration setup
#[utoipa::path(
    get,
    path = "/api/setup",
    responses(
        (status = 200, description = "Current configuration JSON")
    ),
    tag = "Configuration"
)]
async fn get_setup() -> Json<serde_json::Value> {
    let path = Path::new("setup/setup.json");
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            return Json(json);
        }
    }
    Json(serde_json::json!({}))
}

/// Get default template bot and indicator configuration setup
#[utoipa::path(
    get,
    path = "/api/setup/default",
    responses(
        (status = 200, description = "Default configuration JSON")
    ),
    tag = "Configuration"
)]
async fn get_default_setup() -> Json<serde_json::Value> {
    let path = Path::new("setup/default/setup.json");
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            return Json(json);
        }
    }
    Json(serde_json::json!({}))
}

/// Get current settings (settings.json)
#[utoipa::path(
    get,
    path = "/api/settings",
    responses(
        (status = 200, description = "Current settings JSON configuration")
    ),
    tag = "Configuration"
)]
async fn get_settings() -> Json<serde_json::Value> {
    let candidate_paths = ["setup/settings.json", "settings.json", "setup/longterm_settings.json"];
    for p in &candidate_paths {
        let path = Path::new(p);
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                return Json(json);
            }
        }
    }
    Json(serde_json::json!({
        "status": "error",
        "message": "settings.json not found"
    }))
}

async fn save_settings(
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let _ = fs::create_dir_all("setup");
    let path = Path::new("setup/settings.json");
    match serde_json::to_string_pretty(&payload) {
        Ok(json_str) => match fs::write(path, json_str) {
            Ok(_) => Json(serde_json::json!({
                "success": true,
                "status": "success",
                "message": "Settings updated and saved to setup/settings.json successfully"
            })),
            Err(e) => Json(serde_json::json!({
                "success": false,
                "status": "error",
                "message": format!("Failed to write setup/settings.json: {}", e)
            })),
        },
        Err(e) => Json(serde_json::json!({
            "success": false,
            "status": "error",
            "message": format!("Failed to serialize settings: {}", e)
        })),
    }
}

#[derive(Deserialize)]
pub struct SellRequest {
    pub contract_id: i64,
}

async fn handle_post_sell(
    State(state): State<AppState>,
    Json(payload): Json<SellRequest>,
) -> Json<serde_json::Value> {
    println!("Received Sell Request for contract_id: {}", payload.contract_id);
    let _ = state.cmd_tx.send(serde_json::json!({
        "command": "sell",
        "contract_id": payload.contract_id
    }));
    Json(serde_json::json!({ "status": "success", "message": "ส่งคำสั่งขายไปยังบอทแล้ว" }))
}

// ══════════════════════════════════════════════════════════════
//  MANUAL TRADE API — เปิด order แบบ manual (CALL/PUT)
// ══════════════════════════════════════════════════════════════
#[derive(Deserialize)]
pub struct ManualTradeRequest {
    pub asset: String,
    pub contract_type: String, // "CALL" or "PUT"
    pub amount: f64,
    pub target_profit: Option<f64>,
    pub duration: Option<i64>,
    pub exit_strategy: Option<String>,
    pub entry_signal: Option<String>,
}


// ══════════════════════════════════════════════════════════════
//  LT MANUAL TRADE API
// ══════════════════════════════════════════════════════════════
async fn handle_post_lt_manual_trade(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::Json(payload): axum::Json<ManualTradeRequest>,
) -> axum::Json<serde_json::Value> {
    println!("🖐️ [LT Manual Trade] {} ${:.2} on {}", payload.contract_type, payload.amount, payload.asset);
    let _ = state.lt_cmd_tx.send(serde_json::json!({
        "command": "lt_manual_trade",
        "asset": payload.asset,
        "contract_type": payload.contract_type,
        "amount": payload.amount,
        "target_profit": payload.target_profit.unwrap_or(0.0),
        "duration": payload.duration.unwrap_or(3600),
        "exit_strategy": payload.exit_strategy.clone().unwrap_or_else(|| "targetProfit".to_string()),
        "entry_signal": payload.entry_signal.clone().unwrap_or_default()
    }));
    axum::Json(serde_json::json!({
        "status": "success",
        "message": format!("ส่งคำสั่ง {} ${:.2} บน {} สำหรับ LongTerm แล้ว", payload.contract_type, payload.amount, payload.asset)
    }))
}

async fn handle_post_lt_sell(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::Json(payload): axum::Json<SellRequest>,
) -> axum::Json<serde_json::Value> {
    println!("Received LT Sell Request for contract_id: {}", payload.contract_id);
    let _ = state.lt_cmd_tx.send(serde_json::json!({
        "command": "lt_sell",
        "contract_id": payload.contract_id
    }));
    axum::Json(serde_json::json!({ "status": "success", "message": "ส่งคำสั่งขาย LongTerm ไปยังบอทแล้ว" }))
}

#[derive(serde::Deserialize)]
pub struct LtTrackOrderSnapshot {
    pub timestamp: i64,
    #[serde(rename = "timestampDisplay")]
    pub timestamp_display: String,
    pub contract_id: i64,
    pub asset: String,
    pub symbol: String,
    pub contract_type: String,
    pub buy_price: f64,
    pub current_spot: f64,
    pub entry_spot: f64,
    pub profit: f64,
    pub min_profit: f64,
    pub max_profit: f64,
    pub payout: f64,
    pub purchase_time: i64,
    pub date_start: i64,
    pub date_expiry: i64,
    pub duration_seconds: i64,
    pub target_profit: f64,
    pub exit_strategy: String,
    pub entry_signal: String,
    pub trade_code: String,
}

async fn handle_post_lt_save_track_snapshot(
    axum::Json(payload): axum::Json<LtTrackOrderSnapshot>,
) -> axum::Json<serde_json::Value> {
    use chrono::Datelike;
    
    // สร้าง path สำหรับบันทึก track orders
    let purchase_dt = chrono::DateTime::from_timestamp(payload.purchase_time, 0)
        .map(|dt| dt.with_timezone(&chrono::FixedOffset::east_opt(7 * 3600).unwrap()))
        .unwrap_or_else(|| chrono::Local::now().with_timezone(&chrono::FixedOffset::east_opt(7 * 3600).unwrap()));
    
    let month = format!("{:02}-{}", purchase_dt.month(), purchase_dt.year() as i32 + 543);
    let day_folder = format!("{:02}-{:02}-{}", purchase_dt.day(), purchase_dt.month(), purchase_dt.year() as i32 + 543);
    
    // ใช้ asset code แทน symbol (เช่น 1HZ10V)
    let asset_folder = &payload.symbol;
    
    let dir_path = format!("tradeData/{}/{}/{}", month, day_folder, asset_folder);
    let _ = std::fs::create_dir_all(&dir_path);
    
    let track_file = format!("{}/track_orders.json", dir_path);
    
    // อ่านข้อมูลเดิม (ถ้ามี)
    let mut track_orders: Vec<serde_json::Value> = if let Ok(content) = std::fs::read_to_string(&track_file) {
        serde_json::from_str(&content).unwrap_or_else(|_| vec![])
    } else {
        vec![]
    };
    
    // เพิ่ม snapshot ใหม่
    track_orders.push(serde_json::json!({
        "timestamp": payload.timestamp,
        "timestampDisplay": payload.timestamp_display,
        "contractId": payload.contract_id,
        "asset": payload.asset,
        "symbol": payload.symbol,
        "contractType": payload.contract_type,
        "buyPrice": payload.buy_price,
        "currentSpot": payload.current_spot,
        "entrySpot": payload.entry_spot,
        "profit": payload.profit,
        "minProfit": payload.min_profit,
        "maxProfit": payload.max_profit,
        "payout": payload.payout,
        "purchaseTime": payload.purchase_time,
        "dateStart": payload.date_start,
        "dateExpiry": payload.date_expiry,
        "durationSeconds": payload.duration_seconds,
        "targetProfit": payload.target_profit,
        "exitStrategy": payload.exit_strategy,
        "entrySignal": payload.entry_signal,
        "tradeCode": payload.trade_code
    }));
    
    // บันทึกกลับไปยังไฟล์
    if let Ok(json_str) = serde_json::to_string_pretty(&track_orders) {
        let _ = std::fs::write(&track_file, json_str);
        println!("💾 [Track Order] Snapshot saved to: {} (total: {})", track_file, track_orders.len());
    }
    
    axum::Json(serde_json::json!({ "status": "success", "message": "Track order snapshot saved" }))
}

#[derive(serde::Deserialize)]
pub struct LtSetTargetRequest {
    pub contract_id: i64,
    pub target_profit: f64,
}

async fn handle_post_lt_set_target(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::Json(payload): axum::Json<LtSetTargetRequest>,
) -> axum::Json<serde_json::Value> {
    println!("Received LT Set Target for contract_id: {}, target: {}", payload.contract_id, payload.target_profit);
    let _ = state.lt_cmd_tx.send(serde_json::json!({
        "command": "lt_set_target",
        "contract_id": payload.contract_id,
        "target_profit": payload.target_profit
    }));
    axum::Json(serde_json::json!({ "status": "success" }))
}

#[derive(serde::Deserialize)]
pub struct LtTrackOrderSnapshotsBatch {
    pub contract_id: i64,
    pub track_data: Option<serde_json::Value>, // โครงสร้างใหม่: { contract_id, asset, symbol, ..., ListOfTrack: [] }
    #[serde(default)]
    pub snapshots: Vec<serde_json::Value>, // เก็บไว้เพื่อ backward compatibility
}

async fn handle_post_lt_save_track_snapshots_batch(
    axum::Json(payload): axum::Json<LtTrackOrderSnapshotsBatch>,
) -> axum::Json<serde_json::Value> {
    use chrono::Datelike;
    
    // ตรวจสอบว่ามี track_data หรือไม่
    let track_data = match payload.track_data {
        Some(data) => data,
        None => {
            return axum::Json(serde_json::json!({ 
                "status": "error", 
                "message": "No track_data provided" 
            }));
        }
    };
    
    // ดึงข้อมูลจาก track_data
    let purchase_time = track_data.get("purchase_time")
        .and_then(|v| v.as_i64())
        .unwrap_or_else(|| chrono::Local::now().timestamp());
    
    let symbol = track_data.get("symbol")
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN");
    
    let contract_id = track_data.get("contract_id")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    
    let list_of_track = track_data.get("ListOfTrack")
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0);
    
    let purchase_dt = chrono::DateTime::from_timestamp(purchase_time, 0)
        .map(|dt| dt.with_timezone(&chrono::FixedOffset::east_opt(7 * 3600).unwrap()))
        .unwrap_or_else(|| chrono::Local::now().with_timezone(&chrono::FixedOffset::east_opt(7 * 3600).unwrap()));
    
    let month = format!("{:02}-{}", purchase_dt.month(), purchase_dt.year() as i32 + 543);
    let day_folder = format!("{:02}-{:02}-{}", purchase_dt.day(), purchase_dt.month(), purchase_dt.year() as i32 + 543);
    
    let dir_path = format!("tradeData/{}/{}/{}", month, day_folder, symbol);
    let _ = std::fs::create_dir_all(&dir_path);
    
    let track_file = format!("{}/track_orders.json", dir_path);
    
    // อ่านข้อมูลเดิม (ถ้ามี) - รูปแบบเป็น array ของ contract
    let mut all_contracts: Vec<serde_json::Value> = if let Ok(content) = std::fs::read_to_string(&track_file) {
        serde_json::from_str(&content).unwrap_or_else(|_| vec![])
    } else {
        vec![]
    };
    
    // ตรวจสอบว่า contract_id นี้มีอยู่แล้วหรือไม่
    let existing_index = all_contracts.iter().position(|c| {
        c.get("contract_id").and_then(|v| v.as_i64()).unwrap_or(-1) == contract_id
    });
    
    if let Some(index) = existing_index {
        // อัพเดท contract ที่มีอยู่แล้ว (รวม ListOfTrack เข้าด้วยกัน)
        if let Some(existing_list) = all_contracts[index].get_mut("ListOfTrack").and_then(|v| v.as_array_mut()) {
            if let Some(new_list) = track_data.get("ListOfTrack").and_then(|v| v.as_array()) {
                for track_item in new_list {
                    existing_list.push(track_item.clone());
                }
            }
        }
        // อัพเดท max_profit, min_profit, final_profit
        if let Some(obj) = all_contracts[index].as_object_mut() {
            if let Some(max_profit) = track_data.get("max_profit") {
                obj.insert("max_profit".to_string(), max_profit.clone());
            }
            if let Some(min_profit) = track_data.get("min_profit") {
                obj.insert("min_profit".to_string(), min_profit.clone());
            }
            if let Some(final_profit) = track_data.get("final_profit") {
                obj.insert("final_profit".to_string(), final_profit.clone());
            }
        }
        println!("🔄 [Track Order] Updated existing contract {} (total ticks: {})", 
            contract_id, all_contracts[index].get("ListOfTrack").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0));
    } else {
        // เพิ่ม contract ใหม่
        all_contracts.push(track_data);
        println!("➕ [Track Order] Added new contract {} with {} ticks", contract_id, list_of_track);
    }
    
    // บันทึกกลับไปยังไฟล์ (บังคับลำดับ field: ListOfTrack ต้องมาท้ายสุด)
    // serde_json::Map เรียง key แบบ alphabetical → ต้อง reorder ก่อนบันทึก
    let ordered_contracts: Vec<serde_json::Value> = all_contracts.iter().map(|contract| {
        if let Some(obj) = contract.as_object() {
            let mut ordered = serde_json::Map::new();
            // ใส่ field ทั้งหมดยกเว้น ListOfTrack ก่อน
            for (key, value) in obj.iter() {
                if key != "ListOfTrack" {
                    ordered.insert(key.clone(), value.clone());
                }
            }
            // ใส่ ListOfTrack ท้ายสุด (หลัง trade_code)
            if let Some(list) = obj.get("ListOfTrack") {
                ordered.insert("ListOfTrack".to_string(), list.clone());
            }
            serde_json::Value::Object(ordered)
        } else {
            contract.clone()
        }
    }).collect();

    if let Ok(json_str) = serde_json::to_string_pretty(&ordered_contracts) {
        let _ = std::fs::write(&track_file, json_str);
        println!("💾 [Track Order] Saved to: {} (total contracts: {})", 
            track_file, all_contracts.len());
    }
    
    axum::Json(serde_json::json!({ 
        "status": "success", 
        "message": format!("Track data saved for contract {} with {} ticks", contract_id, list_of_track),
        "file": track_file
    }))
}

async fn handle_post_manual_trade(
    State(state): State<AppState>,
    Json(payload): Json<ManualTradeRequest>,
) -> Json<serde_json::Value> {
    println!("🖐️ [Manual Trade] {} {} ${:.2} on {}", payload.contract_type, payload.asset, payload.amount, payload.asset);
    let _ = state.cmd_tx.send(serde_json::json!({
        "command": "manual_trade",
        "asset": payload.asset,
        "contract_type": payload.contract_type,
        "amount": payload.amount
    }));
    Json(serde_json::json!({
        "status": "success",
        "message": format!("ส่งคำสั่ง {} ${:.2} บน {} แล้ว", payload.contract_type, payload.amount, payload.asset)
    }))
}

// ══════════════════════════════════════════════════════════════
//  SAVE MANUAL HISTORY API
// ══════════════════════════════════════════════════════════════
async fn handle_post_save_manual_history(
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    use chrono::Datelike;
    let today = chrono::Local::now();
    let thai_year = today.year() + 543;
    let month_folder = format!("{:02}-{}", today.month(), thai_year);
    let day_folder = format!("{:02}-{:02}-{}", today.day(), today.month(), thai_year);
    
    // Save under the specific asset folder so getTradeHistory can find it, fallback to "Manual"
    let asset = payload.get("asset").and_then(|v| v.as_str())
                       .or_else(|| payload.get("underlying").and_then(|v| v.as_str()))
                       .unwrap_or("Manual").to_string();
    
    let dir_path = format!("tradeData/{}/{}/{}", month_folder, day_folder, asset);
    if let Err(e) = std::fs::create_dir_all(&dir_path) {
        return Json(serde_json::json!({"status": "error", "message": format!("Failed to create directory: {}", e)}));
    }
    
    let log_file = format!("{}/trades.json", dir_path);
    let mut trades = Vec::new();
    if let Ok(contents) = std::fs::read_to_string(&log_file) {
        if let Ok(existing) = serde_json::from_str::<Vec<serde_json::Value>>(&contents) {
            trades = existing;
        }
    }
    
    let mut payload = payload;
    if let Some(obj) = payload.as_object_mut() {
        if !obj.contains_key("serverCode") {
            obj.insert("serverCode".to_string(), serde_json::json!(trade_head::get_server_code()));
        }
        if !obj.contains_key("tradeRoundNo") {
            obj.insert("tradeRoundNo".to_string(), serde_json::json!(get_or_create_trade_control().total_trade));
        }
    }

    trades.push(payload);
    
    if let Err(e) = std::fs::write(&log_file, serde_json::to_string_pretty(&trades).unwrap_or_default()) {
        return Json(serde_json::json!({"status": "error", "message": format!("Failed to save trade: {}", e)}));
    }
    
    Json(serde_json::json!({"status": "success"}))
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

async fn save_setup(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let path = Path::new("setup/setup.json");
    if fs::write(path, serde_json::to_string_pretty(&payload).unwrap_or_default()).is_ok() {
        println!("📝 [save_setup] Saved setup.json. Payload theme: {:?}", payload.get("meta").and_then(|m| m.get("theme")));

        let _ = state.cmd_tx.send(serde_json::json!({
            "command": "reload_history"
        }));

        // Recalculate indicators using the new settings for all cached assets
        match serde_json::from_value::<AppConfigPayload>(payload.clone()) {
            Ok(config) => {
                println!("✅ [save_setup] Deserialized AppConfigPayload successfully.");
                let mut candle_guard = state.candle_data.lock().await;
                println!("📦 [save_setup] Found {} assets in candle_data cache.", candle_guard.len());
                for (asset, cached_val) in candle_guard.iter_mut() {
                    if let Some(data_arr) = cached_val.get("data").and_then(|d| d.as_array()) {
                        println!("🔄 [save_setup] Recalculating {} candles for asset: {}", data_arr.len(), asset);
                        let raw_candles: Vec<full_analysis_ver2::RawCandleInput> = data_arr
                            .iter()
                            .map(|c| full_analysis_ver2::RawCandleInput {
                                epoch: c.get("candletime").and_then(|v| v.as_i64()).unwrap_or(0),
                                open: c.get("open").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                high: c.get("high").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                low: c.get("low").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                close: c.get("close").and_then(|v| v.as_f64()).unwrap_or(0.0),
                            })
                            .collect();

                        let config_str = serde_json::to_string(&config).unwrap_or_default();
                        let v2_config = match serde_json::from_str::<full_analysis_ver2::AppConfigPayload>(&config_str) {
                            Ok(c) => Some(c),
                            Err(e) => {
                                eprintln!("❌ [save_setup] Failed to deserialize full_analysis_ver2::AppConfigPayload: {:?}", e);
                                None
                            }
                        };
                        let analysis_result = full_analysis_ver2::perform_analysis(&raw_candles, v2_config, Some(asset));

                        let new_payload = serde_json::json!({
                            "type": "candles_history",
                            "asset": asset.clone(),
                            "data": analysis_result
                        });

                        // Update cached value
                        *cached_val = new_payload.clone();

                        // Broadcast updated candles history to all connected websocket clients
                        match state.tx.send(new_payload) {
                            Ok(subs) => println!("📢 [save_setup] Broadcasted candles_history for {} to {} subscribers.", asset, subs),
                            Err(e) => println!("⚠️ [save_setup] Broadcast failed for {}: {:?}", asset, e),
                        }
                    } else {
                        println!("⚠️ [save_setup] Asset {} in cache does not have a valid data array.", asset);
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ [save_setup] Failed to deserialize AppConfigPayload from payload: {:?}", e);
            }
        }

        Json(serde_json::json!({"status": "success"}))
    } else {
        Json(serde_json::json!({"status": "error", "message": "Failed to save setup"}))
    }
}

/// Get threshold and filter configurations for trade execution
#[utoipa::path(
    get,
    path = "/api/thereshold",
    responses(
        (status = 200, description = "Threshold and noise filter configuration JSON")
    ),
    tag = "Configuration"
)]
async fn get_thereshold() -> Json<serde_json::Value> {
    let path = Path::new("setup/thereshold.json");
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            return Json(json);
        }
    }
    Json(serde_json::json!({ "thresholds": [], "noiseConfig": {} }))
}

async fn save_thereshold(Json(payload): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let path = Path::new("setup/thereshold.json");
    let mut root = serde_json::json!({
        "thresholds": [],
        "noiseConfig": {}
    });
    
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if json.is_array() {
                root["thresholds"] = json;
            } else {
                root = json;
            }
        }
    }

    if let Some(obj) = payload.as_object() {
        if obj.contains_key("isCheckNoise") {
            let mut noise_config = serde_json::Map::new();
            if let Some(v) = obj.get("isCheckNoise") { noise_config.insert("isCheckNoise".to_string(), v.clone()); }
            if let Some(v) = obj.get("noiseTypeCheck") { noise_config.insert("noiseTypeCheck".to_string(), v.clone()); }
            if let Some(v) = obj.get("flatCondition") { noise_config.insert("flatCondition".to_string(), v.clone()); }
            if let Some(v) = obj.get("flatConditionDesc") { noise_config.insert("flatConditionDesc".to_string(), v.clone()); }
            // ═══ New Noise Filters (Loss Streak Fix) ═══
            if let Some(v) = obj.get("checkAdx") { noise_config.insert("checkAdx".to_string(), v.clone()); }
            if let Some(v) = obj.get("adxThreshold") { noise_config.insert("adxThreshold".to_string(), v.clone()); }
            if let Some(v) = obj.get("checkChoppy") { noise_config.insert("checkChoppy".to_string(), v.clone()); }
            if let Some(v) = obj.get("choppyThreshold") { noise_config.insert("choppyThreshold".to_string(), v.clone()); }
            if let Some(v) = obj.get("checkRange") { noise_config.insert("checkRange".to_string(), v.clone()); }
            if let Some(v) = obj.get("checkBbSqueeze") { noise_config.insert("checkBbSqueeze".to_string(), v.clone()); }
            if let Some(v) = obj.get("bbBandwidthThreshold") { noise_config.insert("bbBandwidthThreshold".to_string(), v.clone()); }
            root["noiseConfig"] = serde_json::Value::Object(noise_config);
        }

        if let Some(asset) = obj.get("asset").and_then(|a| a.as_str()) {
            let mut new_t = serde_json::Map::new();
            if let Some(v) = obj.get("asset") { new_t.insert("asset".to_string(), v.clone()); }
            if let Some(v) = obj.get("assetCode") { new_t.insert("assetCode".to_string(), v.clone()); }
            if let Some(v) = obj.get("flatTheresholdValue") { new_t.insert("flatTheresholdValue".to_string(), v.clone()); }
            if let Some(v) = obj.get("MACDGapValue") { new_t.insert("MACDGapValue".to_string(), v.clone()); }
            if let Some(v) = obj.get("altCandleAtrMultiplier") { new_t.insert("altCandleAtrMultiplier".to_string(), v.clone()); }
            if let Some(v) = obj.get("altCandleSpikeMultiplier") { new_t.insert("altCandleSpikeMultiplier".to_string(), v.clone()); }
            
            let mut found = false;
            if let Some(arr) = root["thresholds"].as_array_mut() {
                for t in arr.iter_mut() {
                    if let Some(a) = t.get("asset").and_then(|v| v.as_str()) {
                        if a == asset {
                            if let Some(existing_obj) = t.as_object() {
                                for (k, v) in existing_obj {
                                    if !new_t.contains_key(k) && k != "isCheckNoise" && k != "noiseTypeCheck" && k != "flatCondition" && k != "flatConditionDesc" {
                                        new_t.insert(k.clone(), v.clone());
                                    }
                                }
                            }
                            *t = serde_json::Value::Object(new_t.clone());
                            found = true;
                            break;
                        }
                    }
                }
                if !found {
                    arr.push(serde_json::Value::Object(new_t));
                }
            }
        }
    }

    if fs::write(path, serde_json::to_string_pretty(&root).unwrap_or_default()).is_ok() {
        Json(serde_json::json!({"status": "success"}))
    } else {
        Json(serde_json::json!({"status": "error", "message": "Failed to save thereshold.json"}))
    }
}

#[derive(Debug, Deserialize, Serialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct HistoryQuery {
    /// Date in YYYY-MM-DD format (e.g. 2026-09-08)
    #[param(example = "2026-09-08")]
    pub date: String,
}

/// Retrieve overall trade history for a specific date
#[utoipa::path(
    get,
    path = "/api/history",
    params(HistoryQuery),
    responses(
        (status = 200, description = "List of trade history records")
    ),
    tag = "Trading & History"
)]
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
//  GET TRADE HISTORY DATA API — ดึงข้อมูล trade เฉพาะ asset ที่ระบุ
// ══════════════════════════════════════════════════════════════
#[derive(Debug, Deserialize, Serialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct StrategyComparisonQuery {
    /// Date in YYYY-MM-DD format (e.g. 2026-09-08)
    #[param(example = "2026-09-08")]
    #[serde(rename = "dayTrade")]
    pub day_trade: String,
}

#[derive(Debug, Deserialize, Serialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct TradeHistoryDataQuery {
    /// Optional trade action filter
    pub action: Option<String>,
    /// Asset code symbol (e.g. R_10)
    #[param(example = "R_10")]
    pub asset: String,
    /// Date in YYYY-MM-DD format (e.g. 2026-09-08)
    #[param(example = "2026-09-08")]
    #[serde(rename = "dayTrade")]
    pub day_trade: String,
}

/// Retrieve trades executed for a specific asset and day
#[utoipa::path(
    get,
    path = "/api/getTradeHistory",
    params(TradeHistoryDataQuery),
    responses(
        (status = 200, description = "Detailed trade execution records")
    ),
    tag = "Trading & History"
)]
async fn get_tradehistory_data(Query(params): Query<TradeHistoryDataQuery>) -> Json<serde_json::Value> {
    // แยกวันที่จาก string เช่น "2026-04-24"
    let parts: Vec<&str> = params.day_trade.split('-').collect();
    if parts.len() != 3 {
        return Json(serde_json::json!([]));
    }
    
    let year: i32 = parts[0].parse().unwrap_or(0);
    let month = parts[1];
    let day = parts[2];
    
    // แปลง ค.ศ. เป็น พ.ศ. (บวก 543)
    let bei_year = year + 543;
    let month_folder = format!("{}-{}", month, bei_year);
    let day_folder = format!("{}-{}-{}", day, month, bei_year);
    
    // สร้าง path ชี้ไปยัง folder tradeData/mm-thaiyear/dd-mm-thaiyear/asset/trades.json
    let path_str = format!("tradeData/{}/{}/{}/trades.json", month_folder, day_folder, params.asset);
    let path = Path::new(&path_str);
    
    // ตรวจสอบว่ามีไฟล์หรือไม่
    if path.exists() {
        if let Ok(contents) = fs::read_to_string(path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
                return Json(json);
            }
        }
    }
    
    // ถ้าไม่มีให้ return array เปล่า
    Json(serde_json::json!([]))
}

/// Retrieve multi-strategy performance comparison data for a given day
#[utoipa::path(
    get,
    path = "/api/getStrategyComparison",
    params(StrategyComparisonQuery),
    responses(
        (status = 200, description = "Strategy comparison results JSON")
    ),
    tag = "Trading & History"
)]
async fn get_strategy_comparison(Query(params): Query<StrategyComparisonQuery>) -> Json<serde_json::Value> {
    let parts: Vec<&str> = params.day_trade.split('-').collect();
    if parts.len() != 3 {
        return Json(serde_json::json!([]));
    }
    
    let year: i32 = parts[0].parse().unwrap_or(0);
    let month = parts[1];
    let day = parts[2];
    
    // แปลง ค.ศ. เป็น พ.ศ. (บวก 543)
    let bei_year = year + 543;
    let month_folder = format!("{}-{}", month, bei_year);
    let day_folder = format!("{}-{}-{}", day, month, bei_year);
    
    // สร้าง path ชี้ไปยัง folder tradeData/mm-thaiyear/dd-mm-thaiyear/strategy_comparison.json
    let path_str = format!("tradeData/{}/{}/strategy_comparison.json", month_folder, day_folder);
    let path = Path::new(&path_str);
    
    // ตรวจสอบว่ามีไฟล์หรือไม่
    if path.exists() {
        if let Ok(contents) = fs::read_to_string(path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
                return Json(json);
            }
        }
    }
    
    // ถ้าไม่มีให้ return array เปล่า
    Json(serde_json::json!([]))
}

// ══════════════════════════════════════════════════════════════
//  CASE CODES 27 PATTERNS API (isSelectedToAction Manager)
//  GET /api/case_codes -> อ่านไฟล์ JSON
//  POST /api/save_case_codes -> บันทึกไฟล์ JSON ลง Disk
// ══════════════════════════════════════════════════════════════
/// Get active PK Trend case code configurations and trigger mappings
#[utoipa::path(
    get,
    path = "/api/case_codes",
    responses(
        (status = 200, description = "Case codes mapping JSON")
    ),
    tag = "Configuration"
)]
async fn handle_get_case_codes() -> Json<serde_json::Value> {
    let candidate_paths = [
        "public/case_codes.json",
        "case_codes.json",
        "../dynamicChart/case_codes.json",
    ];
    for path in &candidate_paths {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                return Json(json);
            }
        }
    }
    Json(serde_json::json!({
        "status": "error",
        "message": "case_codes.json not found"
    }))
}

async fn handle_post_save_case_codes(
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    println!("💾 [Rust API] Saving case_codes.json...");

    let target_paths = [
        "public/case_codes.json",
        "case_codes.json",
        "../dynamicChart/case_codes.json",
        "../dynamicChart/indicator/case_codes.json",
        "../dynamicChart/php/case_codes.json",
    ];

    let json_str = match serde_json::to_string_pretty(&payload) {
        Ok(s) => s,
        Err(e) => {
            return Json(serde_json::json!({
                "success": false,
                "error": format!("Failed to serialize JSON: {}", e)
            }));
        }
    };

    let mut saved_count = 0;
    let mut saved_paths = Vec::new();

    for path in &target_paths {
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                let _ = fs::create_dir_all(parent);
            }
        }
        if let Ok(_) = fs::write(path, &json_str) {
            saved_count += 1;
            saved_paths.push(path.to_string());
            println!("  ✅ [Rust] Saved to {}", path);
        }
    }

    Json(serde_json::json!({
        "success": true,
        "message": format!("Rust backend บันทึกไฟล์ case_codes.json สำเร็จ ({} ตำแหน่ง)", saved_count),
        "saved_count": saved_count,
        "saved_paths": saved_paths
    }))
}

async fn handle_get_strategy_case_codes() -> Json<serde_json::Value> {
    let candidate_paths = [
        "setup/strategy_case_codes.json",
        "indicators_Multiplex_Ver1/setup/strategy_case_codes.json",
        "../setup/strategy_case_codes.json",
    ];
    for path in &candidate_paths {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if json.is_array() {
                    return Json(serde_json::json!({
                        "filters": {
                            "isuseAdx": "y",
                            "adxValue": 25.0,
                            "isuseChoppy": "y",
                            "choppyValue": 38.2,
                            "isuseBbSqueeze": "y",
                            "bbBandwidthThreshold": 0.5
                        },
                        "cases": json
                    }));
                }
                return Json(json);
            }
        }
    }
    Json(serde_json::json!({
        "status": "error",
        "message": "strategy_case_codes.json not found"
    }))
}

async fn handle_post_save_strategy_case_codes(
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    println!("💾 [Rust API] Saving setup/strategy_case_codes.json...");

    let target_paths = [
        "setup/strategy_case_codes.json",
        "indicators_Multiplex_Ver1/setup/strategy_case_codes.json",
    ];

    let json_str = match serde_json::to_string_pretty(&payload) {
        Ok(s) => s,
        Err(e) => {
            return Json(serde_json::json!({
                "success": false,
                "error": format!("Failed to serialize JSON: {}", e)
            }));
        }
    };

    let mut saved_count = 0;
    let mut saved_paths = Vec::new();

    for path in &target_paths {
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                let _ = fs::create_dir_all(parent);
            }
        }
        if let Ok(_) = fs::write(path, &json_str) {
            saved_count += 1;
            saved_paths.push(path.to_string());
            println!("  ✅ [Rust] Saved to {}", path);
        }
    }

    Json(serde_json::json!({
        "success": true,
        "message": format!("บันทึกไฟล์ setup/strategy_case_codes.json สำเร็จ ({} ตำแหน่ง)", saved_count),
        "saved_count": saved_count,
        "saved_paths": saved_paths
    }))
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

pub fn get_vps_name() -> String {
    if let Ok(val) = env::var("VPS_NAME").or_else(|_| env::var("vps_name")) {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    if let Ok(content) = std::fs::read_to_string("setup/vps_name.txt") {
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    if let Ok(host) = env::var("HOSTNAME").or_else(|_| env::var("COMPUTERNAME")) {
        let trimmed = host.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    let sc = trade_head::get_server_code();
    format!("VPS-{}", sc)
}

// ══════════════════════════════════════════════════════════════
//  FINGERPRINT VERIFICATION API
// ══════════════════════════════════════════════════════════════
async fn handle_verify_fingerprint(
    Json(payload): Json<FingerprintPayload>,
) -> Json<serde_json::Value> {
    let visitor_id = payload.fingerprint;
    println!("═══════════════════════════════════════════════════");
    println!("🔍 [DEBUG-FP] ตรวจสอบ Fingerprint: \"{}\"", visitor_id);
    println!("🔍 [DEBUG-FP] visitor_id len={}, bytes={:?}", visitor_id.len(), visitor_id.as_bytes());

    // ─── ขั้นตอน 0: อ่าน USE_FINGERPRINT จาก env::var (ค่าที่โหลดตอน server start) ───
    let use_fp_from_env = env::var("USE_FINGERPRINT").unwrap_or_else(|_| "(NOT SET)".to_string());
    println!("🔍 [DEBUG-FP] env::var(\"USE_FINGERPRINT\") = \"{}\"", use_fp_from_env);

    // ─── ขั้นตอน 0b: อ่าน USE_FINGERPRINT จากไฟล์ .env โดยตรง (hot-reload) ───
    let use_fingerprint_env = if let Ok(env_content) = fs::read_to_string(".env") {
        let mut val = String::new();
        for line in env_content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("USE_FINGERPRINT") {
                if let Some(eq_pos) = trimmed.find('=') {
                    val = trimmed[eq_pos + 1..].trim().to_lowercase();
                    break;
                }
            }
        }
        println!("🔍 [DEBUG-FP] อ่านจากไฟล์ .env โดยตรง: USE_FINGERPRINT = \"{}\"", val);
        if val.is_empty() { use_fp_from_env.to_lowercase() } else { val }
    } else {
        println!("🔍 [DEBUG-FP] อ่านไฟล์ .env ไม่ได้ ใช้ค่าจาก env::var");
        use_fp_from_env.to_lowercase()
    };

    let is_enforced = use_fingerprint_env == "yes" || use_fingerprint_env == "true" || use_fingerprint_env == "1";
    println!("🔍 [DEBUG-FP] is_enforced = {} (value=\"{}\")", is_enforced, use_fingerprint_env);

    // 1. บันทึกลง access_log.json
    let log_path = Path::new("access_log.json");
    let mut logs: Vec<serde_json::Value> = vec![];
    if log_path.exists() {
        if let Ok(content) = fs::read_to_string(log_path) {
            if let Ok(parsed) = serde_json::from_str(&content) {
                logs = parsed;
            }
        }
    }
    
    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    logs.push(serde_json::json!({
        "fingerprint": visitor_id.clone(),
        "device_info": payload.device_info.unwrap_or_else(|| "Unknown Device".to_string()),
        "device_name": payload.device_name.unwrap_or_else(|| "Unknown".to_string()),
        "timestamp": now_str,
        "enforced": is_enforced
    }));
    let _ = fs::write(log_path, serde_json::to_string_pretty(&logs).unwrap_or_default());

    // ถ้าปิดระบบไว้ ให้ข้ามการตรวจ Whitelist และอนุญาตทันที
    if !is_enforced {
        println!("⚠️ [DEBUG-FP] ระบบ Fingerprint ปิดอยู่ (USE_FINGERPRINT={}), อนุญาตให้เข้าถึงได้", use_fingerprint_env);
        println!("═══════════════════════════════════════════════════");
        return Json(serde_json::json!({
            "status": "success",
            "allowed": true
        }));
    }

    // 2. ตรวจสอบกับ allowed_fingerprints.json
    let allowed_path = Path::new("allowed_fingerprints.json");
    let mut is_allowed = false;
    
    println!("🔍 [DEBUG-FP] allowed_fingerprints.json exists = {}", allowed_path.exists());
    
    if allowed_path.exists() {
        match fs::read_to_string(allowed_path) {
            Ok(content) => {
                // แสดง raw bytes แรก 200 ตัว เพื่อเช็ค BOM หรือตัวอักษรแปลก
                let preview_bytes: Vec<u8> = content.bytes().take(200).collect();
                println!("🔍 [DEBUG-FP] raw file first 200 bytes: {:?}", preview_bytes);
                println!("🔍 [DEBUG-FP] file length = {} bytes", content.len());
                
                // ลอง strip BOM ถ้ามี
                let clean_content = if content.starts_with('\u{FEFF}') {
                    println!("⚠️ [DEBUG-FP] พบ BOM (Byte Order Mark)! กำลัง strip ออก...");
                    &content[3..] // UTF-8 BOM = 3 bytes
                } else {
                    &content
                };

                // ลอง parse แบบ HashMap (Object)
                match serde_json::from_str::<std::collections::HashMap<String, serde_json::Value>>(clean_content) {
                    Ok(parsed_map) => {
                        println!("✅ [DEBUG-FP] parse เป็น HashMap สำเร็จ! จำนวน keys = {}", parsed_map.len());
                        for (i, key) in parsed_map.keys().enumerate() {
                            let matches = key == &visitor_id;
                            println!("   [DEBUG-FP] key[{}] = \"{}\" (len={}) | match={}", i, key, key.len(), matches);
                            if !matches && key.len() == visitor_id.len() {
                                // เปรียบเทียบ byte-by-byte หา character ที่ต่างกัน
                                for (pos, (a, b)) in key.bytes().zip(visitor_id.bytes()).enumerate() {
                                    if a != b {
                                        println!("   [DEBUG-FP] ❗ byte diff at pos {}: key=0x{:02x}('{}') vs visitor=0x{:02x}('{}')", 
                                            pos, a, a as char, b, b as char);
                                    }
                                }
                            }
                        }
                        is_allowed = parsed_map.contains_key(&visitor_id);
                        println!("🔍 [DEBUG-FP] contains_key result = {}", is_allowed);
                    },
                    Err(e) => {
                        println!("❌ [DEBUG-FP] parse เป็น HashMap ล้มเหลว: {:?}", e);
                        // ลอง parse แบบ Array
                        match serde_json::from_str::<Vec<String>>(clean_content) {
                            Ok(parsed_array) => {
                                println!("✅ [DEBUG-FP] parse เป็น Array สำเร็จ! จำนวน = {}", parsed_array.len());
                                for (i, fp) in parsed_array.iter().enumerate() {
                                    println!("   [DEBUG-FP] array[{}] = \"{}\" | match={}", i, fp, fp == &visitor_id);
                                }
                                is_allowed = parsed_array.contains(&visitor_id);
                            },
                            Err(e2) => {
                                println!("❌ [DEBUG-FP] parse เป็น Array ก็ล้มเหลว: {:?}", e2);
                            }
                        }
                    }
                }
            },
            Err(e) => {
                println!("❌ [DEBUG-FP] อ่านไฟล์ allowed_fingerprints.json ไม่ได้: {:?}", e);
            }
        }
    } else {
        println!("❌ [DEBUG-FP] ไฟล์ allowed_fingerprints.json ไม่มีอยู่!");
    }
    
    if !is_allowed {
        println!("❌ [DEBUG-FP] ผลลัพธ์สุดท้าย: ปฏิเสธการเข้าถึง ({})", visitor_id);
    } else {
        println!("✅ [DEBUG-FP] ผลลัพธ์สุดท้าย: อนุญาตการเข้าถึง ({})", visitor_id);
    }
    println!("═══════════════════════════════════════════════════");

    Json(serde_json::json!({
        "status": "success",
        "allowed": is_allowed
    }))
}


async fn background_scheduler_loop(state: AppState) {
    println!("🔄 Background Scheduler is now active and monitoring setup.json...");
    let mut is_auto_trading = false;
    let mut last_processed_start = String::new();
    let mut session_start_balance = 0.0;
    // (Startup and Daily Resets are now managed via tradeControl.json)
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let (app_id, api_token, account_id) = crate::get_active_credentials(None);
        if api_token.is_empty() || api_token == "YOUR_API_TOKEN_HERE" { continue; }

        let config: AppConfigPayload = match fs::read_to_string("setup/setup.json") {
            Ok(content) => match serde_json::from_str(&content) { Ok(c) => c, Err(_) => continue },
            Err(_) => continue,
        };

        let tc = get_or_create_trade_control();

        if !tc.use_schedule || config.assets.is_empty() {
            if is_auto_trading {
                println!("🛑 Scheduler: Schedule is disabled in setup.json...");
                is_auto_trading = false;
            }
            continue;
        }

        let start_dt = match NaiveDateTime::parse_from_str(&tc.start_trade_time, "%Y-%m-%d %H:%M:%S") {
            Ok(dt) => match Local.from_local_datetime(&dt).single() { Some(t) => t, None => continue },
            Err(_) => continue,
        };

        let stop_dt = match NaiveDateTime::parse_from_str(&tc.stop_trade_time, "%Y-%m-%d %H:%M:%S") {
            Ok(dt) => match Local.from_local_datetime(&dt).single() { Some(t) => t, None => Local::now() + chrono::Duration::days(365) },
            Err(_) => Local::now() + chrono::Duration::days(365),
        };

        let now = Local::now();

        if now >= start_dt && now < stop_dt {
            if !is_auto_trading && last_processed_start != tc.start_trade_time {
                let mut bot_guard = state.active_bot.lock().await;
                
                // If a bot is already running (started by manual Go Trade), just mark as auto_trading and skip
                let already_running = bot_guard.as_ref().map_or(false, |h| !h.is_finished());
                if already_running {
                    println!("⚡ Scheduler: Bot already running (started manually), skipping auto-start.");
                    is_auto_trading = true;
                    last_processed_start = tc.start_trade_time.clone();
                    drop(bot_guard);
                    continue;
                }

                println!("⚡ Scheduler: Target time reached! Automatically starting trading bots.");
                is_auto_trading = true;
                last_processed_start = tc.start_trade_time.clone();

                // Stop any old stray bots running manually
                if let Some(handle) = bot_guard.take() {
                    handle.abort();
                }
                {
                    let mut ot_guard = state.overtime_signals.lock().await;
                    ot_guard.clear();
                }

                let mut spawned_assets = Vec::new();
                for asset in config.assets.iter() {
                    let ot_signal = Arc::new(AtomicBool::new(false));
                    {
                        let mut ot_guard = state.overtime_signals.lock().await;
                        ot_guard.insert(asset.clone(), ot_signal);
                    }
                    spawned_assets.push(asset.clone());
                }

                let tx = state.tx.clone();
                let app_id_clone = app_id.clone();
                let api_token_clone = api_token.clone();
                let account_id_clone = account_id.clone();
                let config_clone = config.clone();
                let tc = update_trade_control("กำลังเทรด", true, None);
                trade_head::on_bot_start(tc.total_trade, &tc.actual_start_time, &spawned_assets, &config_clone.trade.suggest_strategy);

                let assets_clone = spawned_assets.clone();
                let ot_signals = state.overtime_signals.clone();
                let cmd_rx = state.cmd_tx.subscribe();

                let handle = tokio::spawn(async move {
                    if let Err(e) = deriv::start_deriv_bot_multiplexed(app_id_clone, api_token_clone, account_id_clone, config_clone, assets_clone, tx.clone(), cmd_rx, ot_signals).await {
                        eprintln!("Multiplexed bot crashed: {}", e);
                    }

                    let tc = update_trade_control("ปิดเทรดอยู่", false, Some("เทรดสำเร็จ"));
                    trade_head::on_bot_stop(tc.total_trade, &tc.actual_stop_time);
                    let _ = tx.send(serde_json::json!({
                        "type": "bot_log",
                        "asset": "system",
                        "data": { "message": format!("🏁 [System] เทรดรอบที่ {} จบการทำงาน สถานะ: {}", tc.total_trade, tc.trade_status) }
                    }));
                });

                *bot_guard = Some(handle);
                
                let assets_str = config.assets.join(", ");
                let stop_display = tc.stop_trade_time.clone();

                // ดึงยอด balance ล่าสุดก่อนเริ่มเทรด
                let mut current_bal = 0.0;
                if let Ok(bal) = deriv::get_deriv_balance(&api_token).await {
                    current_bal = bal;
                    let mut bal_guard = state.last_balance.lock().await;
                    *bal_guard = bal;
                }
                session_start_balance = current_bal;

                let vps_name = get_vps_name();
                send_telegram_message(&format!("🟢 [{}] เริ่มเทรดอัตโนมัติ (Background Scheduler)!\n📊 Assets: {}\n⏰ เวลาตั้ง: {}\n🔴 สิ้นสุด: {}\n📋 Condition: {}\n💰 Balance เริ่มต้น: ${:.2}", vps_name, assets_str, start_dt.format("%d/%m/%Y %H:%M:%S"), stop_display, config.trade.condition_stop_trade, current_bal)).await;

                // ส่ง bot_log ไปยังหน้าเว็บ
                let start_log = format!("🟢 [{}] เริ่มเทรดอัตโนมัติ! Assets: {} | สิ้นสุด: {} | Condition: {} | Balance เริ่มต้น: ${:.2}",
                    vps_name, assets_str, stop_display, config.trade.condition_stop_trade, current_bal);
                let _ = state.tx.send(serde_json::json!({
                    "type": "bot_log",
                    "asset": "system",
                    "data": { "message": start_log }
                }));
            }
        }

        // 2. เช็คเวลา Stop Date
        if now >= stop_dt && is_auto_trading && config.trade.condition_stop_trade == "stopDate" {
            println!("🛑 Scheduler: Target Stop time reached!");
            is_auto_trading = false;
            let current_tc = get_or_create_trade_control();
            let round_no = current_tc.total_trade;
            let vps_name = get_vps_name();

            let is_martingale = config.trade.martingale.martingale_type == "martingale";

            if is_martingale {
                // ══ Martingale Overtime: ส่ง signal แทน abort ══
                println!("⏰ Scheduler: Martingale mode → sending overtime signal to all bots");
                {
                    let ot_guard = state.overtime_signals.lock().await;
                    for (asset_name, signal) in ot_guard.iter() {
                        signal.store(true, std::sync::atomic::Ordering::Relaxed);
                        println!("⏰ Sent overtime signal to [{}]", asset_name);
                    }
                }

                let (max_loss, loss_assets) = trade_head::get_round_max_loss(round_no);
                send_telegram_message(&format!("⏰ [{}] OVERTIME MODE (Background Scheduler)!\n⏰ ถึงเวลา Stop: {}\n📊 Martingale → รอให้ asset ที่ loss Win ก่อนหยุด\n📉 MaxLossCon: {} (Asset: {})", vps_name, stop_dt.format("%d/%m/%Y %H:%M:%S"), max_loss, loss_assets)).await;

                // Poll รอจนกว่าบอทจบเอง
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    let bot_guard = state.active_bot.lock().await;
                    let is_running = bot_guard.as_ref().map_or(false, |h| !h.is_finished());
                    drop(bot_guard);
                    if !is_running {
                        println!("✅ Scheduler: Multiplexed bot จบแล้ว (overtime completed)");
                        break;
                    }
                    println!("⏳ Scheduler Overtime: ยังรอ Multiplexed bot");
                }
                // Cleanup
                let mut bot_guard = state.active_bot.lock().await;
                *bot_guard = None;

                // ดึงยอด balance ใหม่เมื่อจบ overtime เพื่อสรุป profit
                let mut final_bal = 0.0;
                if let Ok(bal) = deriv::get_deriv_balance(&api_token).await {
                    final_bal = bal;
                    let mut bal_guard = state.last_balance.lock().await;
                    *bal_guard = bal;
                }
                let profit = final_bal - session_start_balance;
                let sign = if profit >= 0.0 { "+" } else { "" };

                let (max_loss_end, loss_assets_end) = trade_head::get_round_max_loss(round_no);
                send_telegram_message(&format!("🔴 [{}] OVERTIME จบ! ทุก asset Win แล้ว\n⏰ {}\n💰 Balance สุทธิ: ${:.2}\n📈 Profit: {}${:.2}\n📉 MaxLossCon: {} (Asset: {})", vps_name, Local::now().format("%d/%m/%Y %H:%M:%S"), final_bal, sign, profit, max_loss_end, loss_assets_end)).await;
            } else {
                // ไม่ใช้ Martingale → abort ทันที
                let mut bot_guard = state.active_bot.lock().await;
                let count = if bot_guard.is_some() { 1 } else { 0 };
                if let Some(handle) = bot_guard.take() {
                    handle.abort();
                }

                // ดึงยอด balance ใหม่เมื่อจบการทำงานปกติเพื่อสรุป profit
                let mut final_bal = 0.0;
                if let Ok(bal) = deriv::get_deriv_balance(&api_token).await {
                    final_bal = bal;
                    let mut bal_guard = state.last_balance.lock().await;
                    *bal_guard = bal;
                }
                let profit = final_bal - session_start_balance;
                let sign = if profit >= 0.0 { "+" } else { "" };

                let (max_loss_normal, loss_assets_normal) = trade_head::get_round_max_loss(round_no);
                send_telegram_message(&format!("🔴 [{}] หยุดเทรดอัตโนมัติ (Background Scheduler)!\n⏰ ถึงเวลา Stop: {}\n📊 หยุดบอททั้งหมด {} ตัว\n💰 Balance สุทธิ: ${:.2}\n📈 Profit: {}${:.2}\n📉 MaxLossCon: {} (Asset: {})", vps_name, stop_dt.format("%d/%m/%Y %H:%M:%S"), count, final_bal, sign, profit, max_loss_normal, loss_assets_normal)).await;
            }

            // ยกเลิก Schedule ใน tradeControl.json
            let mut tc = get_or_create_trade_control();
            tc.use_schedule = false;
            if let Ok(json_str) = serde_json::to_string_pretty(&tc) {
                let _ = fs::write(get_trade_control_path(), json_str);
            }
        }
    }
}

async fn background_longterm_scheduler_loop(state: AppState) {
    println!("🔄 Long Term Scheduler is now active...");
    let mut is_lt_auto_trading = false;
    let mut last_processed_start = String::new();
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        let tc = get_or_create_lt_trade_control();
        if !tc.use_schedule {
            if is_lt_auto_trading {
                is_lt_auto_trading = false;
            }
            continue;
        }

        let start_dt = match NaiveDateTime::parse_from_str(&tc.start_trade_time, "%Y-%m-%d %H:%M:%S") {
            Ok(dt) => match Local.from_local_datetime(&dt).single() { Some(t) => t, None => continue },
            Err(_) => continue,
        };
        let stop_dt = match NaiveDateTime::parse_from_str(&tc.stop_trade_time, "%Y-%m-%d %H:%M:%S") {
            Ok(dt) => match Local.from_local_datetime(&dt).single() { Some(t) => t, None => continue },
            Err(_) => continue,
        };
        let now = Local::now();

        if now >= start_dt && now < stop_dt {
            if !is_lt_auto_trading && last_processed_start != tc.start_trade_time {
                let mut bot_guard = state.long_term_bot.lock().await;
                let already_running = bot_guard.as_ref().map_or(false, |h| !h.is_finished());
                if already_running {
                    is_lt_auto_trading = true;
                    last_processed_start = tc.start_trade_time.clone();
                    continue;
                }
                println!("⚡ Long Term Scheduler: Target time reached! Starting bot.");
                is_lt_auto_trading = true;
                last_processed_start = tc.start_trade_time.clone();
                
                let setup = get_or_create_lt_setup();
                if setup.assets.is_empty() {
                    continue;
                }
                let tx = state.tx.clone();
                let lt_cmd_tx_clone = state.lt_cmd_tx.clone();
                let assets_for_log = setup.assets.clone();
                let handle = tokio::spawn(async move {
                    if let Err(e) = crate::long_term::start_longterm_bot(setup.assets, setup.granularity, tx, lt_cmd_tx_clone).await {
                        eprintln!("Long Term bot crashed (Scheduler): {}", e);
                    }
                });
                *bot_guard = Some(handle);
                let _ = state.tx.send(serde_json::json!({
                    "type": "lt_bot_started",
                    "assets": assets_for_log
                }));
            }
        }
        
        if now >= stop_dt && is_lt_auto_trading {
            println!("🛑 Long Term Scheduler: Stop time reached!");
            is_lt_auto_trading = false;
            let mut bot_guard = state.long_term_bot.lock().await;
            if let Some(handle) = bot_guard.take() {
                handle.abort();
            }
            let mut tc_update = get_or_create_lt_trade_control();
            tc_update.use_schedule = false;
            if let Ok(json_str) = serde_json::to_string_pretty(&tc_update) {
                let _ = std::fs::write(get_lt_trade_control_path(), json_str);
            }
            let _ = state.tx.send(serde_json::json!({
                "type": "bot_log",
                "asset": "system",
                "data": { "message": "🔴 หยุด Long Term Trade (Schedule)" }
            }));
        }
    }
}

/// Get backend version and compilation timestamp
#[utoipa::path(
    get,
    path = "/api/version",
    responses(
        (status = 200, description = "Backend version and timestamp")
    ),
    tag = "System"
)]
async fn get_version() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "main_rs_version": "v1.3",
        "compiled_at": chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
    }))
}

// ══════════════════════════════════════════════════════════════
//  GET ANALYSIS DATA API — ดึง/สร้าง analysisData ตาม asset + วันที่
//  GET /getanalysisdata?assetcode=R_10&sdate=13-05-2569
//  - ถ้ามีไฟล์อยู่แล้วใน analysisData → ส่ง JSON กลับทันที (cache)
//  - ถ้ายังไม่มี → ดึงข้อมูลจาก Deriv (01:00–23:59) → วิเคราะห์ → บันทึก → ส่งกลับ
// ══════════════════════════════════════════════════════════════
#[derive(Debug, Deserialize, Serialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct GetAnalysisDataQuery {
    /// Asset code symbol (e.g. R_10)
    #[param(example = "R_10")]
    pub assetcode: String,
    /// Thai Buddhist date format: DD-MM-YYYY (e.g. 13-05-2569)
    #[param(example = "13-05-2569")]
    pub sdate: String,
}

/// Retrieve cached or compute daily technical analysis for an asset
#[utoipa::path(
    get,
    path = "/getanalysisdata",
    params(GetAnalysisDataQuery),
    responses(
        (status = 200, description = "Daily analysis data JSON")
    ),
    tag = "Analysis Engine"
)]
async fn handle_get_analysis_data(
    Query(params): Query<GetAnalysisDataQuery>,
) -> Json<serde_json::Value> {
    // ── 1. Parse sdate: "13-05-2569" → day=13, month=05, thai_year=2569 ──
    let parts: Vec<&str> = params.sdate.split('-').collect();
    if parts.len() != 3 {
        return Json(serde_json::json!({
            "status": "error",
            "message": "Invalid sdate format. Expected DD-MM-YYYY (Thai Buddhist year) เช่น 13-05-2569"
        }));
    }

    let day = parts[0];
    let month = parts[1];
    let thai_year = parts[2];

    // ── 2. สร้าง path ตาม structure เดิม: analysisData/{MM-ThaiYear}/{DD-MM-ThaiYear}/{assetcode}/ ──
    let month_folder = format!("{}-{}", month, thai_year);       // "05-2569"
    let day_folder = params.sdate.clone();                        // "13-05-2569"
    let dir_path = format!("analysisData/{}/{}/{}", month_folder, day_folder, params.assetcode);

    // ── 3. ตรวจสอบว่ามีไฟล์ analysis อยู่แล้วหรือไม่ ──
    let analysis_dir = Path::new(&dir_path);
    if analysis_dir.exists() && analysis_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(analysis_dir) {
            for entry in entries.flatten() {
                let filename = entry.file_name().to_string_lossy().to_string();
                if filename.starts_with("analysis_") && filename.ends_with(".json") {
                    // พบไฟล์ analysis แล้ว → อ่านและส่งกลับ
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                            println!("📂 [getanalysisdata] พบไฟล์ที่มีอยู่แล้ว: {}/{}", dir_path, filename);
                            return Json(serde_json::json!({
                                "status": "success",
                                "source": "cache",
                                "asset": params.assetcode,
                                "sdate": params.sdate,
                                "file": format!("{}/{}", dir_path, filename),
                                "count": json.as_array().map(|a| a.len()).unwrap_or(0),
                                "data": json
                            }));
                        }
                    }
                }
            }
        }
    }

    // ── 4. ยังไม่มีไฟล์ → สร้างใหม่ ──
    println!("🔄 [getanalysisdata] ไม่พบไฟล์ cache สำหรับ {} วันที่ {} → กำลังสร้างใหม่...", params.assetcode, params.sdate);

    // แปลง Thai Buddhist year → CE year (พ.ศ. → ค.ศ.)
    let ce_year: i32 = thai_year.parse::<i32>().unwrap_or(0) - 543;
    let day_num: u32 = day.parse().unwrap_or(1);
    let month_num: u32 = month.parse().unwrap_or(1);

    if ce_year <= 0 || month_num == 0 || month_num > 12 || day_num == 0 || day_num > 31 {
        return Json(serde_json::json!({
            "status": "error",
            "message": format!("Invalid date values: day={}, month={}, thai_year={} (CE={})", day, month, thai_year, ce_year)
        }));
    }

    // สร้าง start/end timestamps → 01:00 AM ถึง 23:59 PM ของวันนั้น
    let start_str = format!("{:04}-{:02}-{:02}T01:00", ce_year, month_num, day_num);
    let stop_str = format!("{:04}-{:02}-{:02}T23:59", ce_year, month_num, day_num);

    let start_dt = match NaiveDateTime::parse_from_str(&start_str, "%Y-%m-%dT%H:%M") {
        Ok(dt) => dt,
        Err(_) => return Json(serde_json::json!({"status": "error", "message": "Failed to parse start datetime"}))
    };
    let stop_dt = match NaiveDateTime::parse_from_str(&stop_str, "%Y-%m-%dT%H:%M") {
        Ok(dt) => dt,
        Err(_) => return Json(serde_json::json!({"status": "error", "message": "Failed to parse stop datetime"}))
    };

    let start_epoch = Local.from_local_datetime(&start_dt).single()
        .map(|dt| dt.timestamp())
        .unwrap_or(start_dt.and_utc().timestamp());
    let stop_epoch = Local.from_local_datetime(&stop_dt).single()
        .map(|dt| dt.timestamp())
        .unwrap_or(stop_dt.and_utc().timestamp());

    println!("📡 [getanalysisdata] ดึงข้อมูลจาก Deriv: {} | {} → {} (epoch: {} → {})",
        params.assetcode, start_str, stop_str, start_epoch, stop_epoch);

    // ── 5. ดึงข้อมูลแท่งเทียนจาก Deriv ──
    match deriv::fetch_historical_candles(&params.assetcode, start_epoch, stop_epoch, 60).await {
        Ok(candles) => {
            if candles.is_empty() {
                return Json(serde_json::json!({
                    "status": "error",
                    "message": format!("ไม่พบข้อมูลแท่งเทียนจาก Deriv สำหรับ {} ในช่วง {} ถึง {}", params.assetcode, start_str, stop_str)
                }));
            }

            // โหลด config จาก setup.json
            let config_content = fs::read_to_string("setup/setup.json").unwrap_or_default();
            match serde_json::from_str::<AppConfigPayload>(&config_content) {
                Ok(config) => {
                    let mapped_history: Vec<crate::full_analysis_ver2::RawCandleInput> = candles.iter().map(|c| crate::full_analysis_ver2::RawCandleInput {
                        epoch: c.timestamp,
                        open: c.open,
                        high: c.high,
                        low: c.low,
                        close: c.close,
                    }).collect();
                    let config_for_analysis = match serde_json::from_str::<crate::full_analysis_ver2::AppConfigPayload>(&serde_json::to_string(&config).unwrap()) {
                        Ok(c) => Some(c),
                        Err(e) => {
                            eprintln!("❌ [handle_get_analysis_data] Failed to deserialize full_analysis_ver2::AppConfigPayload: {:?}", e);
                            None
                        }
                    };
                    let results = crate::full_analysis_ver2::perform_analysis(&mapped_history, config_for_analysis, Some(&params.assetcode));

                    // ── 7. บันทึกไฟล์ลง analysisData ──
                    let _ = fs::create_dir_all(&dir_path);

                    let safe_start = start_str.replace(":", "-");
                    let safe_stop = stop_str.replace(":", "-");
                    let file_path = format!("{}/analysis_{}_to_{}.json", dir_path, safe_start, safe_stop);

                    if let Ok(json_str) = serde_json::to_string_pretty(&results) {
                        let _ = fs::write(&file_path, &json_str);
                        println!("💾 [getanalysisdata] บันทึกไฟล์: {}", file_path);
                    }

                    // บันทึก setup.json ที่ใช้ในการวิเคราะห์ด้วย
                    let setup_path = format!("{}/setup.json", dir_path);
                    let _ = fs::write(&setup_path, &config_content);

                    // ── 8. ส่ง response กลับ ──
                    Json(serde_json::json!({
                        "status": "success",
                        "source": "generated",
                        "asset": params.assetcode,
                        "sdate": params.sdate,
                        "file_saved": file_path,
                        "count": results.len(),
                        "start": start_str,
                        "stop": stop_str,
                        "data": results
                    }))
                },
                Err(e) => {
                    Json(serde_json::json!({
                        "status": "error",
                        "message": format!("Failed to load setup.json: {}", e)
                    }))
                }
            }
        },
        Err(e) => {
            Json(serde_json::json!({
                "status": "error",
                "message": format!("Failed to fetch data from Deriv: {}", e)
            }))
        }
    }
}

// ══════════════════════════════════════════════════════════════
//  EXPORT TRADE DATA API
//  GET /api/export_trade_data?startdatetime=2026-05-01
//  - ดึงข้อมูลทั้งหมดจาก folder tradeData/MM-YYYY/DD-MM-YYYY
//  - สร้าง ZIP file และส่งกลับ
// ══════════════════════════════════════════════════════════════
#[derive(Debug, Deserialize, Serialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct ExportTradeDataQuery {
    /// Target trade date in YYYY-MM-DD format (e.g. 2026-09-08)
    #[param(example = "2026-09-08")]
    pub startdatetime: String, // format: YYYY-MM-DD
}

/// Export all trade logs and history for a date as a ZIP archive
#[utoipa::path(
    get,
    path = "/api/export_trade_data",
    params(ExportTradeDataQuery),
    responses(
        (status = 200, description = "ZIP archive of trade data"),
        (status = 400, description = "Invalid date format"),
        (status = 404, description = "Trade data folder not found")
    ),
    tag = "Trading & History"
)]
async fn handle_export_trade_data(
    Query(params): Query<ExportTradeDataQuery>,
) -> Result<impl axum::response::IntoResponse, (axum::http::StatusCode, String)> {
    use chrono::Datelike;
    use std::io::Write;
    use walkdir::WalkDir;
    
    println!("📦 [Export Trade Data] Request for date: {}", params.startdatetime);
    
    // 1. Parse date
    let parsed_date = match chrono::NaiveDate::parse_from_str(&params.startdatetime, "%Y-%m-%d") {
        Ok(d) => d,
        Err(e) => {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                format!("Invalid date format: {}. Expected YYYY-MM-DD", e)
            ));
        }
    };
    
    // 2. สร้าง path ตาม format: tradeData/MM-YYYY/DD-MM-YYYY
    let thai_year = parsed_date.year() + 543;
    let month_folder = format!("{:02}-{}", parsed_date.month(), thai_year);
    let day_folder = format!("{:02}-{:02}-{}", parsed_date.day(), parsed_date.month(), thai_year);
    let target_path = format!("tradeData/{}/{}", month_folder, day_folder);
    
    println!("📂 [Export Trade Data] Target path: {}", target_path);
    
    // 3. ตรวจสอบว่า folder มีอยู่หรือไม่
    if !std::path::Path::new(&target_path).exists() {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            format!("No trade data found for date: {}", params.startdatetime)
        ));
    }
    
    // 4. สร้าง ZIP file ใน memory
    let zip_filename = format!("trade_data_{}.zip", params.startdatetime);
    let mut zip_buffer = std::io::Cursor::new(Vec::new());
    
    {
        let mut zip = zip::ZipWriter::new(&mut zip_buffer);
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755);
        
        // 5. เดินผ่านทุกไฟล์ใน folder
        let mut file_count = 0;
        for entry in WalkDir::new(&target_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            
            // ข้าม directories
            if path.is_dir() {
                continue;
            }
            
            // อ่านไฟล์
            if let Ok(file_content) = std::fs::read(path) {
                // สร้าง path ใน ZIP (relative จาก target_path)
                let relative_path = path.strip_prefix(&target_path)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .replace("\\", "/"); // แปลง Windows path separator เป็น /
                
                // เพิ่มไฟล์ลง ZIP
                if let Err(e) = zip.start_file(relative_path.clone(), options) {
                    eprintln!("❌ [Export] Failed to add file to ZIP: {}", e);
                    continue;
                }
                
                if let Err(e) = zip.write_all(&file_content) {
                    eprintln!("❌ [Export] Failed to write file content: {}", e);
                    continue;
                }
                
                file_count += 1;
                println!("✅ [Export] Added to ZIP: {}", relative_path);
            }
        }
        
        if file_count == 0 {
            return Err((
                axum::http::StatusCode::NOT_FOUND,
                format!("No files found in: {}", target_path)
            ));
        }
        
        // 6. Finalize ZIP
        if let Err(e) = zip.finish() {
            return Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to finalize ZIP: {}", e)
            ));
        }
        
        println!("📦 [Export Trade Data] ZIP created with {} files", file_count);
    }
    
    // 7. ส่ง ZIP file กลับ
    let zip_data = zip_buffer.into_inner();
    let content_disposition = format!("attachment; filename=\"{}\"", zip_filename);
    
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "application/zip".to_string()),
            (axum::http::header::CONTENT_DISPOSITION, content_disposition),
        ],
        zip_data
    ))
}

// ══════════════════════════════════════════════════════════════
//  GET AVAILABLE TRADE DATA LIST API
//  GET /api/availableTradeData
//  - สแกนโฟลเดอร์ tradeData เพื่อส่งคืนรายการ Month, Day, Asset และ path ที่มีอยู่
// ══════════════════════════════════════════════════════════════
/// List available trade data dates and assets stored on disk
#[utoipa::path(
    get,
    path = "/api/availableTradeData",
    responses(
        (status = 200, description = "List of existing trade data files and assets")
    ),
    tag = "Trading & History"
)]
async fn handle_get_available_trade_data() -> Json<serde_json::Value> {
    use walkdir::WalkDir;

    let mut list = Vec::new();
    let base_path = Path::new("tradeData");

    if base_path.exists() {
        for entry in WalkDir::new(base_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.file_name().and_then(|n| n.to_str()) == Some("trades.json") {
                // Structure: tradeData/{month}/{day}/{asset}/trades.json
                if let Ok(rel) = path.strip_prefix(base_path) {
                    let components: Vec<_> = rel.iter().map(|c| c.to_string_lossy().to_string()).collect();
                    if components.len() == 4 {
                        let month = &components[0];
                        let day = &components[1];
                        let asset = &components[2];
                        let web_path = format!("/tradeData/{}/{}/{}/trades.json", month, day, asset);

                        list.push(serde_json::json!({
                            "month": month,
                            "day": day,
                            "asset": asset,
                            "path": web_path
                        }));
                    }
                }
            }
        }
    }

    Json(serde_json::json!({
        "status": "success",
        "count": list.len(),
        "files": list
    }))
}

// ══════════════════════════════════════════════════════════════
//  GET FULL ANALYSIS DATA API (Ver2)
//  GET /getFullAnalysisData?assetcode=R_10&startdatetime=2026-05-01T01:00&enddatetime=2026-05-01T23:59
//  GET /getFullAnalysisData?assetcode=R_10&startdatetime=2026-05-01T01:00&enddatetime=2026-05-01T23:59&timeframe=5M
//  - ดึงข้อมูลแท่งเทียนจาก Deriv ตามช่วงเวลาที่ระบุ
//  - รองรับ timeframe: 1M (default), 3M, 5M, 15M, 30M, 1H, 2H, 4H
//  - วิเคราะห์ด้วย full_analysis_ver2::perform_analysis
//  - ส่ง JSON กลับทันที
// ══════════════════════════════════════════════════════════════
#[derive(Debug, Deserialize, Serialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct GetFullAnalysisDataQuery {
    /// Asset code symbol (e.g. R_100, R_10, 1HZ10V)
    #[param(example = "R_100")]
    pub assetcode: String,
    /// Start datetime in format YYYY-MM-DDTHH:MM
    #[param(example = "2026-09-08T05:00")]
    pub startdatetime: String, // format: YYYY-MM-DDTHH:MM
    /// End datetime in format YYYY-MM-DDTHH:MM
    #[param(example = "2026-09-08T05:30")]
    pub enddatetime: String,   // format: YYYY-MM-DDTHH:MM
    /// Candle timeframe: 1M (default), 3M, 5M, 15M, 30M, 1H, 2H, 4H
    #[param(example = "1M")]
    pub timeframe: Option<String>, // optional: 1M, 3M, 5M, 15M, 30M, 1H, 2H, 4H (default: 1M)
    /// Pass 1 or true to return results as a compressed ZIP file
    #[param(example = "0")]
    pub zip: Option<String>,       // optional: 1, true, y, yes
    /// Alternative format specification: json or zip
    pub format: Option<String>,    // optional: zip, json
}

/// แปลง timeframe string → granularity เป็นวินาที
/// สามารถระบุตัวเลขตามด้วย M (นาที) หรือ H (ชั่วโมง) เช่น "1M", "5M", "10M", "1H"
fn parse_timeframe_to_granularity(timeframe: &str) -> i64 {
    let tf = timeframe.to_uppercase();
    
    // ตรวจสอบลงท้ายด้วย M (นาที)
    if tf.ends_with('M') {
        if let Ok(minutes) = tf.trim_end_matches('M').parse::<i64>() {
            return minutes * 60;
        }
    } 
    // ตรวจสอบลงท้ายด้วย H (ชั่วโมง)
    else if tf.ends_with('H') {
        if let Ok(hours) = tf.trim_end_matches('H').parse::<i64>() {
            return hours * 3600;
        }
    }
    // ตรวจสอบลงท้ายด้วย S (วินาที) สำหรับ Tick / วินาทีเฉพาะทาง
    else if tf.ends_with('S') {
        if let Ok(seconds) = tf.trim_end_matches('S').parse::<i64>() {
            return seconds;
        }
    }

    // Default fallback หากระบุผิดรูปแบบ ให้กลับไปใช้ 1M (60 วินาที)
    60
}

/// Fetch Deriv historical candles, compute multi-indicator analysis (SMC, CI, RSI, PK Trend v5), and return JSON or ZIP
#[utoipa::path(
    get,
    path = "/getFullAnalysisData",
    params(GetFullAnalysisDataQuery),
    responses(
        (status = 200, description = "Full analysis JSON array or ZIP archive if zip=1"),
        (status = 400, description = "Invalid date format or parameters")
    ),
    tag = "Analysis Engine"
)]
async fn handle_get_full_analysis_data(
    Query(params): Query<GetFullAnalysisDataQuery>,
) -> Response {
    // ── Parse timeframe → granularity (default = 1M = 60s) ──
    let timeframe_str = params.timeframe.as_deref().unwrap_or("1M");
    let granularity = parse_timeframe_to_granularity(timeframe_str);

    println!("📊 [getFullAnalysisData] assetcode={}, start={}, end={}, timeframe={} (granularity={}s)",
        params.assetcode, params.startdatetime, params.enddatetime, timeframe_str, granularity);

    // 1. Parse startdatetime
    let start_dt = match NaiveDateTime::parse_from_str(&params.startdatetime, "%Y-%m-%dT%H:%M") {
        Ok(dt) => dt,
        Err(_) => return Json(serde_json::json!({
            "status": "error",
            "message": format!("Invalid startdatetime format '{}'. Expected: YYYY-MM-DDTHH:MM", params.startdatetime)
        })).into_response()
    };

    // 2. Parse enddatetime
    let stop_dt = match NaiveDateTime::parse_from_str(&params.enddatetime, "%Y-%m-%dT%H:%M") {
        Ok(dt) => dt,
        Err(_) => return Json(serde_json::json!({
            "status": "error",
            "message": format!("Invalid enddatetime format '{}'. Expected: YYYY-MM-DDTHH:MM", params.enddatetime)
        })).into_response()
    };

    // 3. Convert to epoch (local timezone)
    let start_epoch = Local.from_local_datetime(&start_dt)
        .single()
        .map(|dt| dt.timestamp())
        .unwrap_or(start_dt.and_utc().timestamp());
    let stop_epoch = Local.from_local_datetime(&stop_dt)
        .single()
        .map(|dt| dt.timestamp())
        .unwrap_or(stop_dt.and_utc().timestamp());

    println!("📡 [getFullAnalysisData] ดึงข้อมูลจาก Deriv: {} | timeframe={} | epoch: {} → {}",
        params.assetcode, timeframe_str, start_epoch, stop_epoch);

    // 4. Fetch historical candles from Deriv (ใช้ granularity ตาม timeframe ที่ระบุ)
    match deriv::fetch_historical_candles(&params.assetcode, start_epoch, stop_epoch, granularity).await {
        Ok(candles) => {
            if candles.is_empty() {
                return Json(serde_json::json!({
                    "status": "error",
                    "message": format!("ไม่พบข้อมูลแท่งเทียนจาก Deriv สำหรับ {} ในช่วง {} ถึง {}", params.assetcode, params.startdatetime, params.enddatetime)
                })).into_response();
            }

            // 5. Convert OHLCV -> RawCandleInput for full_analysis_ver2
            let raw_candles: Vec<full_analysis_ver2::RawCandleInput> = candles.into_iter().map(|c| full_analysis_ver2::RawCandleInput {
                epoch: c.timestamp,
                open: c.open,
                high: c.high,
                low: c.low,
                close: c.close,
            }).collect();

            // 6. Load config from setup.json
            let config_content = fs::read_to_string("setup/setup.json").unwrap_or_default();
            let v2_config = match serde_json::from_str::<full_analysis_ver2::AppConfigPayload>(&config_content) {
                Ok(c) => Some(c),
                Err(e) => {
                    eprintln!("❌ [handle_get_full_analysis_data] Failed to deserialize full_analysis_ver2::AppConfigPayload: {:?}", e);
                    None
                }
            };

            // 7. Run full_analysis_ver2
            let mut results = full_analysis_ver2::perform_analysis(&raw_candles, v2_config, Some(&params.assetcode));

            // Set asset_code for each result
            let symbol = match params.assetcode.as_str() {
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
                _ => &params.assetcode,
            };

            for r in &mut results {
                r.asset_code = symbol.to_string();
            }

            println!("✅ [getFullAnalysisData] วิเคราะห์เสร็จ: {} records for {}", results.len(), params.assetcode);

            let json_response = serde_json::json!({
                "status": "success",
                "asset": params.assetcode,
                "symbol": symbol,
                "count": results.len(),
                "startdatetime": params.startdatetime,
                "enddatetime": params.enddatetime,
                "timeframe": timeframe_str,
                "granularity": granularity,
                "data": results
            });

            // ตรวจสอบว่าผู้ใช้ร้องขอไฟล์ ZIP หรือไม่ (?zip=1, ?zip=true, ?zip=y หรือ ?format=zip)
            let is_zip = params.zip.as_deref().map(|s| {
                s == "1" || s.eq_ignore_ascii_case("true") || s.eq_ignore_ascii_case("y") || s.eq_ignore_ascii_case("yes")
            }).unwrap_or(false) || params.format.as_deref().map(|s| s.eq_ignore_ascii_case("zip")).unwrap_or(false);

            if is_zip {
                use std::io::Write;
                let safe_start = params.startdatetime.replace(":", "-");
                let safe_stop = params.enddatetime.replace(":", "-");
                let zip_filename = format!("analysis_{}_{}_to_{}_{}.zip", params.assetcode, safe_start, safe_stop, timeframe_str);
                let json_filename = format!("analysis_{}_{}_to_{}_{}.json", params.assetcode, safe_start, safe_stop, timeframe_str);

                let json_bytes = serde_json::to_vec_pretty(&json_response).unwrap_or_default();

                let mut zip_buffer = std::io::Cursor::new(Vec::new());
                {
                    let mut zip = zip::ZipWriter::new(&mut zip_buffer);
                    let options = zip::write::FileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated);

                    if let Err(e) = zip.start_file(&json_filename, options) {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({"status": "error", "message": format!("Failed to create file inside zip: {}", e)}))
                        ).into_response();
                    }
                    if let Err(e) = zip.write_all(&json_bytes) {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({"status": "error", "message": format!("Failed to write to zip: {}", e)}))
                        ).into_response();
                    }
                    if let Err(e) = zip.finish() {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({"status": "error", "message": format!("Failed to finalize zip: {}", e)}))
                        ).into_response();
                    }
                }

                let zip_data = zip_buffer.into_inner();
                let content_disposition = format!("attachment; filename=\"{}\"", zip_filename);

                (
                    [
                        (header::CONTENT_TYPE, "application/zip".to_string()),
                        (header::CONTENT_DISPOSITION, content_disposition),
                    ],
                    zip_data
                ).into_response()
            } else {
                Json(json_response).into_response()
            }
        },
        Err(e) => {
            Json(serde_json::json!({
                "status": "error",
                "message": format!("Failed to fetch data from Deriv: {}", e)
            })).into_response()
        }
    }
}


// ══════════════════════════════════════════════════════════════
//  STATUS API — ให้ Frontend ตรวจสอบสถานะบอทปัจจุบัน
//  เครื่อง B เปิดมาจะได้รู้ว่าบอทกำลังทำงานอยู่หรือไม่
// ══════════════════════════════════════════════════════════════
/// Get current engine status, live positions, balance, and WebSocket connectivity
#[utoipa::path(
    get,
    path = "/api/status",
    responses(
        (status = 200, description = "Engine status JSON")
    ),
    tag = "System"
)]
async fn handle_get_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let bot_guard = state.active_bot.lock().await;
    let is_trading = bot_guard.is_some();
    
    let lt_bot_guard = state.long_term_bot.lock().await;
    let is_lt_trading = lt_bot_guard.is_some();
    
    let ot_guard = state.overtime_signals.lock().await;
    let active_assets: Vec<String> = if is_trading { ot_guard.keys().cloned().collect() } else { vec![] };
    let log_count = state.bot_logs.lock().await.len();
    let candle_assets: Vec<String> = state.candle_data.lock().await.keys().cloned().collect();
    let balance = *state.last_balance.lock().await;
    let ws_status_guard = state.lt_ws_status.lock().await;
    let candle_ws_status = ws_status_guard.get("candle").cloned().unwrap_or_else(|| "disconnected".to_string());
    let private_ws_status = ws_status_guard.get("private").cloned().unwrap_or_else(|| "disconnected".to_string());
    let account_name = crate::get_active_account_name();

    Json(serde_json::json!({
        "is_trading": is_trading,
        "is_lt_trading": is_lt_trading,
        "active_assets": active_assets,
        "bot_count": if is_trading { 1 } else { 0 },
        "log_count": log_count,
        "candle_assets": candle_assets,
        "balance": balance,
        "candle_ws_status": candle_ws_status,
        "private_ws_status": private_ws_status,
        "account_name": account_name
    }))
}

async fn handle_post_sync_balance(State(state): State<AppState>) -> Json<serde_json::Value> {
    let (_, api_token, _) = crate::get_active_credentials(None);
    if api_token.is_empty() || api_token == "YOUR_API_TOKEN_HERE" {
        return Json(serde_json::json!({
            "status": "error",
            "message": "Missing API Token in .env"
        }));
    }

    match deriv::get_deriv_balance(&api_token).await {
        Ok(bal) => {
            let mut bal_guard = state.last_balance.lock().await;
            *bal_guard = bal;
            
            // Broadcast the new balance to all connected clients
            let _ = state.tx.send(serde_json::json!({
                "type": "balance_update",
                "data": { "balance": bal }
            }));

            Json(serde_json::json!({
                "status": "success",
                "balance": bal,
                "message": format!("Balance synced: ${:.2}", bal)
            }))
        }
        Err(e) => {
            Json(serde_json::json!({
                "status": "error",
                "message": format!("Failed to sync balance: {}", e)
            }))
        }
    }
}

// ══════════════════════════════════════════════════════════════
//  NO-CACHE Handler สำหรับ index.html
// ══════════════════════════════════════════════════════════════
async fn serve_index(
    host: axum::extract::Host,
    headers: axum::http::HeaderMap,
) -> Response {
    let mut host_str = host.0.clone();
    if let Some(forwarded) = headers.get("x-forwarded-host").and_then(|h| h.to_str().ok()) {
        host_str = forwarded.to_string();
    } else if let Some(forwarded_server) = headers.get("x-forwarded-server").and_then(|h| h.to_str().ok()) {
        host_str = forwarded_server.to_string();
    }
    
    let _ = std::fs::write("setup/active_host.txt", &host_str);
    match fs::read_to_string("public/index.html") {
        Ok(content) => (
            [
                (header::CACHE_CONTROL, "no-cache, no-store, must-revalidate"),
                (header::PRAGMA, "no-cache"),
                (header::EXPIRES, "0"),
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            ],
            content,
        ).into_response(),
        Err(_) => (
            StatusCode::NOT_FOUND,
            "index.html not found",
        ).into_response(),
    }
}

async fn handle_node_sync_ws_route(
    ws: WebSocketUpgrade,
    query: Query<node_sync::NodeSyncQuery>,
    State(state): State<AppState>,
) -> axum::response::Response {
    node_sync::handle_node_sync_ws(ws, query, state.aggregator.clone()).await
}

async fn handle_get_multi_node_summary(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let summary = state.aggregator.get_summary().await;
    Json(serde_json::json!({
        "status": "success",
        "data": summary
    }))
}

// ══════════════════════════════════════════════════════════════
//  OPENAPI 3.0 / SWAGGER SCHEMA DEFINITION (Minimal Binary Technique)
// ══════════════════════════════════════════════════════════════
#[derive(OpenApi)]
#[openapi(
    paths(
        handle_get_full_analysis_data,
        crate::full_analysis_ver2::handle_post_analysis_data,
        handle_get_analysis_data,
        get_setup,
        get_default_setup,
        get_settings,
        get_thereshold,
        handle_get_case_codes,
        handle_get_status,
        get_version,
        get_history,
        get_tradehistory_data,
        get_strategy_comparison,
        handle_export_trade_data,
        handle_get_available_trade_data,
    ),
    components(
        schemas(
            GetFullAnalysisDataQuery,
            GetAnalysisDataQuery,
            HistoryQuery,
            TradeHistoryDataQuery,
            StrategyComparisonQuery,
            ExportTradeDataQuery,
            crate::full_analysis_ver2::RawCandleInput,
        )
    ),
    tags(
        (name = "Analysis Engine", description = "High-performance technical indicator analysis, PK trend detection & data export"),
        (name = "Configuration", description = "Bot parameters, strategy indicators and threshold configuration"),
        (name = "Trading & History", description = "Historical trades, performance comparison and data archiving"),
        (name = "System", description = "System health, engine status and backend metadata")
    ),
    info(
        title = "Turbo Indicators & Multiplex Trading Engine API",
        version = "2.0.0",
        description = "High-performance Axum & SIMD-accelerated trading indicators, multi-timeframe candle analysis, and PK Trend Detection v5 API documentation."
    )
)]
pub struct ApiDoc;

async fn handle_get_openapi_json() -> impl IntoResponse {
    axum::Json(ApiDoc::openapi())
}

#[tokio::main]
async fn main() {
    // ดึงเวลา Build มาจาก Environment Variable ที่ build.rs เซ็ตไว้ให้ตอน Compile
    // ใช้ option_env! เพื่อไม่ให้พังตอน compile แม้ build.rs จะไม่ได้รัน
    let build_time = option_env!("BUILD_TIME").unwrap_or("Unknown");
    let build_profile = option_env!("BUILD_PROFILE").unwrap_or("Unknown");
    dotenv().ok();

    let (app_id, _, _) = crate::get_active_credentials(None);
    let port_str = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let port: u16 = port_str.parse().unwrap_or(3000);

    println!("==========================");
    println!("VERSION 2026-07-09 08:30");
    println!("Build Time (TH): {}", build_time);
    println!("Build Profile: {}", build_profile);
    println!("==========================");
    println!("Starting server on port {}", port);
    println!("Deriv App ID configured: {}", app_id);

    // 2. Set up Axum Router with State
    let (tx, _rx) = broadcast::channel(100);
    let (cmd_tx, _cmd_rx) = broadcast::channel(10);
    let (lt_cmd_tx, _lt_cmd_rx) = broadcast::channel(10);
    let tx_arc = Arc::new(tx);

    let local_node_name = env::var("NODE_NAME")
        .or_else(|_| env::var("VPS_NAME"))
        .unwrap_or_else(|_| format!("Node-{}", port));
    let aggregator = Arc::new(node_sync::MultiNodeAggregator::new(local_node_name, tx_arc.clone()));

    let app_state = AppState {
        tx: tx_arc,
        cmd_tx: Arc::new(cmd_tx),
        lt_cmd_tx: Arc::new(lt_cmd_tx),
        active_bot: Arc::new(Mutex::new(None)),
        long_term_bot: Arc::new(Mutex::new(None)),
        overtime_signals: Arc::new(Mutex::new(HashMap::new())),
        bot_logs: Arc::new(Mutex::new(Vec::new())),
        candle_data: Arc::new(Mutex::new(HashMap::new())),
        last_balance: Arc::new(Mutex::new(0.0)),
        lt_ws_status: Arc::new(Mutex::new(HashMap::new())),
        aggregator: aggregator.clone(),
    };

    // 2.5 Multi-Node Sync Background Tasks
    let sync_aggregator = aggregator.clone();
    tokio::spawn(async move {
        node_sync::run_client_sync_worker(sync_aggregator).await;
    });

    // Periodic broadcast ทำเฉพาะ Hub (standalone) mode เท่านั้น
    // Client mode จะได้รับ summary จาก Hub แล้ว ไม่ต้อง broadcast ซ้ำ (จะทับข้อมูลจาก Hub)
    let primary_hub_url = std::env::var("PRIMARY_HUB_URL").unwrap_or_default();
    if primary_hub_url.trim().is_empty() {
        let periodic_aggregator = aggregator.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
            loop {
                interval.tick().await;
                periodic_aggregator.broadcast_summary().await;
            }
        });
    }

    // 3. เปิดใช้งาน Background Scheduler แบบ Active Polling
    let scheduler_state = app_state.clone();
    tokio::spawn(async move {
        background_scheduler_loop(scheduler_state).await;
    });

    let lt_scheduler_state = app_state.clone();
    tokio::spawn(async move {
        background_longterm_scheduler_loop(lt_scheduler_state).await;
    });

    // 3.5 Storage Subscriber — ดักจับข้อมูลจาก broadcast เพื่อเก็บไว้ให้ client ใหม่
    //     เครื่อง B เปิดมาจะได้รับ snapshot ของข้อมูลทั้งหมดที่สะสมไว้
    let storage_state = app_state.clone();
    let mut storage_rx = app_state.tx.subscribe();
    tokio::spawn(async move {
        const MAX_LOGS: usize = 500;
        loop {
            match storage_rx.recv().await {
                Ok(msg) => {
                    let msg_type = msg.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    match msg_type {
                        "lt_ws_status" => {
                            if let (Some(target), Some(status)) = (
                                msg.get("target").and_then(|t| t.as_str()),
                                msg.get("status").and_then(|s| s.as_str()),
                            ) {
                                let mut map = storage_state.lt_ws_status.lock().await;
                                map.insert(target.to_string(), status.to_string());
                                println!("📦 [storage_subscriber] Cached lt_ws_status for {}: {}", target, status);
                            }
                        }
                        "candles_history" => {
                            if let Some(asset) = msg.get("asset").and_then(|a| a.as_str()).map(|s| s.to_string()) {
                                let mut data = storage_state.candle_data.lock().await;
                                data.insert(asset.clone(), msg);
                                println!("📦 [storage_subscriber] Cached candles_history for asset: {}", asset);
                            }
                        }
                        "ohlc_update" => {
                            // อัปเดตข้อมูลแท่งเทียนล่าสุดใน candle_data เพื่อให้ snapshot ทันสมัย
                            if let Some(asset) = msg.get("asset").and_then(|a| a.as_str()) {
                                let mut data = storage_state.candle_data.lock().await;
                                if let Some(stored) = data.get_mut(asset) {
                                    if let Some(new_data) = msg.get("data") {
                                        let new_time = new_data.get("candletime")
                                            .or_else(|| new_data.get("epoch"))
                                            .and_then(|e| e.as_i64());
                                        if let Some(new_epoch) = new_time {
                                            if let Some(arr) = stored.get_mut("data").and_then(|d| d.as_array_mut()) {
                                                if let Some(last) = arr.last_mut() {
                                                    let last_epoch = last.get("candletime")
                                                        .or_else(|| last.get("epoch"))
                                                        .and_then(|e| e.as_i64())
                                                        .unwrap_or(0);
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
                            // อัปเดตยอดเงินล่าสุดใน AppState
                            if let Some(data) = msg.get("data") {
                                if let Some(bal) = data.get("balance").and_then(|v| v.as_f64()) {
                                    let mut bal_guard = storage_state.last_balance.lock().await;
                                    *bal_guard = bal;

                                    let account_id = env::var("DERIV_ACCOUNT_ID").unwrap_or_else(|_| "Deriv-Acc".to_string());
                                    storage_state.aggregator.update_local_node_with_metrics(&account_id, bal).await;
                                }
                            }

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
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    eprintln!("⚠️ [storage_subscriber] Broadcast channel lagged by {} messages.", n);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    println!("🛑 [storage_subscriber] Broadcast channel closed. Exiting subscriber loop.");
                    break;
                }
            }
        }
    });

    // 3.6 Fetch initial balance from Deriv
    let initial_balance_state = app_state.clone();
    tokio::spawn(async move {
        let (_, api_token, _) = crate::get_active_credentials(None);
        if !api_token.is_empty() && api_token != "YOUR_API_TOKEN_HERE" {
            let preview = if api_token.len() > 10 { &api_token[..10] } else { &api_token };
            println!("🔄 Fetching initial balance from Deriv... (token_len={}, prefix={}...)", api_token.len(), preview);
            match deriv::get_deriv_balance(&api_token).await {
                Ok(bal) => {
                    let mut bal_guard = initial_balance_state.last_balance.lock().await;
                    *bal_guard = bal;
                    println!("💰 Initial Balance: ${:.2}", bal);

                    let account_id = env::var("DERIV_ACCOUNT_ID").unwrap_or_else(|_| "Deriv-Acc".to_string());
                    initial_balance_state.aggregator.update_local_node_with_metrics(&account_id, bal).await;
                }
                Err(e) => {
                    eprintln!("⚠️ Failed to fetch initial balance: {}", e);
                }
            }
        }
    });

    // 3.7 Periodic local node metrics sync (runs on all nodes to keep Hub & Web fresh)
    let sync_local_state = app_state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        let account_id = env::var("DERIV_ACCOUNT_ID").unwrap_or_else(|_| "Deriv-Acc".to_string());
        loop {
            interval.tick().await;
            let current_bal = {
                let guard = sync_local_state.last_balance.lock().await;
                *guard
            };
            if current_bal > 0.0 {
                sync_local_state.aggregator.update_local_node_with_metrics(&account_id, current_bal).await;
            }
        }
    });

    let protected_routes = Router::new()
        .route("/api/trade", post(handle_post_trade))
        .route("/api/stop", post(handle_post_stop))
        .route("/api/terminate", post(handle_post_terminate))
        .route("/api/notify", post(handle_post_notify))
        .route("/api/setup", get(get_setup).post(save_setup))
        .route("/api/setup/default", get(get_default_setup))
        .route("/api/settings", get(get_settings).post(save_settings))
        .route("/api/thereshold", get(get_thereshold).post(save_thereshold))
        .route("/api/case_codes", get(handle_get_case_codes).post(handle_post_save_case_codes))
        .route("/api/save_case_codes", post(handle_post_save_case_codes))
        .route("/api/strategy_case_codes", get(handle_get_strategy_case_codes).post(handle_post_save_strategy_case_codes))
        .route("/api/save_strategy_case_codes", post(handle_post_save_strategy_case_codes))
        .route("/api/sell", post(handle_post_sell))
        .route("/api/manual_trade", post(handle_post_manual_trade))
        .route("/api/save_manual_history", post(handle_post_save_manual_history))
        .route("/api/history", get(get_history))
        .route("/api/getTradeHistory", get(get_tradehistory_data))
        .route("/api/getStrategyComparison", get(get_strategy_comparison))
        .route("/api/generate_analysis", post(handle_post_generate_analysis))
        .route("/api/config/url_ajax_post", get(get_url_ajax_post))
        .route("/api/version", get(get_version))
        .route("/api/status", get(handle_get_status))
        .route("/api/balance/sync", post(handle_post_sync_balance))
        .route("/api/tradeControl", get(handle_get_trade_control).post(handle_post_trade_control))
        .route("/api/system/restart", post(handle_post_restart_service))
        .route("/getAnalysisdata", post(full_analysis_ver2::handle_post_analysis_data))
        .route("/getanalysisdata", get(handle_get_analysis_data))
        .route("/getFullAnalysisData", get(handle_get_full_analysis_data))
        .route("/api/export_trade_data", get(handle_export_trade_data))
        .route("/api/availableTradeData", get(handle_get_available_trade_data))
        .route("/api/longterm/start", post(handle_post_longterm_start))
        .route("/api/longterm/reconnect_ws", post(handle_post_lt_reconnect_ws))
        .route("/api/longterm/schedule", get(handle_get_longterm_schedule).post(handle_post_longterm_schedule))
        .route("/api/longterm/manual_trade", post(handle_post_lt_manual_trade))
        .route("/api/longterm/sell", post(handle_post_lt_sell))
        .route("/api/longterm/target", post(handle_post_lt_set_target))
        .route("/api/longterm/save_track_snapshot", post(handle_post_lt_save_track_snapshot))
        .route("/api/longterm/save_track_snapshots_batch", post(handle_post_lt_save_track_snapshots_batch))

        .route("/api/longterm/stop", post(handle_post_longterm_stop))
        .route("/api/longterm/toggle_auto", post(handle_post_longterm_toggle_auto))
        .route("/api/longterm/candles", get(handle_get_lt_chart_candles))
        .route("/api/longterm/settings", get(handle_get_lt_settings).post(handle_post_lt_settings))
        .route("/api/longterm/active-settings", get(handle_get_active_settings))
        .route("/api/deriv-config", get(handle_get_deriv_config))
        .route("/api/deriv-otp", get(handle_get_deriv_otp))
        .route("/ws", get(ws_handler))
        .route_layer(middleware::from_fn(auth_middleware));

    let cors = tower_http::cors::CorsLayer::permissive();

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/index.html", get(serve_index))
        .route("/docs", get(|| async { axum::response::Redirect::permanent("/docs.html") }))
        .route("/api-docs/openapi.json", get(handle_get_openapi_json))
        .route("/api/verify_fingerprint", post(handle_verify_fingerprint))
        .route("/ws/node-sync", get(handle_node_sync_ws_route))
        .route("/api/multi_node_summary", get(handle_get_multi_node_summary))
        .nest_service("/sounds", ServeDir::new("sounds"))
        .nest_service("/tradeData", ServeDir::new("tradeData"))
        .merge(protected_routes)
        .fallback_service(ServeDir::new("public"))
        .layer(cors)
        .with_state(app_state);

    // 4. Start Server
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    println!("Web Server is running at http://localhost:{}", port);
    println!("Serving UI from the 'public/' directory.");
    println!("📖 Swagger API Documentation: http://localhost:{}/docs.html", port);

    axum::serve(listener, app).await.unwrap();
}

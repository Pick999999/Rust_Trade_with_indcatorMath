use std::fs;
use chrono::{Datelike, Local, NaiveDateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetTradeSummary {
    #[serde(rename = "assetCode")]
    pub asset_code: String,
    #[serde(rename = "MaxWinCon")]
    pub max_win_con: u32,
    #[serde(rename = "MaxLossCon")]
    pub max_loss_con: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeHead {
    #[serde(rename = "serverCode")]
    pub server_code: i32,
    #[serde(rename = "tradeRoundNo")]
    pub trade_round_no: u32,
    #[serde(rename = "usestrategyCode", default)]
    pub use_strategy_code: String,
    #[serde(rename = "startTimeTrade")]
    pub start_time_trade: String,
    #[serde(rename = "stopTimeTrade")]
    pub stop_time_trade: String,
    #[serde(rename = "durationTrade")]
    pub duration_trade: String,
    #[serde(rename = "assetTrade")]
    pub asset_trade: Vec<AssetTradeSummary>,
    #[serde(rename = "MaxLossCon")]
    pub max_loss_con: u32,
}

pub fn get_server_code() -> i32 {
    std::env::var("serverCode")
        .or_else(|_| std::env::var("SERVER_CODE"))
        .unwrap_or_else(|_| "1".to_string())
        .trim()
        .parse::<i32>()
        .unwrap_or(1)
}

pub fn get_trade_head_path() -> String {
    let now = Local::now();
    let month_folder = format!("{:02}-{:04}", now.month(), now.year() as i32 + 543);
    let day_folder = format!("{:02}-{:02}-{:04}", now.day(), now.month(), now.year() as i32 + 543);
    let today_dir = format!("tradeData/{}/{}", month_folder, day_folder);
    let _ = fs::create_dir_all(&today_dir);
    format!("{}/tradeHead.json", today_dir)
}

pub fn load_trade_heads() -> Vec<TradeHead> {
    let path = get_trade_head_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(heads) = serde_json::from_str::<Vec<TradeHead>>(&content) {
            return heads;
        }
    }
    Vec::new()
}

pub fn save_trade_heads(heads: &[TradeHead]) {
    let path = get_trade_head_path();
    if let Ok(json_str) = serde_json::to_string_pretty(heads) {
        let _ = fs::write(&path, json_str);
    }
}

pub fn calculate_duration(start_str: &str, stop_str: &str) -> String {
    if start_str.is_empty() || stop_str.is_empty() {
        return "".to_string();
    }
    if let (Ok(start), Ok(stop)) = (
        NaiveDateTime::parse_from_str(start_str, "%Y-%m-%d %H:%M:%S")
            .or_else(|_| NaiveDateTime::parse_from_str(start_str, "%d/%m/%Y %H:%M:%S")),
        NaiveDateTime::parse_from_str(stop_str, "%Y-%m-%d %H:%M:%S")
            .or_else(|_| NaiveDateTime::parse_from_str(stop_str, "%d/%m/%Y %H:%M:%S")),
    ) {
        let duration = stop.signed_duration_since(start);
        let secs = duration.num_seconds();
        if secs > 0 {
            let hrs = secs / 3600;
            let mins = (secs % 3600) / 60;
            let s = secs % 60;
            if hrs > 0 {
                format!("{}h {}m {}s", hrs, mins, s)
            } else if mins > 0 {
                format!("{}m {}s", mins, s)
            } else {
                format!("{}s", s)
            }
        } else {
            "0s".to_string()
        }
    } else {
        "".to_string()
    }
}

pub fn on_bot_start(round_no: u32, start_time: &str, assets: &[String], strategy_code: &str) {
    let mut heads = load_trade_heads();
    let server_code = get_server_code();

    let asset_trade: Vec<AssetTradeSummary> = assets
        .iter()
        .map(|a| AssetTradeSummary {
            asset_code: crate::deriv::map_asset_to_symbol(a),
            max_win_con: 0,
            max_loss_con: 0,
        })
        .collect();

    if let Some(pos) = heads.iter().position(|h| h.trade_round_no == round_no) {
        heads[pos].server_code = server_code;
        heads[pos].use_strategy_code = strategy_code.to_string();
        heads[pos].start_time_trade = start_time.to_string();
        heads[pos].stop_time_trade = "".to_string();
        heads[pos].duration_trade = "".to_string();
        heads[pos].asset_trade = asset_trade;
        heads[pos].max_loss_con = 0;
    } else {
        heads.push(TradeHead {
            server_code,
            trade_round_no: round_no,
            use_strategy_code: strategy_code.to_string(),
            start_time_trade: start_time.to_string(),
            stop_time_trade: "".to_string(),
            duration_trade: "".to_string(),
            asset_trade,
            max_loss_con: 0,
        });
    }

    save_trade_heads(&heads);
    println!("📋 [TradeHead] บันทึกเริ่มต้น tradeHead.json สำหรับรอบที่ {} (Strategy: {})", round_no, strategy_code);
}

pub fn on_trade_update(round_no: u32, asset_symbol: &str, max_win_con: u32, max_loss_con: u32) {
    let mut heads = load_trade_heads();
    let mut found = false;
    if let Some(head) = heads.iter_mut().find(|h| h.trade_round_no == round_no) {
        if let Some(asset_entry) = head.asset_trade.iter_mut().find(|a| a.asset_code == asset_symbol) {
            if max_win_con > asset_entry.max_win_con {
                asset_entry.max_win_con = max_win_con;
            }
            if max_loss_con > asset_entry.max_loss_con {
                asset_entry.max_loss_con = max_loss_con;
            }
        } else {
            head.asset_trade.push(AssetTradeSummary {
                asset_code: asset_symbol.to_string(),
                max_win_con,
                max_loss_con,
            });
        }
        head.max_loss_con = head.asset_trade.iter().map(|a| a.max_loss_con).max().unwrap_or(0);
        found = true;
    }
    if found {
        save_trade_heads(&heads);
    }
}

pub fn on_bot_stop(round_no: u32, stop_time: &str) {
    let mut heads = load_trade_heads();
    let mut duration_res = String::new();
    if let Some(head) = heads.iter_mut().find(|h| h.trade_round_no == round_no) {
        head.stop_time_trade = stop_time.to_string();
        head.duration_trade = calculate_duration(&head.start_time_trade, stop_time);
        duration_res = head.duration_trade.clone();
    }
    if !duration_res.is_empty() {
        save_trade_heads(&heads);
        println!("📋 [TradeHead] ปิดรอบ tradeHead.json สำหรับรอบที่ {} (ระยะเวลา: {})", round_no, duration_res);
    }
}

pub fn get_round_max_loss(round_no: u32) -> (u32, String) {
    let heads = load_trade_heads();
    let head = heads.iter().rev().find(|h| h.trade_round_no == round_no).or_else(|| heads.last());
    if let Some(h) = head {
        let max_loss = h.max_loss_con;
        if max_loss > 0 {
            let top_assets: Vec<String> = h.asset_trade.iter()
                .filter(|a| a.max_loss_con == max_loss)
                .map(|a| a.asset_code.clone())
                .collect();
            let asset_str = if top_assets.is_empty() {
                "-".to_string()
            } else {
                top_assets.join(", ")
            };
            (max_loss, asset_str)
        } else {
            (0, "-".to_string())
        }
    } else {
        (0, "-".to_string())
    }
}

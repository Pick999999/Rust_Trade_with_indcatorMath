use serde::{Deserialize, Serialize};

// ตัวแทนผลลัพธ์ที่จะคืนค่าไปให้ระบบเทรดหลักนำไปออกออเดอร์
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum TradeAction {
    Call,
    Put,
    Wait,
}

// โครงสร้างที่รับมาจาก lib.rs (Indicator Engine) สำหรับ 1 แท่งเทียน
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisObject {
    pub epoch: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub is_atr: bool,
    pub this_color: String,
    pub ema_short_direction: String,
    pub ema_medium_direction: String,
    pub ema_short_val: f64,
    pub ema_medium_val: f64,
    pub ema_long_val: f64,
}

/// กลยุทธ์หาสีที่ควรเทรด — ตรงตามต้นฉบับ PHP `getSuggestColor` Strategy Type 2
/// 
/// หลักการ (ตรงตาม PHP findWinOnColor + getSuggestColor):
///   - lossCon 0, 1, 2 → suggest = "green" เสมอ (ยืนหยัด CALL)
///   - lossCon >= 3     → เปลี่ยนเล่นตามสีแท่งปัจจุบัน (red→red, green→green)
pub fn get_suggest_color(this_color: &str, loss_con: u32) -> String {
    // Strategy Type 2: Default → suggest green เสมอ
    let mut suggest = "green".to_string();

    // Martingale override: PHP เรียก getSuggestColor เฉพาะเมื่อ lossCon >= 3
    // เมื่อ lossCon >= 3 → เล่นตามสีแท่งปัจจุบัน
    if loss_con >= 3 {
        if this_color == "red" {
            suggest = "red".to_string();
        } else {
            suggest = "green".to_string();
        }
    }

    suggest
}

/// ฟังก์ชันหลักสำหรับหาจุดเข้าเทรด — เลียนแบบจาก `clsATRCandleAnalyzer.php`
///
/// เงื่อนไขเข้าเทรด: 
///   1) แท่ง Spike เกิดขึ้น (is_atr = true)
///   2) ออก suggest color ตามกลยุทธ์ PHP
///   3) แปลง suggest color → Call/Put
pub fn get_trade_action(analysis: &AnalysisObject, loss_con: u32) -> TradeAction {
    // ถ้าแท่งนี้ไม่ใช่ ATR Spike → ไม่เทรด รอต่อ
    if !analysis.is_atr {
        return TradeAction::Wait;
    }

    // เกิด Spike แล้ว → หา suggest color (ตรงตาม PHP getSuggestColor)
    let suggest = get_suggest_color(&analysis.this_color, loss_con);

    println!("🎯 Spike detected! thisColor={}, lossCon={}, suggest={}", 
        analysis.this_color, loss_con, suggest);

    // แปลง suggest color → Trade Action
    match suggest.as_str() {
        "green" => TradeAction::Call,
        "red" => TradeAction::Put,
        _ => TradeAction::Wait,
    }
}


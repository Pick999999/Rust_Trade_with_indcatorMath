//! pkDetectTrend - Trend Detection Engine V5 translated from detectTrend_V5.js
//!
//! ตรวจจับแนวโน้มและรูปแบบแท่งเทียน (24 รูปแบบ) พร้อมวิเคราะห์:
//! 1. Price Action Structure (Higher High, Lower Low, Higher Low, Lower High)
//! 2. Whipsaw Zone Analysis (Color Alternation Sequence 3-4 bars)
//! 3. Trend Strength Score (-100 ถึง +100) พร้อม Score Breakdown 7 มิติ
//! 4. Engulfing / Outside Bar Patterns
//! 5. Spike (ATR Volatility) Rejections & Continuations
//!
//! ออกแบบให้ทำงานแบบ zero-allocation / high-performance สำหรับ `full_analysis_ver2.rs`

use serde::{Deserialize, Serialize};

/// Trait สำหรับโครงสร้างแท่งเทียนที่นำมาตรวจจับ Trend
pub trait TrendCandle {
    fn open(&self) -> f64;
    fn high(&self) -> f64;
    fn low(&self) -> f64;
    fn close(&self) -> f64;
    fn volume(&self) -> Option<f64> {
        None
    }
    fn is_atr(&self) -> bool {
        false
    }
}

/// โครงสร้างข้อมูลแท่งเทียนพื้นฐานสำหรับใช้งานกับ `pkDetectTrend`
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CandleInput {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    #[serde(default)]
    pub volume: Option<f64>,
    #[serde(default)]
    pub is_atr: bool,
}

impl TrendCandle for CandleInput {
    #[inline(always)]
    fn open(&self) -> f64 {
        self.open
    }
    #[inline(always)]
    fn high(&self) -> f64 {
        self.high
    }
    #[inline(always)]
    fn low(&self) -> f64 {
        self.low
    }
    #[inline(always)]
    fn close(&self) -> f64 {
        self.close
    }
    #[inline(always)]
    fn volume(&self) -> Option<f64> {
        self.volume
    }
    #[inline(always)]
    fn is_atr(&self) -> bool {
        self.is_atr
    }
}

impl<T: TrendCandle> TrendCandle for &T {
    #[inline(always)]
    fn open(&self) -> f64 {
        (*self).open()
    }
    #[inline(always)]
    fn high(&self) -> f64 {
        (*self).high()
    }
    #[inline(always)]
    fn low(&self) -> f64 {
        (*self).low()
    }
    #[inline(always)]
    fn close(&self) -> f64 {
        (*self).close()
    }
    #[inline(always)]
    fn volume(&self) -> Option<f64> {
        (*self).volume()
    }
    #[inline(always)]
    fn is_atr(&self) -> bool {
        (*self).is_atr()
    }
}

// รองรับ tuple (open, high, low, close)
impl TrendCandle for (f64, f64, f64, f64) {
    #[inline(always)]
    fn open(&self) -> f64 {
        self.0
    }
    #[inline(always)]
    fn high(&self) -> f64 {
        self.1
    }
    #[inline(always)]
    fn low(&self) -> f64 {
        self.2
    }
    #[inline(always)]
    fn close(&self) -> f64 {
        self.3
    }
}

// รองรับ tuple (open, high, low, close, is_atr)
impl TrendCandle for (f64, f64, f64, f64, bool) {
    #[inline(always)]
    fn open(&self) -> f64 {
        self.0
    }
    #[inline(always)]
    fn high(&self) -> f64 {
        self.1
    }
    #[inline(always)]
    fn low(&self) -> f64 {
        self.2
    }
    #[inline(always)]
    fn close(&self) -> f64 {
        self.3
    }
    #[inline(always)]
    fn is_atr(&self) -> bool {
        self.4
    }
}

// รองรับ turbo_indicators::OHLCV
impl TrendCandle for turbo_indicators::OHLCV {
    #[inline(always)]
    fn open(&self) -> f64 {
        self.open
    }
    #[inline(always)]
    fn high(&self) -> f64 {
        self.high
    }
    #[inline(always)]
    fn low(&self) -> f64 {
        self.low
    }
    #[inline(always)]
    fn close(&self) -> f64 {
        self.close
    }
    #[inline(always)]
    fn volume(&self) -> Option<f64> {
        Some(self.volume)
    }
}

// รองรับ RawCandleInput จาก full_analysis_ver2
impl TrendCandle for crate::full_analysis_ver2::RawCandleInput {
    #[inline(always)]
    fn open(&self) -> f64 {
        self.open
    }
    #[inline(always)]
    fn high(&self) -> f64 {
        self.high
    }
    #[inline(always)]
    fn low(&self) -> f64 {
        self.low
    }
    #[inline(always)]
    fn close(&self) -> f64 {
        self.close
    }
}

// รองรับ FullAnalysisResult จาก full_analysis_ver2
impl TrendCandle for crate::full_analysis_ver2::FullAnalysisResult {
    #[inline(always)]
    fn open(&self) -> f64 {
        self.open
    }
    #[inline(always)]
    fn high(&self) -> f64 {
        self.high
    }
    #[inline(always)]
    fn low(&self) -> f64 {
        self.low
    }
    #[inline(always)]
    fn close(&self) -> f64 {
        self.close
    }
    #[inline(always)]
    fn is_atr(&self) -> bool {
        self.is_atr
    }
}

/// ===============================================
/// TREND_CASES: Registry รวมทุก Case ใน V5 (24 Cases)
/// ===============================================
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrendCase {
    // ---------- System ----------
    InsufficientData,

    // ---------- Up / Down ยืนยันแล้ว ----------
    ConfirmedUp,
    ConfirmedDown,

    // ---------- Rejected (แท่งปกติ) ----------
    StrongBullTrap,
    WeakUpperWick,
    FailedFollowthroughUp,
    StrongBearTrap,
    WeakLowerWick,
    FailedFollowthroughDown,

    // ---------- Sideways (แท่งปกติ) ----------
    InsideBar,
    DojiIndecision,
    MixedSignal,
    MixedSignalDown,
    FlatResistanceTest,
    FlatSupportTest,
    NoClearStructure,

    // ---------- Engulfing / Outside Bar (แท่งปกติ) ----------
    BullishEngulfing,
    BearishEngulfing,
    EngulfingIndecision,

    // ---------- Spike (is_atr === true) ----------
    SpikeBullTrap,
    SpikeBearTrap,
    SpikeContinuationUp,
    SpikeContinuationDown,
    SpikeHesitantUp,
    SpikeHesitantDown,
    SpikeWhipsaw,
    SpikeNoClearDirection,
}

impl TrendCase {
    /// ชื่อ identifier เต็ม (ตรงกับ key ใน JS เช่น 'CONFIRMED_UP')
    pub const fn case_desc(&self) -> &'static str {
        match self {
            Self::InsufficientData => "INSUFFICIENT_DATA",
            Self::ConfirmedUp => "CONFIRMED_UP",
            Self::ConfirmedDown => "CONFIRMED_DOWN",
            Self::StrongBullTrap => "STRONG_BULL_TRAP",
            Self::WeakUpperWick => "WEAK_UPPER_WICK",
            Self::FailedFollowthroughUp => "FAILED_FOLLOWTHROUGH_UP",
            Self::StrongBearTrap => "STRONG_BEAR_TRAP",
            Self::WeakLowerWick => "WEAK_LOWER_WICK",
            Self::FailedFollowthroughDown => "FAILED_FOLLOWTHROUGH_DOWN",
            Self::InsideBar => "INSIDE_BAR",
            Self::DojiIndecision => "DOJI_INDECISION",
            Self::MixedSignal => "MIXED_SIGNAL",
            Self::MixedSignalDown => "MIXED_SIGNAL_DOWN",
            Self::FlatResistanceTest => "FLAT_RESISTANCE_TEST",
            Self::FlatSupportTest => "FLAT_SUPPORT_TEST",
            Self::NoClearStructure => "NO_CLEAR_STRUCTURE",
            Self::BullishEngulfing => "BULLISH_ENGULFING",
            Self::BearishEngulfing => "BEARISH_ENGULFING",
            Self::EngulfingIndecision => "ENGULFING_INDECISION",
            Self::SpikeBullTrap => "SPIKE_BULL_TRAP",
            Self::SpikeBearTrap => "SPIKE_BEAR_TRAP",
            Self::SpikeContinuationUp => "SPIKE_CONTINUATION_UP",
            Self::SpikeContinuationDown => "SPIKE_CONTINUATION_DOWN",
            Self::SpikeHesitantUp => "SPIKE_HESITANT_UP",
            Self::SpikeHesitantDown => "SPIKE_HESITANT_DOWN",
            Self::SpikeWhipsaw => "SPIKE_WHIPSAW",
            Self::SpikeNoClearDirection => "SPIKE_NO_CLEAR_DIRECTION",
        }
    }

    /// รหัสสั้น สื่อความหมาย รูปแบบ <GROUP>-<MEANING> เช่น 'UP-CONFIRM'
    pub const fn case_code(&self) -> &'static str {
        match self {
            Self::InsufficientData => "SYS-NODATA",
            Self::ConfirmedUp => "UP-CONFIRM",
            Self::ConfirmedDown => "DN-CONFIRM",
            Self::StrongBullTrap => "RJ-BULLTRAP-STRONG",
            Self::WeakUpperWick => "RJ-UPWICK-WEAK",
            Self::FailedFollowthroughUp => "RJ-FOLLOWFAIL-UP",
            Self::StrongBearTrap => "RJ-BEARTRAP-STRONG",
            Self::WeakLowerWick => "RJ-LOWWICK-WEAK",
            Self::FailedFollowthroughDown => "RJ-FOLLOWFAIL-DN",
            Self::InsideBar => "SD-INSIDEBAR",
            Self::DojiIndecision => "SD-DOJI",
            Self::MixedSignal => "SD-MIXEDSIGNAL",
            Self::MixedSignalDown => "SD-MIXEDSIGNAL-DN",
            Self::FlatResistanceTest => "SD-FLATRESISTANCE",
            Self::FlatSupportTest => "SD-FLATSUPPORT",
            Self::NoClearStructure => "SD-NOSTRUCTURE",
            Self::BullishEngulfing => "EG-BULLISH",
            Self::BearishEngulfing => "EG-BEARISH",
            Self::EngulfingIndecision => "EG-INDECISION",
            Self::SpikeBullTrap => "SPK-BULLTRAP",
            Self::SpikeBearTrap => "SPK-BEARTRAP",
            Self::SpikeContinuationUp => "SPK-CONTINUE-UP",
            Self::SpikeContinuationDown => "SPK-CONTINUE-DN",
            Self::SpikeHesitantUp => "SPK-HESITANT-UP",
            Self::SpikeHesitantDown => "SPK-HESITANT-DN",
            Self::SpikeWhipsaw => "SPK-WHIPSAW",
            Self::SpikeNoClearDirection => "SPK-NODIRECTION",
        }
    }

    /// หมวดใหญ่ ('UpTrend' | 'DownTrend' | 'Rejected' | 'Sideways' | None)
    pub const fn trend(&self) -> Option<&'static str> {
        match self {
            Self::InsufficientData => None,
            Self::ConfirmedUp | Self::SpikeContinuationUp | Self::BullishEngulfing => Some("UpTrend"),
            Self::ConfirmedDown | Self::SpikeContinuationDown | Self::BearishEngulfing => Some("DownTrend"),
            Self::StrongBullTrap
            | Self::WeakUpperWick
            | Self::FailedFollowthroughUp
            | Self::StrongBearTrap
            | Self::WeakLowerWick
            | Self::FailedFollowthroughDown
            | Self::SpikeBullTrap
            | Self::SpikeBearTrap => Some("Rejected"),
            Self::InsideBar
            | Self::DojiIndecision
            | Self::MixedSignal
            | Self::MixedSignalDown
            | Self::FlatResistanceTest
            | Self::FlatSupportTest
            | Self::NoClearStructure
            | Self::EngulfingIndecision
            | Self::SpikeHesitantUp
            | Self::SpikeHesitantDown
            | Self::SpikeWhipsaw
            | Self::SpikeNoClearDirection => Some("Sideways"),
        }
    }

    /// กลุ่มของ case ('Up' | 'Down' | 'Rejected' | 'Sideways' | 'Engulfing' | 'Spike' | 'System')
    pub const fn group(&self) -> &'static str {
        match self {
            Self::InsufficientData => "System",
            Self::ConfirmedUp => "Up",
            Self::ConfirmedDown => "Down",
            Self::StrongBullTrap
            | Self::WeakUpperWick
            | Self::FailedFollowthroughUp
            | Self::StrongBearTrap
            | Self::WeakLowerWick
            | Self::FailedFollowthroughDown => "Rejected",
            Self::InsideBar
            | Self::DojiIndecision
            | Self::MixedSignal
            | Self::MixedSignalDown
            | Self::FlatResistanceTest
            | Self::FlatSupportTest
            | Self::NoClearStructure => "Sideways",
            Self::BullishEngulfing
            | Self::BearishEngulfing
            | Self::EngulfingIndecision => "Engulfing",
            Self::SpikeBullTrap
            | Self::SpikeBearTrap
            | Self::SpikeContinuationUp
            | Self::SpikeContinuationDown
            | Self::SpikeHesitantUp
            | Self::SpikeHesitantDown
            | Self::SpikeWhipsaw
            | Self::SpikeNoClearDirection => "Spike",
        }
    }

    /// คำอธิบายภาษาไทย
    pub const fn description(&self) -> &'static str {
        match self {
            Self::InsufficientData => "ข้อมูลไม่พอสำหรับเปรียบเทียบ ต้องมีอย่างน้อย 3 แท่ง (ปัจจุบัน + ก่อนหน้า 2 แท่ง)",
            Self::ConfirmedUp => "ทำ Higher High ปิดสูงกว่าหรือเท่าแท่งก่อนหน้า และปิดในโซนบนของแท่งตัวเอง ยืนยันแรงซื้อยังควบคุมตลาด",
            Self::ConfirmedDown => "ทำ Lower Low ปิดต่ำกว่าหรือเท่าแท่งก่อนหน้า และปิดในโซนล่างของแท่งตัวเอง ยืนยันแรงขายยังควบคุมตลาด",
            Self::StrongBullTrap => "ทำ Higher High แต่ถูกแรงขายกดลงมาปิดใกล้จุดต่ำสุดของแท่ง และปิดต่ำกว่าแท่งก่อนหน้าด้วย สัญญาณกลับตัวลงที่ชัดเจน (Bull Trap)",
            Self::WeakUpperWick => "ทำ Higher High และยังปิดสูงกว่าแท่งก่อนหน้า แต่มีไส้บนยาวแสดงว่าเจอแรงขายกดในช่วงท้ายแท่ง ควรระวังแรงซื้อเริ่มอ่อนกำลัง",
            Self::FailedFollowthroughUp => "ทำ Higher High แต่ปิดต่ำกว่าแท่งก่อนหน้า แม้ตำแหน่งปิดในแท่งตัวเองจะยังไม่แย่มาก แสดงว่าโมเมนตัมขาขึ้นเริ่มแผ่ว ควรจับตาแท่งถัดไป",
            Self::StrongBearTrap => "ทำ Lower Low แต่ถูกแรงซื้อดันขึ้นมาปิดใกล้จุดสูงสุดของแท่ง และปิดสูงกว่าแท่งก่อนหน้าด้วย สัญญาณกลับตัวขึ้นที่ชัดเจน (Bear Trap)",
            Self::WeakLowerWick => "ทำ Lower Low และยังปิดต่ำกว่าแท่งก่อนหน้า แต่มีไส้ล่างยาวแสดงว่าเจอแรงซื้อดันกลับในช่วงท้ายแท่ง ควรระวังแรงขายเริ่มอ่อนกำลัง",
            Self::FailedFollowthroughDown => "ทำ Lower Low แต่ปิดสูงกว่าแท่งก่อนหน้า แม้ตำแหน่งปิดในแท่งตัวเองจะยังไม่แย่มาก แสดงว่าโมเมนตัมขาลงเริ่มแผ่ว ควรจับตาแท่งถัดไป",
            Self::InsideBar => "แท่งปัจจุบันมี high/low อยู่ภายในกรอบของแท่งก่อนหน้าทั้งหมด (Inside Bar) ตลาดกำลังหดตัว/พักตัว รอการ breakout เพื่อยืนยันทิศทางถัดไป",
            Self::DojiIndecision => "Body ของแท่งเล็กมากเมื่อเทียบกับ range ทั้งหมด (เปิด-ปิดใกล้กัน) แรงซื้อแรงขายสูสีกัน ไม่มีฝ่ายใดชนะชัดเจนในแท่งนี้",
            Self::MixedSignal => "High สูงกว่าแท่งก่อนหน้าแค่ 1 ใน 2 แท่ง (สัญญาณขัดแย้งกันเอง) ประกอบกับ body เล็ก ตลาดกำลังสับสน ยังไม่มีทิศทางชัดเจนพอจะสรุป",
            Self::MixedSignalDown => "Low ต่ำกว่าแท่งก่อนหน้าแค่ 1 ใน 2 แท่ง (สัญญาณขัดแย้งกันเอง) ประกอบกับ body เล็ก ตลาดกำลังสับสนฝั่งขาลง ยังไม่มีทิศทางชัดเจนพอจะสรุป",
            Self::FlatResistanceTest => "High ของแท่งใกล้เคียงกับ high ของแท่งก่อนหน้ามาก แต่ไปต่อไม่ได้ เหมือนราคาชนแนวต้านเดิมซ้ำ (ทดสอบแนวต้าน) ยังไม่มีแรงมากพอจะทะลุ",
            Self::FlatSupportTest => "Low ของแท่งใกล้เคียงกับ low ของแท่งก่อนหน้ามาก แต่ลงต่อไม่ได้ เหมือนราคาชนแนวรับเดิมซ้ำ (ทดสอบแนวรับ/Double Bottom) ยังไม่มีแรงมากพอจะทะลุ",
            Self::NoClearStructure => "ไม่ได้ทำ Higher High หรือ Lower Low ชัดเจนเมื่อเทียบกับ 2 แท่งก่อนหน้า ตลาดยังไม่มีโครงสร้างที่ชัดเจนพอจะสรุปทิศทาง",
            Self::BullishEngulfing => "Outside Bar ที่กลืนกรอบแท่งก่อนหน้า (high สูงกว่า + low ต่ำกว่า) และปิดในโซนบน สัญญาณกลับตัวขึ้นหรือแรงซื้อเข้ามาชัดเจน",
            Self::BearishEngulfing => "Outside Bar ที่กลืนกรอบแท่งก่อนหน้า (high สูงกว่า + low ต่ำกว่า) และปิดในโซนล่าง สัญญาณกลับตัวลงหรือแรงขายเข้ามาชัดเจน",
            Self::EngulfingIndecision => "Outside Bar ที่กลืนกรอบแท่งก่อนหน้า แต่ปิดใกล้กลางแท่ง แรงซื้อแรงขายสู้กันไม่มีฝ่ายใดชนะ ความผันผวนสูงแต่ไม่มีทิศทางชัดเจน",
            Self::SpikeBullTrap => "แท่งนี้เป็น Spike ที่พุ่งขึ้นแรงทำ Higher High แต่ถูกกดลงมาปิดใกล้จุดต่ำสุดของแท่ง คล้ายการล่าสภาพคล่องฝั่งซื้อ (Stop Hunt / Liquidity Grab) ความเสี่ยงกลับตัวลงสูงมาก ต้องระวังเป็นพิเศษ",
            Self::SpikeBearTrap => "แท่งนี้เป็น Spike ที่ทิ่มลงแรงทำ Lower Low แต่ถูกดันกลับขึ้นมาปิดใกล้จุดสูงสุดของแท่ง คล้ายการล่าสภาพคล่องฝั่งขาย (Stop Hunt / Liquidity Grab) ความเสี่ยงกลับตัวขึ้นสูงมาก ต้องระวังเป็นพิเศษ",
            Self::SpikeContinuationUp => "แท่งนี้เป็น Spike ที่พุ่งขึ้นแรงและปิดยืนในโซนบนของแท่ง ไม่โดนกดกลับ มีโอกาสเป็นแรงส่งจริง (Breakout Momentum) ไม่ใช่แค่หลอก ควรติดตามแท่งถัดไปเพื่อยืนยันการไปต่อ",
            Self::SpikeContinuationDown => "แท่งนี้เป็น Spike ที่ทิ่มลงแรงและปิดยืนในโซนล่างของแท่ง ไม่โดนดันกลับ มีโอกาสเป็นแรงส่งจริง (Breakout Momentum) ฝั่งขาย ควรติดตามแท่งถัดไปเพื่อยืนยันการไปต่อ",
            Self::SpikeHesitantUp => "แท่งนี้เป็น Spike ฝั่ง Higher High แต่ปิดในโซนกลางของแท่ง หรือปิดไม่สูงกว่าแท่งก่อนหน้า มีแรงซื้อแต่ยังไม่ยืนยันชัดเจน ควรรอแท่งถัดไปเพื่อยืนยัน",
            Self::SpikeHesitantDown => "แท่งนี้เป็น Spike ฝั่ง Lower Low แต่ปิดในโซนกลางของแท่ง หรือปิดไม่ต่ำกว่าแท่งก่อนหน้า มีแรงขายแต่ยังไม่ยืนยันชัดเจน ควรรอแท่งถัดไปเพื่อยืนยัน",
            Self::SpikeWhipsaw => "แท่งนี้เป็น Spike ที่ range กว้างผิดปกติ (เกิน ATR) และแกว่งครอบคลุมทั้งจุดสูงและจุดต่ำของแท่งก่อนหน้า (Outside Bar) แต่ปิดใกล้กลางแท่ง แสดงถึงความผันผวนรุนแรงแบบไร้ทิศทาง อาจเป็นจุดพลิกผันของตลาด (Exhaustion) ควรรอสัญญาณยืนยันก่อนเข้าเทรด",
            Self::SpikeNoClearDirection => "แท่งนี้เป็น Spike (range กว้างผิดปกติเกิน ATR) แต่ไม่ได้ทำ Higher High / Lower Low ชัดเจน และไม่ได้แกว่งครอบคลุมสองด้าน อาจเกิดจาก noise หรือข่าวกะทันหัน ไม่ควรใช้สัญญาณปกติตัดสินใจกับแท่งนี้ แนะนำรอแท่งถัดไปยืนยัน",
        }
    }
}

/// พารามิเตอร์กำหนด Thresholds สำหรับการตรวจจับ
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TrendDetectOptions {
    pub close_position_threshold: f64,
    pub strong_rejection_threshold: f64,
    pub doji_body_threshold: f64,
    pub flat_tolerance: f64,
}

impl Default for TrendDetectOptions {
    fn default() -> Self {
        Self {
            close_position_threshold: 0.5,
            strong_rejection_threshold: 0.3,
            doji_body_threshold: 0.15,
            flat_tolerance: 0.05,
        }
    }
}

/// ข้อมูลโครงสร้าง High / Low (Structure Metadata)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct StructureMetadata {
    #[serde(rename = "higherHigh")]
    pub higher_high: bool,
    #[serde(rename = "lowerLow")]
    pub lower_low: bool,
    #[serde(rename = "higherLow")]
    pub higher_low: bool,
    #[serde(rename = "lowerHigh")]
    pub lower_high: bool,
}

/// รายละเอียดการแจกแจงคะแนน Trend Score Breakdown
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ScoreBreakdown {
    #[serde(rename = "basePattern")]
    pub base_pattern: i32,
    #[serde(rename = "closeConviction")]
    pub close_conviction: i32,
    #[serde(rename = "bodyConviction")]
    pub body_conviction: i32,
    pub structure: i32,
    #[serde(rename = "rangeExpansion")]
    pub range_expansion: i32,
    #[serde(rename = "trendStreak")]
    pub trend_streak: i32,
    #[serde(rename = "whipsawPenalty")]
    pub whipsaw_penalty: i32,
}

/// ผลการวิเคราะห์ Whipsaw (Color Alternation)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WhipsawAnalysis {
    #[serde(rename = "colorSequence")]
    pub color_sequence: String,
    #[serde(rename = "colorSwitches")]
    pub color_switches: u32,
    #[serde(rename = "isWhipsaw")]
    pub is_whipsaw: bool,
    #[serde(rename = "whipsawStatus")]
    pub whipsaw_status: String,
    #[serde(rename = "whipsawWarning")]
    pub whipsaw_warning: String,
}

/// ผลลัพธ์จากการตรวจจับ Trend ของแต่ละแท่งตาม detectTrend_V5.js
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrendDetectResult {
    /// แนวโน้มหลัก: "UpTrend" | "DownTrend" | "Rejected" | "Sideways" | None
    pub trend: Option<String>,
    /// รหัสสั้น เช่น "UP-CONFIRM", "SPK-BEARTRAP"
    #[serde(rename = "caseCode")]
    pub case_code: String,
    /// ชื่อ Case เต็ม เช่น "CONFIRMED_UP"
    #[serde(rename = "caseDesc")]
    pub case_desc: String,
    /// กลุ่มของ Case เช่น "Up", "Down", "Rejected", "Sideways", "Spike", "Engulfing", "System"
    pub group: String,
    /// คำอธิบายภาษาไทย
    pub description: String,
    /// เป็นแท่ง Spike หรือไม่ (is_atr)
    #[serde(rename = "isSpike")]
    pub is_spike: bool,
    /// Extreme Trend: "UpTrend" | "DownTrend" | "Sideways" | None
    #[serde(rename = "extremeTrend")]
    pub extreme_trend: Option<String>,
    /// ตำแหน่งราคาปิดในแท่งตัวเอง (0.00 - 1.00)
    #[serde(rename = "closePosition")]
    pub close_position: Option<f64>,
    /// สัดส่วน Body ต่อ Range ทั้งหมด (0.00 - 1.00)
    #[serde(rename = "bodyRatio")]
    pub body_ratio: Option<f64>,
    /// ขนาดแท่งเทียบกับค่าเฉลี่ย 2 แท่งก่อนหน้า
    #[serde(rename = "rangeRatio")]
    pub range_ratio: Option<f64>,
    /// โครงสร้าง High/Low
    pub structure: Option<StructureMetadata>,
    /// ลำดับสีแท่งเทียนย้อนหลัง (G=Green, R=Red, D=Doji)
    #[serde(rename = "colorSequence")]
    pub color_sequence: String,
    /// จำนวนครั้งที่สลับสีในช่วง 4 แท่ง
    #[serde(rename = "colorSwitches")]
    pub color_switches: u32,
    /// อยู่ในโซน Whipsaw หรือไม่
    #[serde(rename = "isWhipsaw")]
    pub is_whipsaw: bool,
    /// สถานะ Whipsaw: 'TRENDING' | 'CHOPPY' | 'ENTERING_WHIPSAW' | 'CONFIRMED_WHIPSAW' | 'NO_DATA'
    #[serde(rename = "whipsawStatus")]
    pub whipsaw_status: String,
    /// คำเตือน Whipsaw
    #[serde(rename = "whipsawWarning")]
    pub whipsaw_warning: String,
    /// คะแนนความแรงของเทรนด์ (-100 ถึง +100)
    #[serde(rename = "trendScore")]
    pub trend_score: i32,
    /// ระดับความแข็งแกร่งของเทรนด์
    #[serde(rename = "trendStrength")]
    pub trend_strength: String,
    /// รายละเอียดคะแนนแต่ละด้าน
    #[serde(rename = "scoreBreakdown")]
    pub score_breakdown: ScoreBreakdown,
    /// (Optional) สัดส่วน Volume เทียบแท่งก่อนหน้า
    #[serde(rename = "volumeRatio", skip_serializing_if = "Option::is_none")]
    pub volume_ratio: Option<f64>,
    /// (Optional) Volume สนับสนุนหรือไม่
    #[serde(rename = "volumeConfirmed", skip_serializing_if = "Option::is_none")]
    pub volume_confirmed: Option<bool>,
}

impl Default for TrendDetectResult {
    fn default() -> Self {
        build_result_internal(
            TrendCase::InsufficientData,
            false,
            None,
            None,
            None,
            None,
            None,
            WhipsawAnalysis {
                color_sequence: String::new(),
                color_switches: 0,
                is_whipsaw: false,
                whipsaw_status: "NO_DATA".to_string(),
                whipsaw_warning: "ข้อมูลไม่เพียงพอ".to_string(),
            },
            0,
            "NO_DATA".to_string(),
            ScoreBreakdown::default(),
            None,
            None,
        )
    }
}

#[inline]
fn round2(val: f64) -> f64 {
    (val * 100.0).round() / 100.0
}

/// Helper function ในการประกอบ `TrendDetectResult`
#[allow(clippy::too_many_arguments)]
fn build_result_internal(
    case: TrendCase,
    is_spike: bool,
    extreme_trend: Option<String>,
    close_position: Option<f64>,
    body_ratio: Option<f64>,
    range_ratio: Option<f64>,
    structure: Option<StructureMetadata>,
    whipsaw: WhipsawAnalysis,
    trend_score: i32,
    trend_strength: String,
    score_breakdown: ScoreBreakdown,
    volume_ratio: Option<f64>,
    volume_confirmed: Option<bool>,
) -> TrendDetectResult {
    TrendDetectResult {
        trend: case.trend().map(|s| s.to_string()),
        case_code: case.case_code().to_string(),
        case_desc: case.case_desc().to_string(),
        group: case.group().to_string(),
        description: case.description().to_string(),
        is_spike,
        extreme_trend,
        close_position,
        body_ratio,
        range_ratio,
        structure,
        color_sequence: whipsaw.color_sequence,
        color_switches: whipsaw.color_switches,
        is_whipsaw: whipsaw.is_whipsaw,
        whipsaw_status: whipsaw.whipsaw_status,
        whipsaw_warning: whipsaw.whipsaw_warning,
        trend_score,
        trend_strength,
        score_breakdown,
        volume_ratio,
        volume_confirmed,
    }
}

/// ตรวจจับสีของแท่งเทียนเดี่ยว: 'G' (Green), 'R' (Red), 'D' (Doji)
#[inline]
pub fn get_candle_color<C: TrendCandle>(c: &C) -> char {
    if c.close() > c.open() {
        'G'
    } else if c.close() < c.open() {
        'R'
    } else {
        'D'
    }
}

/// วิเคราะห์การสลับสีของแท่งเทียนย้อนหลัง 3-4 แท่ง เพื่อตรวจจับ Whipsaw Zone
pub fn analyze_color_alternation<C: TrendCandle>(candles: &[C], index: usize) -> WhipsawAnalysis {
    if candles.is_empty() || index == 0 || index >= candles.len() {
        return WhipsawAnalysis {
            color_sequence: String::new(),
            color_switches: 0,
            is_whipsaw: false,
            whipsaw_status: "NO_DATA".to_string(),
            whipsaw_warning: "ข้อมูลไม่เพียงพอ".to_string(),
        };
    }

    let lookback = std::cmp::min(4, index + 1);
    let start_idx = index + 1 - lookback;
    let mut colors: Vec<char> = Vec::with_capacity(lookback);

    for i in start_idx..=index {
        colors.push(get_candle_color(&candles[i]));
    }

    let mut switches = 0u32;
    for i in 1..colors.len() {
        if colors[i - 1] != colors[i] {
            switches += 1;
        }
    }

    let seq_str = colors
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<String>>()
        .join("-");

    let mut whipsaw_status = "TRENDING".to_string();
    let mut is_whipsaw = false;
    let mut whipsaw_warning = format!("✓ โครงสร้างสีต่อเนื่อง ({})", seq_str);

    if colors.len() >= 4 {
        if switches == 3 {
            whipsaw_status = "CONFIRMED_WHIPSAW".to_string();
            is_whipsaw = true;
            whipsaw_warning = format!(
                "⚠️ สลับสี 4 แท่งติด ({}) อยู่ใน Whipsaw Zone รุนแรง (เสี่ยงโดนหลอกฟันปลาสูง)",
                seq_str
            );
        } else if switches == 2 {
            whipsaw_status = "CHOPPY".to_string();
            is_whipsaw = false;
            whipsaw_warning = format!("〰️ ตลาดเริ่มมีความผันผวนสลับสี ({})", seq_str);
        }
    } else if colors.len() == 3 && switches == 2 {
        whipsaw_status = "ENTERING_WHIPSAW".to_string();
        is_whipsaw = true;
        whipsaw_warning = format!(
            "⚡ สลับสี 3 แท่งติด ({}) กำลังเข้าสู่ Whipsaw Zone (ควรระวังการกลับตัวหลอก)",
            seq_str
        );
    }

    // ตรวจสอบกรณี 4 แท่ง แต่ 3 แท่งท้ายสุดสลับสีติดกัน (เช่น G-G-R-G หรือ R-R-G-R)
    if colors.len() == 4 && switches >= 2 {
        let last3_switches = (if colors[1] != colors[2] { 1 } else { 0 })
            + (if colors[2] != colors[3] { 1 } else { 0 });
        if last3_switches == 2 && whipsaw_status != "CONFIRMED_WHIPSAW" {
            let last3_str = colors[1..]
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<String>>()
                .join("-");
            whipsaw_status = "ENTERING_WHIPSAW".to_string();
            is_whipsaw = true;
            whipsaw_warning = format!("⚡ สลับสี 3 แท่งล่าสุด ({}) กำลังเข้าสู่ Whipsaw Zone", last3_str);
        }
    }

    WhipsawAnalysis {
        color_sequence: seq_str,
        color_switches: switches,
        is_whipsaw,
        whipsaw_status,
        whipsaw_warning,
    }
}

/// คำนวณ Strong Trend Score (-100 ถึง +100) และจำแนกความแข็งแกร่ง (Trend Strength)
pub fn calculate_trend_score<C: TrendCandle>(
    candles: &[C],
    index: usize,
    case: TrendCase,
    close_position: f64,
    body_ratio: f64,
    structure: &StructureMetadata,
    range_ratio: f64,
    whipsaw_status: &str,
) -> (i32, String, ScoreBreakdown) {
    if case == TrendCase::InsufficientData || index < 2 || index >= candles.len() {
        return (0, "NO_DATA".to_string(), ScoreBreakdown::default());
    }

    let case_code = case.case_code();
    let trend = case.trend();
    let group = case.group();

    // 1. Base Pattern Score (Max ±35)
    let base_pattern = match case_code {
        "SPK-CONTINUE-UP" => 35,
        "SPK-CONTINUE-DN" => -35,
        "UP-CONFIRM" => 30,
        "DN-CONFIRM" => -30,
        "EG-BULLISH" => 25,
        "EG-BEARISH" => -25,
        "SPK-HESITANT-UP" => 10,
        "SPK-HESITANT-DN" => -10,
        "RJ-BULLTRAP-STRONG" | "SPK-BULLTRAP" => -25,
        "RJ-BEARTRAP-STRONG" | "SPK-BEARTRAP" => 25,
        "RJ-UPWICK-WEAK" | "RJ-FOLLOWFAIL-UP" => -10,
        "RJ-LOWWICK-WEAK" | "RJ-FOLLOWFAIL-DN" => 10,
        _ => 0,
    };

    let is_up = base_pattern > 0 || trend == Some("UpTrend") || group == "Up";
    let is_dn = base_pattern < 0 || trend == Some("DownTrend") || group == "Down";

    // 2. Close Conviction (Max ±15)
    let cp = close_position;
    let close_conviction = if is_up {
        if cp >= 0.85 {
            15
        } else if cp >= 0.65 {
            8
        } else if cp < 0.40 {
            -10
        } else {
            0
        }
    } else if is_dn {
        if cp <= 0.15 {
            -15
        } else if cp <= 0.35 {
            -8
        } else if cp > 0.60 {
            10
        } else {
            0
        }
    } else {
        0
    };

    // 3. Body Conviction (Max ±10)
    let br = body_ratio;
    let body_conviction = if br >= 0.70 {
        if is_up {
            10
        } else if is_dn {
            -10
        } else {
            0
        }
    } else if br >= 0.50 {
        if is_up {
            5
        } else if is_dn {
            -5
        } else {
            0
        }
    } else {
        0
    };

    // 4. Structure Score (Max ±10)
    let mut structure_score = 0;
    if structure.higher_high && structure.higher_low {
        structure_score += 10;
    } else if structure.higher_high && !structure.lower_low {
        structure_score += 5;
    }

    if structure.lower_low && structure.lower_high {
        structure_score -= 10;
    } else if structure.lower_low && !structure.higher_high {
        structure_score -= 5;
    }

    // 5. Range Expansion (Max ±10)
    let rr = range_ratio;
    let tentative_score = base_pattern + close_conviction + body_conviction + structure_score;
    let range_expansion = if rr >= 1.3 {
        if tentative_score > 0 {
            10
        } else if tentative_score < 0 {
            -10
        } else {
            0
        }
    } else if rr >= 1.1 {
        if tentative_score > 0 {
            5
        } else if tentative_score < 0 {
            -5
        } else {
            0
        }
    } else {
        0
    };

    // 6. Trend Streak (Max ±20)
    let mut trend_streak = 0;
    if index >= 3 && candles.len() > index {
        let mut up_streak = 0;
        let mut dn_streak = 0;
        for i in 1..=3 {
            let prev_c = &candles[index - i];
            if index >= i + 1 {
                let prev2_c = &candles[index - i - 1];
                if prev_c.close() > prev2_c.close() && prev_c.high() >= prev2_c.high() {
                    up_streak += 1;
                }
                if prev_c.close() < prev2_c.close() && prev_c.low() <= prev2_c.low() {
                    dn_streak += 1;
                }
            }
        }
        if up_streak == 3 && tentative_score > 0 {
            trend_streak = 20;
        } else if up_streak >= 2 && tentative_score > 0 {
            trend_streak = 10;
        }

        if dn_streak == 3 && tentative_score < 0 {
            trend_streak = -20;
        } else if dn_streak >= 2 && tentative_score < 0 {
            trend_streak = -10;
        }
    }

    // 7. Whipsaw Penalty (หักคะแนนหากพบการสลับสีฟันปลา)
    let whipsaw_penalty = match whipsaw_status {
        "CONFIRMED_WHIPSAW" => {
            if tentative_score > 0 {
                -25
            } else if tentative_score < 0 {
                25
            } else {
                0
            }
        }
        "ENTERING_WHIPSAW" => {
            if tentative_score > 0 {
                -12
            } else if tentative_score < 0 {
                12
            } else {
                0
            }
        }
        _ => 0,
    };

    let raw_score = base_pattern
        + close_conviction
        + body_conviction
        + structure_score
        + range_expansion
        + trend_streak
        + whipsaw_penalty;

    let score = raw_score.clamp(-100, 100);

    // Classification
    let strength = if whipsaw_status == "CONFIRMED_WHIPSAW" {
        "WHIPSAW_ZONE".to_string()
    } else if score >= 70 {
        "ULTRA_STRONG_UP".to_string()
    } else if score >= 40 {
        "STRONG_UP".to_string()
    } else if score >= 20 {
        "MILD_UP".to_string()
    } else if score <= -70 {
        "ULTRA_STRONG_DOWN".to_string()
    } else if score <= -40 {
        "STRONG_DOWN".to_string()
    } else if score <= -20 {
        "MILD_DOWN".to_string()
    } else {
        "NEUTRAL".to_string()
    };

    let breakdown = ScoreBreakdown {
        base_pattern,
        close_conviction,
        body_conviction,
        structure: structure_score,
        range_expansion,
        trend_streak,
        whipsaw_penalty,
    };

    (score, strength, breakdown)
}

/// ตรวจจับ Trend ของแท่งเทียน ณ ตำแหน่ง `index` ตาม detectTrend_V5.js
///
/// # Arguments
/// * `candles` - Slice ของแท่งเทียนที่ implement `TrendCandle`
/// * `index` - ตำแหน่งของแท่งที่ต้องการวิเคราะห์
/// * `options` - ตัวเลือก thresholds (ใช้ `None` เพื่อใช้ค่าเริ่มต้น)
pub fn detect_trend<C: TrendCandle>(
    candles: &[C],
    index: usize,
    options: Option<&TrendDetectOptions>,
) -> TrendDetectResult {
    let default_opts = TrendDetectOptions::default();
    let opts = options.unwrap_or(&default_opts);

    // Guard: ข้อมูลไม่พอ
    if index < 2 || index >= candles.len() {
        let whipsaw = analyze_color_alternation(candles, index);
        return build_result_internal(
            TrendCase::InsufficientData,
            false,
            None,
            None,
            None,
            None,
            None,
            whipsaw,
            0,
            "NO_DATA".to_string(),
            ScoreBreakdown::default(),
            None,
            None,
        );
    }

    let current = &candles[index];
    let prev1 = &candles[index - 1];
    let prev2 = &candles[index - 2];

    let is_spike = current.is_atr();

    // High comparisons
    let higher_high_p1 = current.high() > prev1.high();
    let higher_high_p2 = current.high() > prev2.high();
    let lower_high_p1 = current.high() < prev1.high();

    // Low comparisons (V5 Symmetric)
    let lower_low_p1 = current.low() < prev1.low();
    let lower_low_p2 = current.low() < prev2.low();
    let higher_low_p1 = current.low() > prev1.low();

    // Close comparisons
    let close_higher_p1 = current.close() > prev1.close();
    let close_lower_p1 = current.close() < prev1.close();
    let close_equal_p1 = !close_higher_p1 && !close_lower_p1;

    // Range / Close Position / Body Ratio
    let range = current.high() - current.low();
    let close_position = if range > 0.0 {
        (current.close() - current.low()) / range
    } else {
        0.5
    };
    let body_ratio = if range > 0.0 {
        (current.close() - current.open()).abs() / range
    } else {
        0.0
    };

    // Aggregate flags
    let made_higher_high = higher_high_p1 || higher_high_p2;
    let made_lower_low = lower_low_p1 || lower_low_p2;

    // Pattern flags
    let is_inside_bar = current.high() <= prev1.high() && current.low() >= prev1.low();
    let is_outside_bar = current.high() > prev1.high() && current.low() < prev1.low();

    // Flat detection
    let high_diff_ratio = if prev1.high() != 0.0 {
        (current.high() - prev1.high()).abs() / prev1.high()
    } else {
        0.0
    };
    let is_flat_high = high_diff_ratio <= opts.flat_tolerance && !higher_high_p1;

    let low_diff_ratio = if prev1.low() != 0.0 {
        (current.low() - prev1.low()).abs() / prev1.low()
    } else {
        0.0
    };
    let is_flat_low = low_diff_ratio <= opts.flat_tolerance && !lower_low_p1;

    // Mixed signal
    let mixed_signal_high = higher_high_p1 != higher_high_p2;
    let mixed_signal_low = lower_low_p1 != lower_low_p2;

    // extremeTrend (V5 สมดุล)
    let extreme_trend = if made_higher_high && made_lower_low {
        if close_position >= 0.5 {
            "UpTrend"
        } else {
            "DownTrend"
        }
    } else if made_higher_high && !made_lower_low {
        "UpTrend"
    } else if made_lower_low && !made_higher_high {
        "DownTrend"
    } else {
        "Sideways"
    };

    // Volume confirmation (optional)
    let volume_ratio = match (current.volume(), prev1.volume()) {
        (Some(curr_v), Some(prev_v)) if prev_v > 0.0 => Some(curr_v / prev_v),
        _ => None,
    };
    let volume_confirmed = volume_ratio.map(|vr| vr > 1.0);

    // Relative candle size (rangeRatio)
    let prev1_range = prev1.high() - prev1.low();
    let prev2_range = prev2.high() - prev2.low();
    let avg_prev_range = (prev1_range + prev2_range) / 2.0;
    let range_ratio = if avg_prev_range > 0.0 {
        range / avg_prev_range
    } else {
        1.0
    };

    // Structure metadata
    let structure = StructureMetadata {
        higher_high: made_higher_high,
        lower_low: made_lower_low,
        higher_low: higher_low_p1,
        lower_high: lower_high_p1,
    };

    // Whipsaw Zone Analysis
    let whipsaw_analysis = analyze_color_alternation(candles, index);

    // Decision Logic
    let selected_case = if is_spike {
        // Layer 1: Spike
        if is_outside_bar {
            if close_position > (1.0 - opts.strong_rejection_threshold) {
                TrendCase::SpikeBearTrap
            } else if close_position < opts.strong_rejection_threshold {
                TrendCase::SpikeBullTrap
            } else if close_position >= opts.close_position_threshold {
                TrendCase::SpikeContinuationUp
            } else if close_position <= (1.0 - opts.close_position_threshold) {
                TrendCase::SpikeContinuationDown
            } else {
                TrendCase::SpikeWhipsaw
            }
        } else if made_higher_high && !made_lower_low {
            if close_position < opts.strong_rejection_threshold {
                TrendCase::SpikeBullTrap
            } else if close_position >= opts.close_position_threshold && (close_higher_p1 || close_equal_p1) {
                TrendCase::SpikeContinuationUp
            } else {
                TrendCase::SpikeHesitantUp
            }
        } else if made_lower_low && !made_higher_high {
            if close_position > (1.0 - opts.strong_rejection_threshold) {
                TrendCase::SpikeBearTrap
            } else if close_position <= (1.0 - opts.close_position_threshold) && (close_lower_p1 || close_equal_p1) {
                TrendCase::SpikeContinuationDown
            } else {
                TrendCase::SpikeHesitantDown
            }
        } else {
            TrendCase::SpikeNoClearDirection
        }
    } else if is_outside_bar {
        // Layer 2: Engulfing / Outside Bar (แท่งปกติ)
        if close_position >= opts.close_position_threshold {
            TrendCase::BullishEngulfing
        } else if close_position <= (1.0 - opts.close_position_threshold) {
            TrendCase::BearishEngulfing
        } else {
            TrendCase::EngulfingIndecision
        }
    } else if extreme_trend == "UpTrend" {
        // Layer 3: แท่งปกติ — UpTrend
        if (close_higher_p1 || close_equal_p1) && close_position >= opts.close_position_threshold {
            TrendCase::ConfirmedUp
        } else if !close_higher_p1 && !close_equal_p1 && close_position < opts.strong_rejection_threshold {
            TrendCase::StrongBullTrap
        } else if close_higher_p1 && close_position < opts.close_position_threshold {
            TrendCase::WeakUpperWick
        } else if mixed_signal_high && body_ratio < opts.doji_body_threshold {
            TrendCase::MixedSignal
        } else {
            TrendCase::FailedFollowthroughUp
        }
    } else if extreme_trend == "DownTrend" {
        // Layer 4: แท่งปกติ — DownTrend
        if (close_lower_p1 || close_equal_p1) && close_position <= (1.0 - opts.close_position_threshold) {
            TrendCase::ConfirmedDown
        } else if !close_lower_p1 && !close_equal_p1 && close_position > (1.0 - opts.strong_rejection_threshold) {
            TrendCase::StrongBearTrap
        } else if close_lower_p1 && close_position > (1.0 - opts.close_position_threshold) {
            TrendCase::WeakLowerWick
        } else if mixed_signal_low && body_ratio < opts.doji_body_threshold {
            TrendCase::MixedSignalDown
        } else {
            TrendCase::FailedFollowthroughDown
        }
    } else {
        // Layer 5: Sideways
        if is_inside_bar {
            TrendCase::InsideBar
        } else if body_ratio < opts.doji_body_threshold {
            TrendCase::DojiIndecision
        } else if is_flat_high {
            TrendCase::FlatResistanceTest
        } else if is_flat_low {
            TrendCase::FlatSupportTest
        } else {
            TrendCase::NoClearStructure
        }
    };

    // คำนวณ Trend Score
    let (score, strength, breakdown) = calculate_trend_score(
        candles,
        index,
        selected_case,
        close_position,
        body_ratio,
        &structure,
        range_ratio,
        &whipsaw_analysis.whipsaw_status,
    );

    build_result_internal(
        selected_case,
        is_spike,
        Some(extreme_trend.to_string()),
        Some(round2(close_position)),
        Some(round2(body_ratio)),
        Some(round2(range_ratio)),
        Some(structure),
        whipsaw_analysis,
        score,
        strength,
        breakdown,
        volume_ratio.map(round2),
        volume_confirmed,
    )
}

/// วิเคราะห์ trend ทุกแท่งใน array (Series)
pub fn detect_trend_series<C: TrendCandle>(
    candles: &[C],
    options: Option<&TrendDetectOptions>,
) -> Vec<TrendDetectResult> {
    (0..candles.len())
        .map(|i| detect_trend(candles, i, options))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insufficient_data() {
        let candles = vec![
            CandleInput { open: 100.0, high: 105.0, low: 95.0, close: 102.0, volume: None, is_atr: false },
            CandleInput { open: 102.0, high: 108.0, low: 100.0, close: 106.0, volume: None, is_atr: false },
        ];
        let res0 = detect_trend(&candles, 0, None);
        assert_eq!(res0.case_desc, "INSUFFICIENT_DATA");
        assert_eq!(res0.case_code, "SYS-NODATA");
        assert_eq!(res0.trend, None);
        assert_eq!(res0.trend_score, 0);

        let res1 = detect_trend(&candles, 1, None);
        assert_eq!(res1.case_desc, "INSUFFICIENT_DATA");
    }

    #[test]
    fn test_confirmed_up() {
        let candles = vec![
            CandleInput { open: 100.0, high: 102.0, low: 98.0, close: 101.0, volume: None, is_atr: false },
            CandleInput { open: 101.0, high: 104.0, low: 100.0, close: 103.0, volume: None, is_atr: false },
            CandleInput { open: 103.0, high: 110.0, low: 102.0, close: 109.0, volume: None, is_atr: false },
        ];
        let res = detect_trend(&candles, 2, None);
        assert_eq!(res.case_desc, "CONFIRMED_UP");
        assert_eq!(res.case_code, "UP-CONFIRM");
        assert_eq!(res.trend.as_deref(), Some("UpTrend"));
        assert_eq!(res.group, "Up");
        assert!(res.trend_score > 0);
    }

    #[test]
    fn test_bullish_engulfing() {
        let candles = vec![
            CandleInput { open: 100.0, high: 102.0, low: 98.0, close: 101.0, volume: None, is_atr: false },
            CandleInput { open: 101.0, high: 104.0, low: 100.0, close: 102.0, volume: None, is_atr: false },
            // Outside bar: high 106 > 104, low 99 < 100, close 105 near high
            CandleInput { open: 100.0, high: 106.0, low: 99.0, close: 105.0, volume: None, is_atr: false },
        ];
        let res = detect_trend(&candles, 2, None);
        assert_eq!(res.case_desc, "BULLISH_ENGULFING");
        assert_eq!(res.case_code, "EG-BULLISH");
        assert_eq!(res.trend.as_deref(), Some("UpTrend"));
        assert_eq!(res.group, "Engulfing");
    }

    #[test]
    fn test_whipsaw_detection() {
        let candles = vec![
            CandleInput { open: 100.0, high: 105.0, low: 95.0, close: 103.0, volume: None, is_atr: false }, // G
            CandleInput { open: 103.0, high: 106.0, low: 98.0, close: 100.0, volume: None, is_atr: false }, // R
            CandleInput { open: 100.0, high: 107.0, low: 99.0, close: 105.0, volume: None, is_atr: false }, // G
            CandleInput { open: 105.0, high: 108.0, low: 97.0, close: 101.0, volume: None, is_atr: false }, // R
        ];
        let whipsaw = analyze_color_alternation(&candles, 3);
        assert_eq!(whipsaw.color_sequence, "G-R-G-R");
        assert_eq!(whipsaw.color_switches, 3);
        assert!(whipsaw.is_whipsaw);
        assert_eq!(whipsaw.whipsaw_status, "CONFIRMED_WHIPSAW");
    }
}

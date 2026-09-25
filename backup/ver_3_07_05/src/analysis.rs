// Stateful Analysis Engine — O(1) per tick
// ใช้ SIMD ema_update + Wilder's ATR smoothing แทนการคำนวณ O(n) ทุก tick
//
// Architecture:
//   Batch:  SIMD sum_f64 (AVX2) สำหรับ SMA เริ่มต้น + ema_update ต่อเนื่อง
//   Stream: snap_* = state ก่อนแท่ง live, ema_*/atr = คำนวณ O(1) จาก snap ทุก tick

use turbo_indicators::{OHLCV, StatefulATR, simd, RSI, ChoppinessIndex};
use crate::get_action::AnalysisObject;
use crate::AppConfigPayload;
use chrono::{TimeZone, Local, Datelike, Timelike};

fn get_thai_time(epoch: i64) -> String {
    if let Some(dt) = Local.timestamp_opt(epoch, 0).single() {
        format!("{:02}-{:02}-{:04} {:02}:{:02}:{:02}", 
            dt.day(), dt.month(), dt.year() as i32 + 543, 
            dt.hour(), dt.minute(), dt.second())
    } else {
        String::new()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// SIMD-Optimized Batch EMA
// ────────────────────────────────────────────────────────────────────────────

/// SIMD-accelerated EMA batch computation
/// - ช่วงแรก: ใช้ SIMD sum_f64 (AVX2 4x f64) สำหรับ SMA เริ่มต้น
/// - ช่วงหลัง: ใช้ SIMD ema_update (inline zero-overhead)
fn compute_ema_simd(prices: &[f64], period: usize) -> Vec<f64> {
    let mut ema_values = vec![0.0; prices.len()];
    if prices.len() < period || period == 0 {
        return ema_values;
    }

    let alpha = 2.0 / (period + 1) as f64;
    
    // Initial SMA — SIMD-accelerated sum (AVX2 when available)
    let initial_sum = simd::sum_f64(&prices[..period]);
    let mut current_ema = initial_sum / period as f64;
    ema_values[period - 1] = current_ema;

    // EMA — SIMD-optimized multiply-add (inline zero-overhead)
    for i in period..prices.len() {
        current_ema = simd::ema_update(current_ema, prices[i], alpha);
        ema_values[i] = current_ema;
    }

    ema_values
}

// ────────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────────

/// Compute True Range สำหรับ ATR (inline zero overhead)
#[inline(always)]
fn true_range(candle: &OHLCV, prev_close: f64) -> f64 {
    let hl = candle.high - candle.low;
    let hc = (candle.high - prev_close).abs();
    let lc = (candle.low - prev_close).abs();
    hl.max(hc).max(lc)
}

fn right_align(vals: Vec<f64>, n: usize) -> Vec<f64> {
    let mut res = vec![0.0; n];
    if vals.len() <= n {
        let offset = n - vals.len();
        for i in 0..vals.len() {
            res[offset + i] = vals[i];
        }
    }
    res
}

fn compute_adx(ohlcv: &[OHLCV], period: usize) -> Vec<f64> {
    let n = ohlcv.len();
    let mut adx_vals = vec![0.0; n];
    if n < period * 2 || period == 0 { return adx_vals; }

    let mut tr_sm = 0.0;
    let mut pdm_sm = 0.0;
    let mut ndm_sm = 0.0;
    
    // Warmup True Range and Directional Movement
    for i in 1..=period {
        let tr = true_range(&ohlcv[i], ohlcv[i-1].close);
        let up = ohlcv[i].high - ohlcv[i-1].high;
        let down = ohlcv[i-1].low - ohlcv[i].low;
        let mut pdm = 0.0;
        let mut ndm = 0.0;
        if up > down && up > 0.0 { pdm = up; }
        if down > up && down > 0.0 { ndm = down; }
        tr_sm += tr;
        pdm_sm += pdm;
        ndm_sm += ndm;
    }
    
    let mut dx_vals = vec![0.0; n];
    for i in period..n {
        if i > period {
            let tr = true_range(&ohlcv[i], ohlcv[i-1].close);
            let up = ohlcv[i].high - ohlcv[i-1].high;
            let down = ohlcv[i-1].low - ohlcv[i].low;
            let mut pdm = 0.0;
            let mut ndm = 0.0;
            if up > down && up > 0.0 { pdm = up; }
            if down > up && down > 0.0 { ndm = down; }
            
            // Wilder's smoothing
            tr_sm = tr_sm - (tr_sm / period as f64) + tr;
            pdm_sm = pdm_sm - (pdm_sm / period as f64) + pdm;
            ndm_sm = ndm_sm - (ndm_sm / period as f64) + ndm;
        }
        
        let mut pdi = 0.0;
        let mut ndi = 0.0;
        if tr_sm > 0.0 {
            pdi = 100.0 * pdm_sm / tr_sm;
            ndi = 100.0 * ndm_sm / tr_sm;
        }
        let diff = (pdi - ndi).abs();
        let sum = pdi + ndi;
        let dx = if sum > 0.0 { 100.0 * diff / sum } else { 0.0 };
        dx_vals[i] = dx;
    }
    
    // Initial ADX
    let mut adx = 0.0;
    for i in period..period * 2 {
        adx += dx_vals[i];
    }
    adx /= period as f64;
    adx_vals[period * 2 - 1] = adx;
    
    // Smoothed ADX
    for i in period * 2..n {
        adx = (adx * (period as f64 - 1.0) + dx_vals[i]) / period as f64;
        adx_vals[i] = adx;
    }
    
    adx_vals
}

// ────────────────────────────────────────────────────────────────────────────
// Stateful Analysis Engine
// ────────────────────────────────────────────────────────────────────────────

/// Stateful Analysis Engine — O(1) per tick
///
/// แทนที่จะคำนวณ EMA/ATR ใหม่ทั้ง array ทุก tick (O(n)),
/// engine นี้จำ state ไว้แล้วคำนวณเฉพาะแท่งปัจจุบัน (O(1)) จาก snapshot
///
/// ```text
/// ┌─────────────────────────────────────────────┐
/// │  Completed candles    │ Live candle          │
/// │  snap_ema_* = state   │ ema_* = computed     │
/// │  snap_atr             │ current_atr          │
/// │  snap_close           │ live_close           │
/// └───────────────────────┴──────────────────────┘
///         ↑                        ↑
///   Fixed until new candle    Recomputed every tick
/// ```
pub struct AnalysisEngine {
    // ── Snapshot: state หลัง candle ที่ complete แล้ว (immutable ระหว่าง live candle) ──
    snap_ema_short: f64,
    snap_ema_medium: f64,
    snap_ema_long: f64,
    snap_atr: f64,
    snap_close: f64,

    // ── Live candle computed values (recomputed every tick) ──
    pub ema_short: f64,
    pub ema_medium: f64,
    pub ema_long: f64,
    pub current_atr: f64,
    live_close: f64,

    // ── SIMD-optimized EMA alphas ──
    alpha_short: f64,
    alpha_medium: f64,
    alpha_long: f64,

    // ── ATR config ──
    atr_alpha: f64,

    // ── Tracking ──
    current_epoch: i64,
    total_candles: usize,

    // ── Indicator Config ──
    smc_period: usize,
    ci_period: usize,
    adx_period: usize,

    // ── Buffer for non-O(1) indicators ──
    recent_candles: Vec<OHLCV>,
}

impl AnalysisEngine {
    /// สร้าง engine ใหม่จาก config
    pub fn new(config: &AppConfigPayload) -> Self {
        let sp = config.ema.short.period.max(1) as usize;
        let mp = config.ema.medium.period.max(1) as usize;
        let lp = config.ema.long.period.max(1) as usize;
        let ap = config.indicators.atr_period.max(1) as usize;
        let smc = config.indicators.smc_period.max(1) as usize;
        let ci = config.indicators.ci_period.max(1) as usize;
        let adx = config.indicators.adx_period.max(1) as usize;

        Self {
            snap_ema_short: 0.0, snap_ema_medium: 0.0, snap_ema_long: 0.0,
            snap_atr: 0.0, snap_close: 0.0,
            ema_short: 0.0, ema_medium: 0.0, ema_long: 0.0,
            current_atr: 0.0, live_close: 0.0,
            alpha_short: 2.0 / (sp + 1) as f64,
            alpha_medium: 2.0 / (mp + 1) as f64,
            alpha_long: 2.0 / (lp + 1) as f64,
            atr_alpha: 1.0 / ap as f64,
            current_epoch: 0,
            total_candles: 0,
            smc_period: smc,
            ci_period: ci,
            adx_period: adx,
            recent_candles: Vec::new(),
        }
    }

    /// Batch: ประมวลผลข้อมูลย้อนหลังทั้งหมด
    /// - SIMD EMA (AVX2 sum + inline ema_update)
    /// - StatefulATR (Wilder's smoothing O(1) per candle)
    ///
    /// Returns Vec<AnalysisObject> สำหรับ frontend chart
    /// + บันทึก state ภายใน engine สำหรับ incremental update_tick()
    pub fn process_history(&mut self, ohlcv: &[OHLCV], config: &AppConfigPayload, asset_name: &str) -> Vec<AnalysisObject> {
        let n = ohlcv.len();
        if n == 0 { return vec![]; }

        let closes: Vec<f64> = ohlcv.iter().map(|c| c.close).collect();

        // ── Batch EMA — SIMD-optimized ──
        let ema_s = compute_ema_simd(&closes, config.ema.short.period.max(1) as usize);
        let ema_m = compute_ema_simd(&closes, config.ema.medium.period.max(1) as usize);
        let ema_l = compute_ema_simd(&closes, config.ema.long.period.max(1) as usize);

        // ── Batch ATR — StatefulATR (O(1) Wilder's smoothing per candle) ──
        let mut stateful_atr = StatefulATR::new(config.indicators.atr_period.max(1) as usize);
        let mut atr_vals = vec![0.0; n];
        for (i, candle) in ohlcv.iter().enumerate() {
            if let Some(atr) = stateful_atr.update(candle) {
                atr_vals[i] = atr;
            }
        }

        // ── Build AnalysisObject array ──
        let atr_multi = config.indicators.atr_multi;
        let mut results = Vec::with_capacity(n);

        let rsi_vals = right_align(RSI::new(self.smc_period).compute(&closes), n);
        let ci_vals = right_align(ChoppinessIndex::new(self.ci_period).compute(ohlcv), n);
        let adx_vals = compute_adx(ohlcv, self.adx_period);

        for i in 0..n {
            let c = &ohlcv[i];
            let this_color = if c.close > c.open { "green" } else { "red" };
            let ema_s_dir = if i > 0 && ema_s[i] >= ema_s[i - 1] { "Up" } else { "Down" };
            let ema_m_dir = if i > 0 && ema_m[i] >= ema_m[i - 1] { "Up" } else { "Down" };
            let ema_l_dir = if i > 0 && ema_l[i] >= ema_l[i - 1] { "Up" } else { "Down" };
            let ema_above = if ema_s[i] >= ema_m[i] { "Short" } else { "Medium" };
            let range = c.high - c.low;
            let is_atr = atr_vals[i] > 0.0 && range > atr_vals[i] * atr_multi;

            results.push(AnalysisObject {
                epoch: c.timestamp,
                asset: asset_name.to_string(),
                time_display: get_thai_time(c.timestamp),
                open: c.open,
                high: c.high,
                low: c.low,
                close: c.close,
                is_atr,
                this_color: this_color.to_string(),
                ema_short_direction: ema_s_dir.to_string(),
                ema_medium_direction: ema_m_dir.to_string(),
                ema_short_val: ema_s[i],
                ema_medium_val: ema_m[i],
                ema_long_val: ema_l[i],
                ema_above: ema_above.to_string(),
                ema_long_direction: ema_l_dir.to_string(),
                atr_value: atr_vals[i],
                choppiness_index: ci_vals[i],
                adx: adx_vals[i],
                rsi: rsi_vals[i],
            });
        }

        // ── บันทึก snapshot state สำหรับ incremental updates ──
        if n >= 2 {
            self.snap_ema_short = ema_s[n - 2];
            self.snap_ema_medium = ema_m[n - 2];
            self.snap_ema_long = ema_l[n - 2];
            self.snap_atr = atr_vals[n - 2];
            self.snap_close = ohlcv[n - 2].close;
        }
        self.ema_short = ema_s[n - 1];
        self.ema_medium = ema_m[n - 1];
        self.ema_long = ema_l[n - 1];
        self.current_atr = atr_vals[n - 1];
        self.live_close = ohlcv[n - 1].close;
        self.current_epoch = ohlcv[n - 1].timestamp;
        self.total_candles = n;
        // Save history for streaming
        self.recent_candles = ohlcv[ohlcv.len().saturating_sub(250)..].to_vec();

        results
    }

    /// Incremental: O(1) per tick ด้วย SIMD ema_update + Wilder's ATR
    ///
    /// - `candle`: OHLCV ของแท่งปัจจุบัน (aggregated high/low/close จาก deriv.rs)
    /// - `is_new_candle`: true เมื่อ epoch เปลี่ยน (แท่งใหม่เกิดขึ้น)
    /// - `atr_multi`: ATR multiplier จาก config (รองรับเปลี่ยนกลางคัน)
    pub fn update_tick(&mut self, candle: &OHLCV, is_new_candle: bool, atr_multi: f64, asset_name: &str) -> AnalysisObject {
        if is_new_candle {
            // แท่งก่อนหน้า complete → เลื่อน snapshot ไปข้างหน้า
            self.snap_ema_short = self.ema_short;
            self.snap_ema_medium = self.ema_medium;
            self.snap_ema_long = self.ema_long;
            self.snap_atr = self.current_atr;
            self.snap_close = self.live_close;
            self.total_candles += 1;
            
            if self.recent_candles.len() >= 250 {
                self.recent_candles.remove(0);
            }
            self.recent_candles.push(*candle);
        } else {
            if let Some(last) = self.recent_candles.last_mut() {
                *last = *candle;
            } else {
                self.recent_candles.push(*candle);
            }
        }

        // ── EMA: SIMD-optimized O(1) update ──
        // simd::ema_update = alpha * new + (1 - alpha) * prev — inline zero-overhead
        self.ema_short = simd::ema_update(self.snap_ema_short, candle.close, self.alpha_short);
        self.ema_medium = simd::ema_update(self.snap_ema_medium, candle.close, self.alpha_medium);
        self.ema_long = simd::ema_update(self.snap_ema_long, candle.close, self.alpha_long);

        // ── ATR: Wilder's smoothing O(1) ──
        // new_atr = prev_atr + alpha * (TR - prev_atr)
        if self.snap_atr > 0.0 && self.snap_close > 0.0 {
            let tr = true_range(candle, self.snap_close);
            self.current_atr = self.snap_atr + self.atr_alpha * (tr - self.snap_atr);
        }

        self.live_close = candle.close;
        self.current_epoch = candle.timestamp;

        // ── Build AnalysisObject ──
        let this_color = if candle.close > candle.open { "green" } else { "red" };
        let ema_s_dir = if self.ema_short >= self.snap_ema_short { "Up" } else { "Down" };
        let ema_m_dir = if self.ema_medium >= self.snap_ema_medium { "Up" } else { "Down" };
        let ema_l_dir = if self.ema_long >= self.snap_ema_long { "Up" } else { "Down" };
        let ema_above = if self.ema_short >= self.ema_medium { "Short" } else { "Medium" };
        let range = candle.high - candle.low;
        let is_atr = self.current_atr > 0.0 && range > self.current_atr * atr_multi;

        let closes: Vec<f64> = self.recent_candles.iter().map(|c| c.close).collect();
        let rsi_val = right_align(RSI::new(self.smc_period).compute(&closes), self.recent_candles.len()).last().copied().unwrap_or(0.0);
        let ci_val = right_align(ChoppinessIndex::new(self.ci_period).compute(&self.recent_candles), self.recent_candles.len()).last().copied().unwrap_or(0.0);
        let adx_val = compute_adx(&self.recent_candles, self.adx_period).last().copied().unwrap_or(0.0);

        AnalysisObject {
            epoch: candle.timestamp,
            asset: asset_name.to_string(),
            time_display: get_thai_time(candle.timestamp),
            open: candle.open,
            high: candle.high,
            low: candle.low,
            close: candle.close,
            is_atr,
            this_color: this_color.to_string(),
            ema_short_direction: ema_s_dir.to_string(),
            ema_medium_direction: ema_m_dir.to_string(),
            ema_short_val: self.ema_short,
            ema_medium_val: self.ema_medium,
            ema_long_val: self.ema_long,
            ema_above: ema_above.to_string(),
            ema_long_direction: ema_l_dir.to_string(),
            atr_value: self.current_atr,
            choppiness_index: ci_val,
            adx: adx_val,
            rsi: rsi_val,
        }
    }
}

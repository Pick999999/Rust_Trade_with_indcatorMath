// Core Technical Indicators - Cache-Optimized Implementation
use crate::{OHLCV, IndicatorResult};
use crate::simd::{mean, ema_update, weighted_sum_f64};

/// Hull Moving Average (HMA)
/// Formula: HMA = WMA(2*WMA(n/2) - WMA(n), sqrt(n))
pub struct HMA {
    pub period: usize,
    half_period: usize,
    sqrt_period: usize,
}

impl HMA {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            half_period: period / 2,
            sqrt_period: (period as f64).sqrt() as usize,
        }
    }

    /// Compute HMA on price slice (zero allocation in hot path)
    pub fn compute(&self, prices: &[f64]) -> IndicatorResult {
        let len = prices.len();
        let mut result = IndicatorResult::with_capacity(len);

        if len < self.period {
            return result;
        }

        // Pre-compute WMA weights (cache-friendly)
        let wma_full_weights = self.wma_weights(self.period);
        let wma_half_weights = self.wma_weights(self.half_period);
        let wma_sqrt_weights = self.wma_weights(self.sqrt_period);

        let mut raw_hma_buffer = Vec::with_capacity(len);

        // First pass: compute 2*WMA(n/2) - WMA(n)
        for i in self.period - 1..len {
            let wma_full = weighted_sum_f64(
                &prices[i + 1 - self.period..=i],
                &wma_full_weights
            );
            
            let wma_half = weighted_sum_f64(
                &prices[i + 1 - self.half_period..=i],
                &wma_half_weights
            );
            
            raw_hma_buffer.push(2.0 * wma_half - wma_full);
        }

        // Second pass: WMA on raw HMA values
        for i in self.sqrt_period - 1..raw_hma_buffer.len() {
            let hma = weighted_sum_f64(
                &raw_hma_buffer[i + 1 - self.sqrt_period..=i],
                &wma_sqrt_weights
            );
            result.values.push(hma);
        }

        result
    }

    /// WMA weights (normalized)
    #[inline]
    fn wma_weights(&self, period: usize) -> Vec<f64> {
        let sum: f64 = (1..=period).sum::<usize>() as f64;
        (1..=period)
            .map(|w| w as f64 / sum)
            .collect()
    }
}

/// MACD (Moving Average Convergence Divergence)
pub struct MACD {
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    fast_alpha: f64,
    slow_alpha: f64,
    signal_alpha: f64,
}

impl MACD {
    pub fn new(fast: usize, slow: usize, signal: usize) -> Self {
        Self {
            fast_period: fast,
            slow_period: slow,
            signal_period: signal,
            fast_alpha: 2.0 / (fast + 1) as f64,
            slow_alpha: 2.0 / (slow + 1) as f64,
            signal_alpha: 2.0 / (signal + 1) as f64,
        }
    }

    /// Returns (MACD line, Signal line, Histogram)
    pub fn compute(&self, prices: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let len = prices.len();
        
        if len < self.slow_period {
            return (vec![], vec![], vec![]);
        }

        let mut macd_line = Vec::with_capacity(len);
        let mut signal_line = Vec::with_capacity(len);
        let mut histogram = Vec::with_capacity(len);

        // Initialize EMAs with SMA
        let mut fast_ema = mean(&prices[..self.fast_period]);
        let mut slow_ema = mean(&prices[..self.slow_period]);

        // Warm up period
        for i in self.slow_period..len {
            fast_ema = ema_update(fast_ema, prices[i], self.fast_alpha);
            slow_ema = ema_update(slow_ema, prices[i], self.slow_alpha);
            
            let macd = fast_ema - slow_ema;
            macd_line.push(macd);
        }

        // Signal line (EMA of MACD)
        if macd_line.len() >= self.signal_period {
            let mut signal_ema = mean(&macd_line[..self.signal_period]);
            
            for i in self.signal_period - 1..macd_line.len() {
                signal_ema = ema_update(signal_ema, macd_line[i], self.signal_alpha);
                signal_line.push(signal_ema);
                histogram.push(macd_line[i] - signal_ema);
            }
        }

        (macd_line, signal_line, histogram)
    }
}

/// RSI (Relative Strength Index)
pub struct RSI {
    period: usize,
    alpha: f64,
}

impl RSI {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            alpha: 1.0 / period as f64,
        }
    }

    pub fn compute(&self, prices: &[f64]) -> Vec<f64> {
        let len = prices.len();
        
        if len < self.period + 1 {
            return vec![];
        }

        let mut rsi_values = Vec::with_capacity(len);

        // Initial average gain/loss
        let mut avg_gain = 0.0;
        let mut avg_loss = 0.0;

        for i in 1..=self.period {
            let change = prices[i] - prices[i - 1];
            if change > 0.0 {
                avg_gain += change;
            } else {
                avg_loss += -change;
            }
        }

        avg_gain /= self.period as f64;
        avg_loss /= self.period as f64;

        // Wilder's smoothing
        for i in self.period + 1..len {
            let change = prices[i] - prices[i - 1];
            
            let gain = if change > 0.0 { change } else { 0.0 };
            let loss = if change < 0.0 { -change } else { 0.0 };

            avg_gain = ema_update(avg_gain, gain, self.alpha);
            avg_loss = ema_update(avg_loss, loss, self.alpha);

            let rs = if avg_loss == 0.0 { 100.0 } else { avg_gain / avg_loss };
            let rsi = 100.0 - (100.0 / (1.0 + rs));
            
            rsi_values.push(rsi);
        }

        rsi_values
    }
}

/// Choppiness Index
pub struct ChoppinessIndex {
    period: usize,
}

impl ChoppinessIndex {
    pub fn new(period: usize) -> Self {
        Self { period }
    }

    pub fn compute(&self, ohlcv: &[OHLCV]) -> Vec<f64> {
        let len = ohlcv.len();
        
        if len < self.period {
            return vec![];
        }

        let mut choppy_values = Vec::with_capacity(len - self.period + 1);
        let log_period = (self.period as f64).ln();

        for i in self.period..=len {
            let window = &ohlcv[i - self.period..i];
            
            // True Range sum
            let mut atr_sum = 0.0;
            for j in 1..window.len() {
                let high_low = window[j].high - window[j].low;
                let high_close = (window[j].high - window[j - 1].close).abs();
                let low_close = (window[j].low - window[j - 1].close).abs();
                
                atr_sum += high_low.max(high_close).max(low_close);
            }

            // High-Low range
            let high = window.iter().map(|c| c.high).fold(f64::NEG_INFINITY, f64::max);
            let low = window.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
            let range = high - low;

            let choppy = if range > 0.0 {
                100.0 * (atr_sum / range).ln() / log_period
            } else {
                0.0
            };

            choppy_values.push(choppy);
        }

        choppy_values
    }
}

/// ATR (Average True Range) - Wilder's Smoothing
pub struct ATR {
    period: usize,
    alpha: f64,
}

impl ATR {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            alpha: 1.0 / period as f64,
        }
    }

    /// Compute True Range for a single candle
    #[inline]
    fn true_range(candle: &OHLCV, prev_close: f64) -> f64 {
        let high_low = candle.high - candle.low;
        let high_close = (candle.high - prev_close).abs();
        let low_close = (candle.low - prev_close).abs();
        high_low.max(high_close).max(low_close)
    }

    /// Batch compute ATR on OHLCV slice
    pub fn compute(&self, ohlcv: &[OHLCV]) -> Vec<f64> {
        let len = ohlcv.len();

        if len < self.period + 1 {
            return vec![];
        }

        let mut atr_values = Vec::with_capacity(len - self.period);

        // Initial ATR = SMA of first `period` True Ranges
        let mut tr_sum = 0.0;
        for i in 1..=self.period {
            tr_sum += Self::true_range(&ohlcv[i], ohlcv[i - 1].close);
        }
        let mut atr = tr_sum / self.period as f64;
        atr_values.push(atr);

        // Wilder's smoothing for subsequent values
        for i in self.period + 1..len {
            let tr = Self::true_range(&ohlcv[i], ohlcv[i - 1].close);
            atr = ema_update(atr, tr, self.alpha);
            atr_values.push(atr);
        }

        atr_values
    }
}

/// Extract close prices (zero-copy view when possible)
#[inline]
pub fn extract_closes(ohlcv: &[OHLCV]) -> Vec<f64> {
    ohlcv.iter().map(|c| c.close).collect()
}

// ============================================================================
// BASELINE IMPLEMENTATIONS (No optimizations) - For Performance Comparison
// ============================================================================

use crate::simd::{mean_baseline, weighted_sum_f64_baseline};

/// Baseline HMA (no SIMD, no optimizations)
pub struct HMABaseline {
    pub period: usize,
    half_period: usize,
    sqrt_period: usize,
}

impl HMABaseline {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            half_period: period / 2,
            sqrt_period: (period as f64).sqrt() as usize,
        }
    }

    pub fn compute(&self, prices: &[f64]) -> IndicatorResult {
        let len = prices.len();
        let mut result = IndicatorResult::with_capacity(len);

        if len < self.period {
            return result;
        }

        let wma_full_weights = self.wma_weights(self.period);
        let wma_half_weights = self.wma_weights(self.half_period);
        let wma_sqrt_weights = self.wma_weights(self.sqrt_period);

        let mut raw_hma_buffer = Vec::with_capacity(len);

        for i in self.period - 1..len {
            let wma_full = weighted_sum_f64_baseline(
                &prices[i + 1 - self.period..=i],
                &wma_full_weights
            );
            
            let wma_half = weighted_sum_f64_baseline(
                &prices[i + 1 - self.half_period..=i],
                &wma_half_weights
            );
            
            raw_hma_buffer.push(2.0 * wma_half - wma_full);
        }

        for i in self.sqrt_period - 1..raw_hma_buffer.len() {
            let hma = weighted_sum_f64_baseline(
                &raw_hma_buffer[i + 1 - self.sqrt_period..=i],
                &wma_sqrt_weights
            );
            result.values.push(hma);
        }

        result
    }

    #[inline]
    fn wma_weights(&self, period: usize) -> Vec<f64> {
        let sum: f64 = (1..=period).sum::<usize>() as f64;
        (1..=period)
            .map(|w| w as f64 / sum)
            .collect()
    }
}

/// Baseline MACD (no optimizations)
pub struct MACDBaseline {
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    fast_alpha: f64,
    slow_alpha: f64,
    signal_alpha: f64,
}

impl MACDBaseline {
    pub fn new(fast: usize, slow: usize, signal: usize) -> Self {
        Self {
            fast_period: fast,
            slow_period: slow,
            signal_period: signal,
            fast_alpha: 2.0 / (fast + 1) as f64,
            slow_alpha: 2.0 / (slow + 1) as f64,
            signal_alpha: 2.0 / (signal + 1) as f64,
        }
    }

    pub fn compute(&self, prices: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let len = prices.len();
        
        if len < self.slow_period {
            return (vec![], vec![], vec![]);
        }

        let mut macd_line = Vec::with_capacity(len);
        let mut signal_line = Vec::with_capacity(len);
        let mut histogram = Vec::with_capacity(len);

        let mut fast_ema = mean_baseline(&prices[..self.fast_period]);
        let mut slow_ema = mean_baseline(&prices[..self.slow_period]);

        for i in self.slow_period..len {
            fast_ema = self.fast_alpha * prices[i] + (1.0 - self.fast_alpha) * fast_ema;
            slow_ema = self.slow_alpha * prices[i] + (1.0 - self.slow_alpha) * slow_ema;
            
            let macd = fast_ema - slow_ema;
            macd_line.push(macd);
        }

        if macd_line.len() >= self.signal_period {
            let mut signal_ema = mean_baseline(&macd_line[..self.signal_period]);
            
            for i in self.signal_period - 1..macd_line.len() {
                signal_ema = self.signal_alpha * macd_line[i] + (1.0 - self.signal_alpha) * signal_ema;
                signal_line.push(signal_ema);
                histogram.push(macd_line[i] - signal_ema);
            }
        }

        (macd_line, signal_line, histogram)
    }
}

/// Baseline RSI (no optimizations)
pub struct RSIBaseline {
    period: usize,
    alpha: f64,
}

impl RSIBaseline {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            alpha: 1.0 / period as f64,
        }
    }

    pub fn compute(&self, prices: &[f64]) -> Vec<f64> {
        let len = prices.len();
        
        if len < self.period + 1 {
            return vec![];
        }

        let mut rsi_values = Vec::with_capacity(len);

        let mut avg_gain = 0.0;
        let mut avg_loss = 0.0;

        for i in 1..=self.period {
            let change = prices[i] - prices[i - 1];
            if change > 0.0 {
                avg_gain += change;
            } else {
                avg_loss += -change;
            }
        }

        avg_gain /= self.period as f64;
        avg_loss /= self.period as f64;

        for i in self.period + 1..len {
            let change = prices[i] - prices[i - 1];
            
            let gain = if change > 0.0 { change } else { 0.0 };
            let loss = if change < 0.0 { -change } else { 0.0 };

            avg_gain = self.alpha * gain + (1.0 - self.alpha) * avg_gain;
            avg_loss = self.alpha * loss + (1.0 - self.alpha) * avg_loss;

            let rs = if avg_loss == 0.0 { 100.0 } else { avg_gain / avg_loss };
            let rsi = 100.0 - (100.0 / (1.0 + rs));
            
            rsi_values.push(rsi);
        }

        rsi_values
    }
}

/// Baseline Choppiness Index (no optimizations)
pub struct ChoppinessIndexBaseline {
    period: usize,
}

impl ChoppinessIndexBaseline {
    pub fn new(period: usize) -> Self {
        Self { period }
    }

    pub fn compute(&self, ohlcv: &[OHLCV]) -> Vec<f64> {
        let len = ohlcv.len();
        
        if len < self.period {
            return vec![];
        }

        let mut choppy_values = Vec::with_capacity(len - self.period + 1);
        let log_period = (self.period as f64).ln();

        for i in self.period..=len {
            let window = &ohlcv[i - self.period..i];
            
            let mut atr_sum = 0.0;
            for j in 1..window.len() {
                let high_low = window[j].high - window[j].low;
                let high_close = (window[j].high - window[j - 1].close).abs();
                let low_close = (window[j].low - window[j - 1].close).abs();
                
                atr_sum += high_low.max(high_close).max(low_close);
            }

            let mut high = window[0].high;
            let mut low = window[0].low;
            for candle in window {
                if candle.high > high {
                    high = candle.high;
                }
                if candle.low < low {
                    low = candle.low;
                }
            }
            let range = high - low;

            let choppy = if range > 0.0 {
                100.0 * (atr_sum / range).ln() / log_period
            } else {
                0.0
            };

            choppy_values.push(choppy);
        }

        choppy_values
    }
}

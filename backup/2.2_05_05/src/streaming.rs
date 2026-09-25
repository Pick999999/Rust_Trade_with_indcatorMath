// Real-time Streaming Indicator Engine
// Lock-free updates with minimal latency

use crate::OHLCV;
use crate::indicators::{HMA, MACD, RSI, ChoppinessIndex};
use std::sync::Arc;
use parking_lot::RwLock;

/// State for streaming HMA calculation
pub struct StreamingHMA {
    hma: HMA,
    buffer: Vec<f64>,
    capacity: usize,
}

impl StreamingHMA {
    pub fn new(period: usize, buffer_capacity: usize) -> Self {
        Self {
            hma: HMA::new(period),
            buffer: Vec::with_capacity(buffer_capacity),
            capacity: buffer_capacity,
        }
    }

    /// Update with new price (returns new HMA value if ready)
    pub fn update(&mut self, price: f64) -> Option<f64> {
        self.buffer.push(price);
        
        // Keep buffer at capacity
        if self.buffer.len() > self.capacity {
            self.buffer.remove(0);
        }

        // Compute if we have enough data
        if self.buffer.len() >= self.hma.period {
            let result = self.hma.compute(&self.buffer);
            result.values.last().copied()
        } else {
            None
        }
    }

    pub fn current(&self) -> Option<f64> {
        if self.buffer.len() >= self.hma.period {
            let result = self.hma.compute(&self.buffer);
            result.values.last().copied()
        } else {
            None
        }
    }
}

/// Stateful WMA Calculator (O(1) updates, completely lock-free and array-backed)
pub struct StatefulWMA {
    period: usize,
    buffer: Vec<f64>,
    head: usize,
    len: usize,
    unweighted_sum: f64,
    weighted_sum: f64,
    sum_of_weights: f64,
}

impl StatefulWMA {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: vec![0.0; period],
            head: 0,
            len: 0,
            unweighted_sum: 0.0,
            weighted_sum: 0.0,
            sum_of_weights: (1..=period).sum::<usize>() as f64,
        }
    }

    pub fn update(&mut self, price: f64) -> Option<f64> {
        if self.len < self.period {
            self.buffer[self.len] = price;
            self.len += 1;
            
            self.unweighted_sum += price;
            self.weighted_sum += (self.len as f64) * price;
            
            if self.len == self.period {
                Some(self.weighted_sum / self.sum_of_weights)
            } else {
                None
            }
        } else {
            let oldest = self.buffer[self.head];
            let old_unweighted = self.unweighted_sum;
            
            self.weighted_sum = self.weighted_sum + (self.period as f64) * price - old_unweighted;
            self.unweighted_sum = self.unweighted_sum + price - oldest;
            
            self.buffer[self.head] = price;
            self.head = (self.head + 1) % self.period;
            
            Some(self.weighted_sum / self.sum_of_weights)
        }
    }
}

/// State for O(1) streaming HMA calculation
pub struct StatefulHMA {
    wma_full: StatefulWMA,
    wma_half: StatefulWMA,
    wma_sqrt: StatefulWMA,
    last_value: Option<f64>,
}

impl StatefulHMA {
    pub fn new(period: usize) -> Self {
        Self {
            wma_full: StatefulWMA::new(period),
            wma_half: StatefulWMA::new(period / 2),
            wma_sqrt: StatefulWMA::new((period as f64).sqrt() as usize),
            last_value: None,
        }
    }

    /// Update with new price in O(1) time (returns new HMA value if ready)
    pub fn update(&mut self, price: f64) -> Option<f64> {
        let full_wma = self.wma_full.update(price);
        let half_wma = self.wma_half.update(price);

        if let (Some(f), Some(h)) = (full_wma, half_wma) {
            let raw_hma = 2.0 * h - f;
            self.last_value = self.wma_sqrt.update(raw_hma);
            self.last_value
        } else {
            None
        }
    }

    pub fn current(&self) -> Option<f64> {
        self.last_value
    }
}


/// State for streaming MACD
pub struct StreamingMACD {
    macd: MACD,
    buffer: Vec<f64>,
    capacity: usize,
}

impl StreamingMACD {
    pub fn new(fast: usize, slow: usize, signal: usize, buffer_capacity: usize) -> Self {
        Self {
            macd: MACD::new(fast, slow, signal),
            buffer: Vec::with_capacity(buffer_capacity),
            capacity: buffer_capacity,
        }
    }

    /// Returns (MACD, Signal, Histogram) if ready
    pub fn update(&mut self, price: f64) -> Option<(f64, f64, f64)> {
        self.buffer.push(price);
        
        if self.buffer.len() > self.capacity {
            self.buffer.remove(0);
        }

        self.current()
    }

    pub fn current(&self) -> Option<(f64, f64, f64)> {
        let (macd_line, signal_line, histogram) = self.macd.compute(&self.buffer);
        
        if let (Some(&m), Some(&s), Some(&h)) = (
            macd_line.last(),
            signal_line.last(),
            histogram.last()
        ) {
            Some((m, s, h))
        } else {
            None
        }
    }
}

/// State for streaming RSI
pub struct StreamingRSI {
    rsi: RSI,
    buffer: Vec<f64>,
    capacity: usize,
}

impl StreamingRSI {
    pub fn new(period: usize, buffer_capacity: usize) -> Self {
        Self {
            rsi: RSI::new(period),
            buffer: Vec::with_capacity(buffer_capacity),
            capacity: buffer_capacity,
        }
    }

    pub fn update(&mut self, price: f64) -> Option<f64> {
        self.buffer.push(price);
        
        if self.buffer.len() > self.capacity {
            self.buffer.remove(0);
        }

        let rsi_values = self.rsi.compute(&self.buffer);
        rsi_values.last().copied()
    }

    pub fn current(&self) -> Option<f64> {
        let rsi_values = self.rsi.compute(&self.buffer);
        rsi_values.last().copied()
    }
}

/// State for streaming Choppiness Index
pub struct StreamingChoppy {
    choppy: ChoppinessIndex,
    buffer: Vec<OHLCV>,
    capacity: usize,
}

impl StreamingChoppy {
    pub fn new(period: usize, buffer_capacity: usize) -> Self {
        Self {
            choppy: ChoppinessIndex::new(period),
            buffer: Vec::with_capacity(buffer_capacity),
            capacity: buffer_capacity,
        }
    }

    pub fn update(&mut self, candle: OHLCV) -> Option<f64> {
        self.buffer.push(candle);
        
        if self.buffer.len() > self.capacity {
            self.buffer.remove(0);
        }

        let choppy_values = self.choppy.compute(&self.buffer);
        choppy_values.last().copied()
    }

    pub fn current(&self) -> Option<f64> {
        let choppy_values = self.choppy.compute(&self.buffer);
        choppy_values.last().copied()
    }
}

/// Stateful ATR - O(1) updates using Wilder's smoothing
/// Keeps only prev_close and current ATR value in memory
pub struct StatefulATR {
    period: usize,
    alpha: f64,
    prev_close: Option<f64>,
    current_atr: Option<f64>,
    tr_buffer: Vec<f64>,
    warmup_count: usize,
}

impl StatefulATR {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            alpha: 1.0 / period as f64,
            prev_close: None,
            current_atr: None,
            tr_buffer: Vec::with_capacity(period),
            warmup_count: 0,
        }
    }

    /// Update with new OHLCV candle, returns ATR if ready
    pub fn update(&mut self, candle: &OHLCV) -> Option<f64> {
        if let Some(prev_close) = self.prev_close {
            // Compute True Range
            let high_low = candle.high - candle.low;
            let high_close = (candle.high - prev_close).abs();
            let low_close = (candle.low - prev_close).abs();
            let tr = high_low.max(high_close).max(low_close);

            if self.current_atr.is_some() {
                // O(1) Wilder's smoothing update
                let atr = self.current_atr.unwrap();
                let new_atr = atr + self.alpha * (tr - atr);
                self.current_atr = Some(new_atr);
            } else {
                // Warm-up phase: collect TR values until we have `period`
                self.tr_buffer.push(tr);
                self.warmup_count += 1;

                if self.warmup_count >= self.period {
                    // Initial ATR = SMA of first `period` True Ranges
                    let sum: f64 = self.tr_buffer.iter().sum();
                    self.current_atr = Some(sum / self.period as f64);
                    self.tr_buffer.clear(); // Free memory
                }
            }
        }

        self.prev_close = Some(candle.close);
        self.current_atr
    }

    pub fn current(&self) -> Option<f64> {
        self.current_atr
    }
}

/// Complete streaming indicator engine (thread-safe)
pub struct IndicatorEngine {
    hma: Arc<RwLock<StreamingHMA>>,
    macd: Arc<RwLock<StreamingMACD>>,
    rsi: Arc<RwLock<StreamingRSI>>,
    choppy: Arc<RwLock<StreamingChoppy>>,
    atr: Arc<RwLock<StatefulATR>>,
}

impl IndicatorEngine {
    pub fn new(
        hma_period: usize,
        macd_config: (usize, usize, usize),
        rsi_period: usize,
        choppy_period: usize,
        atr_period: usize,
        buffer_capacity: usize,
    ) -> Self {
        Self {
            hma: Arc::new(RwLock::new(StreamingHMA::new(hma_period, buffer_capacity))),
            macd: Arc::new(RwLock::new(StreamingMACD::new(
                macd_config.0, macd_config.1, macd_config.2, buffer_capacity
            ))),
            rsi: Arc::new(RwLock::new(StreamingRSI::new(rsi_period, buffer_capacity))),
            choppy: Arc::new(RwLock::new(StreamingChoppy::new(choppy_period, buffer_capacity))),
            atr: Arc::new(RwLock::new(StatefulATR::new(atr_period))),
        }
    }

    /// Update all indicators with new candle
    pub fn update(&self, candle: OHLCV) -> IndicatorSnapshot {
        let price = candle.close;
        
        let hma = self.hma.write().update(price);
        let macd = self.macd.write().update(price);
        let rsi = self.rsi.write().update(price);
        let choppy = self.choppy.write().update(candle);
        let atr = self.atr.write().update(&candle);

        IndicatorSnapshot {
            timestamp: candle.timestamp,
            price,
            hma,
            macd,
            rsi,
            choppy,
            atr,
        }
    }

    /// Get current values without update
    pub fn snapshot(&self) -> IndicatorSnapshot {
        IndicatorSnapshot {
            timestamp: 0,
            price: 0.0,
            hma: self.hma.read().current(),
            macd: self.macd.read().current(),
            rsi: self.rsi.read().current(),
            choppy: self.choppy.read().current(),
            atr: self.atr.read().current(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IndicatorSnapshot {
    pub timestamp: i64,
    pub price: f64,
    pub hma: Option<f64>,
    pub macd: Option<(f64, f64, f64)>,  // (MACD, Signal, Histogram)
    pub rsi: Option<f64>,
    pub choppy: Option<f64>,
    pub atr: Option<f64>,
}

impl IndicatorSnapshot {
    pub fn is_ready(&self) -> bool {
        self.hma.is_some() && 
        self.macd.is_some() && 
        self.rsi.is_some() && 
        self.choppy.is_some() &&
        self.atr.is_some()
    }
}

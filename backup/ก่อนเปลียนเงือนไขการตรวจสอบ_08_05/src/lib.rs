// High-Performance Technical Indicators Library
// Architecture: Zero-Copy, Lock-Free, Cache-Optimized

#![allow(dead_code)]

/// Price data aligned for optimal cache performance
#[repr(C, align(64))]  // Cache line alignment
#[derive(Clone, Copy, Debug)]
pub struct OHLCV {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub timestamp: i64,
}

/// Ring buffer for streaming data - Lock-free reads
pub struct PriceBuffer {
    data: Vec<OHLCV>,
    capacity: usize,
    write_idx: std::sync::atomic::AtomicUsize,
}

impl PriceBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![OHLCV { 
                open: 0.0, high: 0.0, low: 0.0, 
                close: 0.0, volume: 0.0, timestamp: 0 
            }; capacity],
            capacity,
            write_idx: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Lock-free append
    pub fn push(&mut self, candle: OHLCV) {
        let idx = self.write_idx.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % self.capacity;
        self.data[idx] = candle;
    }

    /// Zero-copy slice view
    pub fn as_slice(&self) -> &[OHLCV] {
        &self.data
    }

    pub fn len(&self) -> usize {
        std::cmp::min(
            self.write_idx.load(std::sync::atomic::Ordering::Relaxed),
            self.capacity
        )
    }
}

/// Pre-allocated result buffer to avoid allocations in hot path
#[derive(Clone)]
pub struct IndicatorResult {
    pub values: Vec<f64>,
    pub timestamps: Vec<i64>,
}

impl IndicatorResult {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
            timestamps: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, value: f64, timestamp: i64) {
        self.values.push(value);
        self.timestamps.push(timestamp);
    }

    pub fn clear(&mut self) {
        self.values.clear();
        self.timestamps.clear();
    }
}

// Module exports
pub mod indicators;
pub mod simd;
pub mod streaming;

pub use indicators::*;
pub use streaming::*;

# 📚 Usage Guide - Turbo Indicators

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
turbo-indicators = { path = "../turbo-indicators" }
```

---

## Quick Start Examples

### Example 1: Basic Batch Analysis

```rust
use turbo_indicators::*;

fn main() {
    // Sample price data
    let prices = vec![
        100.0, 101.0, 102.5, 101.8, 103.0,
        104.2, 103.5, 105.0, 106.5, 107.0,
        // ... more prices
    ];

    // Calculate HMA
    let hma = HMA::new(9);
    let result = hma.compute(&prices);
    
    for (i, &value) in result.values.iter().enumerate() {
        println!("HMA[{}] = {:.4}", i, value);
    }
}
```

### Example 2: Multi-Indicator Analysis

```rust
use turbo_indicators::*;

fn analyze_market(candles: &[OHLCV]) {
    let closes = extract_closes(candles);
    
    // Set up indicators
    let hma = HMA::new(20);
    let macd = MACD::new(12, 26, 9);
    let rsi = RSI::new(14);
    let choppy = ChoppinessIndex::new(14);
    
    // Compute all at once
    let hma_values = hma.compute(&closes);
    let (macd_line, signal, histogram) = macd.compute(&closes);
    let rsi_values = rsi.compute(&closes);
    let choppy_values = choppy.compute(candles);
    
    // Analyze last values
    if let Some(&last_hma) = hma_values.values.last() {
        if let Some(&last_rsi) = rsi_values.last() {
            if last_rsi > 70.0 {
                println!("⚠️  OVERBOUGHT: RSI = {:.2}", last_rsi);
            } else if last_rsi < 30.0 {
                println!("⚠️  OVERSOLD: RSI = {:.2}", last_rsi);
            }
        }
    }
    
    if let Some(&choppy_val) = choppy_values.last() {
        if choppy_val > 61.8 {
            println!("📊 Market is CHOPPY (ranging)");
        } else if choppy_val < 38.2 {
            println!("📈 Market is TRENDING");
        }
    }
}
```

### Example 3: Real-Time Trading Bot

```rust
use turbo_indicators::*;
use std::sync::Arc;

struct TradingBot {
    engine: Arc<IndicatorEngine>,
}

impl TradingBot {
    fn new() -> Self {
        let engine = IndicatorEngine::new(
            20,           // HMA period
            (12, 26, 9),  // MACD
            14,           // RSI
            14,           // Choppiness
            500,          // Buffer size
        );
        
        Self { 
            engine: Arc::new(engine) 
        }
    }
    
    fn on_new_candle(&self, candle: OHLCV) {
        let snapshot = self.engine.update(candle);
        
        if !snapshot.is_ready() {
            return; // Not enough data yet
        }
        
        // Trading logic
        if let (Some(rsi), Some((macd, signal, _))) = 
            (snapshot.rsi, snapshot.macd) 
        {
            // Buy signal
            if rsi < 30.0 && macd > signal {
                println!("🟢 BUY SIGNAL at {:.4}", snapshot.price);
                self.execute_buy(snapshot.price);
            }
            
            // Sell signal
            if rsi > 70.0 && macd < signal {
                println!("🔴 SELL SIGNAL at {:.4}", snapshot.price);
                self.execute_sell(snapshot.price);
            }
        }
    }
    
    fn execute_buy(&self, price: f64) {
        // Your buy logic
    }
    
    fn execute_sell(&self, price: f64) {
        // Your sell logic
    }
}

fn main() {
    let bot = TradingBot::new();
    
    // Connect to your data feed
    loop {
        let candle = receive_candle_from_exchange();
        bot.on_new_candle(candle);
    }
}
```

### Example 4: Parallel Backtesting

```rust
use turbo_indicators::*;
use rayon::prelude::*;

struct Strategy {
    hma_period: usize,
    rsi_period: usize,
}

impl Strategy {
    fn backtest(&self, candles: &[OHLCV]) -> f64 {
        let closes = extract_closes(candles);
        
        let hma = HMA::new(self.hma_period);
        let rsi = RSI::new(self.rsi_period);
        
        let hma_values = hma.compute(&closes);
        let rsi_values = rsi.compute(&closes);
        
        // Calculate returns
        let mut total_return = 0.0;
        let mut position = 0.0;
        
        for i in 0..candles.len().min(rsi_values.len()) {
            let rsi = rsi_values[i];
            
            // Simple strategy
            if rsi < 30.0 && position == 0.0 {
                position = candles[i].close;
            } else if rsi > 70.0 && position > 0.0 {
                total_return += (candles[i].close - position) / position;
                position = 0.0;
            }
        }
        
        total_return
    }
}

fn optimize_strategy(candles: &[OHLCV]) -> Strategy {
    // Test multiple parameter combinations in parallel
    let hma_periods = vec![9, 14, 20, 50];
    let rsi_periods = vec![7, 14, 21];
    
    let strategies: Vec<Strategy> = hma_periods.iter()
        .flat_map(|&hma| {
            rsi_periods.iter().map(move |&rsi| {
                Strategy {
                    hma_period: hma,
                    rsi_period: rsi,
                }
            })
        })
        .collect();
    
    // Parallel backtest
    let results: Vec<(Strategy, f64)> = strategies
        .par_iter()
        .map(|strategy| {
            let return_pct = strategy.backtest(candles);
            (strategy.clone(), return_pct)
        })
        .collect();
    
    // Find best strategy
    results.into_iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(strategy, return_pct)| {
            println!("Best: HMA={}, RSI={}, Return={:.2}%", 
                     strategy.hma_period, strategy.rsi_period, return_pct * 100.0);
            strategy
        })
        .unwrap()
}
```

### Example 5: Custom Indicator Combination

```rust
use turbo_indicators::*;

struct TrendStrength {
    hma: HMA,
    choppy: ChoppinessIndex,
}

impl TrendStrength {
    fn new() -> Self {
        Self {
            hma: HMA::new(20),
            choppy: ChoppinessIndex::new(14),
        }
    }
    
    fn analyze(&self, candles: &[OHLCV]) -> TrendSignal {
        let closes = extract_closes(candles);
        let hma_values = self.hma.compute(&closes);
        let choppy_values = self.choppy.compute(candles);
        
        let current_price = candles.last().unwrap().close;
        let last_hma = hma_values.values.last().copied().unwrap_or(current_price);
        let last_choppy = choppy_values.last().copied().unwrap_or(50.0);
        
        // Combine indicators
        if last_choppy < 38.2 {  // Trending market
            if current_price > last_hma {
                TrendSignal::StrongUptrend
            } else {
                TrendSignal::StrongDowntrend
            }
        } else if last_choppy > 61.8 {  // Choppy market
            TrendSignal::Ranging
        } else {
            TrendSignal::Weak
        }
    }
}

enum TrendSignal {
    StrongUptrend,
    StrongDowntrend,
    Weak,
    Ranging,
}

impl TrendSignal {
    fn description(&self) -> &str {
        match self {
            Self::StrongUptrend => "Strong upward trend - Consider long positions",
            Self::StrongDowntrend => "Strong downward trend - Consider short positions",
            Self::Weak => "Weak trend - Wait for confirmation",
            Self::Ranging => "Ranging market - Use range-bound strategies",
        }
    }
}
```

---

## Performance Tips

### 1. Reuse Indicator Objects

```rust
// ❌ Bad: Creates new objects every time
fn slow_analysis(prices: &[f64]) {
    for window in prices.windows(100) {
        let hma = HMA::new(20);  // Allocation overhead!
        let result = hma.compute(window);
    }
}

// ✅ Good: Reuse the object
fn fast_analysis(prices: &[f64]) {
    let hma = HMA::new(20);  // Create once
    for window in prices.windows(100) {
        let result = hma.compute(window);  // Reuse
    }
}
```

### 2. Pre-allocate Buffers

```rust
// For streaming, specify adequate buffer capacity
let engine = IndicatorEngine::new(
    20, (12, 26, 9), 14, 14,
    1000  // Buffer capacity - adjust based on your needs
);
```

### 3. Batch Processing for Historical Data

```rust
// Process large datasets in one go
let hma = HMA::new(20);
let all_results = hma.compute(&all_historical_prices);

// Much faster than iterating and computing for each candle
```

### 4. Use Parallel Processing

```rust
use rayon::prelude::*;

// Analyze multiple symbols in parallel
let results: Vec<_> = symbols
    .par_iter()
    .map(|symbol| {
        let candles = fetch_candles(symbol);
        analyze_market(&candles)
    })
    .collect();
```

---

## Common Patterns

### Pattern 1: Signal Generation

```rust
fn generate_signals(snapshot: &IndicatorSnapshot) -> Vec<Signal> {
    let mut signals = Vec::new();
    
    if let Some(rsi) = snapshot.rsi {
        if rsi > 70.0 {
            signals.push(Signal::Overbought);
        } else if rsi < 30.0 {
            signals.push(Signal::Oversold);
        }
    }
    
    if let Some((macd, signal, _)) = snapshot.macd {
        if macd > signal {
            signals.push(Signal::BullishCrossover);
        } else if macd < signal {
            signals.push(Signal::BearishCrossover);
        }
    }
    
    signals
}
```

### Pattern 2: Risk Management

```rust
fn check_risk(snapshot: &IndicatorSnapshot, position: &Position) -> bool {
    // Use Choppiness to avoid trading in volatile markets
    if let Some(choppy) = snapshot.choppy {
        if choppy > 61.8 {
            println!("⚠️  High volatility - reducing position size");
            return false;
        }
    }
    
    // Use RSI to avoid overextended moves
    if let Some(rsi) = snapshot.rsi {
        if position.is_long() && rsi > 80.0 {
            println!("⚠️  Extremely overbought - consider exit");
            return false;
        }
        if position.is_short() && rsi < 20.0 {
            println!("⚠️  Extremely oversold - consider exit");
            return false;
        }
    }
    
    true
}
```

---

## Error Handling

```rust
fn safe_analysis(candles: &[OHLCV]) -> Option<IndicatorSnapshot> {
    if candles.len() < 50 {
        println!("Not enough data");
        return None;
    }
    
    let engine = IndicatorEngine::new(20, (12, 26, 9), 14, 14, 200);
    
    // Feed all candles
    let mut snapshot = None;
    for candle in candles {
        snapshot = Some(engine.update(*candle));
    }
    
    snapshot
}
```

---

## Integration Examples

### With Binance API

```rust
use turbo_indicators::*;

async fn monitor_binance_btcusdt() {
    let engine = IndicatorEngine::new(20, (12, 26, 9), 14, 14, 500);
    
    // Subscribe to Binance WebSocket
    let mut stream = binance_ws::kline_stream("BTCUSDT", "1m");
    
    while let Some(kline) = stream.next().await {
        let candle = OHLCV {
            open: kline.open,
            high: kline.high,
            low: kline.low,
            close: kline.close,
            volume: kline.volume,
            timestamp: kline.timestamp,
        };
        
        let snapshot = engine.update(candle);
        
        if snapshot.is_ready() {
            process_snapshot(snapshot);
        }
    }
}
```

### With CSV Data

```rust
use turbo_indicators::*;
use csv::Reader;

fn analyze_csv(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = Reader::from_path(filename)?;
    let mut candles = Vec::new();
    
    for result in reader.deserialize() {
        let record: CsvRow = result?;
        candles.push(OHLCV {
            open: record.open,
            high: record.high,
            low: record.low,
            close: record.close,
            volume: record.volume,
            timestamp: record.timestamp,
        });
    }
    
    // Analyze the loaded data
    analyze_market(&candles);
    
    Ok(())
}

#[derive(Deserialize)]
struct CsvRow {
    timestamp: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}
```

---

Happy trading! 🚀📈

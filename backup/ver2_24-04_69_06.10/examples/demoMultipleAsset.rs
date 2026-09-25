// Example: Multi-Asset Stateful Processing
// Demonstrates managing multiple indicator engines for different symbols

use turbo_indicators::*;
use std::collections::HashMap;
use std::time::Instant;

fn main() {
    println!("🚀 Turbo-Indicators: Multi-Asset Stateful Demo\n");

    // 1. Define 10 symbols
    let symbols = vec![
        "BTC/USDT", "ETH/USDT", "SOL/USDT", "BNB/USDT", "ADA/USDT",
        "XRP/USDT", "DOT/USDT", "LINK/USDT", "MATIC/USDT", "AVAX/USDT"
    ];

    // 2. Initialize Engines for each asset
    // We store them in a HashMap for easy access by symbol
    let mut asset_manager: HashMap<String, IndicatorEngine> = HashMap::new();

    for symbol in &symbols {
        let engine = IndicatorEngine::new(
            20,            // HMA
            (12, 26, 9),   // MACD
            14,            // RSI
            14,            // Choppy
            14,            // ATR
            100,           // Buffer Capacity
        );
        asset_manager.insert(symbol.to_string(), engine);
    }

    println!("✅ Initialized engines for {} assets.", symbols.len());
    println!("⏱️  Simulating 1,000 candles per asset...\n");

    let total_candles_per_asset = 1_000;
    let start_time = Instant::now();

    // 3. Simulate Data Stream
    // In a real app, this would be data coming from a WebSocket or Message Queue
    for i in 0..total_candles_per_asset {
        for symbol in &symbols {
            // Generate a dummy candle for this symbol
            let candle = generate_dummy_candle(symbol, i);
            
            // Get the corresponding engine and update it
            if let Some(engine) = asset_manager.get(symbol) {
                let snapshot = engine.update(candle);
                
                // For demonstration, print the results of the 1,000th candle
                if i == total_candles_per_asset - 1 {
                    print_snapshot(symbol, snapshot);
                }
            }
        }
    }

    let elapsed = start_time.elapsed();
    let total_processed = total_candles_per_asset * symbols.len();
    
    println!("🏁 Demo Completed!");
    println!("📊 Total Candles Processed: {}", total_processed);
    println!("⏱️  Total Time: {:?}", elapsed);
    println!("🚀 Throughput: {:.0} candles/sec", 
             total_processed as f64 / elapsed.as_secs_f64());
}

/// Helper to generate a dummy candle based on symbol and index
fn generate_dummy_candle(symbol: &str, index: usize) -> OHLCV {
    // Basic price varying by symbol name and index
    let base_price = (symbol.len() as f64 * 100.0) + (index as f64 * 0.1);
    let drift = (index as f64 * 0.05).sin() * 2.0;
    
    OHLCV {
        open: base_price + drift,
        high: base_price + drift + 1.0,
        low: base_price + drift - 1.0,
        close: base_price + drift + 0.5,
        volume: 1000.0,
        timestamp: 1672531200 + (index as i64 * 60), // 1-minute intervals
    }
}

/// Helper to print indicator results
fn print_snapshot(symbol: &str, snapshot: IndicatorSnapshot) {
    println!("--------------------------------------------------");
    println!("Symbol: {:<10} | Price: {:.2}", symbol, snapshot.price);
    
    if let Some(rsi) = snapshot.rsi {
        println!("  RSI(14): {:.2}", rsi);
    }
    if let Some(atr) = snapshot.atr {
        println!("  ATR(14): {:.4}", atr);
    }
    if let Some(hma) = snapshot.hma {
        println!("  HMA(20): {:.2}", hma);
    }
    if let Some((m, s, h)) = snapshot.macd {
        println!("  MACD: {:.4} | Signal: {:.4} | Hist: {:.4}", m, s, h);
    }
}

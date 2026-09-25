// Example: Demonstrating both Batch and Real-time processing

use turbo_indicators::*;
use std::time::Instant;

fn main() {
    println!("🚀 High-Performance Technical Indicators Demo\n");

    // Generate 10,000 sample candles
    let sample_data = generate_sample_data(10_000);
    
    println!("📊 Dataset: {} candles\n", sample_data.len());
    
    // ============================================
    // PART 1: BATCH PROCESSING (Historical Data)
    // ============================================
    println!("📊 BATCH PROCESSING - OPTIMIZED VERSION");
    println!("{}", "=".repeat(60));
    
    batch_example(&sample_data);
    
    println!("\n");
    
    // ============================================
    // PART 1B: BASELINE COMPARISON
    // ============================================
    println!("📊 BATCH PROCESSING - BASELINE (NO OPTIMIZATIONS)");
    println!("{}", "=".repeat(60));
    
    baseline_example(&sample_data);
    
    println!("\n");
    
    // ============================================
    // PART 2: REAL-TIME STREAMING
    // ============================================
    println!("⚡ REAL-TIME STREAMING");
    println!("{}", "=".repeat(60));
    
    streaming_example(&sample_data);
    
    println!("\n");
    
    // ============================================
    // PART 3: PERFORMANCE COMPARISON
    // ============================================
    println!("🏎️  PERFORMANCE BENCHMARK - OPTIMIZED vs BASELINE");
    println!("{}", "=".repeat(60));
    
    performance_comparison(&sample_data);
    
    println!("\n");
    
    // ============================================
    // PART 4: STREAMING PERFORMANCE COMPARISON
    // ============================================
    println!("⏱️  STREAMING BENCHMARK - O(N) BUFFER vs O(1) STATEFUL");
    println!("{}", "=".repeat(60));
    
    streaming_performance_comparison(&sample_data);
}

fn batch_example(data: &[OHLCV]) {
    let closes = extract_closes(data);
    
    // HMA
    let start = Instant::now();
    let hma = HMA::new(20);
    let hma_result = hma.compute(&closes);
    let hma_time = start.elapsed();
    
    println!("✓ HMA(20): {} values computed in {:?}", 
             hma_result.values.len(), hma_time);
    if let Some(&last) = hma_result.values.last() {
        println!("  Last value: {:.4}", last);
    }
    
    // MACD
    let start = Instant::now();
    let macd = MACD::new(12, 26, 9);
    let (macd_line, signal, histogram) = macd.compute(&closes);
    let macd_time = start.elapsed();
    
    println!("\n✓ MACD(12,26,9): {} values computed in {:?}", 
             macd_line.len(), macd_time);
    if let (Some(&m), Some(&s), Some(&h)) = (
        macd_line.last(), 
        signal.last(), 
        histogram.last()
    ) {
        println!("  MACD: {:.4}, Signal: {:.4}, Histogram: {:.4}", m, s, h);
    }
    
    // RSI
    let start = Instant::now();
    let rsi = RSI::new(14);
    let rsi_values = rsi.compute(&closes);
    let rsi_time = start.elapsed();
    
    println!("\n✓ RSI(14): {} values computed in {:?}", 
             rsi_values.len(), rsi_time);
    if let Some(&last) = rsi_values.last() {
        println!("  Last value: {:.2}", last);
    }
    
    // Choppiness Index
    let start = Instant::now();
    let choppy = ChoppinessIndex::new(14);
    let choppy_values = choppy.compute(data);
    let choppy_time = start.elapsed();
    
    println!("\n✓ Choppiness(14): {} values computed in {:?}", 
             choppy_values.len(), choppy_time);
    if let Some(&last) = choppy_values.last() {
        println!("  Last value: {:.2}", last);
    }
}

fn streaming_example(data: &[OHLCV]) {
    println!("Simulating real-time candle updates...\n");
    
    // Create streaming engine
    let engine = IndicatorEngine::new(
        20,              // HMA period
        (12, 26, 9),     // MACD (fast, slow, signal)
        14,              // RSI period
        14,              // Choppiness period
        14,              // ATR period
        200,             // Buffer capacity
    );
    
    let mut ready_count = 0;
    let start = Instant::now();
    
    // Simulate streaming updates
    for (i, candle) in data.iter().enumerate() {
        let snapshot = engine.update(*candle);
        
        if snapshot.is_ready() {
            ready_count += 1;
            
            // Print first few complete snapshots
            if ready_count <= 3 || i == data.len() - 1 {
                println!("Candle #{} @ {}", i + 1, snapshot.timestamp);
                println!("  Price: {:.4}", snapshot.price);
                
                if let Some(hma) = snapshot.hma {
                    println!("  HMA: {:.4}", hma);
                }
                
                if let Some((m, s, h)) = snapshot.macd {
                    println!("  MACD: {:.4} | Signal: {:.4} | Hist: {:.4}", m, s, h);
                }
                
                if let Some(rsi) = snapshot.rsi {
                    println!("  RSI: {:.2}", rsi);
                }
                
                if let Some(choppy) = snapshot.choppy {
                    println!("  Choppiness: {:.2}", choppy);
                }
                
                println!();
            }
        }
    }
    
    let total_time = start.elapsed();
    let avg_latency = total_time / data.len() as u32;
    
    println!("📈 Processed {} candles", data.len());
    println!("📈 {} complete indicator sets", ready_count);
    println!("⚡ Average latency per candle: {:?}", avg_latency);
    println!("⚡ Throughput: {:.0} candles/sec", 
             data.len() as f64 / total_time.as_secs_f64());
}

fn baseline_example(data: &[OHLCV]) {
    let closes = extract_closes(data);
    
    // HMA Baseline
    let start = Instant::now();
    let hma = HMABaseline::new(20);
    let hma_result = hma.compute(&closes);
    let hma_time = start.elapsed();
    
    println!("✓ HMA(20) Baseline: {} values computed in {:?}", 
             hma_result.values.len(), hma_time);
    if let Some(&last) = hma_result.values.last() {
        println!("  Last value: {:.4}", last);
    }
    
    // MACD Baseline
    let start = Instant::now();
    let macd = MACDBaseline::new(12, 26, 9);
    let (macd_line, signal, histogram) = macd.compute(&closes);
    let macd_time = start.elapsed();
    
    println!("\n✓ MACD(12,26,9) Baseline: {} values computed in {:?}", 
             macd_line.len(), macd_time);
    if let (Some(&m), Some(&s), Some(&h)) = (
        macd_line.last(), 
        signal.last(), 
        histogram.last()
    ) {
        println!("  MACD: {:.4}, Signal: {:.4}, Histogram: {:.4}", m, s, h);
    }
    
    // RSI Baseline
    let start = Instant::now();
    let rsi = RSIBaseline::new(14);
    let rsi_values = rsi.compute(&closes);
    let rsi_time = start.elapsed();
    
    println!("\n✓ RSI(14) Baseline: {} values computed in {:?}", 
             rsi_values.len(), rsi_time);
    if let Some(&last) = rsi_values.last() {
        println!("  Last value: {:.2}", last);
    }
    
    // Choppiness Index Baseline
    let start = Instant::now();
    let choppy = ChoppinessIndexBaseline::new(14);
    let choppy_values = choppy.compute(data);
    let choppy_time = start.elapsed();
    
    println!("\n✓ Choppiness(14) Baseline: {} values computed in {:?}", 
             choppy_values.len(), choppy_time);
    if let Some(&last) = choppy_values.last() {
        println!("  Last value: {:.2}", last);
    }
}

fn performance_comparison(data: &[OHLCV]) {
    let closes = extract_closes(data);
    let iterations = 100;
    
    println!("Running {} iterations on {} candles...\n", iterations, data.len());
    
    println!("┌─────────────────────┬──────────────┬──────────────┬──────────────┐");
    println!("│ Indicator           │ Optimized    │ Baseline     │ Speedup      │");
    println!("├─────────────────────┼──────────────┼──────────────┼──────────────┤");
    
    // Benchmark HMA
    let start = Instant::now();
    for _ in 0..iterations {
        let hma = HMA::new(20);
        let _ = hma.compute(&closes);
    }
    let hma_opt_time = start.elapsed() / iterations;
    
    let start = Instant::now();
    for _ in 0..iterations {
        let hma = HMABaseline::new(20);
        let _ = hma.compute(&closes);
    }
    let hma_base_time = start.elapsed() / iterations;
    let hma_speedup = hma_base_time.as_nanos() as f64 / hma_opt_time.as_nanos() as f64;
    
    println!("│ HMA(20)             │ {:>10.2?} │ {:>10.2?} │ {:>10.2}x │", 
             hma_opt_time, hma_base_time, hma_speedup);
    
    // Benchmark MACD
    let start = Instant::now();
    for _ in 0..iterations {
        let macd = MACD::new(12, 26, 9);
        let _ = macd.compute(&closes);
    }
    let macd_opt_time = start.elapsed() / iterations;
    
    let start = Instant::now();
    for _ in 0..iterations {
        let macd = MACDBaseline::new(12, 26, 9);
        let _ = macd.compute(&closes);
    }
    let macd_base_time = start.elapsed() / iterations;
    let macd_speedup = macd_base_time.as_nanos() as f64 / macd_opt_time.as_nanos() as f64;
    
    println!("│ MACD(12,26,9)       │ {:>10.2?} │ {:>10.2?} │ {:>10.2}x │", 
             macd_opt_time, macd_base_time, macd_speedup);
    
    // Benchmark RSI
    let start = Instant::now();
    for _ in 0..iterations {
        let rsi = RSI::new(14);
        let _ = rsi.compute(&closes);
    }
    let rsi_opt_time = start.elapsed() / iterations;
    
    let start = Instant::now();
    for _ in 0..iterations {
        let rsi = RSIBaseline::new(14);
        let _ = rsi.compute(&closes);
    }
    let rsi_base_time = start.elapsed() / iterations;
    let rsi_speedup = rsi_base_time.as_nanos() as f64 / rsi_opt_time.as_nanos() as f64;
    
    println!("│ RSI(14)             │ {:>10.2?} │ {:>10.2?} │ {:>10.2}x │", 
             rsi_opt_time, rsi_base_time, rsi_speedup);
    
    // Benchmark Choppiness
    let start = Instant::now();
    for _ in 0..iterations {
        let choppy = ChoppinessIndex::new(14);
        let _ = choppy.compute(data);
    }
    let choppy_opt_time = start.elapsed() / iterations;
    
    let start = Instant::now();
    for _ in 0..iterations {
        let choppy = ChoppinessIndexBaseline::new(14);
        let _ = choppy.compute(data);
    }
    let choppy_base_time = start.elapsed() / iterations;
    let choppy_speedup = choppy_base_time.as_nanos() as f64 / choppy_opt_time.as_nanos() as f64;
    
    println!("│ Choppiness(14)      │ {:>10.2?} │ {:>10.2?} │ {:>10.2}x │", 
             choppy_opt_time, choppy_base_time, choppy_speedup);
    
    println!("└─────────────────────┴──────────────┴──────────────┴──────────────┘");
    
    // Calculate total time
    let total_opt = hma_opt_time + macd_opt_time + rsi_opt_time + choppy_opt_time;
    let total_base = hma_base_time + macd_base_time + rsi_base_time + choppy_base_time;
    let total_speedup = total_base.as_nanos() as f64 / total_opt.as_nanos() as f64;
    
    println!("\n📊 Summary:");
    println!("  Optimized Total:    {:>10.2?}", total_opt);
    println!("  Baseline Total:     {:>10.2?}", total_base);
    println!("  Overall Speedup:    {:>10.2}x", total_speedup);
    if total_base >= total_opt {
        println!("  Time Saved:         {:>10.2?} per iteration", total_base - total_opt);
    } else {
        println!("  Time Lost:          {:>10.2?} per iteration", total_opt - total_base);
    }
    
    // Calculate throughput
    let candles_per_sec_opt = (data.len() as f64) / total_opt.as_secs_f64();
    let candles_per_sec_base = (data.len() as f64) / total_base.as_secs_f64();
    
    println!("\n🚀 Throughput:");
    println!("  Optimized:          {:.0} candles/sec", candles_per_sec_opt);
    println!("  Baseline:           {:.0} candles/sec", candles_per_sec_base);
}

fn benchmark(data: &[OHLCV]) {
    let closes = extract_closes(data);
    let iterations = 100;
    
    println!("Running {} iterations on {} candles...\n", iterations, data.len());
    
    // Benchmark HMA
    let start = Instant::now();
    for _ in 0..iterations {
        let hma = HMA::new(20);
        let _ = hma.compute(&closes);
    }
    let hma_time = start.elapsed() / iterations;
    
    // Benchmark MACD
    let start = Instant::now();
    for _ in 0..iterations {
        let macd = MACD::new(12, 26, 9);
        let _ = macd.compute(&closes);
    }
    let macd_time = start.elapsed() / iterations;
    
    // Benchmark RSI
    let start = Instant::now();
    for _ in 0..iterations {
        let rsi = RSI::new(14);
        let _ = rsi.compute(&closes);
    }
    let rsi_time = start.elapsed() / iterations;
    
    // Benchmark Choppiness
    let start = Instant::now();
    for _ in 0..iterations {
        let choppy = ChoppinessIndex::new(14);
        let _ = choppy.compute(data);
    }
    let choppy_time = start.elapsed() / iterations;
    
    println!("Average computation time per indicator:");
    println!("  HMA(20):         {:>8?}", hma_time);
    println!("  MACD(12,26,9):   {:>8?}", macd_time);
    println!("  RSI(14):         {:>8?}", rsi_time);
    println!("  Choppiness(14):  {:>8?}", choppy_time);
    
    let total = hma_time + macd_time + rsi_time + choppy_time;
    println!("\n  Total (all 4):   {:>8?}", total);
}

fn generate_sample_data(count: usize) -> Vec<OHLCV> {
    let mut data = Vec::with_capacity(count);
    let mut price = 100.0;
    
    for i in 0..count {
        // Simple random walk
        let change = (i as f64 * 0.1).sin() * 2.0;
        price += change;
        
        let open = price;
        let high = price + (i as f64 * 0.05).cos().abs() * 1.5;
        let low = price - (i as f64 * 0.07).sin().abs() * 1.5;
        let close = price + change * 0.5;
        let volume = 1000.0 + (i as f64 * 0.03).cos() * 500.0;
        
        data.push(OHLCV {
            open,
            high,
            low,
            close,
            volume,
            timestamp: i as i64,
        });
        
        price = close;
    }
    
    data
}

fn streaming_performance_comparison(data: &[OHLCV]) {
    let closes = extract_closes(data);
    let iterations = 1000;
    
    println!("Running {} iterations on {} candles (Streaming simulation)...\n", iterations, data.len());
    
    println!("┌─────────────────────┬──────────────┬──────────────┬──────────────┐");
    println!("│ Streaming Indicator │ O(N) Buffer  │ O(1) Stateful│ Speedup      │");
    println!("├─────────────────────┼──────────────┼──────────────┼──────────────┤");

    // Buffer-based streaming (current implementation)
    let start = Instant::now();
    for _ in 0..iterations {
        let mut sim_hma = StreamingHMA::new(20, 20);
        for &price in &closes {
            sim_hma.update(price);
        }
    }
    let sim_hma_time = start.elapsed() / iterations as u32;

    // Stateful streaming (O(1) implementation)
    let start = Instant::now();
    for _ in 0..iterations {
        let mut stateful_hma = StatefulHMA::new(20);
        for &price in &closes {
            stateful_hma.update(price);
        }
    }
    let stateful_hma_time = start.elapsed() / iterations as u32;
    let stateful_speedup = sim_hma_time.as_nanos() as f64 / stateful_hma_time.as_nanos() as f64;
    
    println!("│ HMA(20)             │ {:>10.2?} │ {:>10.2?} │ {:>10.2}x │", 
             sim_hma_time, stateful_hma_time, stateful_speedup);
             
    println!("└─────────────────────┴──────────────┴──────────────┴──────────────┘");

    let candles_per_sec_buffer = (data.len() as f64) / sim_hma_time.as_secs_f64();
    let candles_per_sec_stateful = (data.len() as f64) / stateful_hma_time.as_secs_f64();

    println!("\n🚀 Streaming Throughput:");
    println!("  Buffer-based O(N):  {:.0} prices/sec", candles_per_sec_buffer);
    println!("  Stateful O(1):      {:.0} prices/sec", candles_per_sec_stateful);
}

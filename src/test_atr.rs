use turbo_indicators::{OHLCV, StatefulATR};

fn main() {
    let candles = vec![
        OHLCV { timestamp: 0, open: 34389.4641, high: 34389.4641, low: 34367.5050, close: 34372.1374, volume: 0.0 }, // 14:35
        OHLCV { timestamp: 1, open: 34391.3298, high: 34414.3409, low: 34365.7621, close: 34405.5426, volume: 0.0 }, // 14:36
        OHLCV { timestamp: 2, open: 34414.5977, high: 34433.9513, low: 34386.8184, close: 34390.2482, volume: 0.0 }, // 14:37
        OHLCV { timestamp: 3, open: 34397.6779, high: 34454.2674, low: 34397.6779, close: 34426.9034, volume: 0.0 }, // 14:38
        OHLCV { timestamp: 4, open: 34421.4158, high: 34471.5194, low: 34415.3832, close: 34451.1907, volume: 0.0 }, // 14:39
        OHLCV { timestamp: 5, open: 34437.9948, high: 34437.9948, low: 34364.7384, close: 34382.1125, volume: 0.0 }, // 14:40
        OHLCV { timestamp: 6, open: 34393.9653, high: 34404.3072, low: 34377.5239, close: 34383.7665, volume: 0.0 }, // 14:41
        OHLCV { timestamp: 7, open: 34385.9967, high: 34456.5516, low: 34379.2296, close: 34452.4645, volume: 0.0 }, // 14:42
    ];

    let atr_period = 7;
    let atr_multi = 1.3;

    let mut stateful_atr = StatefulATR::new(atr_period);
    for candle in &candles {
        let range = candle.high - candle.low;
        if let Some(atr) = stateful_atr.update(candle) {
            let is_atr = atr > 0.0 && range > atr * atr_multi;
            println!("Candle {}: Range={:.4}, ATR={:.4}, Threshold={:.4}, is_atr={}", 
                candle.timestamp, range, atr, atr * atr_multi, is_atr);
        } else {
            println!("Candle {}: Range={:.4}, ATR=warming up", candle.timestamp, range);
        }
    }
}

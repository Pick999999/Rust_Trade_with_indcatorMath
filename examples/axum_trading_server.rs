use axum::{
    routing::{get, post},
    Router, Json, extract::State, response::Html,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use turbo_indicators::{IndicatorEngine, OHLCV};

#[derive(Deserialize, Debug, Clone)]
pub struct CandleInput {
    pub symbol: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub timestamp: i64,
}

#[derive(Serialize)]
pub struct TradeSignalOutput {
    pub timestamp: i64,
    pub price: f64,
    pub rsi: Option<f64>,
    pub atr: Option<f64>,
    pub hma: Option<f64>,
    pub macd: Option<(f64, f64, f64)>,
    pub signal: String, 
}

struct AppState {
    engine: Mutex<IndicatorEngine>,
}

#[tokio::main]
async fn main() {
    let engine = IndicatorEngine::new(
        20,            // HMA
        (12, 26, 9),   // MACD
        14,            // RSI
        14,            // Choppy
        14,            // ATR
        200,           // Buffer Capacity
    );
    let state = Arc::new(AppState {
        engine: Mutex::new(engine),
    });

    let app = Router::new()
        .route("/", get(index_html))
        .route("/api/tick", post(handle_tick))
        .with_state(state);

    println!("🚀 Starting Stateful Trading Server at http://0.0.0.0:3000/ (Access via your EC2 Public IP)");
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index_html() -> Html<&'static str> {
    Html(include_str!("trading_ui.html"))
}

async fn handle_tick(
    State(state): State<Arc<AppState>>,
    Json(candle): Json<CandleInput>,
) -> Json<TradeSignalOutput> {
    let ohlcv = OHLCV {
        open: candle.open,
        high: candle.high,
        low: candle.low,
        close: candle.close,
        volume: candle.volume,
        timestamp: candle.timestamp,
    };

    let mut engine = state.engine.lock();
    let snapshot = engine.update(ohlcv);

    let mut signal = "NEUTRAL".to_string();
    
    // Simple stateful trading logic based on RSI and HMA
    if let (Some(rsi), Some(hma)) = (snapshot.rsi, snapshot.hma) {
        if rsi < 35.0 && snapshot.price > hma {
            signal = "LONG (RSI Oversold + HMA Crossover)".to_string();
        } else if rsi > 65.0 && snapshot.price < hma {
            signal = "SHORT (RSI Overbought + HMA Crossunder)".to_string();
        }
    }

    Json(TradeSignalOutput {
        timestamp: candle.timestamp,
        price: snapshot.price,
        rsi: snapshot.rsi,
        atr: snapshot.atr,
        hma: snapshot.hma,
        macd: snapshot.macd,
        signal,
    })
}

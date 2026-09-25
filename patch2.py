import re

def patch_file():
    with open("src/deriv.rs", "r", encoding="utf-8") as f:
        content = f.read()

    # 3. Update start_deriv_bot_multiplexed
    # Using regex to find the start and end precisely
    pattern = re.compile(
        r'pub async fn start_deriv_bot_multiplexed\(.*?'
        r'msg_result = ws_stream\.next\(\) => \{\s*'
        r'let msg_data = match msg_result \{\s*'
        r'Some\(Ok\(m\)\) => m,\s*'
        r'_ => break,\s*'
        r'\};\s*',
        re.DOTALL
    )

    new_start_bot = """pub async fn start_deriv_bot_multiplexed(
    app_id: String,
    api_token: String,
    account_id: String,
    config: AppConfigPayload,
    assets: Vec<String>,
    tx: Arc<broadcast::Sender<serde_json::Value>>,
    mut cmd_rx: broadcast::Receiver<serde_json::Value>,
    overtime_signals: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use futures_util::StreamExt;
    
    let public_ws_url = format!("wss://ws.derivws.com/websockets/v3?app_id={}", app_id);
    println!(
        "\\n🔌 [Multiplex] กำลังเชื่อมต่อ Deriv WebSocket สำหรับ {} assets...",
        assets.len()
    );

    let client = reqwest::Client::new();
    let otp_url = format!("https://api.derivws.com/trading/v1/options/accounts/{}/otp", account_id);
    let res = client.post(&otp_url)
        .header("Authorization", format!("Bearer {}", api_token))
        .header("Deriv-App-ID", app_id.clone())
        .header("Content-Type", "application/json")
        .send()
        .await?;
        
    if !res.status().is_success() {
        return Err(format!("Failed to get OTP: {}", res.status()).into());
    }
    
    let otp_data: serde_json::Value = res.json().await?;
    let private_ws_url = otp_data.get("data").and_then(|d| d.get("url")).and_then(|u| u.as_str())
        .ok_or_else(|| "Failed to parse OTP WebSocket URL")?;

    let (public_ws_stream, _) = connect_async(&public_ws_url).await?;
    let (private_ws_stream, _) = connect_async(private_ws_url).await?;
    
    let (mut public_write, mut public_read) = public_ws_stream.split();
    let (mut private_write, mut private_read) = private_ws_stream.split();

    println!("✅ [Multiplex] เชื่อมต่อ WebSocket สำเร็จ (Public & Private)!");

    let mut asset_states: HashMap<String, AssetState> = HashMap::new();
    let mut req_id_to_asset: HashMap<i64, String> = HashMap::new();
    for (idx, asset) in assets.iter().enumerate() {
        let base_req_id = (idx as i64 + 1) * 1000;
        let mut state = AssetState::new(asset.clone(), config.granularity);
        state.req_id = base_req_id;
        println!(
            "📋 [Multiplex] Asset: {} → symbol: {} | req_id_base: {} (ohlc:{}, proposal:{}, buy:{}, track:{})",
            asset, state.deriv_symbol, state.req_id,
            base_req_id, base_req_id + 1, base_req_id + 2, base_req_id + 3
        );
        req_id_to_asset.insert(base_req_id, asset.clone());
        req_id_to_asset.insert(base_req_id + 1, asset.clone());
        req_id_to_asset.insert(base_req_id + 2, asset.clone());
        req_id_to_asset.insert(base_req_id + 3, asset.clone());
        asset_states.insert(asset.clone(), state);
    }

    let req = serde_json::json!({ "balance": 1, "subscribe": 1, "req_id": 1 });
    private_write.send(Message::Text(req.to_string())).await?;
    let tx_sub = serde_json::json!({ "transaction": 1, "subscribe": 1, "req_id": 11111 });
    private_write.send(Message::Text(tx_sub.to_string())).await?;

    let mut _global_balance = 0.0;
    let candle_count: i32 = std::env::var("CANDLE_COUNT")
        .unwrap_or_else(|_| "1000".to_string())
        .parse()
        .unwrap_or(1000);

    for (asset, state) in asset_states.iter_mut() {
        let ticks_request = serde_json::json!({
            "ticks_history": state.deriv_symbol,
            "end": "latest",
            "style": "candles",
            "granularity": config.granularity,
            "count": candle_count,
            "subscribe": 1,
            "req_id": state.req_id
        });
        println!("📦 [{}] ร้องขอข้อมูลแท่งเทียน (Public)", asset);
        public_write.send(Message::Text(ticks_request.to_string())).await?;
    }

    loop {
        let msg_data = tokio::select! {
            cmd_result = cmd_rx.recv() => {
                match cmd_result {
                    Ok(cmd) => {
                        if cmd["command"] == "sell" {
                            if let Some(cid) = cmd["contract_id"].as_i64() {
                                let sell_req = serde_json::json!({ "sell": cid, "price": 0 });
                                let _ = private_write.send(Message::Text(sell_req.to_string())).await;
                                println!("📤 [Multiplex] ส่งคำสั่งขาย Contract ID: {}", cid);
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        println!("⚠️ [Multiplex] Command channel closed. Exiting bot loop.");
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        println!("⚠️ [Multiplex] Command channel lagged by {} messages.", n);
                    }
                }
                continue;
            }
            public_msg = public_read.next() => {
                match public_msg {
                    Some(Ok(m)) => m,
                    _ => break,
                }
            }
            private_msg = private_read.next() => {
                match private_msg {
                    Some(Ok(m)) => m,
                    _ => break,
                }
            }
        };
"""

    if pattern.search(content):
        content = pattern.sub(new_start_bot, content)
        print("Replaced start_bot chunk!")
    else:
        print("Regex failed to find start_bot chunk!")

    with open("src/deriv.rs", "w", encoding="utf-8") as f:
        f.write(content)

if __name__ == "__main__":
    patch_file()

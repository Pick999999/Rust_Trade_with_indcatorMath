import re

def patch_file():
    with open("src/deriv.rs", "r", encoding="utf-8") as f:
        content = f.read()

    # 1. Update get_deriv_balance
    new_balance_func = """pub async fn get_deriv_balance(
    api_token: &str,
) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
    let app_id = env::var("DERIV_APP_ID").unwrap_or_else(|_| "1089".to_string());
    let account_id = env::var("DERIV_ACCOUNT_ID").unwrap_or_else(|_| "".to_string());

    if account_id.is_empty() {
        return Err("DERIV_ACCOUNT_ID is not set".into());
    }

    let otp_url = format!("https://api.derivws.com/trading/v1/options/accounts/{}/otp", account_id);
    
    let client = reqwest::Client::new();
    let res = client.post(&otp_url)
        .header("Authorization", format!("Bearer {}", api_token))
        .header("Deriv-App-ID", app_id)
        .header("Content-Type", "application/json")
        .send()
        .await?;
        
    if !res.status().is_success() {
        return Err(format!("Failed to get OTP: {}", res.status()).into());
    }
    
    let otp_data: serde_json::Value = res.json().await?;
    let ws_url = otp_data.get("data").and_then(|d| d.get("url")).and_then(|u| u.as_str())
        .ok_or_else(|| "Failed to parse OTP WebSocket URL")?;

    let (mut ws_stream, _) = connect_async(ws_url).await?;
    let req = serde_json::json!({ "balance": 1, "subscribe": 1, "req_id": 1 });
    ws_stream.send(Message::Text(req.to_string())).await?;

    while let Some(msg_result) = ws_stream.next().await {
        let msg = msg_result?;
        if let Message::Text(text) = msg {
            let parsed: serde_json::Value = serde_json::from_str(&text)?;
            if let Some(error) = parsed.get("error") {
                return Err(error.get("message").and_then(|v| v.as_str()).unwrap_or("Deriv Error").into());
            }
            if let Some(bal_data) = parsed.get("balance") {
                if let Some(bal) = json_to_f64(bal_data.get("balance").unwrap_or(&serde_json::json!(0.0))) {
                    return Ok(bal);
                }
            }
        }
    }
    Err("Failed to get balance from Deriv".into())
}"""
    
    old_balance_func = re.search(r'pub async fn get_deriv_balance.*?\n\}', content, re.DOTALL).group(0)
    content = content.replace(old_balance_func, new_balance_func)

    # 2. Update fetch_historical_candles URL
    content = content.replace(
        'let ws_url = format!("wss://ws.binaryws.com/websockets/v3?app_id={}", app_id);',
        'let ws_url = format!("wss://ws.derivws.com/websockets/v3?app_id={}", app_id);'
    )

    # 3. Update start_deriv_bot_multiplexed
    old_start_bot = """pub async fn start_deriv_bot_multiplexed(
    app_id: String,
    api_token: String,
    config: AppConfigPayload,
    assets: Vec<String>,
    tx: Arc<broadcast::Sender<serde_json::Value>>,
    mut cmd_rx: broadcast::Receiver<serde_json::Value>,
    overtime_signals: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_url = format!("wss://ws.binaryws.com/websockets/v3?app_id={}", app_id);
    println!(
        "\\n🔌 [Multiplex] กำลังเชื่อมต่อ Deriv WebSocket สำหรับ {} assets...",
        assets.len()
    );

    let (mut ws_stream, _) = connect_async(&ws_url).await?;
    println!("✅ [Multiplex] เชื่อมต่อ WebSocket สำเร็จ!");

    let mut asset_states: HashMap<String, AssetState> = HashMap::new();
    let mut req_id_to_asset: HashMap<i64, String> = HashMap::new();
    for (idx, asset) in assets.iter().enumerate() {
        let base_req_id = (idx as i64 + 1) * 1000; // 1000, 2000, 3000...
        let mut state = AssetState::new(asset.clone(), config.granularity);
        state.req_id = base_req_id;
        println!(
            "📋 [Multiplex] Asset: {} → symbol: {} | req_id_base: {} (ohlc:{}, proposal:{}, buy:{}, track:{})",
            asset, state.deriv_symbol, state.req_id,
            base_req_id, base_req_id + 1, base_req_id + 2, base_req_id + 3
        );
        // Register ALL req_id offsets for this asset (เหมือน JS ที่ใช้ reqId++ แยกกัน)
        req_id_to_asset.insert(base_req_id, asset.clone());      // +0 = ohlc/ticks_history
        req_id_to_asset.insert(base_req_id + 1, asset.clone());  // +1 = proposal
        req_id_to_asset.insert(base_req_id + 2, asset.clone());  // +2 = buy
        req_id_to_asset.insert(base_req_id + 3, asset.clone());  // +3 = proposal_open_contract (track)
        asset_states.insert(asset.clone(), state);
    }

    let auth_request = json!({ "authorize": api_token });
    ws_stream
        .send(Message::Text(auth_request.to_string()))
        .await?;

    loop {
        tokio::select! {
            cmd_result = cmd_rx.recv() => {
                match cmd_result {
                    Ok(cmd) => {
                        if cmd["command"] == "sell" {
                            if let Some(cid) = cmd["contract_id"].as_i64() {
                                let sell_req = json!({ "sell": cid, "price": 0 });
                                let _ = ws_stream.send(Message::Text(sell_req.to_string())).await;
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
            }
            msg_result = ws_stream.next() => {
                let msg_data = match msg_result {
                    Some(Ok(m)) => m,
                    _ => break,
                };"""
    
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
        let base_req_id = (idx as i64 + 1) * 1000; // 1000, 2000, 3000...
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

    // Trigger flow on private WS
    let req = serde_json::json!({ "balance": 1, "subscribe": 1, "req_id": 1 });
    private_write.send(Message::Text(req.to_string())).await?;
    let tx_sub = serde_json::json!({ "transaction": 1, "subscribe": 1, "req_id": 11111 });
    private_write.send(Message::Text(tx_sub.to_string())).await?;

    // Since we don't send authorize anymore, we manually trigger authorize flow to start fetching candles
    let mut _global_balance = 0.0;
    let candle_count: i32 = env::var("CANDLE_COUNT")
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
        };"""

    if old_start_bot in content:
        content = content.replace(old_start_bot, new_start_bot)
    else:
        print("Could not find start_bot chunk to replace!")
        # Let's write output anyway just in case

    # Update usages of ws_stream.send in the giant loop block
    content = content.replace("ws_stream.send(Message::Text(sell_req.to_string())).await?;", "private_write.send(Message::Text(sell_req.to_string())).await?;")
    content = content.replace("ws_stream.send(Message::Text(sell_req.to_string())).await;", "private_write.send(Message::Text(sell_req.to_string())).await;")
    content = content.replace("ws_stream\n                        .send(Message::Text(ticks_request.to_string()))\n                        .await?;", "public_write\n                        .send(Message::Text(ticks_request.to_string()))\n                        .await?;")
    content = content.replace("ws_stream.send(Message::Text(ticks_request.to_string())).await?;", "public_write.send(Message::Text(ticks_request.to_string())).await?;")
    content = content.replace("ws_stream.send(Message::Text(tx_sub.to_string())).await?;", "private_write.send(Message::Text(tx_sub.to_string())).await?;")
    content = content.replace("ws_stream.send(Message::Text(query.to_string())).await?;", "private_write.send(Message::Text(query.to_string())).await?;")
    
    content = content.replace("ws_stream\n                                                .send(Message::Text(query.to_string()))\n                                                .await?;", "private_write\n                                                .send(Message::Text(query.to_string()))\n                                                .await?;")

    content = content.replace("ws_stream\n                                                .send(Message::Text(proposal_request.to_string()))\n                                                .await?;", "private_write\n                                                .send(Message::Text(proposal_request.to_string()))\n                                                .await?;")
    content = content.replace("ws_stream.send(Message::Text(proposal_request.to_string())).await?;", "private_write.send(Message::Text(proposal_request.to_string())).await?;")

    content = content.replace("ws_stream\n                                                .send(Message::Text(buy_request.to_string()))\n                                                .await?;", "private_write\n                                                .send(Message::Text(buy_request.to_string()))\n                                                .await?;")
    content = content.replace("ws_stream.send(Message::Text(buy_request.to_string())).await?;", "private_write.send(Message::Text(buy_request.to_string())).await?;")

    content = content.replace("ws_stream\n                                    .send(Message::Text(buy_request.to_string()))\n                                    .await?;", "private_write\n                                    .send(Message::Text(buy_request.to_string()))\n                                    .await?;")

    content = content.replace("ws_stream\n                                    .send(Message::Text(forget_req.to_string()))\n                                    .await?;", "private_write\n                                    .send(Message::Text(forget_req.to_string()))\n                                    .await?;")
    content = content.replace("ws_stream.send(Message::Text(forget_req.to_string())).await?;", "private_write.send(Message::Text(forget_req.to_string())).await?;")

    content = content.replace("ws_stream\n                            .send(Message::Text(sub_proposal.to_string()))\n                            .await?;", "private_write\n                            .send(Message::Text(sub_proposal.to_string()))\n                            .await?;")
    content = content.replace("ws_stream.send(Message::Text(sub_proposal.to_string())).await?;", "private_write.send(Message::Text(sub_proposal.to_string())).await?;")

    # The block below is no longer needed because we send ticks_request manually during init
    # But just in case, let's remove the authorize block logic to avoid issues.
    # Actually, authorize might still come back in some responses? No, we don't send authorize.
    # But let's just leave the `if let Some(auth_data) = parsed.get("authorize")` untouched. It just won't be triggered.

    # 4. Replace `symbol` with `underlying_symbol` in proposal and buy request payloads.
    content = content.replace('"symbol": state.deriv_symbol,', '"underlying_symbol": state.deriv_symbol,')
    content = content.replace('"symbol": state.deriv_symbol', '"underlying_symbol": state.deriv_symbol')

    with open("src/deriv.rs", "w", encoding="utf-8") as f:
        f.write(content)

    print("Success")

if __name__ == "__main__":
    patch_file()

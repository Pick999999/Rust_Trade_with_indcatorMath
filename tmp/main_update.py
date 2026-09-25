import re

def main():
    with open('src/main.rs', 'r', encoding='utf-8') as f:
        code = f.read()

    # Chunk 1
    code = code.replace(
        "pub active_bots: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,",
        "pub active_bot: Arc<Mutex<Option<JoinHandle<()>>>>,"
    )

    # Chunk 8
    code = code.replace(
        "active_bots: Arc::new(Mutex::new(HashMap::new())),",
        "active_bot: Arc::new(Mutex::new(None)),"
    )

    # Chunk 2
    code = code.replace(
        '''    let mut bot_guard = state.active_bots.lock().await;
    let mut ot_guard = state.overtime_signals.lock().await;
    
    // ปิดบอทตัวเก่าทั้งหมดก่อน (ถ้ามีรันอยู่)
    for (asset_name, handle) in bot_guard.drain() {
        handle.abort();
        println!("🛑 Aborted bot: {}", asset_name);
    }
    ot_guard.clear();

    // Spawn bot แยกตัวต่อ Asset
    let mut spawned_assets = Vec::new();
    for asset in payload.assets.iter() {
        let tx = state.tx.clone();
        let app_id = app_id.clone();
        let api_token = api_token.clone();
        let config = payload.clone();
        let asset_clone = asset.clone();
        let ot_signal = Arc::new(AtomicBool::new(false));
        ot_guard.insert(asset.clone(), ot_signal.clone());

        let handle = tokio::spawn(async move {
            if let Err(e) = deriv::start_deriv_bot(app_id, api_token, config, asset_clone.clone(), tx, ot_signal).await {
                eprintln!("Bot [{}] crashed: {}", asset_clone, e);
            }
        });

        spawned_assets.push(asset.clone());
        bot_guard.insert(asset.clone(), handle);
    }

    let assets_str = spawned_assets.join(", ");
    println!("🚀 Spawned {} bot(s): {:?}", spawned_assets.len(), spawned_assets);''',
        '''    let mut bot_guard = state.active_bot.lock().await;
    let mut ot_guard = state.overtime_signals.lock().await;
    
    // ปิดบอทตัวเก่าทั้งหมดก่อน (ถ้ามีรันอยู่)
    if let Some(handle) = bot_guard.take() {
        handle.abort();
        println!("🛑 Aborted old multiplexed bot");
    }
    ot_guard.clear();

    let mut spawned_assets = Vec::new();
    for asset in payload.assets.iter() {
        let ot_signal = Arc::new(AtomicBool::new(false));
        ot_guard.insert(asset.clone(), ot_signal);
        spawned_assets.push(asset.clone());
    }

    let tx = state.tx.clone();
    let app_id = app_id.clone();
    let api_token = api_token.clone();
    let config = payload.clone();
    let assets = spawned_assets.clone();
    let ot_signals = state.overtime_signals.clone();

    let handle = tokio::spawn(async move {
        if let Err(e) = deriv::start_deriv_bot_multiplexed(app_id, api_token, config, assets, tx, ot_signals).await {
            eprintln!("Multiplexed bot crashed: {}", e);
        }
    });

    *bot_guard = Some(handle);

    let assets_str = spawned_assets.join(", ");
    println!("🚀 Spawned 1 multiplexed bot for {} asset(s): {:?}", spawned_assets.len(), spawned_assets);'''
    )

    # Chunk 3
    code = code.replace(
        '''async fn handle_post_stop(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut bot_guard = state.active_bots.lock().await;
    let count = bot_guard.len();
    let mut stopped_assets = Vec::new();
    for (asset_name, handle) in bot_guard.drain() {
        handle.abort();
        stopped_assets.push(asset_name.clone());
        println!("🛑 หยุดบอท: {}", asset_name);
    }
    println!("🛑 หยุดบอททั้งหมด {} ตัว", count);

    // ส่ง Telegram แจ้งเตือนเมื่อหยุดเทรด
    let now_str = Local::now().format("%d/%m/%Y %H:%M:%S").to_string();
    send_telegram_message(&format!(
        "🔴 หยุดเทรด!\\n📊 Assets: {}\\n⏰ เวลา: {}\\n📊 หยุดบอททั้งหมด {} ตัว",
        stopped_assets.join(", "), now_str, count
    )).await;

    Json(serde_json::json!({
        "status": "success",
        "message": format!("หยุดบอททั้งหมด {} ตัวแล้ว", count)
    }))
}''',
        '''async fn handle_post_stop(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut bot_guard = state.active_bot.lock().await;
    let count = if bot_guard.is_some() { 1 } else { 0 };
    if let Some(handle) = bot_guard.take() {
        handle.abort();
        println!("🛑 หยุดบอท Multiplexed");
    }
    println!("🛑 หยุดบอททั้งหมด {} ตัว", count);

    // ส่ง Telegram แจ้งเตือนเมื่อหยุดเทรด
    let now_str = Local::now().format("%d/%m/%Y %H:%M:%S").to_string();
    send_telegram_message(&format!(
        "🔴 หยุดเทรด!\\n⏰ เวลา: {}\\n📊 หยุดบอททั้งหมด {} ตัว",
        now_str, count
    )).await;

    Json(serde_json::json!({
        "status": "success",
        "message": format!("หยุดบอททั้งหมด {} ตัวแล้ว", count)
    }))
}'''
    )

    # Chunk 4
    code = code.replace(
        '''async fn handle_post_terminate(State(state): State<AppState>) -> Json<serde_json::Value> {
    // ปิดบอททั้งหมดก่อน
    let mut bot_guard = state.active_bots.lock().await;
    for (asset_name, handle) in bot_guard.drain() {
        handle.abort();
        println!("🛑 Terminate: หยุดบอท {}", asset_name);
    }''',
        '''async fn handle_post_terminate(State(state): State<AppState>) -> Json<serde_json::Value> {
    // ปิดบอททั้งหมดก่อน
    let mut bot_guard = state.active_bot.lock().await;
    if let Some(handle) = bot_guard.take() {
        handle.abort();
        println!("🛑 Terminate: หยุดบอท Multiplexed");
    }'''
    )

    # Chunk 5
    code = code.replace(
        '''                let mut bot_guard = state.active_bots.lock().await;

                // Stop any old stray bots running manually
                for (asset_name, handle) in bot_guard.drain() {
                    handle.abort();
                }
                {
                    let mut ot_guard = state.overtime_signals.lock().await;
                    ot_guard.clear();
                }

                let mut spawned_assets = Vec::new();
                for asset in config.assets.iter() {
                    let tx = state.tx.clone();
                    let app_id_clone = app_id.clone();
                    let api_token_clone = api_token.clone();
                    let config_clone = config.clone();
                    let asset_clone = asset.clone();
                    let ot_signal = Arc::new(AtomicBool::new(false));
                    {
                        let mut ot_guard = state.overtime_signals.lock().await;
                        ot_guard.insert(asset.clone(), ot_signal.clone());
                    }

                    let handle = tokio::spawn(async move {
                        if let Err(e) = deriv::start_deriv_bot(app_id_clone, api_token_clone, config_clone, asset_clone.clone(), tx, ot_signal).await {
                            eprintln!("Bot [{}] crashed: {}", asset_clone, e);
                        }
                    });

                    spawned_assets.push(asset.clone());
                    bot_guard.insert(asset.clone(), handle);
                }
                
                let assets_str = config.assets.join(", ");''',
        '''                let mut bot_guard = state.active_bot.lock().await;

                // Stop any old stray bots running manually
                if let Some(handle) = bot_guard.take() {
                    handle.abort();
                }
                {
                    let mut ot_guard = state.overtime_signals.lock().await;
                    ot_guard.clear();
                }

                let mut spawned_assets = Vec::new();
                for asset in config.assets.iter() {
                    let ot_signal = Arc::new(AtomicBool::new(false));
                    {
                        let mut ot_guard = state.overtime_signals.lock().await;
                        ot_guard.insert(asset.clone(), ot_signal);
                    }
                    spawned_assets.push(asset.clone());
                }

                let tx = state.tx.clone();
                let app_id_clone = app_id.clone();
                let api_token_clone = api_token.clone();
                let config_clone = config.clone();
                let assets_clone = spawned_assets.clone();
                let ot_signals = state.overtime_signals.clone();

                let handle = tokio::spawn(async move {
                    if let Err(e) = deriv::start_deriv_bot_multiplexed(app_id_clone, api_token_clone, config_clone, assets_clone, tx, ot_signals).await {
                        eprintln!("Multiplexed bot crashed: {}", e);
                    }
                });

                *bot_guard = Some(handle);
                
                let assets_str = config.assets.join(", ");'''
    )

    # Chunk 5.1 (auto_schedule_trade function)
    code = code.replace(
        '''        let mut bot_guard = state.active_bots.lock().await;
        let mut ot_guard = state.overtime_signals.lock().await;

        // ปิดบอทตัวเก่าทั้งหมดก่อน (ถ้ามีรันอยู่)
        for (asset_name, handle) in bot_guard.drain() {
            handle.abort();
            println!("🛑 Scheduler: Aborted old bot: {}", asset_name);
        }
        ot_guard.clear();

        let mut spawned_assets = Vec::new();
        for asset in config.assets.iter() {
            let tx = state.tx.clone();
            let app_id = app_id.clone();
            let api_token = api_token.clone();
            let config_clone = config.clone();
            let asset_clone = asset.clone();
            let ot_signal = Arc::new(AtomicBool::new(false));
            ot_guard.insert(asset.clone(), ot_signal.clone());

            let handle = tokio::spawn(async move {
                if let Err(e) = deriv::start_deriv_bot(app_id, api_token, config_clone, asset_clone.clone(), tx, ot_signal).await {
                    eprintln!("Bot [{}] crashed: {}", asset_clone, e);
                }
            });

            spawned_assets.push(asset.clone());
            bot_guard.insert(asset.clone(), handle);
        }

        println!("🚀 Scheduler: Spawned {} bot(s): {:?}", spawned_assets.len(), spawned_assets);''',
        '''        let mut bot_guard = state.active_bot.lock().await;
        let mut ot_guard = state.overtime_signals.lock().await;

        // ปิดบอทตัวเก่าทั้งหมดก่อน (ถ้ามีรันอยู่)
        if let Some(handle) = bot_guard.take() {
            handle.abort();
            println!("🛑 Scheduler: Aborted old multiplexed bot");
        }
        ot_guard.clear();

        let mut spawned_assets = Vec::new();
        for asset in config.assets.iter() {
            let ot_signal = Arc::new(AtomicBool::new(false));
            ot_guard.insert(asset.clone(), ot_signal);
            spawned_assets.push(asset.clone());
        }

        let tx = state.tx.clone();
        let app_id = app_id.clone();
        let api_token = api_token.clone();
        let config_clone = config.clone();
        let assets_clone = spawned_assets.clone();
        let ot_signals = state.overtime_signals.clone();

        let handle = tokio::spawn(async move {
            if let Err(e) = deriv::start_deriv_bot_multiplexed(app_id, api_token, config_clone, assets_clone, tx, ot_signals).await {
                eprintln!("Multiplexed bot crashed: {}", e);
            }
        });

        *bot_guard = Some(handle);

        println!("🚀 Scheduler: Spawned 1 multiplexed bot for {} asset(s): {:?}", spawned_assets.len(), spawned_assets);'''
    )

    # Chunk 6.1 (auto_schedule_trade overtime wait)
    code = code.replace(
        '''                // Poll รอจนกว่าบอททุกตัวจบเอง (JoinHandle is_finished)
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    let bot_guard = state.active_bots.lock().await;
                    let still_running: Vec<String> = bot_guard.iter()
                        .filter(|(_, h)| !h.is_finished())
                        .map(|(name, _)| name.clone())
                        .collect();
                    drop(bot_guard);
                    if still_running.is_empty() {
                        println!("✅ ทุก bot จบแล้ว (overtime completed)");
                        break;
                    }
                    println!("⏳ Overtime: ยังรอ {} bot(s): {:?}", still_running.len(), still_running);
                }
                // Cleanup
                let mut bot_guard = state.active_bots.lock().await;
                bot_guard.drain();
            } else {
                // ไม่ใช้ Martingale → abort ทันทีเหมือนเดิม
                let mut bot_guard = state.active_bots.lock().await;
                let count = bot_guard.len();
                for (asset_name, handle) in bot_guard.drain() {
                    handle.abort();
                    println!("🛑 Scheduler: หยุดบอท: {}", asset_name);
                }''',
        '''                // Poll รอจนกว่าบอทจบเอง
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    let bot_guard = state.active_bot.lock().await;
                    let is_running = bot_guard.as_ref().map_or(false, |h| !h.is_finished());
                    drop(bot_guard);
                    if !is_running {
                        println!("✅ ทุก bot จบแล้ว (overtime completed)");
                        break;
                    }
                    println!("⏳ Overtime: ยังรอ Multiplexed bot");
                }
                // Cleanup
                let mut bot_guard = state.active_bot.lock().await;
                *bot_guard = None;
            } else {
                // ไม่ใช้ Martingale → abort ทันทีเหมือนเดิม
                let mut bot_guard = state.active_bot.lock().await;
                let count = if bot_guard.is_some() { 1 } else { 0 };
                if let Some(handle) = bot_guard.take() {
                    handle.abort();
                    println!("🛑 Scheduler: หยุดบอท Multiplexed");
                }'''
    )

    # Chunk 6.2 (background_scheduler_loop overtime wait)
    code = code.replace(
        '''                // Poll รอจนกว่าบอททุกตัวจบเอง
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    let bot_guard = state.active_bots.lock().await;
                    let still_running: Vec<String> = bot_guard.iter()
                        .filter(|(_, h)| !h.is_finished())
                        .map(|(name, _)| name.clone())
                        .collect();
                    drop(bot_guard);
                    if still_running.is_empty() {
                        println!("✅ Scheduler: ทุก bot จบแล้ว (overtime completed)");
                        break;
                    }
                    println!("⏳ Scheduler Overtime: ยังรอ {} bot(s): {:?}", still_running.len(), still_running);
                }
                // Cleanup
                let mut bot_guard = state.active_bots.lock().await;
                bot_guard.drain();

                send_telegram_message(&format!("🔴 OVERTIME จบ! ทุก asset Win แล้ว\\n⏰ {}", Local::now().format("%d/%m/%Y %H:%M:%S"))).await;
            } else {
                // ไม่ใช้ Martingale → abort ทันที
                let mut bot_guard = state.active_bots.lock().await;
                let count = bot_guard.len();
                for (asset_name, handle) in bot_guard.drain() {
                    handle.abort();
                }''',
        '''                // Poll รอจนกว่าบอทจบเอง
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    let bot_guard = state.active_bot.lock().await;
                    let is_running = bot_guard.as_ref().map_or(false, |h| !h.is_finished());
                    drop(bot_guard);
                    if !is_running {
                        println!("✅ Scheduler: Multiplexed bot จบแล้ว (overtime completed)");
                        break;
                    }
                    println!("⏳ Scheduler Overtime: ยังรอ Multiplexed bot");
                }
                // Cleanup
                let mut bot_guard = state.active_bot.lock().await;
                *bot_guard = None;

                send_telegram_message(&format!("🔴 OVERTIME จบ! ทุก asset Win แล้ว\\n⏰ {}", Local::now().format("%d/%m/%Y %H:%M:%S"))).await;
            } else {
                // ไม่ใช้ Martingale → abort ทันที
                let mut bot_guard = state.active_bot.lock().await;
                let count = if bot_guard.is_some() { 1 } else { 0 };
                if let Some(handle) = bot_guard.take() {
                    handle.abort();
                }'''
    )

    # Chunk 7
    code = code.replace(
        '''async fn handle_get_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let bot_guard = state.active_bots.lock().await;
    let active_assets: Vec<String> = bot_guard.keys().cloned().collect();
    let is_trading = !active_assets.is_empty();
    let log_count = state.bot_logs.lock().await.len();
    let candle_assets: Vec<String> = state.candle_data.lock().await.keys().cloned().collect();

    Json(serde_json::json!({
        "is_trading": is_trading,
        "active_assets": active_assets,
        "bot_count": active_assets.len(),
        "log_count": log_count,
        "candle_assets": candle_assets
    }))
}''',
        '''async fn handle_get_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let bot_guard = state.active_bot.lock().await;
    let is_trading = bot_guard.is_some();
    let ot_guard = state.overtime_signals.lock().await;
    let active_assets: Vec<String> = if is_trading { ot_guard.keys().cloned().collect() } else { vec![] };
    let log_count = state.bot_logs.lock().await.len();
    let candle_assets: Vec<String> = state.candle_data.lock().await.keys().cloned().collect();

    Json(serde_json::json!({
        "is_trading": is_trading,
        "active_assets": active_assets,
        "bot_count": if is_trading { 1 } else { 0 },
        "log_count": log_count,
        "candle_assets": candle_assets
    }))
}'''
    )

    with open('src/main.rs', 'w', encoding='utf-8') as f:
        f.write(code)

if __name__ == '__main__':
    main()

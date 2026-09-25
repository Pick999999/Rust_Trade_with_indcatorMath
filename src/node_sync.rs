use axum::{
    extract::{
        ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade},
        Query,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as TungsteniteMessage};

// ═══════════════════════════════════════════════════════════
// 1. DATA MODELS & MESSAGE SCHEMA
// ═══════════════════════════════════════════════════════════

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeBalanceState {
    pub node_name: String,
    pub account_id: String,
    pub balance: f64,
    pub round_profit_usd: f64,
    pub round_profit_thb: f64,
    pub today_profit_usd: f64,
    pub today_profit_thb: f64,
    pub win_count: u32,
    pub loss_count: u32,
    pub is_trading: bool,
    pub last_seen: u64,
    pub is_online: bool,
    #[serde(default)]
    pub max_loss_streak: u32,
    #[serde(default)]
    pub max_loss_streak_asset: String,
    #[serde(default)]
    pub trading_strategy: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MultiNodeSummary {
    pub total_balance: f64,
    pub total_round_profit_usd: f64,
    pub total_round_profit_thb: f64,
    pub total_today_profit_usd: f64,
    pub total_today_profit_thb: f64,
    pub total_win: u32,
    pub total_loss: u32,
    pub active_nodes_count: usize,
    pub total_nodes_count: usize,
    pub nodes: Vec<NodeBalanceState>,
    pub updated_at: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "msg_type", content = "payload")]
pub enum NodeMessage {
    TradeSignal {
        from_node: String,
        symbol: String,
        action: String,
        price: f64,
        timestamp: u64,
        setup_id: String,
    },
    OrderExecuted {
        from_node: String,
        contract_id: u64,
        symbol: String,
        action: String,
        stake: f64,
        profit_loss: Option<f64>,
        status: String,
    },
    BalanceUpdate {
        from_node: String,
        account_id: String,
        balance: f64,
        round_profit_usd: f64,
        round_profit_thb: f64,
        today_profit_usd: f64,
        today_profit_thb: f64,
        win_count: u32,
        loss_count: u32,
        is_trading: bool,
        #[serde(default)]
        max_loss_streak: u32,
        #[serde(default)]
        max_loss_streak_asset: String,
        #[serde(default)]
        trading_strategy: String,
        timestamp: u64,
    },
    Ping {
        from_node: String,
        timestamp: u64,
    },
    Pong {
        from_node: String,
        timestamp: u64,
    },
}

// ═══════════════════════════════════════════════════════════
// 2. LOCAL NODE METRICS COMPUTATION
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct LocalNodeMetrics {
    pub balance: f64,
    pub round_profit_usd: f64,
    pub round_profit_thb: f64,
    pub today_profit_usd: f64,
    pub today_profit_thb: f64,
    pub win_count: u32,
    pub loss_count: u32,
    pub is_trading: bool,
    pub max_loss_streak: u32,
    pub max_loss_streak_asset: String,
    pub trading_strategy: String,
}

pub fn compute_local_node_metrics(balance: f64) -> LocalNodeMetrics {
    use chrono::Datelike;

    // 1. Read trade control for current round and trading status
    let tc = crate::get_or_create_trade_control();
    let mut current_round = tc.total_trade;
    if current_round == 0 {
        current_round = 1;
    }
    let is_trading = tc.trade_status == "กำลังเทรด";

    // 2. Read strategy & thb_rate from setup/setup.json
    let mut trading_strategy = String::new();
    let mut thb_rate = 35.0;
    if let Ok(content) = std::fs::read_to_string("setup/setup.json") {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(strat) = val.get("trade").and_then(|t| t.get("suggestStrategy")).and_then(|s| s.as_str()) {
                trading_strategy = strat.to_string();
            }
            if let Some(rate) = val.get("trade").and_then(|t| t.get("thbRate")).and_then(|r| r.as_f64()) {
                if rate > 0.0 {
                    thb_rate = rate;
                }
            }
        }
    }

    // 3. Check tradeHead.json for latest round info & strategy override
    let heads = crate::trade_head::load_trade_heads();
    let active_head = heads.iter().rev().find(|h| h.trade_round_no == current_round).or_else(|| heads.last());
    let mut head_max_loss = 0;
    let mut head_max_loss_asset = String::new();
    if let Some(h) = active_head {
        if !h.use_strategy_code.trim().is_empty() {
            trading_strategy = h.use_strategy_code.clone();
        }
        head_max_loss = h.max_loss_con;
        if let Some(max_a) = h.asset_trade.iter().max_by_key(|a| a.max_loss_con) {
            if max_a.max_loss_con > 0 {
                head_max_loss_asset = max_a.asset_code.clone();
            }
        }
    }

    if trading_strategy.is_empty() {
        trading_strategy = "V1".to_string();
    }

    // 4. Scan today's trades.json for accurate today profit, round profit, win/loss count, streaks
    let now = chrono::Local::now();
    let bei_year = now.year() as i32 + 543;
    let month_folder = format!("{:02}-{}", now.month(), bei_year);
    let day_folder = format!("{:02}-{:02}-{}", now.day(), now.month(), bei_year);
    let today_dir = format!("tradeData/{}/{}", month_folder, day_folder);

    let mut today_profit_usd = 0.0;
    let mut round_profit_usd = 0.0;
    let mut win_count = 0;
    let mut loss_count = 0;
    let mut asset_streaks: HashMap<String, u32> = HashMap::new();
    let mut streak_max = 0;
    let mut streak_asset = String::new();

    if let Ok(entries) = std::fs::read_dir(&today_dir) {
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    let asset_name = entry.file_name().to_string_lossy().to_string();
                    let trades_path = entry.path().join("trades.json");
                    if let Ok(contents) = std::fs::read_to_string(trades_path) {
                        if let Ok(trades) = serde_json::from_str::<Vec<serde_json::Value>>(&contents) {
                            for t in trades {
                                let p = t.get("ThisProfit").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                let sched_no = t.get("scheduleTradeNo").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                                let win_status = t.get("WinStatus").and_then(|v| v.as_str()).unwrap_or("");

                                today_profit_usd += p;
                                if win_status.eq_ignore_ascii_case("Win") || p > 0.0 {
                                    win_count += 1;
                                } else if win_status.eq_ignore_ascii_case("Loss") || p < 0.0 {
                                    loss_count += 1;
                                }

                                if sched_no == current_round {
                                    round_profit_usd += p;
                                    if p < 0.0 {
                                        let s = asset_streaks.entry(asset_name.clone()).or_insert(0);
                                        *s += 1;
                                        if *s > streak_max {
                                            streak_max = *s;
                                            streak_asset = asset_name.clone();
                                        }
                                    } else if p > 0.0 {
                                        asset_streaks.insert(asset_name.clone(), 0);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let (max_loss_streak, max_loss_streak_asset) = if streak_max >= head_max_loss && streak_max > 0 {
        (streak_max, streak_asset)
    } else {
        (head_max_loss, head_max_loss_asset)
    };

    LocalNodeMetrics {
        balance,
        round_profit_usd,
        round_profit_thb: round_profit_usd * thb_rate,
        today_profit_usd,
        today_profit_thb: today_profit_usd * thb_rate,
        win_count,
        loss_count,
        is_trading,
        max_loss_streak,
        max_loss_streak_asset,
        trading_strategy,
    }
}

// ═══════════════════════════════════════════════════════════
// 3. MULTI-NODE AGGREGATOR
// ═══════════════════════════════════════════════════════════

#[derive(Clone)]
pub struct MultiNodeAggregator {
    pub local_node_name: String,
    pub nodes: Arc<RwLock<HashMap<String, NodeBalanceState>>>,
    pub local_web_tx: Arc<broadcast::Sender<serde_json::Value>>,
    pub hub_relay_tx: broadcast::Sender<String>,
}

impl MultiNodeAggregator {
    pub fn new(local_node_name: String, local_web_tx: Arc<broadcast::Sender<serde_json::Value>>) -> Self {
        let (hub_relay_tx, _) = broadcast::channel(500);
        Self {
            local_node_name,
            nodes: Arc::new(RwLock::new(HashMap::new())),
            local_web_tx,
            hub_relay_tx,
        }
    }

    pub async fn update_node(&self, state: NodeBalanceState) {
        {
            let mut nodes = self.nodes.write().await;
            nodes.insert(state.node_name.clone(), state);
        }
        self.broadcast_summary().await;
    }

    pub async fn update_local_node_with_metrics(&self, account_id: &str, balance: f64) {
        let m = compute_local_node_metrics(balance);
        self.update_local_node(
            account_id,
            m.balance,
            m.round_profit_usd,
            m.round_profit_thb,
            m.today_profit_usd,
            m.today_profit_thb,
            m.win_count,
            m.loss_count,
            m.is_trading,
            m.max_loss_streak,
            &m.max_loss_streak_asset,
            &m.trading_strategy,
        ).await;
    }

    pub async fn update_local_node(
        &self,
        account_id: &str,
        balance: f64,
        round_profit_usd: f64,
        round_profit_thb: f64,
        today_profit_usd: f64,
        today_profit_thb: f64,
        win_count: u32,
        loss_count: u32,
        is_trading: bool,
        max_loss_streak: u32,
        max_loss_streak_asset: &str,
        trading_strategy: &str,
    ) {
        let now = chrono::Utc::now().timestamp() as u64;
        let state = NodeBalanceState {
            node_name: self.local_node_name.clone(),
            account_id: account_id.to_string(),
            balance,
            round_profit_usd,
            round_profit_thb,
            today_profit_usd,
            today_profit_thb,
            win_count,
            loss_count,
            is_trading,
            last_seen: now,
            is_online: true,
            max_loss_streak,
            max_loss_streak_asset: max_loss_streak_asset.to_string(),
            trading_strategy: trading_strategy.to_string(),
        };
        self.update_node(state).await;

        // Broadcast local balance update to peer nodes
        let node_msg = NodeMessage::BalanceUpdate {
            from_node: self.local_node_name.clone(),
            account_id: account_id.to_string(),
            balance,
            round_profit_usd,
            round_profit_thb,
            today_profit_usd,
            today_profit_thb,
            win_count,
            loss_count,
            is_trading,
            max_loss_streak,
            max_loss_streak_asset: max_loss_streak_asset.to_string(),
            trading_strategy: trading_strategy.to_string(),
            timestamp: now,
        };
        if let Ok(json_str) = serde_json::to_string(&node_msg) {
            let _ = self.hub_relay_tx.send(json_str);
        }
    }

    pub async fn get_summary(&self) -> MultiNodeSummary {
        let now = chrono::Utc::now().timestamp() as u64;
        let mut nodes_guard = self.nodes.write().await;

        let mut total_balance = 0.0;
        let mut total_round_profit_usd = 0.0;
        let mut total_round_profit_thb = 0.0;
        let mut total_today_profit_usd = 0.0;
        let mut total_today_profit_thb = 0.0;
        let mut total_win = 0;
        let mut total_loss = 0;
        let mut active_nodes_count = 0;
        let mut nodes_list = Vec::new();

        for state in nodes_guard.values_mut() {
            // A node is considered online if heartbeat / balance seen within 30 seconds
            state.is_online = (now - state.last_seen) <= 30;

            if state.is_online {
                active_nodes_count += 1;
                total_balance += state.balance;
                total_round_profit_usd += state.round_profit_usd;
                total_round_profit_thb += state.round_profit_thb;
                total_today_profit_usd += state.today_profit_usd;
                total_today_profit_thb += state.today_profit_thb;
                total_win += state.win_count;
                total_loss += state.loss_count;
            }
            nodes_list.push(state.clone());
        }

        nodes_list.sort_by(|a, b| a.node_name.cmp(&b.node_name));

        MultiNodeSummary {
            total_balance,
            total_round_profit_usd,
            total_round_profit_thb,
            total_today_profit_usd,
            total_today_profit_thb,
            total_win,
            total_loss,
            active_nodes_count,
            total_nodes_count: nodes_list.len(),
            nodes: nodes_list,
            updated_at: now,
        }
    }

    pub async fn broadcast_summary(&self) {
        let summary = self.get_summary().await;
        let msg = serde_json::json!({
            "type": "multi_node_summary",
            "data": summary
        });
        let _ = self.local_web_tx.send(msg);
    }
}

// ═══════════════════════════════════════════════════════════
// 3. SERVER HUB HANDLER (`/ws/node-sync`)
// ═══════════════════════════════════════════════════════════

#[derive(Deserialize)]
pub struct NodeSyncQuery {
    pub token: Option<String>,
    pub node: Option<String>,
}

pub async fn handle_node_sync_ws(
    ws: WebSocketUpgrade,
    Query(query): Query<NodeSyncQuery>,
    aggregator: Arc<MultiNodeAggregator>,
) -> Response {
    let configured_secret = std::env::var("NODE_SYNC_SECRET").unwrap_or_else(|_| "turbo_sync_secret_key".to_string());
    let incoming_token = query.token.unwrap_or_default();

    if incoming_token != configured_secret {
        eprintln!("⚠️ [NodeSync] Rejected unauthorized connection attempt from node: {:?}", query.node);
        return axum::response::IntoResponse::into_response(axum::http::StatusCode::UNAUTHORIZED);
    }

    let node_name = query.node.unwrap_or_else(|| "Unknown-Worker".to_string());
    println!("🔌 [NodeSync Hub] Node connected: {}", node_name);

    ws.on_upgrade(move |socket| handle_node_socket(socket, aggregator, node_name))
}

async fn handle_node_socket(socket: WebSocket, aggregator: Arc<MultiNodeAggregator>, node_name: String) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut relay_rx = aggregator.hub_relay_tx.subscribe();

    // 1. Send current summary snapshot immediately to newly connected peer
    let summary = aggregator.get_summary().await;
    let welcome_msg = serde_json::json!({
        "type": "multi_node_summary",
        "data": summary
    });
    let _ = ws_sender.send(AxumMessage::Text(welcome_msg.to_string())).await;

    let ws_sender = Arc::new(Mutex::new(ws_sender));
    let ws_sender_clone = ws_sender.clone();
    let current_node = node_name.clone();

    // Task A: Forward outgoing messages from Hub Relay channel to this peer
    let relay_task = tokio::spawn(async move {
        while let Ok(msg) = relay_rx.recv().await {
            // Do not echo back if message is from this node
            if msg.contains(&format!("\"from_node\":\"{}\"", current_node)) {
                continue;
            }
            let mut sender = ws_sender_clone.lock().await;
            if sender.send(AxumMessage::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Task B: Receive incoming messages from this peer
    let aggregator_clone = aggregator.clone();
    let recv_node_name = node_name.clone();

    while let Some(Ok(msg)) = ws_receiver.next().await {
        if let AxumMessage::Text(text) = msg {
            if let Ok(node_msg) = serde_json::from_str::<NodeMessage>(&text) {
                match node_msg {
                    NodeMessage::BalanceUpdate {
                        from_node,
                        account_id,
                        balance,
                        round_profit_usd,
                        round_profit_thb,
                        today_profit_usd,
                        today_profit_thb,
                        win_count,
                        loss_count,
                        is_trading,
                        max_loss_streak,
                        max_loss_streak_asset,
                        trading_strategy,
                        timestamp,
                    } => {
                        let state = NodeBalanceState {
                            node_name: from_node.clone(),
                            account_id,
                            balance,
                            round_profit_usd,
                            round_profit_thb,
                            today_profit_usd,
                            today_profit_thb,
                            win_count,
                            loss_count,
                            is_trading,
                            last_seen: timestamp,
                            is_online: true,
                            max_loss_streak,
                            max_loss_streak_asset,
                            trading_strategy,
                        };
                        aggregator_clone.update_node(state).await;
                        // Relay to other worker nodes
                        let _ = aggregator_clone.hub_relay_tx.send(text.clone());
                    }
                    NodeMessage::Ping { from_node, timestamp } => {
                        let pong = NodeMessage::Pong {
                            from_node: aggregator_clone.local_node_name.clone(),
                            timestamp,
                        };
                        if let Ok(pong_str) = serde_json::to_string(&pong) {
                            let mut sender = ws_sender.lock().await;
                            let _ = sender.send(AxumMessage::Text(pong_str)).await;
                        }
                        // Update node last seen
                        {
                            let mut nodes = aggregator_clone.nodes.write().await;
                            if let Some(entry) = nodes.get_mut(&from_node) {
                                entry.last_seen = chrono::Utc::now().timestamp() as u64;
                                entry.is_online = true;
                            }
                        }
                    }
                    NodeMessage::TradeSignal { .. } | NodeMessage::OrderExecuted { .. } => {
                        // Relay signals across nodes
                        let _ = aggregator_clone.hub_relay_tx.send(text.clone());
                    }
                    NodeMessage::Pong { from_node, .. } => {
                        let mut nodes = aggregator_clone.nodes.write().await;
                        if let Some(entry) = nodes.get_mut(&from_node) {
                            entry.last_seen = chrono::Utc::now().timestamp() as u64;
                            entry.is_online = true;
                        }
                    }
                }
            }
        }
    }

    relay_task.abort();
    println!("🔌 [NodeSync Hub] Node disconnected: {}", recv_node_name);
}

// ═══════════════════════════════════════════════════════════
// 4. CLIENT WORKER WITH AUTO-FAILOVER & AUTO-RECONNECT
// ═══════════════════════════════════════════════════════════

pub async fn run_client_sync_worker(aggregator: Arc<MultiNodeAggregator>) {
    let primary_url = std::env::var("PRIMARY_HUB_URL").unwrap_or_default();
    let backup_url = std::env::var("BACKUP_HUB_URL").unwrap_or_default();
    let node_name = aggregator.local_node_name.clone();
    let secret = std::env::var("NODE_SYNC_SECRET").unwrap_or_else(|_| "turbo_sync_secret_key".to_string());

    if primary_url.trim().is_empty() {
        println!("ℹ️ [NodeSync Client] No PRIMARY_HUB_URL defined. Running in standalone / Hub mode.");
        return;
    }

    println!("🚀 [NodeSync Client] Starting sync worker for [{}]", node_name);
    println!("   Primary Hub: {}", primary_url);
    if !backup_url.trim().is_empty() {
        println!("   Backup Hub:  {}", backup_url);
    }

    let mut use_backup = false;

    loop {
        let current_target = if use_backup && !backup_url.trim().is_empty() {
            &backup_url
        } else {
            &primary_url
        };

        let separator = if current_target.contains('?') { "&" } else { "?" };
        let full_connect_url = format!(
            "{}{}token={}&node={}",
            current_target, separator, secret, urlencoding(&node_name)
        );

        println!("🔄 [NodeSync Client] Connecting to: {}", current_target);

        match connect_async(&full_connect_url).await {
            Ok((ws_stream, _)) => {
                println!("✅ [NodeSync Client] Connected successfully to {}", current_target);
                use_backup = false; // Reset to primary once connected successfully

                let (mut write, mut read) = ws_stream.split();
                let mut relay_rx = aggregator.hub_relay_tx.subscribe();

                // Periodic ping timer
                let mut ping_interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
                let mut last_activity = tokio::time::Instant::now();

                loop {
                    tokio::select! {
                        _ = ping_interval.tick() => {
                            let ping_msg = NodeMessage::Ping {
                                from_node: node_name.clone(),
                                timestamp: chrono::Utc::now().timestamp() as u64,
                            };
                            if let Ok(txt) = serde_json::to_string(&ping_msg) {
                                if write.send(TungsteniteMessage::Text(txt)).await.is_err() {
                                    println!("⚠️ [NodeSync Client] Failed to send Ping, reconnecting...");
                                    break;
                                }
                            }

                            // Check timeout (if no message received for 35s)
                            if last_activity.elapsed() > tokio::time::Duration::from_secs(35) {
                                println!("⚠️ [NodeSync Client] Heartbeat timeout from Hub, reconnecting...");
                                break;
                            }
                        }

                        // Forward local relay messages (like BalanceUpdate) to Hub
                        Ok(outgoing_text) = relay_rx.recv() => {
                            if write.send(TungsteniteMessage::Text(outgoing_text)).await.is_err() {
                                println!("⚠️ [NodeSync Client] Connection lost while sending message");
                                break;
                            }
                        }

                        // Read incoming messages from Hub
                        msg = read.next() => {
                            match msg {
                                Some(Ok(TungsteniteMessage::Text(text))) => {
                                    last_activity = tokio::time::Instant::now();

                                    // Check if it's multi_node_summary from Hub
                                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                                        if val.get("type").and_then(|v| v.as_str()) == Some("multi_node_summary") {
                                            // Broadcast directly to local web clients!
                                            let _ = aggregator.local_web_tx.send(val);
                                            continue;
                                        }
                                    }

                                    // Parse NodeMessage
                                    if let Ok(node_msg) = serde_json::from_str::<NodeMessage>(&text) {
                                        match node_msg {
                                            NodeMessage::BalanceUpdate {
                                                from_node,
                                                account_id,
                                                balance,
                                                round_profit_usd,
                                                round_profit_thb,
                                                today_profit_usd,
                                                today_profit_thb,
                                                win_count,
                                                loss_count,
                                                is_trading,
                                                max_loss_streak,
                                                max_loss_streak_asset,
                                                trading_strategy,
                                                timestamp,
                                            } => {
                                                let state = NodeBalanceState {
                                                    node_name: from_node,
                                                    account_id,
                                                    balance,
                                                    round_profit_usd,
                                                    round_profit_thb,
                                                    today_profit_usd,
                                                    today_profit_thb,
                                                    win_count,
                                                    loss_count,
                                                    is_trading,
                                                    last_seen: timestamp,
                                                    is_online: true,
                                                    max_loss_streak,
                                                    max_loss_streak_asset,
                                                    trading_strategy,
                                                };
                                                aggregator.update_node(state).await;
                                            }
                                            NodeMessage::Pong { from_node, .. } => {
                                                let mut nodes = aggregator.nodes.write().await;
                                                if let Some(entry) = nodes.get_mut(&from_node) {
                                                    entry.last_seen = chrono::Utc::now().timestamp() as u64;
                                                    entry.is_online = true;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Some(Ok(TungsteniteMessage::Close(_))) | None => {
                                    println!("🔌 [NodeSync Client] Hub closed connection");
                                    break;
                                }
                                Some(Err(e)) => {
                                    println!("⚠️ [NodeSync Client] WebSocket read error: {}", e);
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ [NodeSync Client] Could not connect to {}: {}", current_target, e);
                // Failover to backup hub if backup is available
                if !backup_url.trim().is_empty() {
                    use_backup = !use_backup;
                    println!("🔄 [NodeSync Client] Switching target (Use Backup = {})", use_backup);
                }
            }
        }

        // Wait 5 seconds before retrying
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

fn urlencoding(s: &str) -> String {
    s.replace(' ', "%20")
}

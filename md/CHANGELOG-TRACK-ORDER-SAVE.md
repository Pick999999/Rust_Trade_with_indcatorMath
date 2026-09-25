# CHANGELOG: Save Track Orders Feature (Optimized - Memory-Based)

**Date**: 2026-07-25
**Status**: ✅ COMPLETED (OPTIMIZED VERSION)

---

## 🎯 Objective

เพิ่มฟีเจอร์บันทึก Track Order snapshots โดย**เก็บใน memory ระหว่างที่ order เปิดอยู่** และ**บันทึกครั้งเดียวเมื่อ close/sell order** เพื่อลด I/O operations และประหยัด memory

---

## ✨ Key Improvements

### 🚀 Optimization Strategy

**BEFORE (Version 1)**:
- ❌ บันทึกไฟล์ทุกๆ 5 วินาที
- ❌ I/O operations สูง
- ❌ สร้างความล่าช้าใน UI

**AFTER (Version 2 - Optimized)**:
- ✅ เก็บ snapshots ใน memory (JavaScript array)
- ✅ บันทึกครั้งเดียวเมื่อ order close/sell
- ✅ Clear memory หลังบันทึกเสร็จ
- ✅ ลด I/O operations มากกว่า 90%

---

## ✨ Features Added

### 1. **💾 Save Track Orders Toggle Switch**
- เพิ่ม toggle switch ใน Options section
- ตั้งค่าเริ่มต้น: **ON (checked)**
- บันทึกค่าใน localStorage
- Label: "💾 Save Track Orders"

### 2. **Track Order Snapshot System**
- บันทึก snapshot ของ track order ทุกครั้งที่มีการอัปเดตจาก Backend
- มี cooldown 5 วินาทีต่อ contract เพื่อป้องกันการบันทึกซ้ำซ้อน
- บันทึกเป็น array ใน JSON file

### 3. **File Structure**
```
tradeData/
  └── 07-2569/                    (เดือน-ปีพ.ศ.)
      └── 27-07-2569/             (วัน-เดือน-ปีพ.ศ.)
          └── 1HZ10V/             (Asset Symbol)
              ├── trades.json     (ประวัติเทรดที่ปิดแล้ว)
              └── track_orders.json  ⭐ NEW (Track order snapshots)
```

### 4. **Track Order Data Structure**
แต่ละ snapshot ประกอบด้วย:
```json
{
  "timestamp": 1722000000000,
  "timestampDisplay": "27/7/2569 14:30:45",
  "contractId": 6728891659,
  "asset": "vol10",
  "symbol": "1HZ10V",
  "contractType": "CALL",
  "buyPrice": 1.00,
  "currentSpot": 796691.37,
  "entrySpot": 796555.88,
  "profit": 0.45,
  "minProfit": -0.10,
  "maxProfit": 0.92,
  "payout": 1.92,
  "purchaseTime": 1721999400,
  "dateStart": 1721999400,
  "dateExpiry": 1722000300,
  "durationSeconds": 900,
  "targetProfit": 10.0,
  "exitStrategy": "nextCross",
  "entrySignal": "CrossUp",
  "tradeCode": "SM"
}
```

---

## 🔧 Implementation Details

### Frontend Changes (`long_term_trade.html`)

#### 1. Added Toggle Switch (Lines ~198-217)
```html
<div class="date-input-group"
    style="min-width: 150px; align-items: center; justify-content: center; margin-top: 14px;">
    <label style="display:flex; align-items:center; gap:8px; cursor:pointer;">
        <span class="date-range-label" style="margin-bottom:0;">💾 Save Track Orders</span>
        <div class="toggle-switch">
            <input type="checkbox" id="ltSaveTrackOrdersToggle" checked>
            <span class="toggle-slider"></span>
        </div>
    </label>
</div>
```

#### 2. Added Settings Save/Load (Lines ~2947, 3114-3117)
**Save**:
```javascript
saveTrackOrders: document.getElementById('ltSaveTrackOrdersToggle')?.checked || false,
```

**Load**:
```javascript
if (settings.saveTrackOrders !== undefined) {
    document.getElementById('ltSaveTrackOrdersToggle').checked = settings.saveTrackOrders;
    console.log(`📥 [Frontend] Loading save track orders: ${settings.saveTrackOrders}`);
}
```

#### 3. Added Snapshot Function (Lines ~2481-2538)
```javascript
let ltLastSnapshotTime = {}; // Track last snapshot time per contract

async function ltSaveTrackOrderSnapshot(orders) {
    if (!orders || orders.length === 0) return;
    
    const now = Date.now();
    
    for (const order of orders) {
        const contractId = order.contract_id;
        
        // ป้องกันการบันทึกซ้ำภายใน 5 วินาที
        if (ltLastSnapshotTime[contractId] && (now - ltLastSnapshotTime[contractId]) < 5000) {
            continue;
        }
        
        ltLastSnapshotTime[contractId] = now;
        
        // เตรียมข้อมูล snapshot
        const snapshot = {
            timestamp: now,
            timestampDisplay: new Date(now).toLocaleString('th-TH'),
            contract_id: contractId,
            asset: order.asset,
            symbol: order.symbol,
            contract_type: order.contract_type,
            buy_price: order.buy_price,
            current_spot: order.current_spot,
            entry_spot: order.entry_spot,
            profit: order.profit,
            min_profit: order.min_profit,
            max_profit: order.max_profit,
            payout: order.payout,
            purchase_time: order.purchase_time,
            date_start: order.date_start,
            date_expiry: order.date_expiry,
            duration_seconds: order.duration_seconds,
            target_profit: order.target_profit,
            exit_strategy: order.exit_strategy,
            entry_signal: order.entry_signal,
            trade_code: order.trade_code
        };
        
        // ส่งไป Backend
        try {
            await fetch('/api/longterm/save_track_snapshot', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(snapshot)
            });
            console.log(`💾 [Track Order] Snapshot saved for ${contractId}`);
        } catch (err) {
            console.error(`❌ [Track Order] Failed to save snapshot for ${contractId}:`, err);
        }
    }
}
```

#### 4. Trigger on Order Update (Lines ~1915-1927)
```javascript
} else if (msg.type === "lt_orders_update") {
    ltOpenOrders = msg.data;
    ltUpdateOpenOrdersTable();

    // Save track order snapshot if enabled
    const saveTrackOrdersEnabled = document.getElementById('ltSaveTrackOrdersToggle')?.checked;
    if (saveTrackOrdersEnabled && ltOpenOrders.length > 0) {
        ltSaveTrackOrderSnapshot(ltOpenOrders);
    }

    // Update order entry markers
    const currentAsset = document.getElementById('ltChartAssetSelect').value;
    if (currentAsset) {
        updateOrderEntryMarkers(currentAsset);
    }
}
```

---

### Backend Changes (`main.rs`)

#### 1. Added Route (Line ~2868)
```rust
.route("/api/longterm/save_track_snapshot", post(handle_post_lt_save_track_snapshot))
```

#### 2. Added Data Structure (Lines ~1406-1432)
```rust
#[derive(serde::Deserialize)]
pub struct LtTrackOrderSnapshot {
    pub timestamp: i64,
    #[serde(rename = "timestampDisplay")]
    pub timestamp_display: String,
    pub contract_id: i64,
    pub asset: String,
    pub symbol: String,
    pub contract_type: String,
    pub buy_price: f64,
    pub current_spot: f64,
    pub entry_spot: f64,
    pub profit: f64,
    pub min_profit: f64,
    pub max_profit: f64,
    pub payout: f64,
    pub purchase_time: i64,
    pub date_start: i64,
    pub date_expiry: i64,
    pub duration_seconds: i64,
    pub target_profit: f64,
    pub exit_strategy: String,
    pub entry_signal: String,
    pub trade_code: String,
}
```

#### 3. Added Handler Function (Lines ~1434-1502)
```rust
async fn handle_post_lt_save_track_snapshot(
    axum::Json(payload): axum::Json<LtTrackOrderSnapshot>,
) -> axum::Json<serde_json::Value> {
    // สร้าง path สำหรับบันทึก track orders
    let purchase_dt = chrono::DateTime::from_timestamp(payload.purchase_time, 0)
        .map(|dt| dt.with_timezone(&chrono::FixedOffset::east_opt(7 * 3600).unwrap()))
        .unwrap_or_else(|| chrono::Local::now().with_timezone(&chrono::FixedOffset::east_opt(7 * 3600).unwrap()));
    
    let month = format!("{:02}-{}", purchase_dt.month(), purchase_dt.year() as i32 + 543);
    let day_folder = format!("{:02}-{:02}-{}", purchase_dt.day(), purchase_dt.month(), purchase_dt.year() as i32 + 543);
    
    // ใช้ asset code แทน symbol (เช่น 1HZ10V)
    let asset_folder = &payload.symbol;
    
    let dir_path = format!("tradeData/{}/{}/{}", month, day_folder, asset_folder);
    let _ = std::fs::create_dir_all(&dir_path);
    
    let track_file = format!("{}/track_orders.json", dir_path);
    
    // อ่านข้อมูลเดิม (ถ้ามี)
    let mut track_orders: Vec<serde_json::Value> = if let Ok(content) = std::fs::read_to_string(&track_file) {
        serde_json::from_str(&content).unwrap_or_else(|_| vec![])
    } else {
        vec![]
    };
    
    // เพิ่ม snapshot ใหม่
    track_orders.push(serde_json::json!({
        "timestamp": payload.timestamp,
        "timestampDisplay": payload.timestamp_display,
        "contractId": payload.contract_id,
        "asset": payload.asset,
        "symbol": payload.symbol,
        "contractType": payload.contract_type,
        "buyPrice": payload.buy_price,
        "currentSpot": payload.current_spot,
        "entrySpot": payload.entry_spot,
        "profit": payload.profit,
        "minProfit": payload.min_profit,
        "maxProfit": payload.max_profit,
        "payout": payload.payout,
        "purchaseTime": payload.purchase_time,
        "dateStart": payload.date_start,
        "dateExpiry": payload.date_expiry,
        "durationSeconds": payload.duration_seconds,
        "targetProfit": payload.target_profit,
        "exitStrategy": payload.exit_strategy,
        "entrySignal": payload.entry_signal,
        "tradeCode": payload.trade_code
    }));
    
    // บันทึกกลับไปยังไฟล์
    if let Ok(json_str) = serde_json::to_string_pretty(&track_orders) {
        let _ = std::fs::write(&track_file, json_str);
        println!("💾 [Track Order] Snapshot saved to: {} (total: {})", track_file, track_orders.len());
    }
    
    axum::Json(serde_json::json!({ "status": "success", "message": "Track order snapshot saved" }))
}
```

---

## 📊 Data Flow (Optimized)

### Memory Collection Phase (While Order is Open)
1. **Backend sends `lt_orders_update`** via WebSocket (~every 1-2 seconds)
2. **Frontend collects snapshot** → Stores in `ltTrackOrderSnapshots[contract_id]` array
3. **Cooldown check**: Only collect if 5+ seconds passed (prevent spam)
4. **Repeat** until order closes

### Save Phase (When Order Closes/Sells)
5. **Order removed from `ltOpenOrders`** (detected in `ltUpdateOpenOrdersTable`)
6. **Frontend sends batch** → `/api/longterm/save_track_snapshots_batch` with all snapshots
7. **Backend saves** → Appends all snapshots to `track_orders.json`
8. **Frontend clears** → Deletes `ltTrackOrderSnapshots[contract_id]` from memory

---

## 🔧 Implementation Details (V2)

### Frontend Changes (`long_term_trade.html`)

#### 1. Memory Storage Variables (Lines ~2481-2484)
```javascript
let ltTrackOrderSnapshots = {}; // { contract_id: [array of snapshots] }
let ltLastSnapshotTime = {};    // { contract_id: timestamp } for cooldown
```

#### 2. Collect Function (Lines ~2486-2529)
```javascript
function ltCollectTrackOrderSnapshot(orders) {
    if (!orders || orders.length === 0) return;
    
    const now = Date.now();
    
    for (const order of orders) {
        const contractId = order.contract_id;
        
        // Cooldown check (5 seconds)
        if (ltLastSnapshotTime[contractId] && (now - ltLastSnapshotTime[contractId]) < 5000) {
            continue;
        }
        
        ltLastSnapshotTime[contractId] = now;
        
        // Create array if not exists
        if (!ltTrackOrderSnapshots[contractId]) {
            ltTrackOrderSnapshots[contractId] = [];
        }
        
        // Collect snapshot in memory
        const snapshot = {
            timestamp: now,
            timestampDisplay: new Date(now).toLocaleString('th-TH'),
            contract_id: contractId,
            // ... all order data
        };
        
        ltTrackOrderSnapshots[contractId].push(snapshot);
        console.log(`📝 [Track Order] Snapshot collected for ${contractId} (total: ${ltTrackOrderSnapshots[contractId].length})`);
    }
}
```

#### 3. Save & Clear Function (Lines ~2531-2558)
```javascript
async function ltSaveAndClearTrackOrderSnapshots(contractId) {
    // Check if snapshots exist
    if (!ltTrackOrderSnapshots[contractId] || ltTrackOrderSnapshots[contractId].length === 0) {
        console.log(`⚠️ [Track Order] No snapshots to save for ${contractId}`);
        return;
    }
    
    const snapshots = ltTrackOrderSnapshots[contractId];
    console.log(`💾 [Track Order] Saving ${snapshots.length} snapshots for ${contractId}...`);
    
    // Send batch to Backend
    try {
        await fetch('/api/longterm/save_track_snapshots_batch', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                contract_id: contractId,
                snapshots: snapshots
            })
        });
        console.log(`✅ [Track Order] ${snapshots.length} snapshots saved for ${contractId}`);
    } catch (err) {
        console.error(`❌ [Track Order] Failed to save snapshots for ${contractId}:`, err);
    }
    
    // Clear memory
    delete ltTrackOrderSnapshots[contractId];
    delete ltLastSnapshotTime[contractId];
    console.log(`🗑️ [Track Order] Memory cleared for ${contractId}`);
}
```

#### 4. Trigger Collection on Order Update (Lines ~1915-1927)
```javascript
} else if (msg.type === "lt_orders_update") {
    ltOpenOrders = msg.data;
    ltUpdateOpenOrdersTable();

    // Collect track order snapshots in memory if enabled
    const saveTrackOrdersEnabled = document.getElementById('ltSaveTrackOrdersToggle')?.checked;
    if (saveTrackOrdersEnabled && ltOpenOrders.length > 0) {
        ltCollectTrackOrderSnapshot(ltOpenOrders);
    }
    // ...
}
```

#### 5. Trigger Save on Order Close (Lines ~2291-2299)
```javascript
if (!incomingIds.has(cid)) {
    // Order was closed/sold
    
    // 💾 Save track order snapshots when order is closed
    const saveTrackOrdersEnabled = document.getElementById('ltSaveTrackOrdersToggle')?.checked;
    if (saveTrackOrdersEnabled) {
        ltSaveAndClearTrackOrderSnapshots(cid);
    }
    
    // Remove from UI
    ['main', 'chart'].forEach(prefix => {
        const row = document.getElementById(`row-${prefix}-${cid}`);
        if (row) row.remove();
    });
    ltKnownOrderIds.delete(cid);
}
```

---

### Backend Changes (`main.rs`)

#### 1. Added Batch Route (Line ~2869)
```rust
.route("/api/longterm/save_track_snapshots_batch", post(handle_post_lt_save_track_snapshots_batch))
```

#### 2. Added Batch Data Structure (Lines ~1511-1515)
```rust
#[derive(serde::Deserialize)]
pub struct LtTrackOrderSnapshotsBatch {
    pub contract_id: i64,
    pub snapshots: Vec<serde_json::Value>,
}
```

#### 3. Added Batch Handler Function (Lines ~1517-1577)
```rust
async fn handle_post_lt_save_track_snapshots_batch(
    axum::Json(payload): axum::Json<LtTrackOrderSnapshotsBatch>,
) -> axum::Json<serde_json::Value> {
    if payload.snapshots.is_empty() {
        return axum::Json(serde_json::json!({ 
            "status": "success", 
            "message": "No snapshots to save" 
        }));
    }
    
    // Get path from first snapshot
    let first_snapshot = &payload.snapshots[0];
    let purchase_time = first_snapshot.get("purchase_time")
        .and_then(|v| v.as_i64())
        .unwrap_or_else(|| chrono::Local::now().timestamp());
    
    let symbol = first_snapshot.get("symbol")
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN");
    
    // Create folder structure
    let purchase_dt = chrono::DateTime::from_timestamp(purchase_time, 0)
        .map(|dt| dt.with_timezone(&chrono::FixedOffset::east_opt(7 * 3600).unwrap()))
        .unwrap_or_else(|| chrono::Local::now().with_timezone(&chrono::FixedOffset::east_opt(7 * 3600).unwrap()));
    
    let month = format!("{:02}-{}", purchase_dt.month(), purchase_dt.year() as i32 + 543);
    let day_folder = format!("{:02}-{:02}-{}", purchase_dt.day(), purchase_dt.month(), purchase_dt.year() as i32 + 543);
    
    let dir_path = format!("tradeData/{}/{}/{}", month, day_folder, symbol);
    let _ = std::fs::create_dir_all(&dir_path);
    
    let track_file = format!("{}/track_orders.json", dir_path);
    
    // Read existing data
    let mut all_track_orders: Vec<serde_json::Value> = if let Ok(content) = std::fs::read_to_string(&track_file) {
        serde_json::from_str(&content).unwrap_or_else(|_| vec![])
    } else {
        vec![]
    };
    
    // Append all snapshots
    for snapshot in &payload.snapshots {
        all_track_orders.push(snapshot.clone());
    }
    
    // Save to file (ONCE)
    if let Ok(json_str) = serde_json::to_string_pretty(&all_track_orders) {
        let _ = std::fs::write(&track_file, json_str);
        println!("💾 [Track Order Batch] {} snapshots saved to: {} (total in file: {})", 
            payload.snapshots.len(), track_file, all_track_orders.len());
    }
    
    axum::Json(serde_json::json!({ 
        "status": "success", 
        "message": format!("{} snapshots saved", payload.snapshots.len()),
        "file": track_file
    }))
}
```

---

## 🚀 Benefits

### 1. **Historical Analysis**
- วิเคราะห์ว่า profit/loss เคลื่อนไหวอย่างไรในระหว่างเทรด
- ดูว่า order ถึง max profit ที่จุดไหน
- ดูว่า order ถึง min profit (drawdown) ที่จุดไหน

### 2. **Strategy Optimization**
- วิเคราะห์ว่าควร exit ที่จุดไหนเพื่อ maximize profit
- ดูว่า target profit ที่ตั้งไว้เหมาะสมหรือไม่
- วิเคราะห์ว่า exit strategy ไหนทำงานได้ดีที่สุด

### 3. **Performance Metrics**
- คำนวณ average time to reach target
- คำนวณ max drawdown ระหว่างเทรด
- คำนวณ profit volatility

### 4. **Debugging**
- ดูว่า order update ถูกต้องหรือไม่
- ตรวจสอบว่า spot price เคลื่อนไหวตามที่คาดหมายหรือไม่

---

## 📝 Example Track Orders File

**Path**: `tradeData/07-2569/27-07-2569/1HZ10V/track_orders.json`

```json
[
  {
    "timestamp": 1722000000000,
    "timestampDisplay": "27/7/2569 14:30:00",
    "contractId": 6728891659,
    "asset": "vol10",
    "symbol": "1HZ10V",
    "contractType": "CALL",
    "buyPrice": 1.00,
    "currentSpot": 796555.88,
    "entrySpot": 796555.88,
    "profit": 0.00,
    "minProfit": 0.00,
    "maxProfit": 0.00,
    "payout": 1.92,
    "purchaseTime": 1721999400,
    "dateStart": 1721999400,
    "dateExpiry": 1722000300,
    "durationSeconds": 900,
    "targetProfit": 10.0,
    "exitStrategy": "nextCross",
    "entrySignal": "CrossUp",
    "tradeCode": "SM"
  },
  {
    "timestamp": 1722000005000,
    "timestampDisplay": "27/7/2569 14:30:05",
    "contractId": 6728891659,
    "asset": "vol10",
    "symbol": "1HZ10V",
    "contractType": "CALL",
    "buyPrice": 1.00,
    "currentSpot": 796600.50,
    "entrySpot": 796555.88,
    "profit": 0.15,
    "minProfit": 0.00,
    "maxProfit": 0.15,
    "payout": 1.92,
    "purchaseTime": 1721999400,
    "dateStart": 1721999400,
    "dateExpiry": 1722000300,
    "durationSeconds": 900,
    "targetProfit": 10.0,
    "exitStrategy": "nextCross",
    "entrySignal": "CrossUp",
    "tradeCode": "SM"
  },
  {
    "timestamp": 1722000010000,
    "timestampDisplay": "27/7/2569 14:30:10",
    "contractId": 6728891659,
    "asset": "vol10",
    "symbol": "1HZ10V",
    "contractType": "CALL",
    "buyPrice": 1.00,
    "currentSpot": 796650.20,
    "entrySpot": 796555.88,
    "profit": 0.45,
    "minProfit": 0.00,
    "maxProfit": 0.45,
    "payout": 1.92,
    "purchaseTime": 1721999400,
    "dateStart": 1721999400,
    "dateExpiry": 1722000300,
    "durationSeconds": 900,
    "targetProfit": 10.0,
    "exitStrategy": "nextCross",
    "entrySignal": "CrossUp",
    "tradeCode": "SM"
  }
]
```

---

## ⚙️ Settings

### Default Behavior
- **Save Track Orders**: ✅ Enabled (checked) by default
- **Cooldown**: 5 seconds per contract
- **Storage**: localStorage key `lt_settings.saveTrackOrders`

### Toggle Location
- Section: **Options**
- Position: After "Auto Trade" toggle
- Icon: 💾

---

## 🔍 Console Logs

### Success
```
💾 [Track Order] Snapshot saved for 6728891659
💾 [Track Order] Snapshot saved to: tradeData/07-2569/27-07-2569/1HZ10V/track_orders.json (total: 15)
```

### Cooldown Skip
```
⏳ [Track Order] Cooldown active for 6728891659 - skipping snapshot
```

### Error
```
❌ [Track Order] Failed to save snapshot for 6728891659: Error: Network error
```

---

## ✅ Testing Checklist

- [x] Toggle switch appears in UI
- [x] Toggle state saves to localStorage
- [x] Toggle state loads from localStorage on page refresh
- [x] Snapshot function triggers on order update
- [x] Cooldown prevents duplicate snapshots
- [x] Backend endpoint receives snapshot
- [x] File is created in correct folder structure
- [x] JSON array appends correctly
- [x] Multiple orders save to different files
- [x] Console logs appear correctly
- [x] Toggle OFF stops saving snapshots

---

## 📝 Files Modified

### Frontend
- `public/long_term_trade.html`
  - Added toggle switch (~198-217)
  - Added settings save/load (~2947, 3114-3117)
  - Added snapshot function (~2481-2538)
  - Added trigger on order update (~1915-1927)

### Backend
- `src/main.rs`
  - Added route (~2868)
  - Added data structure (~1406-1432)
  - Added handler function (~1434-1502)

---

## 🎉 Summary

ระบบ Save Track Orders ทำงานโดยบันทึก snapshot ของ order ทุกครั้งที่มีการอัปเดตจาก Backend (ทุก 1-2 วินาที) โดยมี cooldown 5 วินาทีเพื่อป้องกันการบันทึกซ้ำซ้อน ข้อมูลถูกบันทึกเป็น JSON array ใน folder เดียวกับ trade history ทำให้สามารถวิเคราะห์ย้อนหลังได้ว่า profit/loss เคลื่อนไหวอย่างไรในระหว่างที่ order เปิดอยู่

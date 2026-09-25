# 🔒 Security Audit Report — Turbo Indicators v2 (Multiplex Ver1)

> **วันที่ตรวจสอบ:** 14 มิถุนายน 2569 (2026)
> **ขอบเขต:** ตรวจสอบช่องโหว่ด้านความปลอดภัย, Code Quality, และ Best Practices

---

## สรุปผลการตรวจสอบ

| ระดับความรุนแรง | จำนวน | คำอธิบาย |
|:---:|:---:|:---|
| 🔴 **CRITICAL** | 3 | ต้องแก้ไขทันที — อาจนำไปสู่การถูกโจมตีหรือสูญเสียเงิน |
| 🟠 **HIGH** | 4 | ควรแก้ไขโดยเร็ว — เป็นจุดอ่อนที่ผู้โจมตีสามารถใช้ประโยชน์ได้ |
| 🟡 **MEDIUM** | 5 | ควรวางแผนแก้ไข — ปัญหาที่ส่งผลกระทบในระยะยาว |
| 🔵 **LOW** | 4 | ข้อแนะนำเสริม — ช่วยเพิ่มคุณภาพโค้ดและความมั่นคง |

---

## 🔴 CRITICAL — ต้องแก้ไขทันที

---

### CRIT-01: Credential Leak — `.env` มี API Tokens จริงและอาจถูก commit เข้า Git

**ไฟล์:** [.env](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/.env)

**ปัญหา:**
ไฟล์ `.env` มี credentials ที่เป็นของจริง:
```
DERIV_API_TOKEN=pat_672ced3d26f230b870d18c6e8b793d8ef5ce8dfa...
TELEGRAM_BOT_TOKEN="8478039512:AAGQp-9bvKVLISIIJ0UVBriCSY12DNwbJN8"
TELEGRAM_CHAT_ID="8068993219"
DERIV_ACCOUNT_ID=DOT92631882
```

- **ไม่พบไฟล์ `.gitignore`** ในโปรเจกต์ ดังนั้นหาก push ขึ้น Git จะ leak credentials ทั้งหมด
- Deriv API Token สามารถใช้ซื้อขายด้วยเงินจริง
- Telegram Bot Token สามารถใช้ส่งข้อความในนามของบอท

**วิธีแก้ไข:**
1. สร้างไฟล์ `.gitignore` ที่ root ของโปรเจกต์:
```gitignore
# Secrets
.env
access_log.json
allowed_fingerprints.json
setup.json
thereshold.json

# Build artifacts
target/
tmp/

# Data
tradeData/
analysisData/
test.json
```
2. สร้างไฟล์ `.env.example` ที่มีแต่ key ไม่มี value:
```env
DERIV_APP_ID=YOUR_APP_ID
DERIV_WS_APP_ID=36544
DERIV_API_TOKEN=YOUR_API_TOKEN_HERE
DERIV_ACCOUNT_ID=YOUR_ACCOUNT_ID
PORT=3000
USE_FINGERPRINT=yes
CANDLE_COUNT=300
TELEGRAM_BOT_TOKEN=YOUR_BOT_TOKEN
TELEGRAM_CHAT_ID=YOUR_CHAT_ID
URLAJAXPOST_FullAnalysis=YOUR_URL
```
3. **Rotate (เปลี่ยน) API Token ทั้งหมดทันที** เพราะ token ปัจจุบันอาจถูก expose แล้ว

---

### CRIT-02: ไม่มีระบบ Authentication — ทุก API Endpoint เปิดให้ใครก็ได้เข้าถึง

**ไฟล์:** [main.rs](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/main.rs#L1844-L1868)

**ปัญหา:**
ทุก API endpoints ไม่มี authentication middleware:
```
POST /api/trade      → เริ่มเทรดด้วยเงินจริง (ไม่ต้อง login!)
POST /api/stop       → หยุดบอท
POST /api/terminate  → ปิดเซิร์ฟเวอร์ทั้งหมด!
POST /api/sell       → ขาย contract
POST /api/notify     → ส่งข้อความ Telegram ในนามของคุณ
POST /api/setup      → เปลี่ยนการตั้งค่าทั้งหมด
GET  /ws             → เชื่อมต่อ WebSocket ดูข้อมูลทั้งหมด
```

ใครก็ตามที่รู้ IP ของเซิร์ฟเวอร์สามารถ:
- เริ่ม/หยุดการเทรดด้วยเงินจริงได้
- ปิดเซิร์ฟเวอร์ทั้งหมดได้ (`/api/terminate`)
- ส่ง Telegram spam ได้ (`/api/notify`)
- แก้ไขกลยุทธ์การเทรดได้ (`/api/setup`)

**วิธีแก้ไข:**
1. เพิ่ม **API Key middleware** หรือ **JWT Authentication**:
```rust
// เพิ่ม middleware สำหรับ verify API key
async fn auth_middleware(
    headers: axum::http::HeaderMap,
    next: axum::middleware::Next,
    request: axum::extract::Request,
) -> Response {
    let api_key = headers.get("X-API-Key")
        .and_then(|v| v.to_str().ok());
    let expected = env::var("SERVER_API_KEY").unwrap_or_default();
    
    if api_key != Some(&expected) {
        return (StatusCode::UNAUTHORIZED, "Invalid API Key").into_response();
    }
    next.run(request).await
}
```
2. ใช้ `axum::middleware::from_fn` กับ sensitive routes
3. อย่างน้อยที่สุด **bind ที่ `127.0.0.1` แทน `0.0.0.0`** เพื่อไม่ให้เข้าถึงจากภายนอก:

```rust
// เปลี่ยนจาก:
let addr = SocketAddr::from(([0, 0, 0, 0], port));
// เป็น:
let addr = SocketAddr::from(([127, 0, 0, 1], port));
```

---

### CRIT-03: Unauthenticated Remote Shutdown — `/api/terminate` ปิดเซิร์ฟเวอร์ได้โดยไม่ต้องยืนยัน

**ไฟล์:** [main.rs L329-349](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/main.rs#L329-L349)

**ปัญหา:**
```rust
async fn handle_post_terminate(State(state): State<AppState>) -> Json<serde_json::Value> {
    // ... ปิดบอท ...
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        std::process::exit(0);  // ❌ ปิดโปรแกรมทั้งหมด!
    });
}
```

- ใครก็ได้สามารถ `curl -X POST http://SERVER:3000/api/terminate` เพื่อปิดเซิร์ฟเวอร์
- ไม่มี graceful shutdown — อาจทำให้ข้อมูลเสียหาย
- ระหว่างเทรด Martingale อยู่ ถ้าโดนปิด จะสูญเสียเงินเพราะไม่ได้ขาย contract

**วิธีแก้ไข:**
1. เพิ่ม authentication ที่ endpoint นี้เป็นลำดับแรก
2. ใช้ graceful shutdown แทน `process::exit(0)`:
```rust
// ใช้ tokio CancellationToken
let shutdown_token = CancellationToken::new();
// ... ส่ง cancel signal แทน process::exit
```
3. ก่อน shutdown ให้ขาย contract ที่เปิดอยู่ทั้งหมดก่อน

---

## 🟠 HIGH — ควรแก้ไขโดยเร็ว

---

### HIGH-01: Path Traversal — ไม่มีการ validate input ที่ใช้สร้าง file path

**ไฟล์:** [main.rs L662-710](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/main.rs#L662-L710)

**ปัญหา:**
หลาย endpoints ใช้ user input ตรงๆ สำหรับสร้าง file path โดยไม่ validate:

```rust
// get_history — ใช้ params.date ตรงๆ
let parts: Vec<&str> = params.date.split('-').collect();
let path_str = format!("tradeData/{}/{}", month_folder, day_folder);

// get_tradehistory_data — ใช้ params.asset ตรงๆ
let path_str = format!("tradeData/{}/{}/{}/trades.json", 
    month_folder, day_folder, params.asset);

// handle_get_analysis_data — ใช้ params.assetcode ตรงๆ
let dir_path = format!("analysisData/{}/{}/{}", 
    month_folder, day_folder, params.assetcode);
```

ผู้โจมตีอาจส่ง `asset=../../etc` เพื่ออ่านไฟล์นอกโฟลเดอร์ที่ต้องการได้

**วิธีแก้ไข:**
```rust
fn sanitize_path_component(input: &str) -> String {
    input.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

// ใช้กับทุก input ที่จะถูกใส่ใน path
let safe_asset = sanitize_path_component(&params.asset);
let path_str = format!("tradeData/{}/{}/{}/trades.json", 
    month_folder, day_folder, safe_asset);
```

---

### HIGH-02: Open Telegram Relay — `/api/notify` เป็น open relay สำหรับส่ง Telegram

**ไฟล์:** [main.rs L491-505](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/main.rs#L491-L505)

**ปัญหา:**
```rust
async fn handle_post_notify(
    Json(payload): Json<NotifyPayload>,
) -> Json<serde_json::Value> {
    send_telegram_message(&payload.message).await;  // ❌ ส่งอะไรก็ได้!
}
```

- ไม่มี rate limiting
- ไม่มี content validation
- ใครก็ส่ง request มาเพื่อ spam Telegram ของคุณได้

**วิธีแก้ไข:**
1. เพิ่ม Rate Limiting (เช่น max 10 ข้อความต่อนาที)
2. เพิ่ม authentication
3. จำกัดความยาว message:
```rust
async fn handle_post_notify(Json(payload): Json<NotifyPayload>) -> ... {
    if payload.message.len() > 500 {
        return Json(json!({"status": "error", "message": "Message too long"}));
    }
    // ... ส่ง
}
```

---

### HIGH-03: Fingerprint Authentication ถูก bypass ได้ง่าย

**ไฟล์:**
- [main.rs L816-878](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/main.rs#L816-L878)
- [allowed_fingerprints.json](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/allowed_fingerprints.json)

**ปัญหา:**
1. **Client-side fingerprinting ปลอมได้ง่ายมาก** — ผู้โจมตีแค่ส่ง fingerprint value ที่ถูกต้องผ่าน HTTP request ก็เข้าได้
2. **Fingerprint whitelist มีแค่ 2 ค่า** และเก็บเป็น plain text
3. **การตรวจสอบ enforce ได้จาก `.env` flag** — ถ้า `USE_FINGERPRINT=no` ก็ข้ามทั้งหมด
4. **ระบบ fingerprint ตรวจสอบที่ client-side เท่านั้น** — server ไม่ได้บังคับเลย (ยกเว้น `/api/verify_fingerprint` endpoint)

**วิธีแก้ไข:**
- ใช้ **server-side session token** แทน fingerprint
- ใช้ **HMAC-based token** ที่มีอายุการใช้งาน
- เพิ่ม **middleware ที่ตรวจสอบทุก request** ไม่ใช่แค่ endpoint เดียว

---

### HIGH-04: WebSocket ไม่มี Authentication — ใครก็เชื่อมต่อดูข้อมูลได้

**ไฟล์:** [main.rs L172-221](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/main.rs#L172-L221)

**ปัญหา:**
```rust
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
    // ❌ ไม่มีการตรวจสอบ authentication
}
```

ใครก็ connect มาที่ `ws://SERVER:3000/ws` แล้วจะได้รับ:
- ข้อมูล candle ทั้งหมด
- ผลลัพธ์การเทรด
- ยอด balance
- bot logs ทั้งหมด

**วิธีแก้ไข:**
ใช้ query parameter token:
```rust
async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    let token = params.get("token").map(|s| s.as_str()).unwrap_or("");
    if !verify_ws_token(token) {
        return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
    }
    ws.on_upgrade(|socket| handle_socket(socket, state))
}
```

---

## 🟡 MEDIUM — ควรวางแผนแก้ไข

---

### MED-01: ไม่มี CORS Configuration — เสี่ยงต่อ Cross-Origin attacks

**ไฟล์:** [main.rs L1844-1869](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/main.rs#L1844-L1869)

**ปัญหา:**
แม้ `tower-http` ถูก import พร้อม feature `cors` แต่ไม่ได้ใช้ CORS middleware จริง — ทำให้ browser อื่นๆ อาจส่ง request มาได้จาก website ภายนอก (CSRF attack)

**วิธีแก้ไข:**
```rust
use tower_http::cors::{CorsLayer, AllowOrigin};

let cors = CorsLayer::new()
    .allow_origin(AllowOrigin::exact("http://localhost:3000".parse().unwrap()))
    .allow_methods([Method::GET, Method::POST])
    .allow_headers([header::CONTENT_TYPE]);

let app = Router::new()
    // ... routes ...
    .layer(cors);
```

---

### MED-02: ไม่มี Rate Limiting — เสี่ยง DoS attack

**ปัญหา:**
ไม่มี rate limiting ในทุก endpoint:
- `/api/trade` สามารถถูก spam เพื่อเปิด trade ซ้ำๆ
- `/api/notify` สามารถถูก spam เพื่อส่ง Telegram ไม่หยุด
- `/api/generate_analysis` เป็น heavy computation — ถูก DoS ได้ง่าย
- `/getanalysisdata` เรียก Deriv API — อาจถูก rate-limit จาก Deriv เอง

**วิธีแก้ไข:**
ใช้ `tower` rate limiting middleware หรือ implement เองด้วย `tokio::time`:
```rust
use tower::limit::RateLimitLayer;
use std::time::Duration;

let app = Router::new()
    // ... routes ...
    .layer(RateLimitLayer::new(50, Duration::from_secs(60))); // 50 req/min
```

---

### MED-03: Sensitive Data Logging — พิมพ์ API Token ลงบน console

**ไฟล์:** [deriv.rs L326-330](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/deriv.rs#L326-L330)

**ปัญหา:**
```rust
println!("🔑 [Multiplex] OTP Request:");
println!("   Token (first 20): {}...", &api_token[..20.min(api_token.len())]);
println!("   App-ID: '{}'", app_id);
println!("   Account-ID: '{}'", account_id);
println!("   Token len: {}", api_token.len());
```

และใน [main.rs L1829](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/main.rs#L1829):
```rust
let preview = if api_token.len() > 10 { &api_token[..10] } else { &api_token };
println!("🔄 Fetching initial balance from Deriv... (token_len={}, prefix={}...)",
    api_token.len(), preview);
```

Partial token ที่ถูก log อาจถูก expose ผ่าน log files, systemd journal, หรือ stdout redirection

**วิธีแก้ไข:**
```rust
// ลบ token preview ออก ให้แสดงแค่ length
println!("🔑 [Multiplex] OTP Request:");
println!("   Token: [REDACTED] (len={})", api_token.len());
println!("   App-ID: [REDACTED]");
println!("   Account-ID: [REDACTED]");
```

---

### MED-04: Blocking File I/O ใน Async Context

**ไฟล์หลายที่ใน:** [main.rs](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/main.rs), [deriv.rs](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/deriv.rs)

**ปัญหา:**
ใช้ `std::fs::read_to_string` และ `std::fs::write` ภายใน async functions — ซึ่งจะ block tokio runtime thread:
```rust
// ตัวอย่างจาก main.rs
let config_content = fs::read_to_string("setup.json").unwrap_or_default();
// ... ในหลาย handlers
```

ใน deriv.rs trading loop ก็มีปัญหาเดียวกัน — อ่าน/เขียนไฟล์ `thereshold.json`, `trades.json`, `setup.json` ซ้ำทุก tick

**วิธีแก้ไข:**
```rust
// ใช้ tokio::fs แทน std::fs
let content = tokio::fs::read_to_string("setup.json").await.unwrap_or_default();

// หรือใช้ spawn_blocking สำหรับ heavy file operations
let content = tokio::task::spawn_blocking(|| {
    std::fs::read_to_string("setup.json").unwrap_or_default()
}).await.unwrap();
```

---

### MED-05: `setup.json` อ่าน/เขียนจากหลาย threads พร้อมกัน — Race Condition

**ปัญหา:**
`setup.json` ถูก:
- **อ่าน** โดย: `background_scheduler_loop` (ทุก 2 วินาที), `handle_post_trade`, `save_setup`, `deriv bot loop` (ทุก tick)
- **เขียน** โดย: `save_setup`, `background_scheduler_loop` (reset scheduleTradeNo, disable schedule)

ไม่มี file lock — ทำให้เกิด data corruption ได้หากเขียนพร้อมกัน

**วิธีแก้ไข:**
- ใช้ `tokio::sync::RwLock` เพื่อ wrap config access
- หรือใช้ file locking: `fs2::FileExt::lock_exclusive()`

---

## 🔵 LOW — ข้อแนะนำเสริม

---

### LOW-01: Fake Login Page ที่ `/login.html` — อาจสร้างความสับสน

**ไฟล์:** [login.html](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/public/login.html)

**ปัญหา:**
Login page นี้เป็น **หน้าหลอก** (decoy) — ไม่ส่ง request ไปที่ backend จริง แค่แสดง error ทุกครั้ง แต่ยังมีอยู่ใน public folder ทำให้ผู้ใช้อาจเข้าใจผิดว่ามีระบบ login จริง

**วิธีแก้ไข:**
- ถ้าต้องการเป็น security honeypot: เพิ่ม logging ส่ง alert เมื่อมีคน submit credentials
- ถ้าไม่ต้องการ: ลบออกเพื่อลดความสับสน
- ถ้าต้องการ login จริง: สร้างระบบ authentication ที่แท้จริง

---

### LOW-02: `unwrap()` calls ที่อาจทำให้ panic ใน production

**ปัญหา:**
พบ `unwrap()` ที่อาจ panic ใน production:

| ไฟล์ | บรรทัด | โค้ด |
|------|--------|------|
| [main.rs](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/main.rs#L1873) | 1873 | `TcpListener::bind(&addr).await.unwrap()` |
| [main.rs](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/main.rs#L1878) | 1878 | `axum::serve(listener, app).await.unwrap()` |
| [deriv.rs](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/deriv.rs#L1028) | 1028 | `DateTime::from_timestamp(...).unwrap()` |

**วิธีแก้ไข:**
ใช้ `expect()` พร้อม message ที่ชัดเจน หรือ handle error อย่างเหมาะสม:
```rust
let listener = tokio::net::TcpListener::bind(&addr)
    .await
    .expect(&format!("Failed to bind to port {}", port));
```

---

### LOW-03: Server Binds to `0.0.0.0` — เปิดรับ connections จากทุก network interface

**ไฟล์:** [main.rs L1872](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/main.rs#L1872)

```rust
let addr = SocketAddr::from(([0, 0, 0, 0], port));
```

ถ้าเครื่อง server มี public IP จะเปิดให้ทุกคนเข้าถึง API ได้โดยตรง

**วิธีแก้ไข:**
```rust
// เปลี่ยนเป็น localhost เว้นแต่ต้องการเข้าจากเครือข่ายอื่นจริงๆ
let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1".to_string());
let addr: SocketAddr = format!("{}:{}", bind_addr, port).parse().unwrap();
```

---

### LOW-04: Large Broadcast Channel — อาจเกิด Memory Issues

**ไฟล์:** [main.rs L1721](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/main.rs#L1721)

```rust
let (tx, _rx) = broadcast::channel(100);
```

Channel capacity = 100 อาจน้อยเกินไป (เห็น `Lagged` errors ใน error handling) หรืออาจ buffer ข้อมูลมากเกินไปถ้า client ไม่ consume

**วิธีแก้ไข:**
- ทำ benchmark เพื่อหา capacity ที่เหมาะสม
- เพิ่ม monitoring สำหรับ `RecvError::Lagged`

---

## 📋 สรุปลำดับความสำคัญของการแก้ไข

### 🔥 ทำทันที (วันนี้):
1. **สร้าง `.gitignore`** — ป้องกัน credential leak
2. **Rotate API Tokens ทั้งหมด** — เปลี่ยน Deriv token, Telegram token
3. **เปลี่ยน bind address เป็น `127.0.0.1`** — ป้องกันการเข้าถึงจากภายนอก

### 🟠 ทำภายในสัปดาห์นี้:
4. เพิ่ม **API Key authentication** สำหรับทุก sensitive endpoint
5. เพิ่ม **input sanitization** สำหรับ path parameters
6. **ลบ token logging** ออกจาก production

### 🟡 ทำภายในเดือนนี้:
7. เพิ่ม **CORS middleware**
8. เพิ่ม **Rate Limiting**
9. แก้ **blocking I/O** เป็น async
10. แก้ **race condition** ใน `setup.json`

---

> **หมายเหตุ:** รายงานนี้ครอบคลุมเฉพาะช่องโหว่ที่พบจากการ code review เท่านั้น
> ไม่ได้ทำ penetration testing จริง หากต้องการความมั่นใจเพิ่มเติม
> ควรทำ dynamic security testing (DAST) และ dependency scanning เพิ่มเติม

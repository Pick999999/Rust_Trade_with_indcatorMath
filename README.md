# 🚀 Rust Trade with Indicator Math (Turbo Indicators Multiplex)

ระบบคำนวณอินดิเคเตอร์เชิงเทคนิคอลความเร็วสูงพิเศษ (High-Performance Technical Indicators) และระบบบอทเทรดอัตโนมัติสำหรับ **Deriv API** พัฒนาด้วยภาษา **Rust** ออกแบบด้วยสถาปัตยกรรม **Zero-Copy**, **Lock-Free Concurrency**, การเร่งความเร็วระดับฮาร์ดแวร์ด้วย **SIMD (AVX2)** และรองรับการกระจายการทำงานแบบ **Multi-Node Sync** ข้ามคลาวด์ (AWS, GCP, Oracle Cloud VPS)

---

## 🌟 คุณสมบัติเด่น (Key Features)

### 1. ⚡ สถาปัตยกรรมความเร็วสูงระดับไมโครวินาที (Microsecond Latency)
- **Zero-Copy & Memory Alignment:** โครงสร้างข้อมูลแท่งเทียน `OHLCV` มีการจัดเรียงหน่วยความจำแบบ 64-byte Cache Line Alignment (`#[repr(C, align(64))]`) เพื่อประสิทธิภาพสูงสุดของ CPU L1/L2/L3 Cache
- **Lock-Free Ring Buffer:** ใช้โครงสร้าง `PriceBuffer` แบบ Ring Buffer ร่วมกับ Atomic Operations (`AtomicUsize`) สำหรับการอ่านและเขียนข้อมูลราคาแบบ Real-Time Streaming โดยไม่มี Lock Contention
- **SIMD Acceleration:** รองรับ Vectorization ประมวลผลข้อมูลทศนิยม 64-bit ได้พร้อมกัน 4 ตัวผ่านคำสั่ง AVX2 ของ CPU พร้อมระบบ Automatic Fallback เป็นการประมวลผลปกติหากฮาร์ดแวร์ไม่รองรับ
- **Pre-Allocated Memory:** ผลลัพธ์อินดิเคเตอร์ใช้ Memory Buffer ที่จองไว้ล่วงหน้า (`IndicatorResult::with_capacity`) ป้องกัน Memory Allocation ซ้ำซ้อนใน Hot Path

### 2. 📊 อินดิเคเตอร์และโมเดลการวิเคราะห์ทางเทคนิค (Core Indicators & Analytics)
- **HMA (Hull Moving Average):** เส้นค่าเฉลี่ยที่ลด Lag ให้ความเร็วและความแม่นยำสูง
- **MACD (Moving Average Convergence Divergence):** คำนวณ MACD Line, Signal และ Histogram
- **RSI (Relative Strength Index):** วัดโมเมนตัม Overbought / Oversold
- **Choppiness Index & ATR / ADX:** ตรวจสอบสภาวะตลาดแบบ Sideway vs Trending และวัดความผันผวน
- **PK Detect Trend & PK Trend v5:** ระบบตรวจจับโครงสร้างแท่งเทียนและทิศทางแนวโน้มตลาด
- **Case Code Evaluation:** ระบบประเมินและจัดหมวดหมู่พฤติกรรมราคาตามรหัสสัญญาณ (Case Codes) เพื่อตัดสินใจสั่งซื้อขาย
- **Predict Next Candle:** โมดูลทำนายทิศทางแท่งเทียนถัดไปด้วยความน่าจะเป็นทางสถิติ
- **Filter Noise:** ตัวกรองสัญญาณรบกวนของราคา (Noise Filter) เพื่อลด False Signal

### 3. 🌐 การทำงานแบบหลายโหนด (Multi-Node Synchronization)
- เชื่อมต่อและซิงค์ข้อมูลสถานะการเทรดระหว่างหลายเซิร์ฟเวอร์พร้อมกัน (เช่น Oracle VPS 3, Oracle VPS 4, AWS, GCP)
- มีระบบ **Primary Hub** และ **Client Nodes** ส่งข้อมูลสรุปผ่าน WebSocket และ REST API แบบ Real-Time
- รองรับการกระจายคำสั่งและรวมรายงานพอร์ต (Multi-Node Portfolio Aggregator)

### 4. 🖥️ Web UI Dashboard & Real-Time WebSocket
- มีเว็บเซิร์ฟเวอร์ในตัวพัฒนาด้วย **Axum** + **Tokio**
- แสดงผล Dashboard ผ่านเว็บแบบ Real-Time (แสดงกราฟแท่งเทียน, สถานะพอร์ต, ประวัติการเทรด และบันทึก Log)
- มีหน้าเอกสาร Swagger / OpenAPI Documentation ในตัวที่ `/docs.html`

---

## 🏗️ โครงสร้างสถาปัตยกรรม (System Architecture)

```
┌─────────────────────────────────────────────────────────────┐
│                    Web Clients & Dashboard                  │
│       (Browser UI / WebSocket Client / Swagger Docs)        │
└──────────────────────────────┬──────────────────────────────┘
                               │ WebSocket / HTTP REST
┌──────────────────────────────▼──────────────────────────────┐
│                    Axum Web Server (Tokio)                  │
│  - REST API Routes (/api/longterm/*, /api/multi_node_summary)│
│  - WebSocket Handlers (/ws, /ws/node-sync)                  │
│  - Authentication Middleware & Fingerprint Verification    │
└──────────────────────────────┬──────────────────────────────┘
                               │
       ┌───────────────────────┼───────────────────────┐
       ▼                       ▼                       ▼
┌──────────────┐       ┌──────────────┐       ┌─────────────────┐
│ Trading Bots │       │ Multi-Node   │       │ Indicator Engine│
│ - Deriv WS   │       │ Sync Worker  │       │ (Zero-Copy &    │
│ - Order Exec │       │ (Hub/Client) │       │  SIMD AVX2)     │
└──────┬───────┘       └──────┬───────┘       └────────┬────────┘
       │                      │                        │
       └──────────────────────┼────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              Hardware / Cache Line Aligned (64B)            │
│       [PriceBuffer] ── Lock-Free Atomic ── [OHLCV Data]     │
└─────────────────────────────────────────────────────────────┘
```

---

## 📁 โครงสร้างโปรเจกต์ (Project Structure)

```text
indicators_Multiplex_Ver1/
├── src/                               # ซอร์สโค้ดภาษา Rust
│   ├── main.rs                        # จุดเริ่มต้นโปรแกรม, Axum Web Server & Routing
│   ├── lib.rs                         # Core Library สำหรับ Zero-Copy Buffer & SIMD
│   ├── indicators.rs                  # ฟังก์ชันคำนวณ HMA, MACD, RSI, Choppiness ฯลฯ
│   ├── simd.rs                        # ฟังก์ชัน Vectorization ด้วย SIMD (AVX2)
│   ├── streaming.rs                   # Streaming Indicator Engine แบบ Lock-Free
│   ├── deriv.rs                       # การเชื่อมต่อ Deriv WebSocket API และการจัดการออเดอร์
│   ├── get_action.rs                  # ลอจิกการตัดสินใจและส่ง Action ในการเทรด
│   ├── full_analysis_ver2.rs          # โมดูลวิเคราะห์ตลาดแบบครอบคลุมเวอร์ชัน 2
│   ├── pkDetectTrend.rs / v5.rs       # โมดูลตรวจจับทิศทางแนวโน้มและโครงสร้างราคา
│   ├── node_sync.rs                   # ระบบซิงค์ข้อมูลระหว่างโหนดเครือข่าย
│   ├── filter_noise.rs                # ตัวกรองสัญญาณรบกวนของราคา
│   ├── predict_next_candle.rs         # โมเดลคาดการณ์แท่งเทียนถัดไป
│   └── long_term.rs                   # กลยุทธ์การเทรดระยะยาว
├── setup/                             # ไฟล์ Configuration รูปแบบ JSON
│   ├── longTermSetup.json             # ค่าพารามิเตอร์ของบอท Long Term
│   └── strategy_case_codes.json       # กฎและเงื่อนไข Case Codes สำหรับกลยุทธ์
├── public/                            # ส่วน Frontend UI และ Dashboard
│   ├── trade_report_dashboard.html    # แดชบอร์ดสรุปผลการเทรด
│   ├── docs.html                      # หน้าเอกสาร API Documentation
│   └── getactionByJS.js               # สคริปต์เสริมสำหรับ Dashboard
├── environment/                       # โฟลเดอร์แยกคอนฟิกสำหรับแต่ละสภาพแวดล้อม (AWS, GCP, Oracle)
├── tradeData/                         # บันทึกประวัติและผลการเทรดรายวัน
├── .env.example                       # ตัวอย่างการตั้งค่า Environment Variables
├── Cargo.toml                         # รายการ Dependencies และการตั้งค่า Build Profile
└── .gitignore                         # กำหนดไฟล์ที่ยกเว้นขึ้น Git (ป้องกันไฟล์ .env อย่างเข้มงวด)
```

---

## ⚙️ ข้อกำหนดเบื้องต้น (Prerequisites)

- **Rust:** เวอร์ชัน 1.75 ขึ้นไป (แนะนำใช้ `rustup` อัปเดตเป็น Stable ล่าสุด)
- **Cargo:** ติดตั้งมาพร้อมกับ Rust Toolchain
- **CPU:** รองรับสถาปัตยกรรม x86_64 ที่มีคำสั่ง AVX2 (หากไม่มี ระบบจะสลับไปใช้ Scalar อัตโนมัติ)
- **ระบบปฏิบัติการ:** Windows 10/11 หรือ Linux (Ubuntu 20.04/22.04 LTS สำหรับ Server Deployment)

---

## 🚀 การติดตั้งและเริ่มต้นใช้งาน (Getting Started)

### 1. โคลน Repository

```bash
git clone https://github.com/Pick999999/Rust_Trade_with_indcatorMath.git
cd Rust_Trade_with_indcatorMath
```

### 2. ตั้งค่าตัวแปรสภาพแวดล้อม (Environment Variables)

คัดลอกไฟล์ `.env.example` เป็น `.env` และแก้ไขค่าให้ตรงกับการใช้งานของคุณ:

```bash
# บน Linux/macOS
cp .env.example .env

# บน Windows PowerShell
Copy-Item .env.example .env
```

แก้ไขไฟล์ `.env`:

```env
# ตั้งค่าพอร์ตและคีย์ความปลอดภัย
PORT=3000
SERVER_API_KEY=your_secure_server_api_key_here
SERVER_CODE=ORACLE_NODE_1

# บัญชี Deriv
DERIV_APP_ID=1089
DERiv_TOKEN=your_deriv_api_token_here
CANDLE_COUNT=1000

# การตั้งค่า Multi-Node Sync (กรณีใช้งานหลายโหนด)
NODE_NAME=Oracle-Node-1
NODE_SYNC_SECRET=your_node_sync_secret_key_here
PRIMARY_HUB_URL=
BACKUP_HUB_URL=
```

> ⚠️ **คำเตือนด้านความปลอดภัย:**  
> ไฟล์ `.env` มีข้อมูล API Token และรหัสผ่านที่สำคัญ ห้ามนำไฟล์ `.env` ขึ้น Git Repository เป็นอันขาด (โปรเจกต์นี้ตั้งค่า `.gitignore` บล็อกไฟล์ `.env` ทั้งหมดไว้เรียบร้อยแล้ว)

### 3. Build และ รันระบบ

#### โหมดทดสอบ (Debug Mode):
```bash
cargo run
```

#### โหมดทำงานจริง (Production Release Mode - แนะนำเพื่อความเร็วสูงสุด):
```bash
cargo run --release
```

เมื่อระบบเริ่มทำงาน จะปรากฏข้อความบน Terminal:
```text
Web Server is running at http://localhost:3000
Serving UI from the 'public/' directory.
📖 Swagger API Documentation: http://localhost:3000/docs.html
```

---

## 🐧 การติดตั้งเป็น Service บน Ubuntu / Linux VPS (Deployment)

หากต้องการให้บอททำงานอัตโนมัติตลอด 24 ชั่วโมง สามารถใช้ `systemd` จัดการ Process ได้:

1. สร้างไฟล์ Service:
```bash
sudo nano /etc/systemd/system/tradeMultiplex.service
```

2. ตัวอย่างการตั้งค่า Service:
```ini
[Unit]
Description=Rust Turbo Indicators Trade Multiplex Server
After=network.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/home/ubuntu/indicators_Multiplex_Ver1
ExecStart=/home/ubuntu/indicators_Multiplex_Ver1/target/release/turbo-indicators
Restart=always
RestartSec=5
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
```

3. สั่งโหลดและเริ่มใช้งาน Service:
```bash
sudo systemctl daemon-reload
sudo systemctl enable tradeMultiplex
sudo systemctl start tradeMultiplex
sudo systemctl status tradeMultiplex
```

---

## 🔒 ความปลอดภัยและข้อควรระวัง (Security & Best Practices)

1. **การรักษาความลับ API Token:** ตรวจสอบให้แน่ใจเสมอว่าไม่ได้เผลอ Commit ไฟล์ `.env` หรือ Hardcode Token ลงในโค้ด
2. **การป้องกันพอร์ต Server:** ในกรณีที่รันบน Cloud VPS (AWS/GCP/Oracle) ควรกำหนด Firewall หรือ Security Group ให้เข้าถึงได้เฉพาะ IP ที่ได้รับอนุญาต หรือใช้งานผ่าน Reverse Proxy (เช่น Nginx) พร้อม SSL/TLS
3. **การทดสอบความเสี่ยง:** ควรทดสอบการทำงานของบอทและกลยุทธ์บน Demo Account ของ Deriv ให้มั่นใจก่อนเชื่อมต่อกับ Real Account

---

## 📄 ใบอนุญาต (License)

โปรเจกต์นี้จัดทำขึ้นเพื่อการวิจัย พัฒนาอัลกอริทึมการเทรด และการวิเคราะห์ทางเทคนิคอลความเร็วสูง

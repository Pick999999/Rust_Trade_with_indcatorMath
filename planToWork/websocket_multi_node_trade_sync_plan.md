# แผนงาน: เชื่อมต่อระบบเทรด Multi-VPS ด้วย Rust WebSocket & Multi-Node Balance Aggregator

## 1. บทนำและวัตถุประสงค์ (Overview & Objective)
สร้างระบบเครือข่ายสื่อสารระหว่างอินสแตนซ์ Rust ข้ามโฮสต์ (Google Cloud, Oracle 3, Oracle 4, AWS และ VPS ใหม่ในอนาคต) เพื่อส่งข้อมูลการเทรดแบบ Real-time (Low Latency) โดยใช้ **WebSocket** ผ่านไลบรารี Rust Asynchronous (`axum` + `tokio-tungstenite`)
* **กินทรัพยากรต่ำมาก:** RAM เพิ่มขึ้นเพียง 1–2 MB ต่อเครื่อง, Bandwidth รวมทั้งเดือน < 300 MB
* **ความเร็วสูง:** ปฏิกิริยาสัญญาณเทรดส่งถึงกันภายในหลัก Millisecond
* **ไม่กระทบการทำงานหลัก:** แยก Thread / Async Task ผ่าน Tokio Event Loop
* **Multi-VPS Balance & Profit Aggregator:** รวบรวมยอด Balance และกำไร/ขาดทุนจากทุก VPS มาคำนวณยอดรวมสุทธิและแสดงผลแบบ Real-time บนหน้า `index_short_term.html`
* **High Availability (ไม่มีจุดตายเดี่ยว):** รองรับการเลือก Hub อิสระ และมีระบบ Failover / Mesh ป้องกันระบบสะดุดเมื่อ Hub ใดตัวหนึ่ง Down

---

## 2. โครงสร้างโหนดและการแก้ปัญหา Hub ล่ม (High Availability Topology)

### รายชื่อโหนดในระบบ:
| โหนด | ผู้ให้บริการ | บทบาทเริ่มต้น | IP / โดเมน | การเชื่อมต่อ |
| :--- | :--- | :--- | :--- | :--- |
| **Node 1** | Oracle-Free-Tier-3 | Worker / Backup Hub | `161.118.203.228:3000` | VCN Direct / Public |
| **Node 2** | Oracle-Free-Tier-4 | Worker | `161.118.217.177:3000` | VCN Direct / Public |
| **Node 3** | Google Cloud | Primary Hub / Master | `gpkderiv.shop` | Public / HTTPS-WSS |
| **Node 4+** | AWS / VPS เพิ่มเติม | Worker (Plug & Play) | `pkderiv.online` / อื่นๆ | Public WSS |

### การรับมือกรณี Hub Down (ป้องกันระบบสะดุด):
เพื่อไม่ให้ระบบสะดุดเมื่อโหนดที่เป็น Hub ดับ หรือเน็ตเวิร์กของคลาวด์เจ้านั้นมีปัญหา เราออกแบบรองรับ **2 รูปแบบ** ที่สามารถเลือกปรับได้:

#### รูปแบบที่ 1: Dynamic Hub with Auto-Failover (แนะนำสำหรับเซ็ตอัปง่าย)
* **เลือก VPS ตัวไหนเป็น Hub ก็ได้** ผ่านตัวแปรคอนฟิกใน `.env`:
  ```env
  NODE_ROLE="hub"          # หรือ "client"
  ```
* **ระบบ Hub สำรอง (Primary & Standby Hub):**
  * เครื่องลูก (Client) ทุกตัวจะตั้งค่าที่อยู่ Hub หลัก และ Hub สำรอง:
    ```env
    PRIMARY_HUB_URL="wss://gpkderiv.shop/ws/node-sync"
    BACKUP_HUB_URL="ws://161.118.203.228:3000/ws/node-sync"
    ```
  * **Auto-Failover Mechanism:** หาก Primary Hub ขาดการส่ง Heartbeat เกิน 5 วินาที หรือต่อไม่ติด เครื่องลูกทุกตัวจะตัดสลับไปต่อกับ Backup Hub ทันทีแบบอัตโนมัติ ทำให้การส่งข้อมูลดำเนินต่อไปได้โดยไม่สะดุด
  * เมื่อ Primary Hub ฟื้นตัว สามารถ Re-balance กลับมาได้

#### รูปแบบที่ 2: Full Mesh Topology (Peer-to-Peer: ไร้จุดตายเดี่ยว 100%)
* สำหรับระบบที่มี VPS 3–5 เครื่อง ทุกโหนดจะเปิด WebSocket Server และเชื่อมโยงหากันและกันโดยตรง (A ↔ B, B ↔ C, A ↔ C)
* **ข้อดีเด่น:** **ไม่มี Hub ตัวใดตัวหนึ่งเป็นจุดตายเดี่ยว (Zero Single Point of Failure)** หาก Google Cloud ล่ม Oracle 3 กับ Oracle 4 ก็ยังคุยกันต่อเนื่อง 100% 
* แต่ละเครื่องกิน Connection เพียง 2–4 connections ซึ่ง Rust และ Tokio รองรับได้สบายมากโดยไม่เปลืองทรัพยากร

---

## 3. รูปแบบข้อมูลที่ส่งระหว่างกัน (Message Schema)

ข้อมูลทั้งหมด serialize ด้วย `serde_json` ใน Rust เพื่อประสิทธิภาพสูงสุด:

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NodeMessage {
    // 1. ส่งเมื่อมีสัญญาณเทรดใหม่เกิดขึ้น (ป้องกันการเข้าเทรดชนกัน)
    TradeSignal {
        from_node: String,       // เช่น "Oracle-3", "Oracle-4", "GoogleCloud"
        symbol: String,          // เช่น "1HZ75V", "R_100"
        action: String,          // "BUY" หรือ "SELL"
        price: f64,
        timestamp: u64,
        setup_id: String,
    },
    // 2. ส่งเมื่อเปิดหรือปิดออเดอร์สำเร็จ
    OrderExecuted {
        from_node: String,
        contract_id: u64,
        symbol: String,
        action: String,
        stake: f64,
        profit_loss: Option<f64>,
        status: String,          // "OPEN", "WON", "LOST"
    },
    // 3. ข้อมูลสรุปยอดสำหรับ Multi-VPS Aggregator
    BalanceUpdate {
        from_node: String,        // เช่น "Oracle-3"
        account_id: String,       // เช่น "CR123456"
        balance: f64,             // ยอดเงินคงเหลือล่าสุด
        round_profit_usd: f64,    // กำไรรอบนี้ (USD)
        round_profit_thb: f64,    // กำไรรอบนี้ (THB)
        today_profit_usd: f64,    // กำไรวันนี้ (USD)
        today_profit_thb: f64,    // กำไรวันนี้ (THB)
        win_count: u32,           // ชนะ (ไม้)
        loss_count: u32,          // แพ้ (ไม้)
        is_trading: bool,         // สถานะกำลังเทรดอยู่หรือไม่
        timestamp: u64,
    },
    // 4. สัญญาณชีพจรตรวจสอบความพร้อมของโหนด (Heartbeat)
    Ping { timestamp: u64 },
    Pong { timestamp: u64 },
}
```

---

## 4. ระบบ Multi-VPS Balance & Profit Aggregator

### โครงสร้างการทำงานภายใน Rust (`Aggregator State`):
1. แต่ละโหนดเก็บ State ตารางสรุปของทุกโหนดไว้ใน Memory:
   ```rust
   pub struct MultiNodeAggregator {
       pub nodes: Arc<RwLock<HashMap<String, NodeBalanceState>>>,
   }
   ```
2. เมื่อได้รับข้อความ `BalanceUpdate` จากโหนดใด จะอัปเดตข้อมูลของโหนดนั้น พร้อมคำนวณ **Grand Total รวมทุก VPS**:
   * **Grand Total Balance:** ผลรวมยอดเงินทุกบัญชี
   * **Grand Total Round Profit ($/฿):** ผลรวมกำไรรอบนี้จากทุกเครื่อง
   * **Grand Total Today Profit ($/฿):** ผลรวมกำไรวันนี้ทั้งหมด
   * **Win / Loss Ratio:** อัตราชนะ/แพ้รวมของทุกโหนด
3. Rust จะส่งข้อมูลสรุปนี้ผ่าน Local WebSocket (`ws://localhost:3000/ws`) ให้กับหน้าเบราว์เซอร์ `index_short_term.html` อัตโนมัติ

### การแสดงผลบนหน้า `index_short_term.html`:
เพิ่มส่วนแสดงผล **"Multi-VPS Live Overview"** เหนือหรือในกล่อง Balance เดิม:
1. **การ์ดยอดรวมสุทธิ (All VPS Grand Total):**
   * ยอดเงินรวมทุกพอร์ต (Total Balance)
   * กำไรรวมรอบนี้ (Total Round Profit)
   * กำไรรวมวันนี้ (Total Today Profit)
2. **การ์ดแยกสถานะรายโหนด (Per-Node Badges):**
   * แสดงชื่อโหนด + จุดสถานะ (🟢 Online / 🔴 Offline หรือหลุดการเชื่อมต่อ)
   * แสดง Balance และกำไรวันนี้ของแต่ละเครื่อง
   * คลิกที่ Badge เพื่อสลับไปเปิดหน้าต่างของ VPS นั้นได้ทันที

---

## 5. ความปลอดภัยและการยืนยันตัวตน (Security & Authentication)

1. **Pre-Shared Secret Key (PSK):**
   * กำหนด Token ลับไว้ใน `.env` เช่น `NODE_SYNC_SECRET=your_ultra_secure_token_here`
   * ตอน Client เชื่อมต่อ จะส่ง Header หรือ Query Param: `wss://.../ws/node-sync?token=your_ultra_secure_token_here`
2. **IP Whitelisting & Firewall Rules:**
   * ตรวจสอบ IP ต้นทาง ไม่รับการเชื่อมต่อจากบุคคลภายนอกที่ไม่ได้รับอนุญาต
3. **Internal VCN Routing (Oracle 3 <-> Oracle 4):**
   * สำหรับเครื่องใน Data Center เดียวกัน สามารถใช้ Private IP วงภายในเพื่อความปลอดภัยสูงสุด

---

## 6. ขั้นตอนการเพิ่ม VPS ใหม่เข้าสู่ระบบ (How to Add a New VPS)

การเพิ่ม VPS ใหม่สามารถทำได้แบบ **Plug & Play** โดยไม่ต้องแก้ไขโค้ด:
1. นำ Binary Rust และไฟล์หน้าเว็บไปติดตั้งบน VPS ใหม่
2. ตั้งค่าไฟล์ `.env` ของเครื่องใหม่:
   ```env
   NODE_NAME="Oracle-5"
   NODE_ROLE="client"
   PRIMARY_HUB_URL="wss://gpkderiv.shop/ws/node-sync"
   BACKUP_HUB_URL="ws://161.118.203.228:3000/ws/node-sync"
   NODE_SYNC_SECRET="secret_token_เดียวกัน"
   DERIV_API_TOKEN="token_พอร์ตใหม่"
   ```
3. รันโปรแกรม Rust บนเครื่องใหม่ โหนดจะ Connect เข้า Hub อัตโนมัติ และเริ่ม Broadcast ยอด Balance เข้าสู่หน้าจอรวมทันที
4. เพิ่มชื่อและ IP ของ VPS ใหม่ใน `vpsMasterList` ใน [public/pkderiv.js](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/public/pkderiv.js) เพื่อให้มีปุ่มสลับเครื่องบน Navigation Bar

---

## 7. แผนขั้นตอนการพัฒนา (Step-by-Step Implementation Roadmap)

### เฟส 1: สร้างโมดูลสื่อสาร & Failover ใน Rust (`src/node_sync.rs`)
- [ ] สร้างไฟล์ [src/node_sync.rs](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/src/node_sync.rs)
- [ ] กำหนด Struct `NodeMessage`, `NodeBalanceState`, และ `MultiNodeAggregator`
- [ ] เขียน Server Handler (Axum WS) สำหรับรองรับการเชื่อมต่อเข้ามา
- [ ] เขียน Client Connector (Tokio Tungstenite) พร้อม **Auto-Failover** (สลับ Primary ↔ Backup Hub อัตโนมัติเมื่อตรวจพบเน็ตหลุด)

### เฟส 2: พัฒนาระบบ Aggregator & ผูกเข้ากับระบบเทรดหลัก (`src/main.rs`)
- [ ] เมื่อออเดอร์เปิด/ปิด หรือยอด Balance มีการเปลี่ยนแปลง ให้ส่ง `NodeMessage::BalanceUpdate` เข้าสู่เครือข่าย Sync ทันที
- [ ] ฟังก์ชันคำนวณ Grand Total (รวม Balance, รวม Profit ทุกโหนด)
- [ ] ส่งข้อความ WebSocket แบบ Real-time ไปยังหน้าเว็บเบราว์เซอร์

### เฟส 3: ปรับปรุง UI หน้า `index_short_term.html` & `public/pkderiv.js`
- [ ] เพิ่ม UI Component สำหรับแสดง **Multi-VPS Balance & Profit Summary** ใน [index_short_term.html](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/public/index_short_term.html)
- [ ] เขียนฟังก์ชันใน [pkderiv.js](file:///d:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/public/pkderiv.js) เพื่อดักจับ Event สรุปยอด Multi-VPS แล้วอัปเดตตัวเลขแบบ Real-time
- [ ] แสดงสถานะ Online/Offline ของแต่ละ VPS บน Dashboard

### เฟส 4: ทดสอบการเชื่อมต่อระดับ Local และ Dev
- [ ] ทดสอบรัน 2–3 Instance จำลองบนเครื่อง Local ด้วยพอร์ตต่างกัน (เช่น 3000, 3001, 3002)
- [ ] ทดสอบปิดเครื่องหลัก (Simulate Hub Down) เพื่อตรวจสอบว่าโหนดลูกตัดสลับไปหา Backup Hub ได้อย่างราบรื่นหรือไม่
- [ ] ตรวจสอบความถูกต้องของการคำนวณยอดรวม Balance และ Profit

### เฟส 5: ติดตั้งและทดสอบใช้งานจริงบน Production VPS
- [ ] Build และ Deploy Binary บน **Google Cloud** และ **Oracle 3**
- [ ] Sync Binary ไปยัง **Oracle 4** และ **AWS**
- [ ] มอนิเตอร์ Log การเชื่อมต่อและสถานะ Sync ผ่าน Web Terminal / VPS Schedule Manager

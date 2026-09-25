// ============================================================
//  Deriv WebSocket Trading Flow - Pure JavaScript
//  ขั้นตอน: Proposal → Buy → Track Order (จนปิด contract)
// ============================================================

const API_TOKEN = "YOUR_API_TOKEN";   // <<< ใส่ token ของคุณ
const WS_URL   = "wss://ws.binaryws.com/websockets/v3?app_id=1089";

// ── สร้าง WebSocket connection ──────────────────────────────
const ws = new WebSocket(WS_URL);

// ── เก็บ req_id ล่าสุด เพื่อแยกแต่ละ request ────────────────
let reqId = 1;
const pendingRequests = {}; // { req_id: "proposal" | "buy" | "track" }

// ============================================================
//  1. เมื่อ connection เปิดสำเร็จ → Authorize ก่อนเสมอ
// ============================================================
ws.addEventListener("open", () => {
  console.log("✅ WebSocket connected");
  authorize();
});

// ============================================================
//  2. รับ message กลับมา → แยกตาม msg_type
// ============================================================
ws.addEventListener("message", (event) => {
  const data = JSON.parse(event.data);

  // ถ้า API ส่ง error กลับมา → หยุดทำงาน
  if (data.error) {
    console.error("❌ API Error:", data.error.message);
    return;
  }

  // แยก handler ตาม msg_type ที่ Deriv ส่งกลับมา
  switch (data.msg_type) {

    case "authorize":
      // Authorization สำเร็จ → เริ่มขอ Proposal
      console.log("🔑 Authorized as:", data.authorize.loginid);
      sendProposal();
      break;

    case "proposal":
      // ได้ proposal กลับมาแล้ว → ดู ask_price แล้วส่งคำสั่ง Buy
      console.log("📋 Proposal received:");
      console.log("   ID       :", data.proposal.id);
      console.log("   Ask Price:", data.proposal.ask_price);
      console.log("   Payout   :", data.proposal.payout);
      buyContract(data.proposal.id, data.proposal.ask_price);
      break;

    case "buy":
      // Buy สำเร็จ → ได้ contract_id มา → เริ่ม track
      console.log("🛒 Buy successful:");
      console.log("   Contract ID  :", data.buy.contract_id);
      console.log("   Buy Price    :", data.buy.buy_price);
      console.log("   Payout       :", data.buy.payout);
      trackOrder(data.buy.contract_id);
      break;

    case "proposal_open_contract":
      // Track order กลับมา → เช็คว่า contract ปิดหรือยัง
      handleTrackOrder(data.proposal_open_contract);
      break;

    default:
      // msg_type อื่น ๆ ที่ไม่ได้ handle (เช่น ping, etc.)
      console.log("ℹ️ Unhandled msg_type:", data.msg_type);
  }
});

// ============================================================
//  3. ถ้า connection ถูกปิด
// ============================================================
ws.addEventListener("close", () => {
  console.log("🔌 WebSocket closed");
});

// ============================================================
//  ฟังก์ชัน Helper: ส่ง JSON ผ่าน WebSocket
// ============================================================
function send(payload) {
  payload.req_id = reqId++;             // แนบ req_id ทุกครั้ง
  ws.send(JSON.stringify(payload));
  console.log("📤 Sent:", payload);
}

// ============================================================
//  STEP 1: Authorize
// ============================================================
function authorize() {
  send({
    authorize: API_TOKEN
  });
}

// ============================================================
//  STEP 2: ขอ Proposal
//  - contract_type: "CALL" (ราคาขึ้น) | "PUT" (ราคาลง)
//  - duration: ระยะเวลา | duration_unit: "t"=tick, "s"=วินาที, "m"=นาที
//  - basis: "stake" (กำหนดเงินเดิมพัน) | "payout" (กำหนด payout)
//  - symbol: instrument เช่น "R_100", "frxEURUSD"
// ============================================================
function sendProposal() {
  console.log("\n──────────────────────────────────");
  console.log("📝 STEP 1: Requesting Proposal...");

  send({
    proposal: 1,
    contract_type: "CALL",      // CALL = คาดว่าราคาจะขึ้น
    symbol: "R_100",            // Volatility 100 Index
    duration: 5,                // ระยะเวลา 5
    duration_unit: "t",         // หน่วย: t = ticks
    basis: "stake",             // กำหนดเงินเดิมพัน
    amount: 10,                 // เดิมพัน 10 USD
    currency: "USD"
  });
}

// ============================================================
//  STEP 3: Buy Contract
//  - proposal_id: ID ที่ได้จาก proposal response
//  - price: ราคาที่ยอมจ่าย (ควรใส่ ask_price จาก proposal)
// ============================================================
function buyContract(proposalId, askPrice) {
  console.log("\n──────────────────────────────────");
  console.log("🛒 STEP 2: Buying contract...");

  send({
    buy: proposalId,            // proposal ID ที่ได้มา
    price: askPrice             // จ่ายตาม ask_price ที่ proposal บอก
  });
}

// ============================================================
//  STEP 4: Track Order (subscribe เพื่อรับ update แบบ real-time)
//  - subscribe: 1 → Deriv จะส่ง update กลับมาทุก tick จนปิด
// ============================================================
function trackOrder(contractId) {
  console.log("\n──────────────────────────────────");
  console.log("📡 STEP 3: Tracking order, contract_id:", contractId);

  send({
    proposal_open_contract: 1,
    contract_id: contractId,
    subscribe: 1                // รับ update แบบ real-time
  });
}

// ============================================================
//  Handler: จัดการ track order response
// ============================================================
function handleTrackOrder(contract) {
  console.log("\n📊 Contract Update:");
  console.log("   Status      :", contract.status);           // open / sold / won / lost
  console.log("   Underlying  :", contract.underlying);       // ชื่อ asset
  console.log("   Entry Spot  :", contract.entry_spot);       // ราคาเข้า
  console.log("   Current Spot:", contract.current_spot);     // ราคาปัจจุบัน
  console.log("   Profit/Loss :", contract.profit);           // กำไร/ขาดทุน ณ ตอนนี้

  // ── ตรวจว่า contract ปิดแล้วหรือยัง ──
  if (contract.is_sold) {
    // Contract ปิดแล้ว (หมดเวลา หรือถูก sell ออก)
    console.log("\n🏁 Contract CLOSED!");
    console.log("   Exit Spot   :", contract.exit_tick_display_value);
    console.log("   Final Profit:", contract.profit);
    console.log("   Result      :", contract.profit >= 0 ? "✅ WIN" : "❌ LOSS");

    // ยกเลิก subscription เพื่อไม่รับ update เพิ่ม
    forgetSubscription(contract.id);

    // ปิด WebSocket เมื่อ flow จบ
    ws.close();

  } else {
    // Contract ยังเปิดอยู่ → รอรับ update tick ถัดไปต่อ
    console.log("   ⏳ Contract still open, waiting...");
  }
}

// ============================================================
//  ยกเลิก Subscription เมื่อไม่ต้องการ track แล้ว
// ============================================================
function forgetSubscription(subscriptionId) {
  send({
    forget: subscriptionId
  });
  console.log("🔕 Unsubscribed from:", subscriptionId);
}
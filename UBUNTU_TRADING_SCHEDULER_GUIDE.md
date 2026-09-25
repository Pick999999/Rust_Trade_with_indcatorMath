# คู่มือเปรียบเทียบแนวทางการตั้งเวลาเทรดอัตโนมัติบน Ubuntu (Trading Scheduler Guide)

เอกสารฉบับนี้สรุปแนวทางและเทคนิคในการตั้งเวลาเปิด-ปิดการเทรดอัตโนมัติ (Automated Trading Schedule) บนระบบปฏิบัติการ **Ubuntu Linux** โดยเฉพาะระบบเทรดอนุพันธ์ความเร็วสูง (เช่น Deriv API / Binary Options / Forex) เพื่อใช้เป็นแนวทางในการตัดสินใจเลือกระบบที่เหมาะสมที่สุด

---

## สารบัญ
1. [ข้อควรพิจารณาสำคัญสำหรับระบบเทรด (Trading System Constraints)](#1-ข้อควรพิจารณาสำคัญสำหรับระบบเทรด)
2. [แนวทางที่ 1: Linux Crontab (พื้นฐานและรวดเร็ว)](#แนวทางที่-1-linux-crontab-พื้นฐานและรวดเร็ว)
3. [แนวทางที่ 2: Systemd Timers (มาตรฐาน Production ระดับองค์กร)](#แนวทางที่-2-systemd-timers-มาตรฐาน-production-ระดับองค์กร)
4. [แนวทางที่ 3: In-App Time Gate ใน Rust (แนะนำสูงสุดสำหรับบอทเทรด)](#แนวทางที่-3-in-app-time-gate-ใน-rust-แนะนำสูงสุดสำหรับบอทเทรด)
5. [แนวทางที่ 4: Webhook / REST API ควบคุมจาก Dashboard หรือ PHP](#แนวทางที่-4-webhook--rest-api-ควบคุมจาก-dashboard-หรือ-php)
6. [ตารางเปรียบเทียบข้อดี-ข้อจำกัด (Comparison Matrix)](#ตารางเปรียบเทียบข้อดี-ข้อจำกัด)

---

## 1. ข้อควรพิจารณาสำคัญสำหรับระบบเทรด

ระบบเทรดต่างจากงานทั่วไป (เช่น แบ็กอัปฐานข้อมูล หรือลบไฟล์ขยะ) เนื่องจากมีข้อจำกัดทางเทคนิคเฉพาะตัว:

> [!IMPORTANT]
> 1. **การวอร์มข้อมูลอินดิเคเตอร์ (Indicator Warm-up):**
>    อินดิเคเตอร์อย่าง EMA 50/200, Bollinger Bands, ATR, ADX ต้องใช้แท่งเทียนย้อนหลังหลายสิบแท่ง หากเปิดโปรแกรมสตาร์ทตอน 01:00 น. พอดี บอทจะยังคำนวณกราฟไม่ได้จนกว่าประวัติแท่งเทียนจะโหลดครบ
> 2. **WebSocket Connection Overhead:**
>    การเชื่อมต่อ WebSocket และ Subscribe สตรีมติ๊ก/แท่งเทียน มีเวลา Handshake และ Ping-Pong หากเปิด-ปิดบอทบ่อยๆ อาจเจอปัญหา Reconnect ล่าช้า
> 3. **ความแม่นยำระดับวินาที/มิลลิวินาที:**
>    Cron ทั่วไปรันได้ละเอียดที่สุดระดับ "นาที" แต่การเทรดอาจต้องตัดสินใจ ณ วินาทีที่ 00 ของแท่งเทียน

---

## แนวทางที่ 1: Linux Crontab (พื้นฐานและรวดเร็ว)

### 📌 รูปแบบการทำงาน
ใช้ยูทิลิตี้พื้นฐานของ Linux สั่งรันสคริปต์หรือยิง cURL ตามนาทีและชั่วโมงที่ระบุ

### ⚙️ ตัวอย่างการตั้งค่า (`crontab -e`)
```cron
# รันทุกวันเวลา 01:00 น. และ 05:00 น. เพื่อส่งคำสั่งเริ่มเทรด
0 1,5 * * * /usr/bin/curl -s -X POST http://127.0.0.1:3000/api/trade/control -H "Content-Type: application/json" -d '{"action":"start"}' >> /var/log/trading_cron.log 2>&1

# รันทุกวันเวลา 02:00 น. และ 06:00 น. เพื่อส่งคำสั่งหยุดเทรด
0 2,6 * * * /usr/bin/curl -s -X POST http://127.0.0.1:3000/api/trade/control -H "Content-Type: application/json" -d '{"action":"stop"}' >> /var/log/trading_cron.log 2>&1
```

### ✅ ข้อดี:
- ใช้งานง่ายมาก มีใน Ubuntu ทุกเครื่อง ไม่ต้องติดตั้งอะไรเพิ่ม
- ใช้ร่วมกับสคริปต์ Shell / Python / PHP ที่มีอยู่เดิมได้ทันที

### ❌ ข้อจำกัด:
- ควบคุมได้ละเอียดแค่ระดับ "นาที" (ไม่สามารถตั้งระดับวินาทีได้)
- เป็นแบบ Fire-and-forget (ยิงคำสั่งแล้วจบ) ไม่มีการตรวจสอบว่าคำสั่งสำเร็จจริงหรือไม่ หรือ Service ค้างหรือไม่

---

## แนวทางที่ 2: Systemd Timers (มาตรฐาน Production ระดับองค์กร)

บน Ubuntu รุ่นใหม่ นิยมใช้ **Systemd Timers** แทน Cron สำหรับ Service หลักของระบบ

### 📌 รูปแบบการทำงาน
สร้างไฟล์ 2 ส่วน:
1. `trading-session.service` (ตัวกำหนด Action ว่าจะให้ทำอะไร)
2. `trading-session.timer` (ตัวจับเวลาตามปฏิทิน OnCalendar)

### ⚙️ ตัวอย่างการตั้งค่า

#### ไฟล์ `/etc/systemd/system/trading-session.timer`:
```ini
[Unit]
Description=Timer for Trading Sessions (01:00 and 05:00)

[Timer]
# รันทุกวันตอน 01:00 น. และ 05:00 น.
OnCalendar=*-*-* 01,05:00:00
Persistent=true

[Install]
WantedBy=timers.target
```

#### คำสั่งจัดการ:
```bash
sudo systemctl daemon-reload
sudo systemctl enable --now trading-session.timer
# ดูรอบเวลาถัดไปที่จะทำงาน
systemctl list-timers --all
# ดูประวัติ Log การทำงาน
journalctl -u trading-session.service -f
```

### ✅ ข้อดี:
- เชื่อถือได้สูงสุด มีระบบคุม Dependency
- ดูประวัติ Log รวมศูนย์ผ่าน `journalctl` สะดวกมาก
- มีระบบ Restart อัตโนมัติหากโปรเซสเกิด Crash

### ❌ ข้อจำกัด:
- การเขียน Configuration มีขั้นตอนมากกว่า Crontab เล็กน้อย

---

## แนวทางที่ 3: In-App Time Gate ใน Rust (แนะนำสูงสุดสำหรับบอทเทรด)

> [!TIP]
> **หลักการทำงาน:** เปิดโปรแกรม Rust รันแบบ Background Service ตลอด 24 ชั่วโมง เพื่อให้ WebSocket ทำงานต่อเนื่องและคำนวณ Indicators เตรียมไว้ตลอดเวลา แต่มี **ตัวตรวจจับเวลา (Time Gate Filter)** ภายในโค้ด

### 📌 สถาปัตยกรรม (Architecture Diagram)
```text
┌─────────────────────────────────────────────────────────────┐
│                 Rust Engine (รัน 24/7)                       │
│                                                             │
│  [Deriv WebSocket] ──> [Live Ticks / Candles]                │
│                              │                              │
│                              ▼                              │
│                 [Indicators Calculation]                    │
│                 (EMA, ATR, BB, ADX, Choppy)                 │
│                              │                              │
│                              ▼                              │
│                 ┌─────────────────────────┐                 │
│                 │      Time Gate Gate     │                 │
│                 │ เวลา 01:00-02:00 หรือ   │                 │
│                 │      05:00-06:00 ?      │                 │
│                 └────────────┬────────────┘                 │
│                              │                              │
│               ┌──────────────┴──────────────┐               │
│               │ ใช่                         │ ไม่ใช่        │
│               ▼                             ▼               │
│      [ส่ง Action สู่ตลาดจริง]           [IDLE / พักรอ]       │
└─────────────────────────────────────────────────────────────┘
```

### ⚙️ โค้ดตัวอย่างแนวคิดใน Rust (ใช้ไลบรารี `chrono`):
```rust
use chrono::{Local, Timelike};

pub fn is_trading_session_active() -> bool {
    let now = Local::now();
    let hour = now.hour();

    // กำหนดช่วงเวลาเทรด: ช่วงชั่วโมงที่ 1 (01:00-01:59) และชั่วโมงที่ 5 (05:00-05:59)
    match hour {
        1 | 5 => true,
        _ => false,
    }
}

// ในลูปประเมินผล Action
pub fn evaluate_trade_signal(analysis: &FullAnalysisResult) -> &'static str {
    // 1. ตรวจสอบเงื่อนไขเวลาเป็นด่านแรก
    if !is_trading_session_active() {
        return "IDLE_OFF_SESSION";
    }

    // 2. หากอยู่ในเวลาเทรด ค่อยตรวจสอบ Pre-filters (ADX, Choppy, BB Squeeze)
    // 3. ตรวจสอบ Case Code เพื่อออกออเดอร์ CALL / PUT
    match_strategy_cases(analysis)
}
```

### ✅ ข้อดี:
- **ข้อมูลไม่ขาดตอน (No Cold Start):** อินดิเคเตอร์พร้อมยิงออเดอร์ตั้งแต่วินาทีแรกที่เข้าสู่รอบเวลา
- **ความแม่นยำระดับมิลลิวินาที (ms):** ตัดสินใจทันทีตามเวลาเครื่อง ไม่มีความหน่วงจากการ Reconnect
- กำหนดตารางเวลาผ่านไฟล์ config เช่น `setup/trading_schedule.json` แล้วสั่ง Hot-reload ได้ทันทีโดยไม่ต้องรีสตาร์ทบอท

### ❌ ข้อจำกัด:
- บอทกิน RAM และ CPU ต่อเนื่องตลอดเวลา (แต่สำหรับ Rust ใช้ทรัพยากรน้อยมาก เพียง 20-50MB RAM)

---

## แนวทางที่ 4: Webhook / REST API ควบคุมจาก Dashboard หรือ PHP

### 📌 รูปแบบการทำงาน
ใช้ตัวจัดการฝั่งเว็บหรือสคริปต์ภายนอก เช่น [vps_schedule_manager.php](file:///D:/Rust/turbo-indicators/turbo-indicators_v2/indicators_Multiplex_Ver1/php/vps_schedule_manager.php) สั่งส่ง Request มาที่ Backend

* **ข้อดี:** ผู้ใช้งานสามารถดูสถานะและกดแทรกแซง (Manual Override) ผ่านหน้าจอเว็บได้ เช่น ถ้าเห็นข่าวแรง สามารถกด "Pause Trading" ชั่วคราวได้ทันที

---

## ตารางเปรียบเทียบข้อดี-ข้อจำกัด

| มิติการเปรียบเทียบ | 1. Linux Crontab | 2. Systemd Timer | 3. In-App Rust Gate (แนะนำ) | 4. Webhook / Dashboard |
| :--- | :---: | :---: | :---: | :---: |
| **ความง่ายในการติดตั้ง** | ง่ายมาก ⭐⭐⭐⭐⭐ | ปานกลาง ⭐⭐⭐ | ต้องเขียนโค้ดเพิ่ม ⭐⭐⭐ | ปานกลาง ⭐⭐⭐ |
| **ความแม่นยำของเวลา** | ระดับนาที | ระดับวินาที | ระดับมิลลิวินาที ⭐⭐⭐⭐⭐ | ขึ้นกับตัวทริกเกอร์ |
| **ความพร้อมของอินดิเคเตอร์** | ❌ ข้อมูลอาจไม่ทัน (Cold) | ❌ ข้อมูลอาจไม่ทัน (Cold) | ✅ อุ่นข้อมูลพร้อม 24 ชม. ⭐⭐⭐⭐⭐ | ✅ อุ่นข้อมูลพร้อม |
| **ระบบดู Log / ตรวจสอบ** | ไฟล์ Text ธรรมดา | `journalctl` ยอดเยี่ยม ⭐⭐⭐⭐⭐ | Log รวมใน Rust App | หน้าเว็บ Dashboard ⭐⭐⭐⭐⭐ |
| **การกด Override ด้วยมือ** | ทำได้ยาก | ต้องใช้ SSH สั่ง | ต้องมี Endpoint รองรับ | ง่ายมากผ่าน UI ⭐⭐⭐⭐⭐ |

---

## 🎯 สรุปคำแนะนำสำหรับระบบนี้

สำหรับโปรเจกต์ **`indicators_Multiplex_Ver1`**:
1. **แกนหลัก:** ใช้ **แนวทางที่ 3 (In-App Time Gate ใน Rust)** โดยให้ Rust รัน WebSocket รอไว้ตลอดเวลาเพื่อความสมบูรณ์ของ Indicators
2. **การสั่งการและตั้งค่า:** อ่านช่วงเวลาเทรด (เช่น `["01:00-02:00", "05:00-06:00"]`) จากไฟล์ JSON ใน `setup/` เพื่อให้สามารถเปลี่ยนเวลาหรือเปิด-ปิดได้จากหน้าเว็บทันที
3. **ระบบเฝ้าระวัง:** ใช้ **Systemd Service** ในการควบคุมโปรเซสของ Rust ให้ทำงานตลอดเวลาและเปิดใหม่เองอัตโนมัติหาก VPS รีบูต

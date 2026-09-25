# Critical Bug Fixes — ห้ามลบหรือแก้ไข

## Schedule Trade Round +1 Bug

บัก: เมื่อเปิดหน้าเว็บ (index_short_term.html) ระหว่างที่ backend กำลังเทรดอยู่ (เช่นจาก Schedule Auto Trade) 
รอบการเทรด (totalTrade / scheduleTradeNo) จะถูก +1 ซ้ำผิดพลาด เพราะ Frontend Schedule Timer สั่ง goTradeBtn.click() ซ้ำ

### จุดที่ห้ามแก้ไขหรือลบออก (3 จุด):

1. **pkderiv.js — fetchTradeControl()** (~line 244-260)
   - เมื่อ `tc.TradeStatus === 'กำลังเทรด'` ต้องตั้ง `isTradeRunning = true` ทันที
   - ห้ามเปลี่ยนกลับเป็น "รอ WebSocket ยืนยัน" เพราะจะทำให้ Schedule Timer trigger ซ้ำ

2. **pkderiv.js — SCHEDULE TIMER setInterval** (~line 3187-3230)
   - ก่อน auto-start เทรด ต้อง `fetch('/api/status')` ตรวจสอบก่อนว่าบอทรันอยู่แล้วหรือไม่
   - ห้ามเปลี่ยนเป็นสั่ง `goTradeBtn.click()` โดยตรงโดยไม่ตรวจสอบ

3. **src/main.rs — handle_post_trade()** (~line 621-660)
   - ต้องตรวจ `was_already_running` ก่อน +1 `totalTrade`
   - ห้ามเปลี่ยน `update_trade_control("กำลังเทรด", true, None)` โดยไม่ตรวจ was_already_running

### มองหา comment ในโค้ด:
```
╔══════════════════════════════════════════════════════════════════╗
║ ⚠️ CRITICAL BUG FIX — ห้ามลบหรือแก้ไขโค้ดส่วนนี้! ⚠️           ║
║ ป้องกัน "Schedule Trade Round +1 Bug"                           ║
╚══════════════════════════════════════════════════════════════════╝
```

> **สำคัญ:** ถ้าต้องแก้ไข feature เกี่ยวกับ Schedule, Trade Control, Go Trade, หรือ fetchTradeControl 
> ต้องตรวจสอบว่าการแก้ไขไม่ทำให้ 3 จุดด้านบนถูกลบออกหรือเปลี่ยนพฤติกรรม

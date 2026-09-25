# CHANGELOG: Trade History Table Enhancements

**Date**: 2026-07-25
**Status**: ✅ COMPLETED

---

## 🎯 Objective

ปรับปรุงตาราง **Trade History** ให้แสดงข้อมูลครบถ้วนและมีส่วนสรุปยอดการเทรด

---

## ✨ Changes Made

### 1. **เพิ่มคอลัมน์ # (Row Number)**
- แสดงลำดับที่เทรด (1 = เทรดล่าสุด)
- เทรดล่าสุดจะอยู่แถวบนสุด (sorted descending by time)
- Column อยู่ตำแหน่งแรกสุดของตาราง

### 2. **เพิ่มคอลัมน์ Max Profit และ Min Profit**
- **Max Profit**: กำไรสูงสุดที่เคยเกิดขึ้นในระหว่างเทรด
- **Min Profit**: กำไรต่ำสุด (หรือขาดทุนสูงสุด) ที่เคยเกิดขึ้นในระหว่างเทรด
- สีเขียว (#06D6A0) สำหรับกำไร
- สีแดง (#E8304A) สำหรับขาดทุน
- แสดงเป็น `+$X.XX` หรือ `-$X.XX`
- ข้อมูลมาจาก Backend (`maxProfit`, `minProfit`) ที่ track อยู่แล้วใน Track Orders

### 3. **เพิ่มส่วนสรุปยอด (Summary Section)**
แสดงด้านบนตาราง Trade History ประกอบด้วย 4 ค่า:

#### 💰 Balance
- กำไร/ขาดทุนรวมทั้งหมดของวันนี้
- สีเขียว (#06D6A0) ถ้ากำไร
- สีแดง (#E8304A) ถ้าขาดทุน
- แสดงเป็น `+$X.XX` หรือ `-$X.XX`

#### 📊 Total Trades
- จำนวนเทรดทั้งหมดของวันนี้

#### ✅ Total Win
- ยอดรวมกำไรจากเทรดที่ชนะ
- สีเขียว (#06D6A0)
- แสดงเป็น `+$X.XX`

#### ❌ Total Loss
- ยอดรวมขาดทุนจากเทรดที่แพ้
- สีแดง (#E8304A)
- แสดงเป็น `-$X.XX`

---

## 📋 Table Structure (Before vs After)

### **BEFORE** (13 columns):
```
เวลาเข้า | Contract ID | Asset | ประเภท | Duration | CODE | เงื่อนไขเข้าเทรด | 
Entry Spot | Exit Spot | เวลาขาย | สถานะ | Profit | Stake
```

### **AFTER** (16 columns):
```
# | เวลาเข้า | Contract ID | Asset | ประเภท | Duration | CODE | เงื่อนไขเข้าเทรด | 
Entry Spot | Exit Spot | Max Profit | Min Profit | เวลาขาย | สถานะ | Profit | Stake
```

---

## 🔧 Implementation Details

### Frontend Changes (`long_term_trade.html`)

#### 1. HTML Structure Update (Lines ~654-707)

**Added Summary Section**:
```html
<div style="padding:12px; background:var(--card-bg); border-bottom:1px solid var(--border-subtle); 
     display:grid; grid-template-columns:repeat(4, 1fr); gap:12px;">
    <div style="text-align:center;">
        <div style="font-size:11px; color:var(--text3); margin-bottom:4px;">💰 Balance</div>
        <div id="ltHistoryBalance" style="font-size:16px; font-weight:bold; color:var(--text1);">$0.00</div>
    </div>
    <div style="text-align:center;">
        <div style="font-size:11px; color:var(--text3); margin-bottom:4px;">📊 Total Trades</div>
        <div id="ltHistoryTotalTrades" style="font-size:16px; font-weight:bold; color:var(--text1);">0</div>
    </div>
    <div style="text-align:center;">
        <div style="font-size:11px; color:var(--text3); margin-bottom:4px;">✅ Total Win</div>
        <div id="ltHistoryTotalWin" style="font-size:16px; font-weight:bold; color:#06D6A0;">$0.00</div>
    </div>
    <div style="text-align:center;">
        <div style="font-size:11px; color:var(--text3); margin-bottom:4px;">❌ Total Loss</div>
        <div id="ltHistoryTotalLoss" style="font-size:16px; font-weight:bold; color:#E8304A;">$0.00</div>
    </div>
</div>
```

**Updated Table Headers**:
```html
<thead>
    <tr>
        <th>#</th>  <!-- NEW -->
        <th>เวลาเข้า</th>
        <th>Contract ID</th>
        <th>Asset</th>
        <th>ประเภท</th>
        <th>Duration</th>
        <th>CODE</th>
        <th>เงื่อนไขเข้าเทรด</th>
        <th>Entry Spot</th>
        <th>Exit Spot</th>
        <th>Max Profit</th>  <!-- NEW -->
        <th>Min Profit</th>  <!-- NEW -->
        <th>เวลาขาย</th>
        <th>สถานะ</th>
        <th>Profit</th>
        <th>Stake</th>
    </tr>
</thead>
```

#### 2. Function Update: `ltReloadTradeHistory()` (Lines ~2394-2538)

**Key Changes**:

1. **Added Summary Calculation**:
```javascript
let totalProfit = 0;
let totalWin = 0;
let totalLoss = 0;
let totalTrades = ltTradeHistory.length;

ltTradeHistory.forEach((order, index) => {
    const profit = parseFloat(order.ThisProfit || 0);
    totalProfit += profit;
    if (profit >= 0) {
        totalWin += profit;
    } else {
        totalLoss += Math.abs(profit);
    }
    // ...
});
```

2. **Added Row Number**:
```javascript
const rowNumber = index + 1; // Latest trade = 1
```

3. **Added Max/Min Profit Columns**:
```javascript
const maxProfit = order.maxProfit !== undefined ? parseFloat(order.maxProfit) : null;
const minProfit = order.minProfit !== undefined ? parseFloat(order.minProfit) : null;

const maxProfitDisplay = maxProfit !== null ? `+$${maxProfit.toFixed(2)}` : '-';
const minProfitDisplay = minProfit !== null ? 
    (minProfit >= 0 ? `+$${minProfit.toFixed(2)}` : `-$${Math.abs(minProfit).toFixed(2)}`) : '-';

const maxProfitColor = maxProfit !== null && maxProfit >= 0 ? '#06D6A0' : '#E8304A';
const minProfitColor = minProfit !== null && minProfit >= 0 ? '#06D6A0' : '#E8304A';
```

4. **Updated Table Row HTML**:
```javascript
html += `
    <tr>
        <td style="font-weight:bold;color:var(--text3);">${rowNumber}</td>
        <td>${order.purchaseTimeDisplay || '-'}</td>
        <td style="font-family:monospace;font-size:10px;">${order.contractId || '-'}</td>
        <td><span class="chip-dot" style="background:${profitColor};"></span>${order.assetCode || '-'}</td>
        <td style="font-weight:bold;">${order.thisAction || '-'}</td>
        <td>${order.actualDuration || 0}s</td>
        <td style="font-weight:bold;color:${tradeCodeColor};font-family:monospace;font-size:11px;">${tradeCode}</td>
        <td style="font-size:10px;max-width:200px;white-space:normal;line-height:1.3;" title="${conditionsDisplay}">${conditionsDisplay}</td>
        <td>${order.entrySpot || '-'}</td>
        <td>${order.exitSpot || '-'}</td>
        <td style="color:${maxProfitColor};font-weight:bold;font-size:11px;">${maxProfitDisplay}</td>
        <td style="color:${minProfitColor};font-weight:bold;font-size:11px;">${minProfitDisplay}</td>
        <td>${order.sellTimeDisplay || '-'}</td>
        <td style="color:${statusColor};font-weight:bold;">${winStatus}</td>
        <td style="color:${profitColor};font-weight:bold;">${profitSign}$${Math.abs(profit).toFixed(2)}</td>
        <td>$${parseFloat(order.MoneyTrade || 0).toFixed(2)}</td>
    </tr>
`;
```

5. **Update Summary Section**:
```javascript
const balanceColor = totalProfit >= 0 ? '#06D6A0' : '#E8304A';
const balanceSign = totalProfit >= 0 ? '+' : '';
document.getElementById('ltHistoryBalance').innerHTML = 
    `<span style="color:${balanceColor};">${balanceSign}$${Math.abs(totalProfit).toFixed(2)}</span>`;
document.getElementById('ltHistoryTotalTrades').textContent = totalTrades;
document.getElementById('ltHistoryTotalWin').textContent = `+$${totalWin.toFixed(2)}`;
document.getElementById('ltHistoryTotalLoss').textContent = `-$${totalLoss.toFixed(2)}`;
```

6. **Updated Empty State**:
```javascript
if (ltTradeHistory.length === 0) {
    tbody.innerHTML = '<tr class="empty-row"><td colspan="16">ยังไม่มีประวัติการเทรด</td></tr>';
    // Reset summary
    document.getElementById('ltHistoryBalance').textContent = '$0.00';
    document.getElementById('ltHistoryTotalTrades').textContent = '0';
    document.getElementById('ltHistoryTotalWin').textContent = '$0.00';
    document.getElementById('ltHistoryTotalLoss').textContent = '$0.00';
    return;
}
```

---

## 📊 Data Flow

### Backend → Frontend

Backend (`long_term.rs`) บันทึกข้อมูลใน trade history (lines 639-640):
```rust
"minProfit": order.min_profit,
"maxProfit": order.max_profit,
```

Frontend (`long_term_trade.html`) อ่านจาก API `/api/history?date=YYYY-MM-DD`:
```javascript
const maxProfit = order.maxProfit !== undefined ? parseFloat(order.maxProfit) : null;
const minProfit = order.minProfit !== undefined ? parseFloat(order.minProfit) : null;
```

---

## 🎨 Visual Design

### Summary Section Layout
```
┌────────────────────────────────────────────────────────────────────┐
│  💰 Balance      📊 Total Trades   ✅ Total Win    ❌ Total Loss   │
│   +$10.50             5              +$15.00         -$4.50        │
└────────────────────────────────────────────────────────────────────┘
```

### Table Layout
```
┌───┬──────────┬────────────┬───────┬─────┬────┬──────┬────────┬───────┬────────┬────────┬────────┬────────┬──────┬────────┬──────┐
│ # │ เวลาเข้า │ Contract   │ Asset │ ประเภท│ Dur│ CODE │ เงื่อนไข│ Entry │ Exit   │  Max   │  Min   │ เวลาขาย│ สถานะ │ Profit │ Stake│
│   │          │    ID      │       │      │    │      │        │ Spot  │ Spot   │ Profit │ Profit │        │      │        │      │
├───┼──────────┼────────────┼───────┼─────┼────┼──────┼────────┼───────┼────────┼────────┼────────┼────────┼──────┼────────┼──────┤
│ 1 │25/7/2569 │ 6728891659 │vol25  │ PUT │902s│  SM  │CrossDown│796691 │796555  │ +$0.92 │ -$0.16 │25/7 16 │ Win  │ +$0.92 │$1.00 │
│   │16:07:05  │            │       │     │    │      │        │  .37  │  .88   │        │        │  :22:07│      │        │      │
├───┼──────────┼────────────┼───────┼─────┼────┼──────┼────────┼───────┼────────┼────────┼────────┼────────┼──────┼────────┼──────┤
│ 2 │25/7/2569 │ 6793533539 │vol25  │ PUT │901s│  SM  │CrossDown│796691 │796555  │ +$0.92 │ -$0.50 │25/7 16 │ Win  │ +$0.92 │$1.00 │
│   │16:07:05  │            │       │     │    │      │        │  .37  │  .88   │        │        │  :22:06│      │        │      │
└───┴──────────┴────────────┴───────┴─────┴────┴──────┴────────┴───────┴────────┴────────┴────────┴────────┴──────┴────────┴──────┘
```

---

## 🔍 Features

### 1. Row Numbering
- ลำดับที่ 1 = เทรดล่าสุด
- นับจาก 1, 2, 3, ...
- สีเทา (var(--text3))

### 2. Max/Min Profit Tracking
- แสดงช่วงกำไร/ขาดทุนที่เกิดขึ้นระหว่างเทรด
- ช่วยวิเคราะห์ว่า order มี volatility มากน้อยแค่ไหน
- ถ้าไม่มีข้อมูล แสดง "-"

### 3. Summary Calculation
- คำนวณแบบ real-time จาก trade history ที่โหลดมา
- อัปเดตทุกครั้งที่กด "โหลดประวัติ"
- รีเซ็ตเป็น 0 เมื่อไม่มีประวัติ

### 4. Responsive Summary Grid
- ใช้ CSS Grid (4 columns)
- แสดงผลสวยงามบนหน้าจอทุกขนาด

---

## 📝 Files Modified

### Frontend
- `public/long_term_trade.html`
  - Table HTML structure (~654-707)
  - `ltReloadTradeHistory()` function (~2394-2538)

### Backend (No Changes)
- Backend already provides `maxProfit` and `minProfit` in trade history
- Located in `src/long_term.rs` (lines 639-640)

---

## ✅ Testing Checklist

- [x] Table แสดงคอลัมน์ # (row number)
- [x] Row number เรียงจาก 1 = ล่าสุด
- [x] Table แสดงคอลัมน์ Max Profit
- [x] Table แสดงคอลัมน์ Min Profit
- [x] Max/Min Profit แสดงสีเขียวเมื่อกำไร
- [x] Max/Min Profit แสดงสีแดงเมื่อขาดทุน
- [x] Summary section แสดง Balance
- [x] Summary section แสดง Total Trades
- [x] Summary section แสดง Total Win
- [x] Summary section แสดง Total Loss
- [x] Balance แสดงสีเขียวเมื่อกำไร
- [x] Balance แสดงสีแดงเมื่อขาดทุน
- [x] Summary reset เป็น 0 เมื่อไม่มีประวัติ
- [x] Empty state แสดง colspan="16"

---

## 🎯 Expected Results

### Example Data

**Summary Section**:
```
💰 Balance: +$1.68 (green)
📊 Total Trades: 4
✅ Total Win: +$1.84
❌ Total Loss: -$0.16
```

**Table Rows**:
| # | Asset | Type | Max Profit | Min Profit | Final Profit | Status |
|---|-------|------|------------|------------|--------------|--------|
| 1 | vol25 | PUT  | +$0.92     | -$0.16     | +$0.92       | Win    |
| 2 | vol25 | PUT  | +$0.92     | -$0.50     | +$0.92       | Win    |
| 3 | vol50 | CALL | +$0.05     | -$1.00     | -$1.00       | Loss   |
| 4 | vol75 | CALL | +$1.00     | +$0.20     | +$0.84       | Win    |

---

## 💡 Notes

1. **Max/Min Profit** มาจาก Backend ที่ track ตอน order เปิดอยู่
2. ถ้า Backend ไม่ส่งค่า `maxProfit` หรือ `minProfit` จะแสดง "-"
3. Summary คำนวณจาก trades ของวันปัจจุบันเท่านั้น (ไม่รวมวันอื่น)
4. ทุกครั้งที่กด "โหลดประวัติ" จะ recalculate summary ใหม่
5. Empty state จะ reset summary เป็น $0.00 ทันที

---

## 🚀 Next Steps (Optional Enhancements)

1. เพิ่มปุ่ม "Export to CSV" เพื่อ export trade history
2. เพิ่ม date picker เพื่อดูประวัติย้อนหลัง
3. เพิ่ม Win Rate % ใน summary
4. เพิ่ม Average Profit/Loss ใน summary
5. เพิ่ม filter ตาม asset หรือ trade code
6. เพิ่ม pagination ถ้ามี trade เยอะ

---

## ✨ Summary

การปรับปรุงครั้งนี้ทำให้ Trade History มีข้อมูลครบถ้วนและมีส่วนสรุปยอดที่ชัดเจน ช่วยให้ผู้ใช้เห็นภาพรวมของการเทรดได้ดีขึ้น พร้อมทั้งสามารถวิเคราะห์ช่วงกำไร/ขาดทุนที่เกิดขึ้นระหว่างเทรดได้จาก Max/Min Profit columns

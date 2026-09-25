# CHANGELOG: Trade Code Implementation

**Date:** 2026-07-24  
**File:** `public/long_term_trade.html`  
**Status:** ✅ Completed

---

## 📋 Overview

เพิ่มระบบ **Trade Code** เพื่อระบุประเภทของเงื่อนไขการเข้าเทรดแต่ละรายการ ทำให้สามารถติดตามและวิเคราะห์ว่าแต่ละ Order เข้าเทรดมาจากเงื่อนไขใด

---

## 🎯 Trade Code Definitions

| Code | เงื่อนไข | คำอธิบาย |
|------|---------|----------|
| **SM** | EMA Short × EMA Medium | เข้าเทรดเมื่อ EMA Short ตัดกับ EMA Medium |
| **SML** | EMA Long × Short & Medium | เข้าเทรดเมื่อ EMA Long ตัดกับทั้ง Short และ Medium |
| **SL** | EMA Short × EMA Long | เข้าเทรดเมื่อ EMA Short ตัดกับ EMA Long |
| **SM+SL** | Multiple Conditions | เข้าเทรดเมื่อเปิดใช้หลายเงื่อนไขพร้อมกัน (เช่น SM และ SL) |
| **SM+SML** | Multiple Conditions | เข้าเทรดเมื่อเปิดใช้หลายเงื่อนไขพร้อมกัน (เช่น SM และ SML) |
| **SL+SML** | Multiple Conditions | เข้าเทรดเมื่อเปิดใช้หลายเงื่อนไขพร้อมกัน (เช่น SL และ SML) |
| **SM+SL+SML** | All Conditions | เข้าเทรดเมื่อเปิดใช้ทั้ง 3 เงื่อนไขพร้อมกัน |
| **MANUAL** | Manual Trade | เข้าเทรดแบบ Manual (ไม่ได้เปิดเงื่อนไขใดๆ) |

---

## 🔧 Changes Made

### 1. **Entry Conditions Section** (Lines ~250-270)
กำหนด Trade Code สำหรับแต่ละเงื่อนไข:
- `ltCondShortMedium` → **SM**
- `ltCondLongCross` → **SML**
- `ltCondShortLong` → **SL**

### 2. **ltExecuteBuyOrder Function** (Lines ~2600-2700)

#### 2.1 สร้าง Trade Code Array
```javascript
const tradeCodesArray = [];

if (document.getElementById('ltCondShortMedium')?.checked) {
    entryConditionsArray.push('⚡ EMA Short × EMA Medium');
    tradeCodesArray.push('SM');
}
if (document.getElementById('ltCondLongCross')?.checked) {
    entryConditionsArray.push('🔀 EMA Long × Short & Medium');
    tradeCodesArray.push('SML');
}
if (document.getElementById('ltCondShortLong')?.checked) {
    entryConditionsArray.push('🔄 EMA Short × EMA Long');
    tradeCodesArray.push('SL');
}
```

#### 2.2 สร้าง Trade Code String
```javascript
const tradeCode = tradeCodesArray.length > 0 ? tradeCodesArray.join('+') : 'MANUAL';
```

#### 2.3 เพิ่ม Log แสดง Trade Code
```javascript
ltAddLog(`🏷️ Trade Code: ${tradeCode}`, 'system');
```

#### 2.4 ส่ง Trade Code ไปยัง Backend
```javascript
fetch('/api/longterm/manual_trade', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
        asset,
        contract_type: contractType,
        amount,
        target_profit: targetPercent,
        check_duplicate_asset: checkDuplicate,
        exit_strategy: exitStrategy,
        duration,
        entry_signal: entryConditionsText,
        entry_conditions: entryConditionsText,
        trade_code: tradeCode  // ✅ เพิ่มบรรทัดนี้
    })
});
```

---

### 3. **Track Orders Table - Chart View** (Lines ~570-603)

#### Before:
```html
<th>Entry Signal</th>
<th>TARGET PROFIT (%)</th>
<th>การจัดการ</th>
```
```html
<td colspan="17">ไม่มีออเดอร์ที่เปิดอยู่</td>
```

#### After:
```html
<th>CODE</th>
<th>Entry Signal</th>
<th>TARGET PROFIT (%)</th>
<th>การจัดการ</th>
```
```html
<td colspan="18">ไม่มีออเดอร์ที่เปิดอยู่</td>
```

---

### 4. **Track Orders Table - Main View** (Lines ~605-650)

#### Before:
```html
<th>Entry Signal</th>
<th>TARGET PROFIT (%)</th>
<th>การจัดการ</th>
```
```html
<td colspan="17">ไม่มีออเดอร์ที่เปิดอยู่</td>
```

#### After:
```html
<th>CODE</th>
<th>Entry Signal</th>
<th>TARGET PROFIT (%)</th>
<th>การจัดการ</th>
```
```html
<td colspan="18">ไม่มีออเดอร์ที่เปิดอยู่</td>
```

---

### 5. **Trade History Table** (Lines ~649-690)

#### Before:
```html
<th>Duration</th>
<th>เงื่อนไขเข้าเทรด</th>
<th>Entry Spot</th>
```
```html
<td colspan="12">ยังไม่มีประวัติการเทรด</td>
```

#### After:
```html
<th>Duration</th>
<th>CODE</th>
<th>เงื่อนไขเข้าเทรด</th>
<th>Entry Spot</th>
```
```html
<td colspan="13">ยังไม่มีประวัติการเทรด</td>
```

---

### 6. **ltUpdateOpenOrdersTable Function** (Lines ~2124-2350)

#### 6.1 Update Empty Row Colspan
```javascript
// Before
const emptyHtml = '<tr class="empty-row"><td colspan="17">ไม่มีออเดอร์ที่เปิดอยู่</td></tr>';

// After
const emptyHtml = '<tr class="empty-row"><td colspan="18">ไม่มีออเดอร์ที่เปิดอยู่</td></tr>';
```

#### 6.2 เพิ่ม Trade Code ในการสร้างแถวใหม่
```javascript
// Get trade code from order
const tradeCode = order.trade_code || order.tradeCode || 'MANUAL';
const tradeCodeColor = tradeCode === 'MANUAL' ? 'var(--text3)' : 'var(--long-term-accent)';

tr.innerHTML = `
    ...
    <td id="td-rem-${prefix}-${order.contract_id}" ...>${remainingTime}</td>
    <td id="td-code-${prefix}-${order.contract_id}" style="font-weight:bold;color:${tradeCodeColor};font-family:monospace;font-size:11px;">${tradeCode}</td>
    <td>Manual</td>
    ...
`;
```

---

### 7. **ltReloadTradeHistory Function** (Lines ~2355-2430)

#### 7.1 Update Empty Row Colspan
```javascript
// Before
tbody.innerHTML = '<tr class="empty-row"><td colspan="12">ยังไม่มีประวัติการเทรด</td></tr>';

// After
tbody.innerHTML = '<tr class="empty-row"><td colspan="13">ยังไม่มีประวัติการเทรด</td></tr>';
```

#### 7.2 เพิ่ม Trade Code ในตาราง
```javascript
// Get trade code
const tradeCode = order.tradeCode || order.trade_code || '-';
const tradeCodeColor = tradeCode === 'MANUAL' ? 'var(--text3)' : 'var(--long-term-accent)';

html += `
    <tr>
        ...
        <td>${order.actualDuration || 0}s</td>
        <td style="font-weight:bold;color:${tradeCodeColor};font-family:monospace;font-size:11px;">${tradeCode}</td>
        <td style="font-size:10px;max-width:200px;..." title="${conditionsDisplay}">${conditionsDisplay}</td>
        ...
    </tr>
`;
```

#### 7.3 Update Error Message Colspan
```javascript
// Before
tbody.innerHTML = '<tr class="empty-row"><td colspan="12">โหลดประวัติล้มเหลว</td></tr>';

// After
tbody.innerHTML = '<tr class="empty-row"><td colspan="13">โหลดประวัติล้มเหลว</td></tr>';
```

---

## 🎨 Styling

### Trade Code Colors:
- **MANUAL**: `var(--text3)` (สีเทา - แสดงว่าเป็น Manual Trade)
- **SM / SML / SL / Combined**: `var(--long-term-accent)` (สีม่วง - แสดงว่าเข้าตามเงื่อนไข)

### Font Style:
- `font-family: monospace`
- `font-size: 11px`
- `font-weight: bold`

---

## 📊 Table Structure Changes

### Track Orders Tables (Chart View & Main View):
| Column Order | Field | Type |
|--------------|-------|------|
| 1 | ลำดับ | Static |
| 2 | Contract ID | Static |
| 3 | Symbol | Dynamic |
| 4 | ประเภท | Static |
| 5 | ราคาซื้อ | Static |
| 6 | Entry Spot | Dynamic |
| 7 | Current Spot | Dynamic |
| 8 | กำไร/ขาดทุน | Dynamic |
| 9 | Min Profit | Dynamic |
| 10 | Max Profit | Dynamic |
| 11 | เวลาซื้อ | Static |
| 12 | Duration | Dynamic |
| 13 | เวลาหมดอายุ | Dynamic |
| 14 | เวลาที่เหลือ | Dynamic |
| 15 | **CODE** | **🆕 New** |
| 16 | Entry Signal | Static |
| 17 | TARGET PROFIT (%) | Editable |
| 18 | การจัดการ | Actions |

**Total Columns:** 18 (เพิ่มจาก 17)

---

### Trade History Table:
| Column Order | Field | Type |
|--------------|-------|------|
| 1 | เวลาเข้า | Static |
| 2 | Contract ID | Static |
| 3 | Asset | Static |
| 4 | ประเภท | Static |
| 5 | Duration | Static |
| 6 | **CODE** | **🆕 New** |
| 7 | เงื่อนไขเข้าเทรด | Static |
| 8 | Entry Spot | Static |
| 9 | Exit Spot | Static |
| 10 | เวลาขาย | Static |
| 11 | สถานะ | Static |
| 12 | Profit | Static |
| 13 | Stake | Static |

**Total Columns:** 13 (เพิ่มจาก 12)

---

## 🔍 Backend Requirements

Backend ต้องรองรับการรับและเก็บข้อมูล `trade_code` field:

### Request Body (Manual Trade API):
```json
{
  "asset": "vol10",
  "contract_type": "CALL",
  "amount": 1.0,
  "target_profit": 10.0,
  "check_duplicate_asset": true,
  "exit_strategy": "targetProfit",
  "duration": 3600,
  "entry_signal": "⚡ EMA Short × EMA Medium | 📊 ADX Filter (min: 25)",
  "entry_conditions": "⚡ EMA Short × EMA Medium | 📊 ADX Filter (min: 25)",
  "trade_code": "SM"
}
```

### Order Object (WebSocket Updates):
```json
{
  "contract_id": 123456789,
  "symbol": "R_10",
  "contract_type": "CALL",
  "buy_price": 1.0,
  "current_spot": 1234.56,
  "profit": 0.85,
  "trade_code": "SM",
  ...
}
```

### Trade History Response:
```json
{
  "contractId": "123456789",
  "assetCode": "Vol 10",
  "thisAction": "CALL",
  "tradeCode": "SM",
  "entryConditions": "⚡ EMA Short × EMA Medium",
  ...
}
```

---

## ✅ Testing Checklist

- [x] Trade Code สร้างถูกต้องใน `ltExecuteBuyOrder`
- [x] Trade Code ส่งไปยัง Backend ผ่าน API
- [x] Trade Code แสดงในตาราง Track Orders (Chart View)
- [x] Trade Code แสดงในตาราง Track Orders (Main View)
- [x] Trade Code แสดงในตาราง Trade History
- [x] สี Trade Code แสดงถูกต้อง (MANUAL = เทา, อื่นๆ = ม่วง)
- [x] Colspan อัพเดตถูกต้องทุกตาราง (17→18 และ 12→13)
- [x] Log แสดง Trade Code ใน Bot Log
- [x] Multiple conditions รวมกันด้วย `+` (เช่น SM+SL)

---

## 🚀 Usage Examples

### Example 1: Single Condition (SM)
- ✅ เปิดเงื่อนไข: **EMA Short × EMA Medium** เท่านั้น
- 📋 Trade Code: **SM**
- 📊 Entry Conditions: `⚡ EMA Short × EMA Medium`

### Example 2: Multiple Conditions (SM+SML)
- ✅ เปิดเงื่อนไข: **EMA Short × EMA Medium** + **EMA Long × Short & Medium**
- 📋 Trade Code: **SM+SML**
- 📊 Entry Conditions: `⚡ EMA Short × EMA Medium | 🔀 EMA Long × Short & Medium`

### Example 3: All Conditions (SM+SL+SML)
- ✅ เปิดเงื่อนไขทั้ง 3 แบบ
- 📋 Trade Code: **SM+SL+SML**
- 📊 Entry Conditions: `⚡ EMA Short × EMA Medium | 🔀 EMA Long × Short & Medium | 🔄 EMA Short × EMA Long`

### Example 4: Manual Trade
- ❌ ไม่เปิดเงื่อนไขใดๆ (กดปุ่ม Manual Trade)
- 📋 Trade Code: **MANUAL**
- 📊 Entry Conditions: `Manual Trade`

---

## 📝 Notes

1. **Trade Code Position**: วางไว้ระหว่าง "เวลาที่เหลือ" และ "Entry Signal" ในตาราง Track Orders
2. **Trade Code Position (History)**: วางไว้ระหว่าง "Duration" และ "เงื่อนไขเข้าเทรด" ในตาราง Trade History
3. **Backend Compatibility**: Backend ต้องรองรับ field `trade_code` (snake_case) หรือ `tradeCode` (camelCase)
4. **Color Scheme**: ใช้สีตาม theme variable (`--text3` และ `--long-term-accent`)
5. **Font**: ใช้ monospace font เพื่อความชัดเจนของ code

---

## 🔗 Related Files

- `long_term_trade.html` — Main implementation
- Backend API endpoint: `/api/longterm/manual_trade`
- WebSocket message type: `lt_orders_update`
- History API endpoint: `/api/history?date=YYYY-MM-DD`

---

**Implementation Complete! ✅**

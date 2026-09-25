# API Documentation: Export Trade Data

**Endpoint**: `GET /api/export_trade_data`  
**Purpose**: Export trade data for a specific date as a ZIP file

---

## 📋 Query Parameters

| Parameter | Type | Required | Format | Description |
|-----------|------|----------|--------|-------------|
| `startdatetime` | String | ✅ Yes | `YYYY-MM-DD` | วันที่ต้องการ export เช่น `2026-05-01` |

---

## 📝 How It Works

1. รับ parameter `startdatetime` (format: YYYY-MM-DD)
2. แปลงเป็น Thai year และสร้าง path: `tradeData/MM-YYYY/DD-MM-YYYY`
3. ดึงไฟล์ทั้งหมดใน folder นั้น (รวม subfolder)
4. สร้าง ZIP file ใน memory
5. ส่ง ZIP file กลับเป็น download

---

## 🎯 ตัวอย่างการใช้งาน

### Example 1: Export data for May 1, 2026
```
GET http://localhost:3000/api/export_trade_data?startdatetime=2026-05-01
```

**Result**: ดาวน์โหลดไฟล์ `trade_data_2026-05-01.zip` ที่มีเนื้อหาจาก:
```
tradeData/05-2569/01-05-2569/
  ├── 1HZ10V/
  │   ├── trades.json
  │   └── track_orders.json
  ├── 1HZ25V/
  │   ├── trades.json
  │   └── track_orders.json
  └── ...
```

### Example 2: Export data for May 13, 2026
```
GET http://localhost:3000/api/export_trade_data?startdatetime=2026-05-13
```

**Result**: ดาวน์โหลดไฟล์ `trade_data_2026-05-13.zip`

---

## 📂 Folder Structure

API จะดึงข้อมูลจาก path:
```
tradeData/{MM-YYYY}/{DD-MM-YYYY}/
```

โดย:
- `MM-YYYY` = เดือน (2 หลัก) - ปีไทย (4 หลัก)
- `DD-MM-YYYY` = วัน (2 หลัก) - เดือน (2 หลัก) - ปีไทย (4 หลัก)

**ตัวอย่าง**:
- วันที่ 2026-05-01 → `tradeData/05-2569/01-05-2569/`
- วันที่ 2026-05-13 → `tradeData/05-2569/13-05-2569/`
- วันที่ 2026-07-25 → `tradeData/07-2569/25-07-2569/`

---

## 📦 ZIP File Structure

ZIP file จะมีโครงสร้างแบบ relative path:

```
trade_data_2026-05-01.zip
│
├── tradeHead.json          # ข้อมูลภาพรวมการเทรด / รอบการเทรด
│
├── 1HZ10V/
│   ├── trades.json
│   └── track_orders.json
│
├── 1HZ25V/
│   ├── trades.json
│   └── track_orders.json
│
├── 1HZ50V/
│   ├── trades.json
│   └── track_orders.json
│
└── 1HZ75V/
    ├── trades.json
    └── track_orders.json
```

---

## 📤 Response

### Success Response
- **HTTP Status**: `200 OK`
- **Content-Type**: `application/zip`
- **Content-Disposition**: `attachment; filename="trade_data_YYYY-MM-DD.zip"`
- **Body**: Binary ZIP file

### Error Responses

#### 1. Invalid Date Format
- **HTTP Status**: `400 Bad Request`
- **Body**: 
  ```json
  {
    "error": "Invalid date format: ... Expected YYYY-MM-DD"
  }
  ```

#### 2. No Data Found
- **HTTP Status**: `404 Not Found`
- **Body**: 
  ```json
  {
    "error": "No trade data found for date: 2026-05-01"
  }
  ```

#### 3. No Files in Folder
- **HTTP Status**: `404 Not Found`
- **Body**: 
  ```json
  {
    "error": "No files found in: tradeData/05-2569/01-05-2569"
  }
  ```

#### 4. ZIP Creation Error
- **HTTP Status**: `500 Internal Server Error`
- **Body**: 
  ```json
  {
    "error": "Failed to finalize ZIP: ..."
  }
  ```

---

## 🔍 Backend Console Logs

### Success Case
```
📦 [Export Trade Data] Request for date: 2026-05-01
📂 [Export Trade Data] Target path: tradeData/05-2569/01-05-2569
✅ [Export] Added to ZIP: 1HZ10V/trades.json
✅ [Export] Added to ZIP: 1HZ10V/track_orders.json
✅ [Export] Added to ZIP: 1HZ25V/trades.json
✅ [Export] Added to ZIP: 1HZ25V/track_orders.json
✅ [Export] Added to ZIP: 1HZ50V/trades.json
✅ [Export] Added to ZIP: 1HZ50V/track_orders.json
📦 [Export Trade Data] ZIP created with 6 files
```

### Error Case
```
📦 [Export Trade Data] Request for date: 2026-05-01
📂 [Export Trade Data] Target path: tradeData/05-2569/01-05-2569
❌ [Export] Folder not found
```

---

## 💻 Frontend Usage (JavaScript/AJAX)

### Using Fetch API
```javascript
async function exportTradeData(date) {
    try {
        const response = await fetch(`/api/export_trade_data?startdatetime=${date}`);
        
        if (!response.ok) {
            const error = await response.text();
            console.error('Export failed:', error);
            alert('Export failed: ' + error);
            return;
        }
        
        // Get the blob
        const blob = await response.blob();
        
        // Create download link
        const url = window.URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `trade_data_${date}.zip`;
        document.body.appendChild(a);
        a.click();
        
        // Cleanup
        window.URL.revokeObjectURL(url);
        document.body.removeChild(a);
        
        console.log('✅ Export successful');
    } catch (error) {
        console.error('❌ Export error:', error);
        alert('Export error: ' + error.message);
    }
}

// Usage
exportTradeData('2026-05-01');
```

### Using XMLHttpRequest
```javascript
function exportTradeData(date) {
    const xhr = new XMLHttpRequest();
    xhr.open('GET', `/api/export_trade_data?startdatetime=${date}`, true);
    xhr.responseType = 'blob';
    
    xhr.onload = function() {
        if (xhr.status === 200) {
            const blob = xhr.response;
            const url = window.URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `trade_data_${date}.zip`;
            document.body.appendChild(a);
            a.click();
            window.URL.revokeObjectURL(url);
            document.body.removeChild(a);
            console.log('✅ Export successful');
        } else {
            console.error('❌ Export failed:', xhr.statusText);
            alert('Export failed');
        }
    };
    
    xhr.onerror = function() {
        console.error('❌ Network error');
        alert('Network error');
    };
    
    xhr.send();
}

// Usage
exportTradeData('2026-05-01');
```

### Using jQuery
```javascript
function exportTradeData(date) {
    $.ajax({
        url: '/api/export_trade_data',
        type: 'GET',
        data: { startdatetime: date },
        xhrFields: { responseType: 'blob' },
        success: function(blob) {
            const url = window.URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `trade_data_${date}.zip`;
            document.body.appendChild(a);
            a.click();
            window.URL.revokeObjectURL(url);
            document.body.removeChild(a);
            console.log('✅ Export successful');
        },
        error: function(xhr) {
            console.error('❌ Export failed:', xhr.responseText);
            alert('Export failed');
        }
    });
}

// Usage
exportTradeData('2026-05-01');
```

---

## 🎨 UI Integration Example

### Add Export Button in HTML
```html
<button id="exportTradeDataBtn" class="fetch-btn lt-primary">
    📦 Export Trade Data
</button>

<input type="date" id="exportDatePicker" value="2026-05-01">
```

### Add Event Listener
```javascript
document.getElementById('exportTradeDataBtn').addEventListener('click', async () => {
    const date = document.getElementById('exportDatePicker').value;
    
    if (!date) {
        alert('กรุณาเลือกวันที่');
        return;
    }
    
    console.log(`📦 Exporting trade data for ${date}...`);
    await exportTradeData(date);
});
```

---

## ⚙️ Configuration

### Compression Method
API ใช้ `Deflated` compression (gzip-like) ซึ่งให้ขนาดไฟล์เล็กและเร็ว

### File Permissions
ไฟล์ใน ZIP จะมี Unix permissions: `0o755` (rwxr-xr-x)

---

## 🔧 Dependencies

### Rust Crates
- `zip = "0.6"` - For creating ZIP archives
- `walkdir = "2.5"` - For recursive directory traversal
- `chrono = "0.4.38"` - For date parsing

---

## 📊 Performance

- **File I/O**: อ่านไฟล์ทีละไฟล์แล้วเพิ่มลง ZIP
- **Memory**: สร้าง ZIP ใน memory (ใช้ `Cursor<Vec<u8>>`)
- **Network**: ส่งไฟล์กลับทันทีหลัง ZIP เสร็จ
- **Compression**: ใช้ Deflated algorithm (ประมาณ 50-70% ของขนาดเดิม)

---

## 🐛 Troubleshooting

### ปัญหา: ZIP file corrupt
- ตรวจสอบว่าไม่มี error ใน Backend console
- ตรวจสอบว่าไฟล์ต้นทางไม่ corrupt

### ปัญหา: Download ไม่ได้
- ตรวจสอบ browser console สำหรับ JavaScript errors
- ตรวจสอบว่า CORS settings ถูกต้อง

### ปัญหา: ไม่มีไฟล์ใน ZIP
- ตรวจสอบว่า folder path ถูกต้อง
- ตรวจสอบว่ามีไฟล์อยู่จริงใน folder

---

## 🎯 Use Cases

1. **Backup**: สำรองข้อมูลการเทรดของแต่ละวัน
2. **Analysis**: ดาวน์โหลดเพื่อวิเคราะห์ด้วย tools อื่น
3. **Archive**: เก็บข้อมูลเก่าไว้ offline
4. **Share**: แชร์ข้อมูลกับทีมหรือคนอื่น
5. **Migration**: ย้ายข้อมูลไปยังระบบใหม่

---

## ✅ Summary

API นี้ใช้สำหรับ **export trade data ของวันที่ระบุเป็น ZIP file** พร้อมดาวน์โหลดทันที

- ✅ รองรับทุก asset ใน folder
- ✅ รองรับทั้ง `trades.json` และ `track_orders.json`
- ✅ สร้าง ZIP ใน memory (ไม่สร้างไฟล์ชั่วคราว)
- ✅ ส่งกลับเป็น download ทันที
- ✅ มี error handling ครบถ้วน

**Ready to use!** 🚀

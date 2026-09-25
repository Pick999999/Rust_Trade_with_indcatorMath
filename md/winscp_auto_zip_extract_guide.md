# คู่มือการตั้งค่า WinSCP: บีบอัดไฟล์ (Zip) และแตกไฟล์ (Extract) อัตโนมัติไปยัง VPS

เอกสารนี้รวบรวม 3 วิธีการแก้ปัญหาการส่งไฟล์ขนาดใหญ่/หลายไฟล์ผ่าน WinSCP ไปยัง VPS ให้รวดเร็วและสะดวกที่สุด

---

## 📌 ข้อจำกัดเดิมของ WinSCP GUI
WinSCP GUI **ไม่มีฟังก์ชันสำเร็จรูป (Native on-the-fly)** ที่เพียงแค่ลากไฟล์วางแล้วโปรแกรมจะ Zip ฝั่งเรา ส่งไป แล้วแตกไฟล์ฝั่ง Linux ให้อัตโนมัติในคลิกเดียว

อย่างไรก็ตาม คุณสามารถใช้ **3 เทคนิค** ด้านล่างนี้เพื่อแก้ปัญหาได้อย่างสมบูรณ์:

---

## 🚀 วิธีที่ 1: เปิดใช้ SSH Compression ใน WinSCP (✨ แนะนำที่สุด — ง่ายที่สุด)

WinSCP รองรับการบีบอัดข้อมูลแบบ **Zlib compression (Gzip stream)** ขณะส่งผ่านท่อ SSH
- **หลักการทำงาน:** ข้อมูลจะถูกบีบอัดแบบ On-the-fly ขณะส่งผ่าน Network และเมื่อถึง VPS ปลายทาง ระบบ SSH จะคลายข้อมูลออกเป็นไฟล์เดิมให้ทันที 100%
- **ข้อดี:** ส่งเร็วเท่ากับการส่งไฟล์ Zip แต่**ไม่ต้องสร้างไฟล์ `.zip`** และ**ไม่ต้องสั่ง extract** ใดๆ ทั้งสิ้น

### ขั้นตอนการตั้งค่า:
1. เปิด WinSCP ในหน้า Login (Session List)
2. เลือก VPS ที่ต้องการ จากนั้นกดปุ่ม **Edit**
3. กดปุ่ม **Advanced...** (ขั้นสูง)
4. ที่เมนูด้านซ้าย เลือกหัวข้อ **SSH**
5. ติ๊กเครื่องหมายถูกที่ช่อง **☑ Enable compression** (เปิดใช้การบีบอัดข้อมูล)
6. กด **OK** แล้วกด **Save** บันทึก Session

---

## ⚡ วิธีที่ 2: ตั้งค่า Custom Command ใน WinSCP (คลิกขวาแตกไฟล์บน VPS ทันที)

หากต้องการ Zip ไฟล์จากเครื่องเราก่อนส่ง (เช่น รวมหลายโฟลเดอร์เป็น `.zip` หรือ `.tar.gz` ก้อนเดียว) แล้วอยากสั่งแตกไฟล์บน VPS ได้ทันทีโดยไม่ต้องเปิด PuTTY / Terminal แยก:

### ขั้นตอนการตั้งค่า WinSCP:
1. ไปที่เมนูด้านบนของ WinSCP: **Options** -> **Preferences**
2. เลือกเมนูด้านซ้าย **Commands** -> กดปุ่ม **Add...**
3. กรอกรายละเอียด:
   - **Name:** `Unzip Here`
   - **Command:** `unzip -o !.!` (หรือถ้าใช้ tar ให้ใส่: `tar -xzf !.!`)
   - เลือกประเภทเป็น **Remote command**
   - ติ๊กถูกที่ช่อง **☑ Show on context menu**
4. กด **OK** เพื่อบันทึกคำสั่ง

### วิธีใช้งาน:
1. ลากไฟล์ `.zip` หรือ `.tar.gz` จากเครื่องเราไปวางบนฝั่ง VPS ใน WinSCP
2. เมื่ออัปโหลดเสร็จ **คลิกขวาที่ไฟล์นั้นบนฝั่ง VPS**
3. เลือก **Custom Commands** -> **Unzip Here**
4. ไฟล์จะถูกแตกออกบนโฟลเดอร์นั้นบน VPS ทันที

---

## 🤖 วิธีที่ 3: สคริปต์อัตโนมัติ Batch + WinSCP.com (Zip -> Upload -> Auto Extract ใน 1 คลิก)

เหมาะอย่างยิ่งสำหรับขั้นตอนการ **Deploy ระบบ (เช่น Binary Rust หรือโค้ดโปรเจกต์)** ที่ต้องการให้คำสั่งรันทุกอย่างจบในไฟล์เดียว:

### ตัวอย่างไฟล์ `deploy_oracle.bat`:
```bat
@echo off
chcp 65001 >nul
setlocal

:: ─── การตั้งค่า VPS ───
set VPS_HOST=161.118.203.228
set VPS_USER=ubuntu
set REMOTE_DIR=/home/ubuntu/turbo-indicators
set WINSCP_PATH="C:\Program Files (x86)\WinSCP\WinSCP.com"

echo [1/4] บีบอัดไฟล์ (Zip / Tar)...
tar -czf update.tar.gz -C target/release turbo-indicators

echo [2/4] อัปโหลดไฟล์ไปยัง VPS ผ่าน WinSCP...
%WINSCP_PATH% /command ^
  "open sftp://%VPS_USER%@%VPS_HOST%/ -hostkey=*" ^
  "put update.tar.gz %REMOTE_DIR%/" ^
  "call cd %REMOTE_DIR% && tar -xzf update.tar.gz && rm update.tar.gz" ^
  "call sudo systemctl restart turbo-indicators" ^
  "exit"

if %ERRORLEVEL% equ 0 (
    echo [3/4] แตกไฟล์และ Restart Service เรียบร้อยแล้ว!
    del update.tar.gz
    echo [4/4] เสร็จสมบูรณ์ (Deploy Successful)
) else (
    echo [X] เกิดข้อผิดพลาดในการส่งข้อมูล
)

pause
```

> **คำอธิบายคำสั่ง `call` ของ WinSCP:**
> คำสั่ง `call` ใน WinSCP Script จะส่งคำสั่งไปรันบน Shell ของ Linux โดยตรง ทำให้สามารถแตกไฟล์ (`tar -xzf`), ลบไฟล์ zip ทิ้ง (`rm`), และสั่งรีสตาร์ท service (`systemctl restart`) ได้โดยอัตโนมัติ

---

## 📊 ตารางเปรียบเทียบการใช้งาน

| วิธีการ | ความสะดวก | ความเร็ว | เหมาะสำหรับ |
| :--- | :---: | :---: | :--- |
| **วิธีที่ 1: SSH Compression** | ⭐⭐⭐⭐⭐ (ตั้งค่าครั้งเดียว) | เร็วมาก | ส่งไฟล์งานทั่วไป, อัปเดตไฟล์รายวัน |
| **วิธีที่ 2: Custom Commands** | ⭐⭐⭐⭐ (คลิกขวาบน GUI) | เร็วมาก | มีไฟล์ zip อยู่แล้ว และต้องการแตกบน VPS โดยไม่เปิด Terminal |
| **วิธีที่ 3: Batch Script (CLI)** | ⭐⭐⭐⭐⭐ (ดับเบิลคลิกเดียวจบ) | เร็วที่สุด | งาน Deploy ระบบ, อัปเดต Binary และ Restart service อัตโนมัติ |

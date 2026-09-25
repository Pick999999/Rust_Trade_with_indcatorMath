# การตั้งค่าให้ Server ทำงานตลอดเวลาบน AWS EC2

เพื่อให้โปรแกรม (Axum Trading Server) รันค้างไว้บน AWS EC2 ได้ แม้ว่าเราจะปิด Terminal (SSH) หรือ Browser ไปแล้ว วิธีที่เป็นมาตรฐานและดีที่สุดคือการสร้าง **Systemd Service** (ใช้ได้บนระบบปฏิบัติการ Linux เช่น Ubuntu, Amazon Linux) 

ไฟล์เทมเพลตสำหรับ service ชื่อ `axum_trading_server.service` ได้ถูกสร้างเตรียมไว้ให้แล้วในโปรเจกต์

---

## วิธีที่ 1: การตั้งค่า Systemd Service (แนะนำสำหรับ Production)

**1. แก้ไขไฟล์ `axum_trading_server.service`ให้ถูกต้อง**
คุณต้องตรวจสอบและปรับแก้ Path (`WorkingDirectory` และ `ExecStart`) และ User ให้ตรงกับเครื่อง EC2 ของคุณเสียก่อน

**2. นำไฟล์ไปวางในระบบ**
บนเครื่อง EC2 ให้ใช้สิทธิ root คัดลอกไฟล์นี้ไปไว้ที่ `/etc/systemd/system/` 
```bash
sudo cp axum_trading_server.service /etc/systemd/system/
```

**3. สมัคร/โหลด Service ใหม่**
ทุกครั้งที่มีการสร้างหรือแก้ไขไฟล์ `.service` ต้องสั่งให้ระบบโหลดข้อมูลขึ้นมาใหม่
```bash
sudo systemctl daemon-reload
```

**4. สั่งให้โปรแกรมทำงาน (Start)**
```bash
sudo systemctl start axum_trading_server
```

**5. ตั้งค่าให้เปิดตัวเองอัตโนมัติเมื่อรีเซ็ตเครื่อง (Enable)**
ป้องกันกรณี EC2 มีการรีสตาร์ทตัวเอง หรือถูก Force stop แล้วเปิดใหม่
```bash
sudo systemctl enable axum_trading_server
```

**6. วิธีเช็คสถานะการทำงาน**
ดูว่า service ขึ้นสีเขียว (active/running) หรือไม่
```bash
sudo systemctl status axum_trading_server
```

**7. วิธีดู Log หรือข้อความที่โปรแกรม print ออกมา**
```bash
# ดู log แบบ realtime (เหมือนตอนรันบน terminal) ย้อนหลัง 100 บรรทัด
sudo journalctl -u axum_trading_server -f -n 100
```
*(ถ้าต้องการหยุดดู ให้กด `Ctrl+C`)*

---

## วิธีที่ 2: ใช้ `tmux` (วิธีด่วน ไม่ต้องตั้งค่าระบบ)

ถ้าคุณกำลังทดสอบ และเขียนโค้ดเพิ่มเรื่อยๆ และยังไม่อยากเซ็ตอัพ Systemd คุณใช้ `tmux` เป็นหน้าต่างจำลองแทนได้:

1. พิมพ์คำสั่งรัน `tmux` ใน EC2 (ถ้าไม่มีให้ติดตั้งก่อน: `sudo apt install tmux`)
2. ระบบจะเปิดหน้าต่าง Terminal พิเศษขึ้นมา ให้คุณสั่ง `cargo run...` หรือ `./target/release/...` ตามปกติ
3. วิธีการออกโดย**ไม่ให้มันดับ** (Detach): ให้กดปุ่ม **`Ctrl+b`** แล้วปล่อย จากนั้นกดปุ่ม **`d`**
4. หลังจากกด คุณจะเด้งกลับมาที่หน้าจอ Terminal ปกติ ปิดคอมได้เลย โปรแกรมจะยังทำงานต่อไปใน background

**การกลับเข้าไปในหน้าต่างเดิม:**
```bash
tmux attach
```

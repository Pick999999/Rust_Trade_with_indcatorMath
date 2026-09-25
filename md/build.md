# การ Build และนำไฟล์ไปใช้งานบน e2.micro

เมื่อคุณทำการรันคำสั่ง `cargo build --release` บนเครื่องเซิร์ฟเวอร์เสร็จสิ้น ไฟล์ binary ที่พร้อมใช้งาน (Executable) จะถูกสร้างขึ้นและเก็บไว้ที่โฟลเดอร์:

```
target/release/turbo-indicators
```

**พาธเต็มบนเครื่อง Ubuntu:**
```
/home/ubuntu/indicators_Multiplex/target/release/turbo-indicators
```

---

## วิธีการนำไฟล์ไปใช้งานบน e2.micro

มี 3 วิธีหลักๆ ในการส่งไฟล์ไปรันบน e2.micro สามารถเลือกใช้ตามความสะดวกครับ:

### วิธีที่ 1: ดาวน์โหลดลง Local ก่อน แล้วค่อยอัปโหลดขึ้น e2.micro
เหมาะสำหรับกรณีที่คุณไม่มีการตั้งค่าให้เซิร์ฟเวอร์ทั้งสองเครื่องรู้จักกันโดยตรง

1. **ดาวน์โหลดจากเครื่อง Build ลงคอมพิวเตอร์ของคุณ (Local):**
   ```bash
   scp -i your-key.pem ubuntu@<IP-เครื่อง-build>:/home/ubuntu/indicators_Multiplex/target/release/turbo-indicators .
   ```
2. **อัปโหลดจาก Local ขึ้นไปยังเครื่อง e2.micro:**
   ```bash
   scp -i your-key.pem turbo-indicators ubuntu@<IP-e2-micro>:/home/ubuntu/indicators_Multiplex/
   ```

### วิธีที่ 2: ส่งตรงระหว่างเซิร์ฟเวอร์ด้วย `scp`
รันคำสั่งนี้จาก **เครื่อง Build** เพื่อส่งไฟล์ข้ามไปยัง e2.micro โดยตรง (ต้องมั่นใจว่าเครื่อง Build สามารถ SSH เข้า e2.micro ได้)

```bash
scp /home/ubuntu/indicators_Multiplex/target/release/turbo-indicators ubuntu@<IP-e2-micro>:/home/ubuntu/indicators_Multiplex/
```

### วิธีที่ 3: ส่งตรงระหว่างเซิร์ฟเวอร์ด้วย `rsync` (แนะนำสำหรับการทำบ่อยๆ)
`rsync` จะส่งเฉพาะส่วนที่มีการเปลี่ยนแปลง ทำให้รวดเร็วกว่าในกรณีที่มีไฟล์อื่นร่วมด้วย

```bash
rsync -avz -e "ssh -i your-key.pem" ubuntu@<IP-build>:/home/ubuntu/indicators_Multiplex/target/release/turbo-indicators ubuntu@<IP-e2-micro>:/home/ubuntu/indicators_Multiplex/
```

---

## การเตรียมไฟล์ให้พร้อมรัน ⚠️

หลังจากที่คุณคัดลอกไฟล์ไปยังเครื่อง e2.micro เรียบร้อยแล้ว อย่าลืมให้สิทธิ์การรัน (Execute Permission) กับไฟล์ด้วยคำสั่ง:

```bash
chmod +x /home/ubuntu/indicators_Multiplex/turbo-indicators
```

จากนั้นสามารถรันโปรแกรมได้ตามปกติด้วยคำสั่ง:
```bash
./turbo-indicators
```

# แผนงานจัดระเบียบและเข้ารหัส JavaScript (Obfuscation Plan)

แผนงานนี้มีวัตถุประสงค์เพื่อแยกโค้ด JavaScript ที่ฝังอยู่ใน `index.html` ออกมาเป็นไฟล์แยก (Modularization) จัดโครงสร้างใหม่ด้วย ES6 Class เพื่อให้ง่ายต่อการดูแลรักษา และนำไปผ่านกระบวนการเข้ารหัส (Obfuscation) เพื่อป้องกันการคัดลอก คาดเดา หรือดัดแปลงโค้ดจากผู้ไม่หวังดี

## ระยะที่ 1: การจัดโครงสร้างใหม่ (Refactoring & Modularization)

ในขั้นตอนนี้เราจะดึงโค้ด JavaScript กว่า 3,000 บรรทัดออกจาก `public/index.html` และจัดระเบียบใหม่เป็นคลาสต่างๆ ในโฟลเดอร์ `public/src/`

### 1.1 สร้างโฟลเดอร์สำหรับ JavaScript
- สร้างโครงสร้างไดเรกทอรีใหม่: `public/src/js/`
- โครงสร้างคลาสที่นำเสนอ:
  - `App.js` (คลาสหลัก ควบคุมการเริ่มต้นโปรแกรม)
  - `FingerprintManager.js` (จัดการ UAParser, FingerprintJS และการเข้าสู่ระบบ)
  - `WebSocketClient.js` (จัดการการเชื่อมต่อ รับ-ส่งข้อความกับ Rust Backend)
  - `ChartController.js` (จัดการ Lightweight Charts, กราฟแท่งเทียน, Indicator ต่างๆ)
  - `UIManager.js` (จัดการ Event Listeners, การเปลี่ยนธีม, การแสดงผลตาราง)

### 1.2 จัดการ Global Variables
- ย้ายตัวแปร Global (เช่น `appConfig`, `candleSeries`, `indicators`) เข้าไปเป็น Property ของคลาสต่างๆ (เช่น `this.appConfig` ใน `App.js`)
- ลบ `<script>` แท็กเดิมออกจาก `index.html`

### 1.3 ทดสอบการทำงาน (Integration Test)
- โหลดไฟล์แบบ ES Module ในช่วงพัฒนาก่อน: `<script type="module" src="./src/js/App.js"></script>`
- ทดสอบระบบให้มั่นใจว่าทำงานได้เหมือนเดิม 100%

---

## ระยะที่ 2: การติดตั้ง Build Tools (Bundling)

เมื่อโค้ดถูกแยกเป็นไฟล์ เราจำเป็นต้องรวมไฟล์ทั้งหมดกลับเป็นไฟล์เดียวเพื่อให้ง่ายต่อการแจกจ่ายและการ Obfuscate

### 2.1 ติดตั้ง Node.js & Vite
- สร้างไฟล์ `package.json` ในโฟลเดอร์หลักหรือโฟลเดอร์ `public/`
- รันคำสั่งติดตั้ง Vite: `npm install vite --save-dev`
- *เหตุผลที่เลือก Vite*: เป็น Build Tool สมัยใหม่ ทำงานเร็ว และคอนฟิกง่าย

### 2.2 สร้าง `vite.config.js`
- กำหนดให้ Vite ทำการ Build ไฟล์จาก `public/src/js/App.js` 
- กำหนด Output ออกไปที่โฟลเดอร์ `public/dist/`
- ปรับแต่งให้รองรับการโหลดจาก Root Directory ของหน้าเว็บ

---

## ระยะที่ 3: การเข้ารหัสและป้องกันโค้ด (Obfuscation)

ขั้นตอนนี้คือเป้าหมายหลัก เพื่อแปลงโค้ดให้อ่านไม่ออกและป้องกันการทำ Reverse Engineering

### 3.1 ติดตั้ง Obfuscator Plugin
- ติดตั้ง Plugin สำหรับ Vite: `npm install vite-plugin-javascript-obfuscator javascript-obfuscator --save-dev`

### 3.2 ตั้งค่าความปลอดภัย (Obfuscator Options)
ใน `vite.config.js` เราจะตั้งค่าการ Obfuscate ระดับสูง (High Obfuscation) ดังนี้:

```javascript
{
  compact: true,
  controlFlowFlattening: true,       // กวนลำดับการทำงาน (ทำให้แกะโค้ดยากมาก)
  controlFlowFlatteningThreshold: 1,
  numbersToExpressions: true,        // แปลงตัวเลขเป็นสูตรคณิตศาสตร์
  simplify: true,
  stringArray: true,                 // ซ่อน String ต่างๆ
  stringArrayEncoding: ['rc4'],      // เข้ารหัส String
  stringArrayThreshold: 1,
  splitStrings: true,
  disableConsoleOutput: true,        // ปิด console.log อัตโนมัติ (ป้องกันคนแอบดูข้อมูล)
  debugProtection: true,             // ป้องกันการเปิดหน้าจอ Developer Tools (F12 / Debugger)
  debugProtectionInterval: 5000,
  domainLock: ['yourdomain.com']     // (Optional) ล็อกให้รันได้เฉพาะบนโดเมนของคุณเท่านั้น
}
```

> [!WARNING]
> การเปิด `controlFlowFlattening` และตั้งค่าการเข้ารหัสระดับสูง อาจทำให้ประสิทธิภาพการประมวลผล (Performance) ของ JavaScript ช้าลงเล็กน้อย (ประมาณ 10-30%) ต้องทดสอบการเรนเดอร์กราฟ (Chart) หลังจาก Obfuscate เสมอว่ายังลื่นไหลหรือไม่

---

## ระยะที่ 4: การนำไปใช้จริง (Deployment)

### 4.1 เชื่อมต่อกับ HTML
- แก้ไขไฟล์ `index.html` ให้โหลดไฟล์ที่ถูก Build แล้วเพียงไฟล์เดียว:
  `<script src="/dist/app.bundle.js"></script>`

### 4.2 สร้าง Build Script
- เพิ่มสคริปต์ใน `package.json` ให้สามารถ Build ได้ง่ายๆ
  `"build": "vite build"`
- ในอนาคตเมื่อมีการแก้ไขโค้ด เพียงแค่รัน `npm run build` ระบบจะจัดการรวบรวมไฟล์และ Obfuscate ให้อัตโนมัติ

### 4.3 ปรับปรุงไฟล์ `.gitignore`
- ป้องกันไม่ให้เผลออัพโหลดโฟลเดอร์ `node_modules` และ `public/dist` ขึ้น Git (ถ้ามี)

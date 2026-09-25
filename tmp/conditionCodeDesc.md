# รหัสเงื่อนไข (Condition Codes)

ไฟล์นี้อธิบายรหัสเงื่อนไขการตัดสินใจ (Condition Codes) ของแต่ละกลยุทธ์ที่ใช้ในระบบเทรด ซึ่งรหัสเหล่านี้จะถูกบันทึกลงในฟิลด์ `codeStrategy` ของไฟล์ `trades.json` เมื่อจบไม้แต่ละไม้ เพื่อนำไปใช้วิเคราะห์การทำงานของบอท

## กลยุทธ์ V1 (ดั้งเดิม)
*   **`V1-A`** : เข้าเงื่อนไข Default (lossCon < 2) → แนะนำ Green
*   **`V1-B`** : เข้าเงื่อนไข lossCon >= 2 และแท่งเทียนปัจจุบันสีเขียว → แนะนำ Green
*   **`V1-C`** : เข้าเงื่อนไข lossCon >= 2 และแท่งเทียนปัจจุบันสีแดง → แนะนำ Red

## กลยุทธ์ V2 (Early React)
*   **`V2-A`** : เข้าเงื่อนไข Default (lossCon < 1) → แนะนำ Green
*   **`V2-B`** : เข้าเงื่อนไข lossCon >= 1 และแท่งเทียนปัจจุบันสีเขียว → แนะนำ Green
*   **`V2-C`** : เข้าเงื่อนไข lossCon >= 1 และแท่งเทียนปัจจุบันสีแดง → แนะนำ Red

## กลยุทธ์ V3A (EMA Consensus)
*   **`V3A-A`** : เข้าเงื่อนไข Default (lossCon < 2) → แนะนำ Green
*   **`V3A-B`** : เข้าเงื่อนไข lossCon >= 2 และเสียงส่วนใหญ่ของ 3 EMA ชี้ขึ้น (Majority Up) → แนะนำ Green
*   **`V3A-C`** : เข้าเงื่อนไข lossCon >= 2 และเสียงส่วนใหญ่ของ 3 EMA ชี้ลง (Majority Down) → แนะนำ Red

## กลยุทธ์ V3B (EMA Short + CutType)
*   **`V3B-A`** : เข้าเงื่อนไข Default (lossCon < 2) → แนะนำ Green
*   **`V3B-B`** : เข้าเงื่อนไข lossCon >= 2 และเส้น EMA Short ชี้ขึ้น → แนะนำ Green
*   **`V3B-C`** : เข้าเงื่อนไข lossCon >= 2 และเส้น EMA Short ชี้ลง → แนะนำ Red
*   **`V3B-D`** : เข้าเงื่อนไข lossCon >= 2 เกิดการตัดขึ้น (CutUp Override) → แนะนำ Green
*   **`V3B-E`** : เข้าเงื่อนไข lossCon >= 2 เกิดการตัดลง (CutDown Override) → แนะนำ Red

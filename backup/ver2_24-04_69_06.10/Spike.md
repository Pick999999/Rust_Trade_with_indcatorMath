แท่ง A เกิด Spike (is_atr = true), สีเขียว
  │
  ├─→ suggest = "green" → เทรด CALL ทันที
  │
  ├─ ชนะ? → ✅ หยุด, reset lossCon = 0, รอ Spike ใหม่
  │
  └─ แพ้? → lossCon = 1, เข้า Martingale mode
       │
       ├─→ แท่ง B (ถัดไป): suggest = "green" → เทรด CALL ทันที
       │   ├─ ชนะ? → ✅ หยุด
       │   └─ แพ้? → lossCon = 2
       │
       ├─→ แท่ง C: suggest = "green" → เทรด CALL ทันที  
       │   ├─ ชนะ? → ✅ หยุด
       │   └─ แพ้? → lossCon = 3
       │
       └─→ แท่ง D: lossCon >= 3 → suggest = ตามสีแท่ง D
            ├─ แท่ง D = green → เทรด CALL
            └─ แท่ง D = red   → เทรด PUT

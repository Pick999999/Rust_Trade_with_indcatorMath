/**
 * pkTrendEntryEvaluator.js
 * ==================================================================
 * 📖 รายละเอียดและคู่มือการตั้งค่า Parameter ทั้งหมด อ่านได้ที่: Pktrendentryevaluator.md
 * ==================================================================
 * ประเมินสัญญาณเข้าเทรดจาก field "pkTrend" เพียงอย่างเดียว
 * Threshold ต่าง ๆ อ้างอิงจากการวิเคราะห์สถิติจริงของไฟล์ pkTrend.json
 * (421 records) ที่ผู้ใช้ให้มา:
 *
 *   trendStrength        trendScore range (จริง)
 *   ULTRA_STRONG_UP        70 ถึง 100
 *   STRONG_UP               40 ถึง 68
 *   MILD_UP                 20 ถึง 38
 *   NEUTRAL                -18 ถึง 18
 *   MILD_DOWN              -38 ถึง -20
 *   STRONG_DOWN            -68 ถึง -40
 *   ULTRA_STRONG_DOWN     -100 ถึง -70
 *   WHIPSAW_ZONE           -55 ถึง 55   (คาบเกี่ยวช่วงอื่นหมด → ต้องกรองด้วย isWhipsaw เสมอ)
 *
 *   isWhipsaw = true  ⇔ whipsawStatus ∈ {ENTERING_WHIPSAW, CONFIRMED_WHIPSAW}
 *   isWhipsaw = false ⇔ whipsawStatus ∈ {TRENDING, CHOPPY}
 *
 *   closePosition เฉลี่ย: UP-CONFIRM 0.84 (0.50–1.00) / DN-CONFIRM 0.18 (0–0.48)
 *                          SPK-CONTINUE-UP 0.88 (0.66–1.00) / SPK-CONTINUE-DN 0.06 (0–0.31)
 *
 *   caseCode ทั้งหมดที่พบ (20 ตัว) แบ่งตาม group:
 *     Up:        UP-CONFIRM
 *     Down:      DN-CONFIRM
 *     Spike:     SPK-CONTINUE-UP, SPK-CONTINUE-DN, SPK-BULLTRAP, SPK-BEARTRAP,
 *                SPK-NODIRECTION, SPK-HESITANT-UP
 *     Engulfing: EG-BULLISH, EG-BEARISH
 *     Rejected:  RJ-BULLTRAP-STRONG, RJ-BEARTRAP-STRONG, RJ-FOLLOWFAIL-UP,
 *                RJ-FOLLOWFAIL-DN, RJ-UPWICK-WEAK, RJ-LOWWICK-WEAK
 *     Sideways:  SD-INSIDEBAR, SD-MIXEDSIGNAL, SD-MIXEDSIGNAL-DN
 *     System:    SYS-NODATA
 * ==================================================================
 *
 * วิธีติดตั้ง / เรียกใช้ (Node.js)
 * ------------------------------------------------------------------
 *   const {
 *     evaluatePkTrendEntry,
 *     filterPkTrendArray,
 *     filterCandlesByPkTrend,
 *     DEFAULT_OPTIONS,
 *   } = require('./pkTrendEntryEvaluator.js');
 *
 * ถ้าใช้เป็น ES Module (import/export) ให้เปลี่ยนท้ายไฟล์จาก
 * `module.exports = {...}` เป็น `export {...}` เอง
 *
 *
 * ==================================================================
 * 1) evaluatePkTrendEntry(pkTrend, options)
 * ==================================================================
 * หน้าที่:
 *   เช็ค pkTrend "1 แท่งเทียน" ว่าควรเข้าเทรดหรือไม่ ใช้ตอนรัน real-time
 *   ทีละแท่ง (เช่น แท่งใหม่ปิดมาแล้วอยากรู้ว่าเข้าได้ไหม)
 *
 * พารามิเตอร์:
 *   - pkTrend (object, จำเป็น): ตัว object จาก field "pkTrend" ของแท่งนั้น ๆ
 *     ต้องมี field อย่างน้อย: trend, group, caseCode, trendScore,
 *     closePosition, trendStrength, extremeTrend, isWhipsaw, whipsawStatus
 *   - options (object, ไม่บังคับ): ใช้ override ค่า default เช่น
 *       { minTrendScoreStrong: 50 }  → เข้มขึ้น ต้อง score แรงกว่าเดิม
 *       { allowSpikeContinuation: false } → ปิดการเข้าจากสัญญาณ Spike
 *       { allowTrapReversal: false }      → ปิดการเข้าแบบสวนเทรนด์ (trap)
 *     ดูค่า default ทั้งหมดได้ที่ตัวแปร DEFAULT_OPTIONS ด้านล่าง
 *
 * ค่าที่ return:
 *   {
 *     signal:  'BUY' | 'SELL' | 'WAIT',   // คำตัดสินสุดท้าย
 *     reason:  string,                     // เหตุผลเป็นข้อความ อ่านง่าย
 *     strength?: string,                   // ระดับความแรง เช่น 'ULTRA_STRONG_UP' หรือ 'REVERSAL'
 *     danger?: true,                       // ถ้ามี = แท่งนี้เข้าข่ายอันตราย/หลอก ควรระวังเป็นพิเศษ
 *   }
 *
 * ตัวอย่างการใช้งาน:
 *   const candle = require('./someCandle.json');       // มี field pkTrend อยู่ข้างใน
 *   const result = evaluatePkTrendEntry(candle.pkTrend);
 *
 *   if (result.signal === 'BUY') {
 *     console.log('เข้าซื้อ:', result.reason);
 *   } else if (result.signal === 'SELL') {
 *     console.log('เข้าขาย:', result.reason);
 *   } else {
 *     console.log('รอก่อน:', result.reason);
 *   }
 *
 *   // ตัวอย่างการปรับ threshold ให้เข้มขึ้น (เฉพาะสัญญาณแรงมาก ๆ เท่านั้น)
 *   const strictResult = evaluatePkTrendEntry(candle.pkTrend, {
 *     minTrendScoreStrong: 70,        // ต้องเป็นระดับ ULTRA_STRONG เท่านั้น
 *     minCloseConvictionUp: 0.85,
 *     maxCloseConvictionDown: 0.15,
 *     allowTrapReversal: false,       // ไม่เอาสัญญาณ reversal เลย เอาแต่ trend-following
 *   });
 *
 *
 * ==================================================================
 * 2) filterPkTrendArray(pkTrendArray, options)
 * ==================================================================
 * หน้าที่:
 *   ใช้กับไฟล์ pkTrend.json ที่มีโครงสร้างเป็น "array ของ pkTrend object
 *   ตรง ๆ" (แบบเดียวกับไฟล์ที่อัปโหลดมาในบทสนทนานี้) ฟังก์ชันจะไล่เช็ค
 *   ทุก record ให้เองแล้วคืนเฉพาะแท่งที่มีสัญญาณ BUY/SELL เท่านั้น
 *   (แท่งที่เป็น WAIT จะถูกกรองทิ้งไป ไม่ต้องมาไล่ดูเอง)
 *
 * พารามิเตอร์:
 *   - pkTrendArray (array, จำเป็น): array ของ pkTrend object เช่น
 *     ข้อมูลจาก JSON.parse(fs.readFileSync('pkTrend.json'))
 *   - options (object, ไม่บังคับ): เหมือนกับของ evaluatePkTrendEntry เป๊ะ
 *
 * ค่าที่ return:
 *   array ของ object แต่ละอันคือแท่งที่ "ผ่านเงื่อนไข" เท่านั้น มี field:
 *   { index, signal, reason, strength? }
 *   โดย index คือลำดับตำแหน่งในไฟล์ (นับจาก 0) ใช้ไปจับคู่กับแท่งเทียน
 *   จริงใน dataset อื่นได้
 *
 * ตัวอย่างการใช้งาน:
 *   const fs = require('fs');
 *   const pkTrendData = JSON.parse(fs.readFileSync('pkTrend.json', 'utf-8'));
 *
 *   const signals = filterPkTrendArray(pkTrendData);
 *   console.log('พบสัญญาณทั้งหมด:', signals.length);
 *
 *   const buySignals  = signals.filter(s => s.signal === 'BUY');
 *   const sellSignals = signals.filter(s => s.signal === 'SELL');
 *   console.log('BUY:', buySignals.length, 'SELL:', sellSignals.length);
 *
 *   signals.forEach(s => {
 *     console.log(`แท่งที่ ${s.index}: ${s.signal} — ${s.reason}`);
 *   });
 *
 *
 * ==================================================================
 * 3) filterCandlesByPkTrend(candles, options)
 * ==================================================================
 * หน้าที่:
 *   เหมือนกับ filterPkTrendArray() ทุกอย่าง แต่ใช้กับข้อมูลที่เป็น
 *   "แท่งเทียนเต็มรูปแบบ" ที่มี field pkTrend ซ้อนอยู่ข้างใน — คือ
 *   โครงสร้างแบบเดียวกับที่ได้ตรงจาก API getFullAnalysisData
 *   (แต่ละแท่งมี open/high/low/close/... และมี candle.pkTrend อยู่ด้วย)
 *   เหมาะกับตอนที่มีข้อมูลราคาเต็มและอยากได้ context ราคา/เวลากลับมาด้วย
 *
 * พารามิเตอร์:
 *   - candles (array, จำเป็น): array ของแท่งเทียน แต่ละอันต้องมี field
 *     `pkTrend` อยู่ข้างใน (เช่น candle.pkTrend) ส่วน field อื่น ๆ เช่น
 *     candletime_display, close จะถูกดึงมาแนบกลับมาด้วยอัตโนมัติ
 *   - options (object, ไม่บังคับ): เหมือนกับของ evaluatePkTrendEntry เป๊ะ
 *
 * ค่าที่ return:
 *   array ของ object ที่ผ่านเงื่อนไข มี field:
 *   { index, time, close, signal, reason, strength? }
 *   โดย time = candletime_display (เวลาที่อ่านง่าย) และ close = ราคาปิด
 *   ของแท่งนั้น ทำให้เอาไปโชว์ผลหรือ backtest ต่อได้สะดวกขึ้น
 *
 * ตัวอย่างการใช้งาน:
 *   // สมมติดึงข้อมูลจาก API getFullAnalysisData มาเก็บเป็น apiResponse
 *   const candles = apiResponse.data;   // array ของแท่งเทียนเต็ม
 *
 *   const signals = filterCandlesByPkTrend(candles);
 *   signals.forEach(s => {
 *     console.log(`${s.time} | close=${s.close} | ${s.signal} | ${s.reason}`);
 *   });
 *
 *
 * ==================================================================
 * 4) DEFAULT_OPTIONS
 * ==================================================================
 * หน้าที่:
 *   object เก็บค่า threshold เริ่มต้นทั้งหมดที่ฟังก์ชันด้านบนใช้ ถ้าไม่ได้
 *   ส่ง options เข้าไป ระบบจะใช้ค่าจากตัวนี้เสมอ เอาไว้ดูค่าปัจจุบัน หรือ
 *   เอาไปทำ options ใหม่โดย spread ทับบางค่าก็ได้ เช่น
 *
 *   const myOptions = { ...DEFAULT_OPTIONS, minTrendScoreStrong: 60 };
 *   const result = evaluatePkTrendEntry(pkTrend, myOptions);
 * ------------------------------------------------------------------
 */

const DEFAULT_OPTIONS = {
  // ต้อง trendScore แรงระดับ STRONG ขึ้นไปถึงจะนับเป็นสัญญาณตามเทรนด์
  minTrendScoreStrong: 40,
  // closePosition ต้องยืนยันแรงซื้อ/ขายชัดเจน (อิงจากค่าเฉลี่ยจริงของ UP-CONFIRM / DN-CONFIRM)
  minCloseConvictionUp: 0.7,
  maxCloseConvictionDown: 0.3,
  // เปิด/ปิดการเข้าแบบ reversal (bull trap / bear trap ที่ยืนยันแล้ว)
  allowTrapReversal: true,
  // ปิด/เปิดการเข้าแบบ breakout ต่อเนื่องจาก spike
  allowSpikeContinuation: true,
};

/**
 * ประเมิน pkTrend object เดียว (1 แท่งเทียน) ว่าควรเข้าเทรดไหม
 * @param {object} pkTrend - object จาก field pkTrend ของแต่ละแท่ง
 * @param {object} [options] - override threshold ได้ตามต้องการ
 * @returns {{signal: 'BUY'|'SELL'|'WAIT', reason: string, strength?: string, danger?: boolean}}
 */
function evaluatePkTrendEntry(pkTrend, options = {}) {
  const opt = { ...DEFAULT_OPTIONS, ...options };

  if (!pkTrend || pkTrend.group === 'System' || pkTrend.trend === null) {
    return { signal: 'WAIT', reason: 'ข้อมูลไม่พอ (SYS-NODATA)' };
  }

  const {
    trend, group, caseCode, trendScore,
    closePosition, trendStrength, extremeTrend,
    isWhipsaw, whipsawStatus,
  } = pkTrend;

  // 0) กันสัญญาณหลอกก่อนเสมอ — whipsaw ต้องถูกกรองทิ้งไม่ว่า trendScore จะสูงแค่ไหน
  if (isWhipsaw === true) {
    return {
      signal: 'WAIT',
      reason: `${whipsawStatus} — เสี่ยงโดนหลอกฟันปลาสูง (${caseCode})`,
      danger: true,
    };
  }

  // 0.1) ตลาดพัก ไม่มีทิศทางชัดเจน
  if (group === 'Sideways') {
    return { signal: 'WAIT', reason: `Sideways: ${caseCode}` };
  }

  // 0.2) กลุ่ม spike-trap อันตราย (stop hunt / liquidity grab) — เตือนไม่ให้เข้าเสมอ
  if (caseCode === 'SPK-BULLTRAP' || caseCode === 'SPK-BEARTRAP' || caseCode === 'SPK-NODIRECTION' || caseCode === 'SPK-HESITANT-UP') {
    return { signal: 'WAIT', reason: `${caseCode} — ความเสี่ยงกลับตัวสูง ไม่ควรเข้า`, danger: true };
  }

  // ===== 1) Trend-following entry: ตามทิศเทรนด์หลัก =====
  const trendFollowGroups = ['Up', 'Down', 'Engulfing'].concat(
    opt.allowSpikeContinuation ? ['Spike'] : []
  );

  if (trendFollowGroups.includes(group) && trend !== 'Sideways' && trend !== 'Rejected') {
    if (trend === 'UpTrend' && trendScore >= opt.minTrendScoreStrong && closePosition >= opt.minCloseConvictionUp) {
      return {
        signal: 'BUY',
        reason: `${caseCode} (${trendStrength}) score=${trendScore} closePos=${closePosition}`,
        strength: trendStrength,
      };
    }
    if (trend === 'DownTrend' && trendScore <= -opt.minTrendScoreStrong && closePosition <= opt.maxCloseConvictionDown) {
      return {
        signal: 'SELL',
        reason: `${caseCode} (${trendStrength}) score=${trendScore} closePos=${closePosition}`,
        strength: trendStrength,
      };
    }
  }

  // ===== 2) Reversal entry: bull trap / bear trap ที่ยืนยันแล้ว =====
  if (opt.allowTrapReversal && group === 'Rejected') {
    if (caseCode === 'RJ-BULLTRAP-STRONG' && extremeTrend === 'UpTrend' && closePosition <= 0.2) {
      return { signal: 'SELL', reason: `Bull Trap ยืนยัน closePos=${closePosition}`, strength: 'REVERSAL' };
    }
    if (caseCode === 'RJ-BEARTRAP-STRONG' && extremeTrend === 'DownTrend' && closePosition >= 0.8) {
      return { signal: 'BUY', reason: `Bear Trap ยืนยัน closePos=${closePosition}`, strength: 'REVERSAL' };
    }
  }

  return { signal: 'WAIT', tradeType: null, reason: `${caseCode || 'N/A'} ไม่เข้าเงื่อนไขที่ตั้งไว้ (score=${trendScore})` };
}

/**
 * ปรับปรุงผลลัพธ์ให้มี tradeType ('CALL' สำหรับ BUY, 'PUT' สำหรับ SELL)
 */
function evaluateTradeSpot(pkTrend, options = {}) {
  const result = evaluatePkTrendEntry(pkTrend, options);
  const tradeType = result.signal === 'BUY' ? 'CALL' : (result.signal === 'SELL' ? 'PUT' : null);
  return {
    ...result,
    tradeType,
  };
}

/**
 * รับ array ของ pkTrend object ตรง ๆ (โครงสร้างเดียวกับไฟล์ pkTrend.json ที่ให้มา)
 * แล้วคืนเฉพาะแท่งที่มีสัญญาณ BUY/SELL พร้อม index อ้างอิง
 * @param {object[]} pkTrendArray
 * @param {object} [options]
 * @returns {{index:number, signal:string, tradeType:string, reason:string}[]}
 */
function filterPkTrendArray(pkTrendArray, options = {}) {
  return pkTrendArray
    .map((pk, index) => ({ index, ...evaluateTradeSpot(pk, options) }))
    .filter((r) => r.signal !== 'WAIT');
}

/**
 * เผื่อกรณีข้อมูลเป็น "แท่งเทียนเต็ม" (ที่มี field pkTrend ซ้อนอยู่ข้างใน)
 * แบบที่ได้จาก API getFullAnalysisData โดยตรง
 * @param {object[]} candles - array ของแท่งเทียน แต่ละอันมี candle.pkTrend
 * @param {object} [options]
 */
function filterCandlesByPkTrend(candles, options = {}) {
  return candles
    .map((c, i) => ({
      index: c.index ?? i,
      time: c.candletime_display,
      close: c.close,
      ...evaluateTradeSpot(c.pkTrend, options),
    }))
    .filter((r) => r.signal !== 'WAIT');
}

// UMD / Global Export (Node.js + Browser compatible)
if (typeof module !== 'undefined' && module.exports) {
  module.exports = {
    evaluatePkTrendEntry,
    evaluateTradeSpot,
    filterPkTrendArray,
    filterCandlesByPkTrend,
    DEFAULT_OPTIONS,
  };
}
if (typeof window !== 'undefined') {
  window.evaluatePkTrendEntry = evaluatePkTrendEntry;
  window.evaluateTradeSpot = evaluateTradeSpot;
  window.filterPkTrendArray = filterPkTrendArray;
  window.filterCandlesByPkTrend = filterCandlesByPkTrend;
  window.DEFAULT_OPTIONS = DEFAULT_OPTIONS;
  window.PkTrendEntryEvaluator = {
    evaluatePkTrendEntry,
    evaluateTradeSpot,
    filterPkTrendArray,
    filterCandlesByPkTrend,
    DEFAULT_OPTIONS,
  };
}
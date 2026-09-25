/**
 * ===============================================
 * detectTrend_V5.js
 * ===============================================
 * ปรับปรุงจาก V4 ครอบคลุม 11 จุดที่วิเคราะห์พบ:
 *
 * [Critical #1]  DownTrend ใช้ LOW เปรียบเทียบแล้ว (ไม่ใช่แค่ high)
 * [Critical #2]  เพิ่ม Lower Low / Higher Low / Lower High / Higher Hight structure
 * [Critical #3]  extremeTrend สมดุล — UpTrend ใช้ OR, DownTrend ก็ใช้ OR เหมือนกัน
 * [Moderate #4]  Volume confirmation (optional — ถ้า candle มี field volume)
 * [Moderate #5]  Spike gap cases เติมครบ (SPIKE_HESITANT_UP / DOWN)
 * [Moderate #6]  DownTrend branch มี MIXED_SIGNAL_DOWN แล้ว
 * [Moderate #7]  เพิ่ม FLAT_SUPPORT_TEST (symmetric กับ FLAT_RESISTANCE_TEST)
 * [Design   #8]  เพิ่ม rangeRatio วัดขนาดแท่งเทียบค่าเฉลี่ย 2 แท่งก่อนหน้า
 * [Design   #9]  เพิ่ม structure object (higherHigh, lowerLow, higherLow, lowerHigh)
 * [Edge    #10]  close === prev close จัดการชัดเจน (closeEqualP1)
 * [Design  #11]  เพิ่ม Engulfing pattern สำหรับแท่งปกติ (BULLISH / BEARISH / INDECISION)
 *
 * key ของ TREND_CASES = caseDesc (ชื่อเต็ม ใช้เป็น identifier หลัก)
 * - caseCode : รหัสสั้น สื่อความหมาย เช่น 'UP-CONFIRM', 'SPK-BEARTRAP'
 * - trend    : หมวดใหญ่ ('UpTrend' | 'DownTrend' | 'Rejected' | 'Sideways' | null)
 * - group    : กลุ่ม ('Up' | 'Down' | 'Rejected' | 'Sideways' | 'Spike' | 'Engulfing' | 'System')
 * - description : คำอธิบายภาษาไทย
 */
const TREND_CASES = {
  // ========== System ==========
  INSUFFICIENT_DATA: {
    caseCode: 'SYS-NODATA',
    trend: null, group: 'System',
    description: 'ข้อมูลไม่พอสำหรับเปรียบเทียบ ต้องมีอย่างน้อย 3 แท่ง (ปัจจุบัน + ก่อนหน้า 2 แท่ง)',
  },

  // ========== Up / Down ยืนยันแล้ว ==========
  CONFIRMED_UP: {
    caseCode: 'UP-CONFIRM',
    trend: 'UpTrend', group: 'Up',
    description: 'ทำ Higher High ปิดสูงกว่าหรือเท่าแท่งก่อนหน้า และปิดในโซนบนของแท่งตัวเอง ยืนยันแรงซื้อยังควบคุมตลาด',
  },
  CONFIRMED_DOWN: {
    caseCode: 'DN-CONFIRM',
    trend: 'DownTrend', group: 'Down',
    description: 'ทำ Lower Low ปิดต่ำกว่าหรือเท่าแท่งก่อนหน้า และปิดในโซนล่างของแท่งตัวเอง ยืนยันแรงขายยังควบคุมตลาด',
  },

  // ========== Rejected (แท่งปกติ) ==========
  STRONG_BULL_TRAP: {
    caseCode: 'RJ-BULLTRAP-STRONG',
    trend: 'Rejected', group: 'Rejected',
    description: 'ทำ Higher High แต่ถูกแรงขายกดลงมาปิดใกล้จุดต่ำสุดของแท่ง และปิดต่ำกว่าแท่งก่อนหน้าด้วย สัญญาณกลับตัวลงที่ชัดเจน (Bull Trap)',
  },
  WEAK_UPPER_WICK: {
    caseCode: 'RJ-UPWICK-WEAK',
    trend: 'Rejected', group: 'Rejected',
    description: 'ทำ Higher High และยังปิดสูงกว่าแท่งก่อนหน้า แต่มีไส้บนยาวแสดงว่าเจอแรงขายกดในช่วงท้ายแท่ง ควรระวังแรงซื้อเริ่มอ่อนกำลัง',
  },
  FAILED_FOLLOWTHROUGH_UP: {
    caseCode: 'RJ-FOLLOWFAIL-UP',
    trend: 'Rejected', group: 'Rejected',
    description: 'ทำ Higher High แต่ปิดต่ำกว่าแท่งก่อนหน้า แม้ตำแหน่งปิดในแท่งตัวเองจะยังไม่แย่มาก แสดงว่าโมเมนตัมขาขึ้นเริ่มแผ่ว ควรจับตาแท่งถัดไป',
  },
  STRONG_BEAR_TRAP: {
    caseCode: 'RJ-BEARTRAP-STRONG',
    trend: 'Rejected', group: 'Rejected',
    description: 'ทำ Lower Low แต่ถูกแรงซื้อดันขึ้นมาปิดใกล้จุดสูงสุดของแท่ง และปิดสูงกว่าแท่งก่อนหน้าด้วย สัญญาณกลับตัวขึ้นที่ชัดเจน (Bear Trap)',
  },
  WEAK_LOWER_WICK: {
    caseCode: 'RJ-LOWWICK-WEAK',
    trend: 'Rejected', group: 'Rejected',
    description: 'ทำ Lower Low และยังปิดต่ำกว่าแท่งก่อนหน้า แต่มีไส้ล่างยาวแสดงว่าเจอแรงซื้อดันกลับในช่วงท้ายแท่ง ควรระวังแรงขายเริ่มอ่อนกำลัง',
  },
  FAILED_FOLLOWTHROUGH_DOWN: {
    caseCode: 'RJ-FOLLOWFAIL-DN',
    trend: 'Rejected', group: 'Rejected',
    description: 'ทำ Lower Low แต่ปิดสูงกว่าแท่งก่อนหน้า แม้ตำแหน่งปิดในแท่งตัวเองจะยังไม่แย่มาก แสดงว่าโมเมนตัมขาลงเริ่มแผ่ว ควรจับตาแท่งถัดไป',
  },

  // ========== Sideways (แท่งปกติ) ==========
  INSIDE_BAR: {
    caseCode: 'SD-INSIDEBAR',
    trend: 'Sideways', group: 'Sideways',
    description: 'แท่งปัจจุบันมี high/low อยู่ภายในกรอบของแท่งก่อนหน้าทั้งหมด (Inside Bar) ตลาดกำลังหดตัว/พักตัว รอการ breakout เพื่อยืนยันทิศทางถัดไป',
  },
  DOJI_INDECISION: {
    caseCode: 'SD-DOJI',
    trend: 'Sideways', group: 'Sideways',
    description: 'Body ของแท่งเล็กมากเมื่อเทียบกับ range ทั้งหมด (เปิด-ปิดใกล้กัน) แรงซื้อแรงขายสูสีกัน ไม่มีฝ่ายใดชนะชัดเจนในแท่งนี้',
  },
  MIXED_SIGNAL: {
    caseCode: 'SD-MIXEDSIGNAL',
    trend: 'Sideways', group: 'Sideways',
    description: 'High สูงกว่าแท่งก่อนหน้าแค่ 1 ใน 2 แท่ง (สัญญาณขัดแย้งกันเอง) ประกอบกับ body เล็ก ตลาดกำลังสับสน ยังไม่มีทิศทางชัดเจนพอจะสรุป',
  },
  // V5 NEW: symmetric กับ MIXED_SIGNAL สำหรับ DownTrend branch
  MIXED_SIGNAL_DOWN: {
    caseCode: 'SD-MIXEDSIGNAL-DN',
    trend: 'Sideways', group: 'Sideways',
    description: 'Low ต่ำกว่าแท่งก่อนหน้าแค่ 1 ใน 2 แท่ง (สัญญาณขัดแย้งกันเอง) ประกอบกับ body เล็ก ตลาดกำลังสับสนฝั่งขาลง ยังไม่มีทิศทางชัดเจนพอจะสรุป',
  },
  FLAT_RESISTANCE_TEST: {
    caseCode: 'SD-FLATRESISTANCE',
    trend: 'Sideways', group: 'Sideways',
    description: 'High ของแท่งใกล้เคียงกับ high ของแท่งก่อนหน้ามาก แต่ไปต่อไม่ได้ เหมือนราคาชนแนวต้านเดิมซ้ำ (ทดสอบแนวต้าน) ยังไม่มีแรงมากพอจะทะลุ',
  },
  // V5 NEW: symmetric กับ FLAT_RESISTANCE_TEST
  FLAT_SUPPORT_TEST: {
    caseCode: 'SD-FLATSUPPORT',
    trend: 'Sideways', group: 'Sideways',
    description: 'Low ของแท่งใกล้เคียงกับ low ของแท่งก่อนหน้ามาก แต่ลงต่อไม่ได้ เหมือนราคาชนแนวรับเดิมซ้ำ (ทดสอบแนวรับ/Double Bottom) ยังไม่มีแรงมากพอจะทะลุ',
  },
  NO_CLEAR_STRUCTURE: {
    caseCode: 'SD-NOSTRUCTURE',
    trend: 'Sideways', group: 'Sideways',
    description: 'ไม่ได้ทำ Higher High หรือ Lower Low ชัดเจนเมื่อเทียบกับ 2 แท่งก่อนหน้า ตลาดยังไม่มีโครงสร้างที่ชัดเจนพอจะสรุปทิศทาง',
  },

  // ========== Engulfing / Outside Bar (แท่งปกติ ไม่ใช่ Spike) — V5 NEW ==========
  BULLISH_ENGULFING: {
    caseCode: 'EG-BULLISH',
    trend: 'UpTrend', group: 'Engulfing',
    description: 'Outside Bar ที่กลืนกรอบแท่งก่อนหน้า (high สูงกว่า + low ต่ำกว่า) และปิดในโซนบน สัญญาณกลับตัวขึ้นหรือแรงซื้อเข้ามาชัดเจน',
  },
  BEARISH_ENGULFING: {
    caseCode: 'EG-BEARISH',
    trend: 'DownTrend', group: 'Engulfing',
    description: 'Outside Bar ที่กลืนกรอบแท่งก่อนหน้า (high สูงกว่า + low ต่ำกว่า) และปิดในโซนล่าง สัญญาณกลับตัวลงหรือแรงขายเข้ามาชัดเจน',
  },
  ENGULFING_INDECISION: {
    caseCode: 'EG-INDECISION',
    trend: 'Sideways', group: 'Engulfing',
    description: 'Outside Bar ที่กลืนกรอบแท่งก่อนหน้า แต่ปิดใกล้กลางแท่ง แรงซื้อแรงขายสู้กันไม่มีฝ่ายใดชนะ ความผันผวนสูงแต่ไม่มีทิศทางชัดเจน',
  },

  // ========== Spike (อ้างอิงจาก field is_atr ของ FullAnalysisEngine.js) ==========
  SPIKE_BULL_TRAP: {
    caseCode: 'SPK-BULLTRAP',
    trend: 'Rejected', group: 'Spike',
    description: 'แท่งนี้เป็น Spike ที่พุ่งขึ้นแรงทำ Higher High แต่ถูกกดลงมาปิดใกล้จุดต่ำสุดของแท่ง คล้ายการล่าสภาพคล่องฝั่งซื้อ (Stop Hunt / Liquidity Grab) ความเสี่ยงกลับตัวลงสูงมาก ต้องระวังเป็นพิเศษ',
  },
  SPIKE_BEAR_TRAP: {
    caseCode: 'SPK-BEARTRAP',
    trend: 'Rejected', group: 'Spike',
    description: 'แท่งนี้เป็น Spike ที่ทิ่มลงแรงทำ Lower Low แต่ถูกดันกลับขึ้นมาปิดใกล้จุดสูงสุดของแท่ง คล้ายการล่าสภาพคล่องฝั่งขาย (Stop Hunt / Liquidity Grab) ความเสี่ยงกลับตัวขึ้นสูงมาก ต้องระวังเป็นพิเศษ',
  },
  SPIKE_CONTINUATION_UP: {
    caseCode: 'SPK-CONTINUE-UP',
    trend: 'UpTrend', group: 'Spike',
    description: 'แท่งนี้เป็น Spike ที่พุ่งขึ้นแรงและปิดยืนในโซนบนของแท่ง ไม่โดนกดกลับ มีโอกาสเป็นแรงส่งจริง (Breakout Momentum) ไม่ใช่แค่หลอก ควรติดตามแท่งถัดไปเพื่อยืนยันการไปต่อ',
  },
  SPIKE_CONTINUATION_DOWN: {
    caseCode: 'SPK-CONTINUE-DN',
    trend: 'DownTrend', group: 'Spike',
    description: 'แท่งนี้เป็น Spike ที่ทิ่มลงแรงและปิดยืนในโซนล่างของแท่ง ไม่โดนดันกลับ มีโอกาสเป็นแรงส่งจริง (Breakout Momentum) ฝั่งขาย ควรติดตามแท่งถัดไปเพื่อยืนยันการไปต่อ',
  },
  // V5 NEW: เติม gap case — Spike UpTrend แต่ close อยู่โซนกลาง หรือ close ไม่สูงกว่า prev
  SPIKE_HESITANT_UP: {
    caseCode: 'SPK-HESITANT-UP',
    trend: 'Sideways', group: 'Spike',
    description: 'แท่งนี้เป็น Spike ฝั่ง Higher High แต่ปิดในโซนกลางของแท่ง หรือปิดไม่สูงกว่าแท่งก่อนหน้า มีแรงซื้อแต่ยังไม่ยืนยันชัดเจน ควรรอแท่งถัดไปเพื่อยืนยัน',
  },
  // V5 NEW: เติม gap case — Spike DownTrend แต่ close อยู่โซนกลาง หรือ close ไม่ต่ำกว่า prev
  SPIKE_HESITANT_DOWN: {
    caseCode: 'SPK-HESITANT-DN',
    trend: 'Sideways', group: 'Spike',
    description: 'แท่งนี้เป็น Spike ฝั่ง Lower Low แต่ปิดในโซนกลางของแท่ง หรือปิดไม่ต่ำกว่าแท่งก่อนหน้า มีแรงขายแต่ยังไม่ยืนยันชัดเจน ควรรอแท่งถัดไปเพื่อยืนยัน',
  },
  SPIKE_WHIPSAW: {
    caseCode: 'SPK-WHIPSAW',
    trend: 'Sideways', group: 'Spike',
    description: 'แท่งนี้เป็น Spike ที่ range กว้างผิดปกติ (เกิน ATR) และแกว่งครอบคลุมทั้งจุดสูงและจุดต่ำของแท่งก่อนหน้า (Outside Bar) แต่ปิดใกล้กลางแท่ง แสดงถึงความผันผวนรุนแรงแบบไร้ทิศทาง อาจเป็นจุดพลิกผันของตลาด (Exhaustion) ควรรอสัญญาณยืนยันก่อนเข้าเทรด',
  },
  SPIKE_NO_CLEAR_DIRECTION: {
    caseCode: 'SPK-NODIRECTION',
    trend: 'Sideways', group: 'Spike',
    description: 'แท่งนี้เป็น Spike (range กว้างผิดปกติเกิน ATR) แต่ไม่ได้ทำ Higher High / Lower Low ชัดเจน และไม่ได้แกว่งครอบคลุมสองด้าน อาจเกิดจาก noise หรือข่าวกะทันหัน ไม่ควรใช้สัญญาณปกติตัดสินใจกับแท่งนี้ แนะนำรอแท่งถัดไปยืนยัน',
  },
};

/**
 * ตรวจจับ trend ของแท่งเทียนปัจจุบัน
 * เทียบ high + low กับแท่งก่อนหน้า 1-2 แท่ง
 * ยืนยันด้วย close + close position + Volume (optional) + Spike flag
 *
 * @param {Array} candles - array ของแท่งเทียน { open, high, low, close, volume?, is_atr? }
 *                           ใช้ output จาก engine.performAnalysis() ได้ตรง ๆ
 * @param {number} index  - ตำแหน่งของแท่งที่ต้องการวิเคราะห์
 * @param {Object} options
 * @param {number} [options.closePositionThreshold=0.5]   - ค่าตัดสิน close position สำหรับโซนบน/ล่าง
 * @param {number} [options.strongRejectionThreshold=0.3] - ค่าตัดสิน rejection ที่ชัดเจน
 * @param {number} [options.dojiBodyThreshold=0.15]       - bodyRatio ต่ำกว่านี้ = Doji
 * @param {number} [options.flatTolerance=0.05]           - % tolerance สำหรับ flat high/low
 * @returns {Object} ผลลัพธ์ตาม format ด้านล่าง
 */
function detectTrend(candles, index, options = {}) {
  const {
    closePositionThreshold = 0.5,
    strongRejectionThreshold = 0.3,
    dojiBodyThreshold = 0.15,
    flatTolerance = 0.05,
  } = options;

  // helper: ประกอบผลลัพธ์จาก key ของ registry (caseDesc) โดย lookup caseCode/trend/group/description พร้อมคำนวณ trendScore
  const buildResult = (caseDesc, extra = {}) => {
    const info = TREND_CASES[caseDesc] || {};
    const base = {
      trend: info.trend,
      caseCode: info.caseCode,
      caseDesc,
      group: info.group,
      description: info.description,
      ...extra,
    };
    const scoreObj = calculateTrendScore(candles, index, base);
    return {
      ...base,
      trendScore: scoreObj.score,
      trendStrength: scoreObj.strength,
      scoreBreakdown: scoreObj.breakdown,
    };
  };

  // ---------- Guard: ข้อมูลไม่พอ ----------
  if (index < 2) {
    return buildResult('INSUFFICIENT_DATA', {
      isSpike: false, extremeTrend: null, closePosition: null,
      bodyRatio: null, structure: null, rangeRatio: null,
      colorSequence: '', colorSwitches: 0, isWhipsaw: false,
      whipsawStatus: 'NO_DATA', whipsawWarning: 'ข้อมูลไม่เพียงพอ',
    });
  }

  const current = candles[index];
  const prev1   = candles[index - 1];
  const prev2   = candles[index - 2];

  // ---------- Spike flag ----------
  const isSpike = current.is_atr === true || current.isAtr === true;

  // =================================================================
  // V5 FIX #1 + #2: เปรียบเทียบทั้ง HIGH และ LOW อย่างครบถ้วน
  // =================================================================

  // --- HIGH comparisons ---
  const higherHighP1 = current.high > prev1.high;
  const higherHighP2 = current.high > prev2.high;
  const lowerHighP1  = current.high < prev1.high;
  const lowerHighP2  = current.high < prev2.high;

  // --- LOW comparisons (V5 NEW — V4 ไม่เคยเปรียบเทียบ low เลย) ---
  const lowerLowP1  = current.low < prev1.low;
  const lowerLowP2  = current.low < prev2.low;
  const higherLowP1 = current.low > prev1.low;
  const higherLowP2 = current.low > prev2.low;

  // --- CLOSE comparisons ---
  const closeHigherP1 = current.close > prev1.close;
  const closeLowerP1  = current.close < prev1.close;
  const closeEqualP1  = !closeHigherP1 && !closeLowerP1; // V5 FIX #10

  // --- Range / Close Position / Body Ratio ---
  const range = current.high - current.low;
  const closePosition = range > 0 ? (current.close - current.low) / range : 0.5;
  const bodyRatio     = range > 0 ? Math.abs(current.close - current.open) / range : 0;

  // --- Aggregate flags ---
  const madeHigherHigh = higherHighP1 || higherHighP2;
  const madeLowerLow   = lowerLowP1   || lowerLowP2;

  // --- Pattern flags ---
  const isInsideBar  = current.high <= prev1.high && current.low >= prev1.low;
  const isOutsideBar = current.high >  prev1.high && current.low <  prev1.low;

  // --- Flat detection (V5: เพิ่ม low) ---
  const highDiffRatio = prev1.high !== 0
    ? Math.abs(current.high - prev1.high) / prev1.high : 0;
  const isFlatHigh    = highDiffRatio <= flatTolerance && !higherHighP1;

  const lowDiffRatio = prev1.low !== 0
    ? Math.abs(current.low - prev1.low) / prev1.low : 0;
  const isFlatLow    = lowDiffRatio <= flatTolerance && !lowerLowP1;   // V5 NEW #7

  // --- Mixed signal ---
  const mixedSignalHigh = higherHighP1 !== higherHighP2;  // high ขัดแย้ง
  const mixedSignalLow  = lowerLowP1   !== lowerLowP2;    // low ขัดแย้ง (V5 NEW #6)

  // =================================================================
  // V5 FIX #3: extremeTrend สมดุล — ใช้ทั้ง high AND low
  // =================================================================
  let extremeTrend = 'Sideways';
  if (madeHigherHigh && madeLowerLow) {
    // Range expansion ทั้งสองด้าน → ให้ close position ตัดสิน
    extremeTrend = closePosition >= 0.5 ? 'UpTrend' : 'DownTrend';
  } else if (madeHigherHigh && !madeLowerLow) {
    extremeTrend = 'UpTrend';
  } else if (madeLowerLow && !madeHigherHigh) {
    extremeTrend = 'DownTrend';
  }

  // =================================================================
  // V5 FIX #4: Volume confirmation (optional)
  // =================================================================
  const hasVolume = typeof current.volume === 'number' && typeof prev1.volume === 'number';
  let volumeRatio    = null;
  let volumeConfirmed = null;
  if (hasVolume && prev1.volume > 0) {
    volumeRatio     = current.volume / prev1.volume;
    volumeConfirmed = volumeRatio > 1.0;
  }

  // =================================================================
  // V5 FIX #8: Relative candle size (rangeRatio)
  // =================================================================
  const prev1Range   = prev1.high - prev1.low;
  const prev2Range   = prev2.high - prev2.low;
  const avgPrevRange = (prev1Range + prev2Range) / 2;
  const rangeRatio   = avgPrevRange > 0 ? range / avgPrevRange : 1;

  // =================================================================
  // V5 FIX #9: Structure metadata
  // =================================================================
  const structure = {
    higherHigh: madeHigherHigh,
    lowerLow:   madeLowerLow,
    higherLow:  higherLowP1,
    lowerHigh:  lowerHighP1,
  };

  // =================================================================
  // V5 NEW: วิเคราะห์การสลับสีแท่งเทียนย้อนหลัง 3-4 แท่ง (Whipsaw Zone Analysis)
  // =================================================================
  const whipsawAnalysis = analyzeColorAlternation(candles, index);

  // ---------- ประกอบ extra สำหรับผลลัพธ์ ----------
  const extra = {
    isSpike,
    extremeTrend,
    closePosition:  Number(closePosition.toFixed(2)),
    bodyRatio:      Number(bodyRatio.toFixed(2)),
    structure,
    rangeRatio:     Number(rangeRatio.toFixed(2)),
    colorSequence:  whipsawAnalysis.colorSequence,
    colorSwitches:  whipsawAnalysis.colorSwitches,
    isWhipsaw:      whipsawAnalysis.isWhipsaw,
    whipsawStatus:  whipsawAnalysis.whipsawStatus,
    whipsawWarning: whipsawAnalysis.whipsawWarning,
  };
  // เพิ่ม volume fields เฉพาะเมื่อมีข้อมูล
  if (volumeRatio !== null) {
    extra.volumeRatio     = Number(volumeRatio.toFixed(2));
    extra.volumeConfirmed = volumeConfirmed;
  }

  // ╔═══════════════════════════════════════════════════════════╗
  // ║  Layer 1: SPIKE (is_atr === true)                        ║
  // ╚═══════════════════════════════════════════════════════════╝
  if (isSpike) {
    // --- 1a: Spike + Outside Bar ---
    if (isOutsideBar) {
      // Spike กลืนกรอบแท่งก่อนหน้า — จำแนกตาม close position
      if (closePosition > (1 - strongRejectionThreshold)) {
        // ปิดใกล้ high → ดันขึ้นจากล่าง = Bear Trap
        return buildResult('SPIKE_BEAR_TRAP', extra);
      }
      if (closePosition < strongRejectionThreshold) {
        // ปิดใกล้ low → กดลงจากบน = Bull Trap
        return buildResult('SPIKE_BULL_TRAP', extra);
      }
      if (closePosition >= closePositionThreshold) {
        return buildResult('SPIKE_CONTINUATION_UP', extra);
      }
      if (closePosition <= (1 - closePositionThreshold)) {
        return buildResult('SPIKE_CONTINUATION_DOWN', extra);
      }
      return buildResult('SPIKE_WHIPSAW', extra);
    }

    // --- 1b: Spike + Higher High only (ไม่ Lower Low) ---
    if (madeHigherHigh && !madeLowerLow) {
      if (closePosition < strongRejectionThreshold) {
        return buildResult('SPIKE_BULL_TRAP', extra);
      }
      if (closePosition >= closePositionThreshold && (closeHigherP1 || closeEqualP1)) {
        return buildResult('SPIKE_CONTINUATION_UP', extra);
      }
      // V5 FIX #5: เติม gap case — ก่อนหน้า case นี้ตกไป NO_CLEAR_DIRECTION
      return buildResult('SPIKE_HESITANT_UP', extra);
    }

    // --- 1c: Spike + Lower Low only (ไม่ Higher High) ---
    if (madeLowerLow && !madeHigherHigh) {
      if (closePosition > (1 - strongRejectionThreshold)) {
        return buildResult('SPIKE_BEAR_TRAP', extra);
      }
      if (closePosition <= (1 - closePositionThreshold) && (closeLowerP1 || closeEqualP1)) {
        return buildResult('SPIKE_CONTINUATION_DOWN', extra);
      }
      // V5 FIX #5: เติม gap case
      return buildResult('SPIKE_HESITANT_DOWN', extra);
    }

    // --- 1d: Spike ที่ไม่ทำ Higher High หรือ Lower Low ---
    return buildResult('SPIKE_NO_CLEAR_DIRECTION', extra);
  }

  // ╔═══════════════════════════════════════════════════════════╗
  // ║  Layer 2: ENGULFING / OUTSIDE BAR (V5 NEW #11)           ║
  // ╚═══════════════════════════════════════════════════════════╝
  if (isOutsideBar) {
    if (closePosition >= closePositionThreshold) {
      return buildResult('BULLISH_ENGULFING', extra);
    }
    if (closePosition <= (1 - closePositionThreshold)) {
      return buildResult('BEARISH_ENGULFING', extra);
    }
    return buildResult('ENGULFING_INDECISION', extra);
  }

  // ╔═══════════════════════════════════════════════════════════╗
  // ║  Layer 3: แท่งปกติ — UpTrend                             ║
  // ╚═══════════════════════════════════════════════════════════╝
  if (extremeTrend === 'UpTrend') {
    // V5 FIX #10: closeEqualP1 ก็นับเป็น "ไม่แย่" — ยืนยันได้ถ้า position ดี
    if ((closeHigherP1 || closeEqualP1) && closePosition >= closePositionThreshold) {
      return buildResult('CONFIRMED_UP', extra);
    }
    if (!closeHigherP1 && !closeEqualP1 && closePosition < strongRejectionThreshold) {
      return buildResult('STRONG_BULL_TRAP', extra);
    }
    if (closeHigherP1 && closePosition < closePositionThreshold) {
      return buildResult('WEAK_UPPER_WICK', extra);
    }
    if (mixedSignalHigh && bodyRatio < dojiBodyThreshold) {
      return buildResult('MIXED_SIGNAL', extra);
    }
    return buildResult('FAILED_FOLLOWTHROUGH_UP', extra);
  }

  // ╔═══════════════════════════════════════════════════════════╗
  // ║  Layer 4: แท่งปกติ — DownTrend                           ║
  // ╚═══════════════════════════════════════════════════════════╝
  if (extremeTrend === 'DownTrend') {
    // V5 FIX #10: closeEqualP1 ก็นับเป็น "ไม่แย่" — ยืนยันได้ถ้า position ดี
    if ((closeLowerP1 || closeEqualP1) && closePosition <= (1 - closePositionThreshold)) {
      return buildResult('CONFIRMED_DOWN', extra);
    }
    if (!closeLowerP1 && !closeEqualP1 && closePosition > (1 - strongRejectionThreshold)) {
      return buildResult('STRONG_BEAR_TRAP', extra);
    }
    if (closeLowerP1 && closePosition > (1 - closePositionThreshold)) {
      return buildResult('WEAK_LOWER_WICK', extra);
    }
    // V5 FIX #6: เพิ่ม MIXED_SIGNAL_DOWN — symmetric กับ UpTrend branch
    if (mixedSignalLow && bodyRatio < dojiBodyThreshold) {
      return buildResult('MIXED_SIGNAL_DOWN', extra);
    }
    return buildResult('FAILED_FOLLOWTHROUGH_DOWN', extra);
  }

  // ╔═══════════════════════════════════════════════════════════╗
  // ║  Layer 5: Sideways (extremeTrend === 'Sideways')         ║
  // ╚═══════════════════════════════════════════════════════════╝
  if (isInsideBar) return buildResult('INSIDE_BAR', extra);
  if (bodyRatio < dojiBodyThreshold) return buildResult('DOJI_INDECISION', extra);
  if (isFlatHigh) return buildResult('FLAT_RESISTANCE_TEST', extra);
  if (isFlatLow)  return buildResult('FLAT_SUPPORT_TEST', extra);   // V5 NEW #7
  return buildResult('NO_CLEAR_STRUCTURE', extra);
}

/**
 * วิเคราะห์การสลับสีของแท่งเทียนย้อนหลัง 3-4 แท่ง เพื่อตรวจจับ Whipsaw Zone
 * @param {Array} candles - array แท่งเทียน
 * @param {number} index - index แท่งปัจจุบัน
 * @returns {Object} { colorSequence, colorSwitches, isWhipsaw, whipsawStatus, whipsawWarning }
 */
function analyzeColorAlternation(candles, index) {
  if (!candles || index < 1) {
    return {
      colorSequence: '',
      colorSwitches: 0,
      isWhipsaw: false,
      whipsawStatus: 'NO_DATA',
      whipsawWarning: 'ข้อมูลไม่เพียงพอ'
    };
  }

  // ระบุสีของแท่งเทียน: 'G' (Green / Bullish), 'R' (Red / Bearish), 'D' (Doji)
  function getCandleColor(c) {
    if (!c) return 'D';
    if (c.close > c.open) return 'G';
    if (c.close < c.open) return 'R';
    return 'D';
  }

  // ดูย้อนหลังสูงสุด 4 แท่ง (index-3 ถึง index)
  const lookback = Math.min(4, index + 1);
  const startIdx = index - lookback + 1;
  const colors = [];

  for (let i = startIdx; i <= index; i++) {
    colors.push(getCandleColor(candles[i]));
  }

  // นับจำนวนครั้งที่มีการสลับสี
  let switches = 0;
  for (let i = 1; i < colors.length; i++) {
    const prev = colors[i - 1];
    const curr = colors[i];
    if (prev !== curr) {
      switches++;
    }
  }

  const seqStr = colors.join('-');
  let whipsawStatus = 'TRENDING';
  let isWhipsaw = false;
  let whipsawWarning = `✓ โครงสร้างสีต่อเนื่อง (${seqStr})`;

  if (colors.length >= 4) {
    if (switches === 3) {
      // สลับสี 4 แท่งติด: G-R-G-R หรือ R-G-R-G
      whipsawStatus = 'CONFIRMED_WHIPSAW';
      isWhipsaw = true;
      whipsawWarning = `⚠️ สลับสี 4 แท่งติด (${seqStr}) อยู่ใน Whipsaw Zone รุนแรง (เสี่ยงโดนหลอกฟันปลาสูง)`;
    } else if (switches === 2) {
      whipsawStatus = 'CHOPPY';
      isWhipsaw = false;
      whipsawWarning = `〰️ ตลาดเริ่มมีความผันผวนสลับสี (${seqStr})`;
    }
  } else if (colors.length === 3) {
    if (switches === 2) {
      // สลับสี 3 แท่งติด: G-R-G หรือ R-G-R
      whipsawStatus = 'ENTERING_WHIPSAW';
      isWhipsaw = true;
      whipsawWarning = `⚡ สลับสี 3 แท่งติด (${seqStr}) กำลังเข้าสู่ Whipsaw Zone (ควรระวังการกลับตัวหลอก)`;
    }
  }

  // ตรวจสอบกรณี 4 แท่ง แต่ 3 แท่งท้ายสุดสลับสีติดกัน (เช่น G-G-R-G หรือ R-R-G-R)
  if (colors.length === 4 && switches >= 2) {
    const last3Switches = (colors[1] !== colors[2] ? 1 : 0) + (colors[2] !== colors[3] ? 1 : 0);
    if (last3Switches === 2 && whipsawStatus !== 'CONFIRMED_WHIPSAW') {
      whipsawStatus = 'ENTERING_WHIPSAW';
      isWhipsaw = true;
      whipsawWarning = `⚡ สลับสี 3 แท่งล่าสุด (${colors.slice(1).join('-')}) กำลังเข้าสู่ Whipsaw Zone`;
    }
  }

  return {
    colorSequence: seqStr,
    colorSwitches: switches,
    isWhipsaw,
    whipsawStatus,
    whipsawWarning
  };
}

/**
 * คำนวณ Strong Trend Score (-100 ถึง +100) และจำแนกความแข็งแกร่ง (Trend Strength)
 * @param {Array} candles - array แท่งเทียน
 * @param {number} index - index แท่งปัจจุบัน
 * @param {Object} baseResult - ผลลัพธ์เบื้องต้นจาก detectTrend
 * @returns {Object} { score: number, strength: string, breakdown: Object }
 */
function calculateTrendScore(candles, index, baseResult) {
  if (!baseResult || baseResult.caseCode === 'SYS-NODATA' || index < 2) {
    return {
      score: 0,
      strength: 'NO_DATA',
      breakdown: {
        basePattern: 0,
        closeConviction: 0,
        bodyConviction: 0,
        structure: 0,
        rangeExpansion: 0,
        trendStreak: 0,
        whipsawPenalty: 0
      }
    };
  }

  const { caseCode, group, trend, closePosition = 0.5, bodyRatio = 0, structure = {}, rangeRatio = 1, whipsawStatus = 'TRENDING' } = baseResult;
  let basePattern = 0;
  let closeConviction = 0;
  let bodyConviction = 0;
  let structureScore = 0;
  let rangeExpansion = 0;
  let trendStreak = 0;
  let whipsawPenalty = 0;

  // 1. Base Pattern Score (Max ±35)
  switch (caseCode) {
    case 'SPK-CONTINUE-UP': basePattern = 35; break;
    case 'SPK-CONTINUE-DN': basePattern = -35; break;
    case 'UP-CONFIRM':      basePattern = 30; break;
    case 'DN-CONFIRM':      basePattern = -30; break;
    case 'EG-BULLISH':      basePattern = 25; break;
    case 'EG-BEARISH':      basePattern = -25; break;
    case 'SPK-HESITANT-UP': basePattern = 10; break;
    case 'SPK-HESITANT-DN': basePattern = -10; break;
    case 'RJ-BULLTRAP-STRONG':
    case 'SPK-BULLTRAP':    basePattern = -25; break; // กับดักขาขึ้น -> เสี่ยงลงแรง
    case 'RJ-BEARTRAP-STRONG':
    case 'SPK-BEARTRAP':    basePattern = 25; break;  // กับดักขาลง -> เสี่ยงขึ้นแรง
    case 'RJ-UPWICK-WEAK':
    case 'RJ-FOLLOWFAIL-UP': basePattern = -10; break;
    case 'RJ-LOWWICK-WEAK':
    case 'RJ-FOLLOWFAIL-DN': basePattern = 10; break;
    default: basePattern = 0; break;
  }

  const isUp = basePattern > 0 || trend === 'UpTrend' || group === 'Up';
  const isDn = basePattern < 0 || trend === 'DownTrend' || group === 'Down';

  // 2. Close Conviction (Max ±15)
  const cp = typeof closePosition === 'number' ? closePosition : 0.5;
  if (isUp) {
    if (cp >= 0.85) closeConviction = 15;
    else if (cp >= 0.65) closeConviction = 8;
    else if (cp < 0.40) closeConviction = -10;
  } else if (isDn) {
    if (cp <= 0.15) closeConviction = -15;
    else if (cp <= 0.35) closeConviction = -8;
    else if (cp > 0.60) closeConviction = 10;
  }

  // 3. Body Conviction (Max ±10)
  const br = typeof bodyRatio === 'number' ? bodyRatio : 0;
  if (br >= 0.70) {
    if (isUp) bodyConviction = 10;
    else if (isDn) bodyConviction = -10;
  } else if (br >= 0.50) {
    if (isUp) bodyConviction = 5;
    else if (isDn) bodyConviction = -5;
  }

  // 4. Structure Score (Max ±10)
  const st = structure || {};
  if (st.higherHigh && st.higherLow) structureScore += 10;
  else if (st.higherHigh && !st.lowerLow) structureScore += 5;

  if (st.lowerLow && st.lowerHigh) structureScore -= 10;
  else if (st.lowerLow && !st.higherHigh) structureScore -= 5;

  // 5. Range Expansion (Max ±10)
  const rr = typeof rangeRatio === 'number' ? rangeRatio : 1;
  const tentativeScore = basePattern + closeConviction + bodyConviction + structureScore;
  if (rr >= 1.3) {
    if (tentativeScore > 0) rangeExpansion = 10;
    else if (tentativeScore < 0) rangeExpansion = -10;
  } else if (rr >= 1.1) {
    if (tentativeScore > 0) rangeExpansion = 5;
    else if (tentativeScore < 0) rangeExpansion = -5;
  }

  // 6. Trend Streak (Max ±20)
  if (index >= 3 && candles && candles.length > index) {
    let upStreak = 0;
    let dnStreak = 0;
    for (let i = 1; i <= 3; i++) {
      const prevC = candles[index - i];
      const prev2C = index - i - 1 >= 0 ? candles[index - i - 1] : null;
      if (prevC && prev2C) {
        if (prevC.close > prev2C.close && prevC.high >= prev2C.high) upStreak++;
        if (prevC.close < prev2C.close && prevC.low <= prev2C.low) dnStreak++;
      }
    }
    if (upStreak === 3 && tentativeScore > 0) trendStreak = 20;
    else if (upStreak >= 2 && tentativeScore > 0) trendStreak = 10;

    if (dnStreak === 3 && tentativeScore < 0) trendStreak = -20;
    else if (dnStreak >= 2 && tentativeScore < 0) trendStreak = -10;
  }

  // 7. Whipsaw Penalty (หักคะแนนหากพบการสลับสีฟันปลา)
  if (whipsawStatus === 'CONFIRMED_WHIPSAW') {
    if (tentativeScore > 0) whipsawPenalty = -25;
    else if (tentativeScore < 0) whipsawPenalty = 25;
  } else if (whipsawStatus === 'ENTERING_WHIPSAW') {
    if (tentativeScore > 0) whipsawPenalty = -12;
    else if (tentativeScore < 0) whipsawPenalty = 12;
  }

  const rawScore = basePattern + closeConviction + bodyConviction + structureScore + rangeExpansion + trendStreak + whipsawPenalty;
  const score = Math.max(-100, Math.min(100, Math.round(rawScore)));

  // Classification
  let strength = 'NEUTRAL';
  if (whipsawStatus === 'CONFIRMED_WHIPSAW') {
    strength = 'WHIPSAW_ZONE';
  } else if (score >= 70) strength = 'ULTRA_STRONG_UP';
  else if (score >= 40) strength = 'STRONG_UP';
  else if (score >= 20) strength = 'MILD_UP';
  else if (score <= -70) strength = 'ULTRA_STRONG_DOWN';
  else if (score <= -40) strength = 'STRONG_DOWN';
  else if (score <= -20) strength = 'MILD_DOWN';

  return {
    score,
    strength,
    breakdown: {
      basePattern,
      closeConviction,
      bodyConviction,
      structure: structureScore,
      rangeExpansion,
      trendStreak,
      whipsawPenalty
    }
  };
}

/**
 * วิเคราะห์ trend ทุกแท่งใน array
 * @param {Array} candles
 * @param {Object} options
 * @returns {Array}
 */
function detectTrendSeries(candles, options = {}) {
  return candles.map((_, i) => detectTrend(candles, i, options));
}

// Export
if (typeof module !== 'undefined' && module.exports) {
  module.exports = { detectTrend, detectTrendSeries, calculateTrendScore, analyzeColorAlternation, TREND_CASES };
}
if (typeof window !== 'undefined') {
  window.TrendDetector = { detectTrend, detectTrendSeries, calculateTrendScore, analyzeColorAlternation, TREND_CASES };
}

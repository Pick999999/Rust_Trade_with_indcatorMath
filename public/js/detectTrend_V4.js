/**
 * ===============================================
 * TREND_CASES: Registry รวมทุก Case ที่ตรวจจับได้
 * ===============================================
 * key ของ object = caseDesc (ชื่อเต็มอ่านง่าย ใช้เป็น identifier หลักในโค้ด)
 * แต่ละ Case มี:
 * - caseCode: รหัสสั้น สื่อความหมาย รูปแบบ <GROUP>-<MEANING> เช่น 'UP-CONFIRM', 'SPK-BEARTRAP'
 * - trend: หมวดใหญ่ ('UpTrend' | 'DownTrend' | 'Rejected' | 'Sideways' | null)
 * - group: กลุ่มของ case ('Up' | 'Down' | 'Rejected' | 'Sideways' | 'Spike' | 'System')
 * - description: คำอธิบายภาษาไทย
 */
const TREND_CASES = {
  // ---------- System ----------
  INSUFFICIENT_DATA: {
    caseCode: 'SYS-NODATA',
    trend: null, group: 'System',
    description: 'ข้อมูลไม่พอสำหรับเปรียบเทียบ ต้องมีอย่างน้อย 3 แท่ง (ปัจจุบัน + ก่อนหน้า 2 แท่ง)',
  },

  // ---------- Up / Down ปกติ (ยืนยันแล้ว) ----------
  CONFIRMED_UP: {
    caseCode: 'UP-CONFIRM',
    trend: 'UpTrend', group: 'Up',
    description: 'ทำจุดสูงใหม่ ปิดสูงกว่าแท่งก่อนหน้า และปิดในโซนบนของแท่งตัวเอง ยืนยันแรงซื้อยังควบคุมตลาด',
  },
  CONFIRMED_DOWN: {
    caseCode: 'DN-CONFIRM',
    trend: 'DownTrend', group: 'Down',
    description: 'ทำจุดต่ำใหม่ ปิดต่ำกว่าแท่งก่อนหน้า และปิดในโซนล่างของแท่งตัวเอง ยืนยันแรงขายยังควบคุมตลาด',
  },

  // ---------- Rejected (แท่งปกติ) ----------
  STRONG_BULL_TRAP: {
    caseCode: 'RJ-BULLTRAP-STRONG',
    trend: 'Rejected', group: 'Rejected',
    description: 'ทำจุดสูงใหม่ แต่ถูกแรงขายกดลงมาปิดใกล้จุดต่ำสุดของแท่ง และปิดต่ำกว่าแท่งก่อนหน้าด้วย สัญญาณกลับตัวลงที่ชัดเจน (Bull Trap)',
  },
  WEAK_UPPER_WICK: {
    caseCode: 'RJ-UPWICK-WEAK',
    trend: 'Rejected', group: 'Rejected',
    description: 'ทำจุดสูงใหม่ และยังปิดสูงกว่าแท่งก่อนหน้า แต่มีไส้บนยาวแสดงว่าเจอแรงขายกดในช่วงท้ายแท่ง ควรระวังแรงซื้อเริ่มอ่อนกำลัง',
  },
  FAILED_FOLLOWTHROUGH_UP: {
    caseCode: 'RJ-FOLLOWFAIL-UP',
    trend: 'Rejected', group: 'Rejected',
    description: 'ทำจุดสูงใหม่ แต่ปิดต่ำกว่าแท่งก่อนหน้า แม้ตำแหน่งปิดในแท่งตัวเองจะยังไม่แย่มาก แสดงว่าโมเมนตัมขาขึ้นเริ่มแผ่ว ควรจับตาแท่งถัดไป',
  },
  STRONG_BEAR_TRAP: {
    caseCode: 'RJ-BEARTRAP-STRONG',
    trend: 'Rejected', group: 'Rejected',
    description: 'ทำจุดต่ำใหม่ แต่ถูกแรงซื้อดันขึ้นมาปิดใกล้จุดสูงสุดของแท่ง และปิดสูงกว่าแท่งก่อนหน้าด้วย สัญญาณกลับตัวขึ้นที่ชัดเจน (Bear Trap)',
  },
  WEAK_LOWER_WICK: {
    caseCode: 'RJ-LOWWICK-WEAK',
    trend: 'Rejected', group: 'Rejected',
    description: 'ทำจุดต่ำใหม่ และยังปิดต่ำกว่าแท่งก่อนหน้า แต่มีไส้ล่างยาวแสดงว่าเจอแรงซื้อดันกลับในช่วงท้ายแท่ง ควรระวังแรงขายเริ่มอ่อนกำลัง',
  },
  FAILED_FOLLOWTHROUGH_DOWN: {
    caseCode: 'RJ-FOLLOWFAIL-DN',
    trend: 'Rejected', group: 'Rejected',
    description: 'ทำจุดต่ำใหม่ แต่ปิดสูงกว่าแท่งก่อนหน้า แม้ตำแหน่งปิดในแท่งตัวเองจะยังไม่แย่มาก แสดงว่าโมเมนตัมขาลงเริ่มแผ่ว ควรจับตาแท่งถัดไป',
  },

  // ---------- Sideways (แท่งปกติ) ----------
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
  FLAT_RESISTANCE_TEST: {
    caseCode: 'SD-FLATRESISTANCE',
    trend: 'Sideways', group: 'Sideways',
    description: 'High ของแท่งใกล้เคียงกับ high ของแท่งก่อนหน้ามาก แต่ไปต่อไม่ได้ เหมือนราคาชนแนวต้านเดิมซ้ำ (ทดสอบแนวต้าน) ยังไม่มีแรงมากพอจะทะลุ',
  },
  NO_CLEAR_STRUCTURE: {
    caseCode: 'SD-NOSTRUCTURE',
    trend: 'Sideways', group: 'Sideways',
    description: 'High ของแท่งไม่ได้ทำจุดสูงใหม่หรือจุดต่ำใหม่ชัดเจนเมื่อเทียบกับ 2 แท่งก่อนหน้า ตลาดยังไม่มีโครงสร้างที่ชัดเจนพอจะสรุปทิศทาง',
  },

  // ---------- Spike (อ้างอิงจาก field is_atr ของ FullAnalysisEngine.js) ----------
  SPIKE_BULL_TRAP: {
    caseCode: 'SPK-BULLTRAP',
    trend: 'Rejected', group: 'Spike',
    description: 'แท่งนี้เป็น Spike ที่พุ่งขึ้นแรงทำจุดสูงใหม่ แต่ถูกกดลงมาปิดใกล้จุดต่ำสุดของแท่ง คล้ายการล่าสภาพคล่องฝั่งซื้อ (Stop Hunt / Liquidity Grab) ความเสี่ยงกลับตัวลงสูงมาก ต้องระวังเป็นพิเศษ',
  },
  SPIKE_BEAR_TRAP: {
    caseCode: 'SPK-BEARTRAP',
    trend: 'Rejected', group: 'Spike',
    description: 'แท่งนี้เป็น Spike ที่ทิ่มลงแรงทำจุดต่ำใหม่ แต่ถูกดันกลับขึ้นมาปิดใกล้จุดสูงสุดของแท่ง คล้ายการล่าสภาพคล่องฝั่งขาย (Stop Hunt / Liquidity Grab) ความเสี่ยงกลับตัวขึ้นสูงมาก ต้องระวังเป็นพิเศษ',
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
  SPIKE_WHIPSAW: {
    caseCode: 'SPK-WHIPSAW',
    trend: 'Sideways', group: 'Spike',
    description: 'แท่งนี้เป็น Spike ที่ range กว้างผิดปกติ (เกิน ATR) และแกว่งครอบคลุมทั้งจุดสูงและจุดต่ำของแท่งก่อนหน้า (Outside Bar) แต่ปิดใกล้กลางแท่ง แสดงถึงความผันผวนรุนแรงแบบไร้ทิศทาง อาจเป็นจุดพลิกผันของตลาด (Exhaustion) ควรรอสัญญาณยืนยันก่อนเข้าเทรด',
  },
  SPIKE_NO_CLEAR_DIRECTION: {
    caseCode: 'SPK-NODIRECTION',
    trend: 'Sideways', group: 'Spike',
    description: 'แท่งนี้เป็น Spike (range กว้างผิดปกติเกิน ATR) แต่ไม่ได้ทำจุดสูง/ต่ำใหม่ชัดเจน และไม่ได้แกว่งครอบคลุมสองด้าน อาจเกิดจาก noise หรือข่าวกะทันหัน ไม่ควรใช้สัญญาณปกติตัดสินใจกับแท่งนี้ แนะนำรอแท่งถัดไปยืนยัน',
  },
};

/**
 * ตรวจจับ trend ของแท่งเทียนปัจจุบัน โดยเทียบ high กับแท่งก่อนหน้า 1-2 แท่ง
 * ยืนยันด้วย close + close position + ตรวจ Spike (จาก field is_atr ของ FullAnalysisEngine.js)
 *
 * @param {Array} candles - array ของแท่งเทียน { open, high, low, close, is_atr? }
 *                           ใช้ output จาก engine.performAnalysis() ได้ตรง ๆ
 * @param {number} index
 * @param {Object} options
 * @returns {Object} { trend, caseCode, caseDesc, description, group, isSpike, extremeTrend, closePosition, bodyRatio }
 */
function detectTrend(candles, index, options = {}) {
  const {
    closePositionThreshold = 0.5,
    strongRejectionThreshold = 0.3,
    dojiBodyThreshold = 0.15,
    flatTolerance = 0.05,
  } = options;

  // helper: ประกอบผลลัพธ์จาก key ของ registry (caseDesc) โดย lookup caseCode/trend/group/description
  const buildResult = (caseDesc, extra = {}) => {
    const info = TREND_CASES[caseDesc];
    return {
      trend: info.trend,
      caseCode: info.caseCode,   // รหัสสั้น สื่อความหมาย เช่น 'UP-CONFIRM'
      caseDesc: caseDesc,        // ชื่อ key เต็ม เช่น 'CONFIRMED_UP'
      group: info.group,
      description: info.description,
      ...extra,
    };
  };

  if (index < 2) {
    return buildResult('INSUFFICIENT_DATA', {
      isSpike: false, extremeTrend: null, closePosition: null, bodyRatio: null,
    });
  }

  const current = candles[index];
  const prev1 = candles[index - 1];
  const prev2 = candles[index - 2];

  const isSpike = current.is_atr === true || current.isAtr === true;

  const higherThanPrev1 = current.high > prev1.high;
  const higherThanPrev2 = current.high > prev2.high;
  const lowerThanPrev1 = current.high < prev1.high;
  const lowerThanPrev2 = current.high < prev2.high;

  const closeHigherThanPrev1 = current.close > prev1.close;
  const closeLowerThanPrev1 = current.close < prev1.close;

  const range = current.high - current.low;
  const closePosition = range > 0 ? (current.close - current.low) / range : 0.5;
  const bodyRatio = range > 0 ? Math.abs(current.close - current.open) / range : 0;

  const isInsideBar = current.high <= prev1.high && current.low >= prev1.low;
  const highDiffRatio = prev1.high !== 0 ? Math.abs(current.high - prev1.high) / prev1.high : 0;
  const isFlatHigh = highDiffRatio <= flatTolerance && !higherThanPrev1;
  const mixedSignal = higherThanPrev1 !== higherThanPrev2;
  const isEngulfingRange = current.high > prev1.high && current.low < prev1.low;

  let extremeTrend = 'Sideways';
  if (higherThanPrev1 || higherThanPrev2) extremeTrend = 'UpTrend';
  else if (lowerThanPrev1 && lowerThanPrev2) extremeTrend = 'DownTrend';

  const extra = {
    isSpike,
    extremeTrend,
    closePosition: Number(closePosition.toFixed(2)),
    bodyRatio: Number(bodyRatio.toFixed(2)),
  };

  // ===== ชั้นที่ 1: Spike =====
  if (isSpike) {
    if (isEngulfingRange && closePosition >= 0.4 && closePosition <= 0.6) {
      return buildResult('SPIKE_WHIPSAW', extra);
    }
    if (extremeTrend === 'UpTrend' && closePosition < strongRejectionThreshold) {
      return buildResult('SPIKE_BULL_TRAP', extra);
    }
    if (extremeTrend === 'UpTrend' && closePosition >= closePositionThreshold && closeHigherThanPrev1) {
      return buildResult('SPIKE_CONTINUATION_UP', extra);
    }
    if (extremeTrend === 'DownTrend' && closePosition > (1 - strongRejectionThreshold)) {
      return buildResult('SPIKE_BEAR_TRAP', extra);
    }
    if (extremeTrend === 'DownTrend' && closePosition <= (1 - closePositionThreshold) && closeLowerThanPrev1) {
      return buildResult('SPIKE_CONTINUATION_DOWN', extra);
    }
    return buildResult('SPIKE_NO_CLEAR_DIRECTION', extra);
  }

  // ===== ชั้นที่ 2: แท่งปกติ =====
  if (extremeTrend === 'UpTrend') {
    if (closeHigherThanPrev1 && closePosition >= closePositionThreshold) {
      return buildResult('CONFIRMED_UP', extra);
    }
    if (!closeHigherThanPrev1 && closePosition < strongRejectionThreshold) {
      return buildResult('STRONG_BULL_TRAP', extra);
    }
    if (closeHigherThanPrev1 && closePosition < closePositionThreshold) {
      return buildResult('WEAK_UPPER_WICK', extra);
    }
    if (mixedSignal && bodyRatio < dojiBodyThreshold) {
      return buildResult('MIXED_SIGNAL', extra);
    }
    return buildResult('FAILED_FOLLOWTHROUGH_UP', extra);
  }

  if (extremeTrend === 'DownTrend') {
    if (closeLowerThanPrev1 && closePosition <= (1 - closePositionThreshold)) {
      return buildResult('CONFIRMED_DOWN', extra);
    }
    if (!closeLowerThanPrev1 && closePosition > (1 - strongRejectionThreshold)) {
      return buildResult('STRONG_BEAR_TRAP', extra);
    }
    if (closeLowerThanPrev1 && closePosition > (1 - closePositionThreshold)) {
      return buildResult('WEAK_LOWER_WICK', extra);
    }
    return buildResult('FAILED_FOLLOWTHROUGH_DOWN', extra);
  }

  // extremeTrend === 'Sideways'
  if (isInsideBar) return buildResult('INSIDE_BAR', extra);
  if (bodyRatio < dojiBodyThreshold) return buildResult('DOJI_INDECISION', extra);
  if (isFlatHigh) return buildResult('FLAT_RESISTANCE_TEST', extra);
  return buildResult('NO_CLEAR_STRUCTURE', extra);
}

function detectTrendSeries(candles, options = {}) {
  return candles.map((_, i) => detectTrend(candles, i, options));
}

// Export
if (typeof module !== 'undefined' && module.exports) {
  module.exports = { detectTrend, detectTrendSeries, TREND_CASES };
}
if (typeof window !== 'undefined') {
  window.TrendDetector = { detectTrend, detectTrendSeries, TREND_CASES };
}
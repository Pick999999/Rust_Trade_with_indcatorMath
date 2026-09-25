/**
 * detectAlternatingCandles.js
 * ────────────────────────────────────────────────────
 * ตรวจจับคู่แท่งเทียนที่สลับสี (เขียว-แดง / แดง-เขียว)
 * และมีขนาด body ใกล้เคียงกัน (BodyDiff < ATR(3) × multiplier)
 * ────────────────────────────────────────────────────
 */

/**
 * คำนวณ ATR แบบ rolling (ใช้แท่งล่าสุด n แท่ง)
 * @param {Array} candles - candle data array
 * @param {number} endIndex - index สุดท้ายที่จะคำนวณ (inclusive)
 * @param {number} period - จำนวนแท่งย้อนหลัง (default 3)
 * @returns {number} ATR value
 */
function calculateATR(candles, endIndex, period = 3) {
  if (endIndex < 1) return candles[0] ? parseFloat(candles[0].high) - parseFloat(candles[0].low) : 0;

  const start = Math.max(0, endIndex - period + 1);
  let sum = 0;
  let count = 0;

  for (let i = start; i <= endIndex; i++) {
    const high = parseFloat(candles[i].high);
    const low = parseFloat(candles[i].low);
    if (i === 0) {
      sum += high - low;
    } else {
      const prevClose = parseFloat(candles[i - 1].close);
      sum += Math.max(
        high - low,
        Math.abs(high - prevClose),
        Math.abs(low - prevClose)
      );
    }
    count++;
  }

  return count > 0 ? sum / count : 0;
}

/**
 * ตรวจจับคู่แท่งเทียนที่สลับสีและมี body ใกล้เคียงกัน
 *
 * @param {Array} candles - array ของ candle objects ({open, high, low, close, epoch})
 * @param {number} [atrMultiplier=0.5] - ตัวคูณ ATR สำหรับเปรียบเทียบ BodyDiff
 * @param {number} [sequenceLength=2] - จำนวนแท่งเทียนในชุดที่ต้องการหา (เช่น 2, 3, 4)
 * @returns {Array} array ของ index ที่ตรวจพบ (index ของแท่งสุดท้ายในชุด)
 */
function detectAlternatingSimularCandles(candles, atrMultiplier = 0.5, sequenceLength = 2) {
  const results = [];

  if (sequenceLength < 2) return results;

  for (let i = sequenceLength - 1; i < candles.length; i++) {
    let isValidSequence = true;

    for (let j = 0; j < sequenceLength - 1; j++) {
      const currIdx = i - j;
      const prevIdx = i - j - 1;

      const cCurr = candles[currIdx];
      const cPrev = candles[prevIdx];

      const prevOpen = parseFloat(cPrev.open);
      const prevClose = parseFloat(cPrev.close);
      const currOpen = parseFloat(cCurr.open);
      const currClose = parseFloat(cCurr.close);

      const prevBullish = prevClose > prevOpen;
      const currBullish = currClose > currOpen;
      const colorAlternates = prevBullish !== currBullish;

      const bodyPrev = Math.abs(prevClose - prevOpen);
      const bodyCurr = Math.abs(currClose - currOpen);
      const atr = calculateATR(candles, currIdx, 3);

      const sizeSimilar = Math.abs(bodyPrev - bodyCurr) < atr * atrMultiplier;

      if (!colorAlternates || !sizeSimilar) {
        isValidSequence = false;
        break;
      }
    }

    if (isValidSequence) {
      results.push(i);
    }
  }

  return results;
}

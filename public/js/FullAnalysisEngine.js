/**
 * FullAnalysisEngine.js
 * ====================
 * 
 * 📊 **ที่มา (Source):**
 * แปลงมาจาก src/full_analysis_ver2.rs ในโปรเจค Rust turbo-indicators
 * เป็น Pure JavaScript Class เพื่อใช้งานในโปรเจคอื่นๆ โดยไม่ต้องพึ่ง Backend
 * 
 * 🎯 **วัตถุประสงค์:**
 * วิเคราะห์ข้อมูลแท่งเทียน (Candlestick) และคำนวณ Technical Indicators แบบครบวงจร
 * รองรับการคำนวณ EMA, MACD, RSI, ADX, ATR, Bollinger Bands, Choppiness Index, 
 * Range Detector, SMC Analysis, และ Alternating Pattern Detection
 * 
 * 📦 **วิธีนำไปใช้งาน (Usage):**
 * 
 * ```javascript
 * // 1. สร้าง Instance
 * const engine = new FullAnalysisEngine();
 * 
 * // 2. เตรียมข้อมูลแท่งเทียน
 * const candles = [
 *   { epoch: 1704067200, open: 100.5, high: 101.2, low: 100.1, close: 100.8 },
 *   { epoch: 1704067260, open: 100.8, high: 101.5, low: 100.6, close: 101.3 },
 *   // ... เพิ่มแท่งเทียนต่อไป
 * ];
 * 
 * // 3. กำหนดค่า Config (Optional - ถ้าไม่ใส่จะใช้ค่า default)
 * const config = {
 *   ema: {
 *     short: { period: 9, type: 'ema' },
 *     medium: { period: 21, type: 'ema' },
 *     long: { period: 50, type: 'ema' }
 *   },
 *   indicators: {
 *     adxPeriod: 14,
 *     atrPeriod: 7,
 *     atrMulti: 1.3,
 *     bbPeriod: 20,
 *     ciPeriod: 14,
 *     smcPeriod: 50
 *   }
 * };
 * 
 * // 4. เรียกใช้งานวิเคราะห์
 * const results = engine.performAnalysis(candles, config, 'R_10');
 * 
 * // 5. ใช้ผลลัพธ์
 * results.forEach(candle => {
 *   console.log(`Time: ${candle.candletime_display}`);
 *   console.log(`Close: ${candle.close}`);
 *   console.log(`EMA Short: ${candle.ema_short_value}`);
 *   console.log(`EMA Cross: ${candle.ema_cut_all_type}`);
 *   console.log(`RSI: ${candle.rsi_value}`);
 *   console.log(`ADX: ${candle.adx_value}`);
 * });
 * ```
 * 
 * 📋 **ข้อมูล Analysis Data ที่ได้รับ (Output Fields):**
 * 
 * **1. ข้อมูลเบื้องต้น (Basic Info):**
 * - index: ลำดับแท่งเทียน
 * - candletime: เวลา Unix timestamp
 * - candletime_display: เวลาแสดงผล (YYYY-MM-DD HH:MM:SS)
 * - open, high, low, close: ราคา OHLC
 * - color: สีแท่งเทียน ("green", "red", "equal")
 * - pip_size: ขนาด Pip (|open - close|)
 * 
 * **2. EMA (Exponential Moving Average):**
 * - ema_short_value: ค่า EMA ระยะสั้น
 * - ema_short_direction: ทิศทาง ("Up" / "Down")
 * - ema_short_turn_type: จุดเปลี่ยนทิศ ("TurnUp" / "TurnDown" / "-")
 * - ema_short_slope_value: ความชัน
 * - ema_short_flat: แบน? ("y" / "n")
 * - ema_medium_value, ema_long_value: ค่า EMA กลาง และ ยาว (โครงสร้างเหมือนกัน)
 * 
 * **3. EMA Cross Signals:**
 * - ema_cut_position: Short กับ Medium Cross ("CrossUp" / "CrossDown" / "-")
 * - ema_cut_long_type: Medium กับ Long Cross
 * - ema_cut_short_long_type: Short กับ Long Cross
 * - ema_cut_all_type: All EMA aligned ("AllCrossUp" / "AllCrossDown" / "-")
 * - ema_above: Short vs Medium ("ShortAbove" / "MediumAbove")
 * - ema_long_above: Medium vs Long ("MediumAbove" / "LongAbove")
 * 
 * **4. MACD (Moving Average Convergence Divergence):**
 * - macd_12: MACD Line (EMA 12-26)
 * - macd_23: Signal Line (EMA 9 of MACD)
 * - ema_convergence_type: การลู่เข้า ("convergence" / "divergence")
 * 
 * **5. Technical Indicators:**
 * - rsi_value: Relative Strength Index (0-100)
 * - adx_value: Average Directional Index (แรงเทรนด์)
 * - choppy_indicator: Choppiness Index (ตลาดสับ)
 * - atr_value: Average True Range (ความผันผวน)
 * 
 * **6. Bollinger Bands:**
 * - bb_values: { upper, middle, lower } ราคา BB
 * - bb_position: ตำแหน่งราคา ("AboveUpper" / "NearUpper" / "NearLower" / "BelowLower")
 * - bb_bandwidth: ความกว้าง BB (%)
 * - is_bb_squeeze: BB บีบแคบ? (true/false)
 * 
 * **7. Candle Anatomy:**
 * - body: ขนาดตัวเทียน
 * - u_wick: หางบน
 * - l_wick: หางล่าง
 * - body_percent, u_wick_percent, l_wick_percent: % ของ range
 * - is_abnormal_candle: แท่งผิดปกติ (ใหญ่กว่า ATR * multiplier)
 * - is_abnormal_atr: ATR พุ่งขึ้นกะทันหัน
 * 
 * **8. EMA Position on Candle:**
 * - ema_short_pos: ตำแหน่ง EMA Short ("AboveHigh" / "UpperWick" / "Body" / "LowerWick" / "BelowLow")
 * - ema_medium_pos, ema_long_pos: ตำแหน่ง EMA กลาง และ ยาว
 * 
 * **9. Range Detector:**
 * - range_detector.in_range: อยู่ในกรอบ Range? (true/false)
 * - range_detector.range_top, range_bottom, range_avg: ราคากรอบ
 * - range_detector.range_state: สถานะ ("unbroken" / "up" / "down" / "none")
 * 
 * **10. Alternating Pattern (รูปแบบแท่งสลับสี):**
 * - is_alternating_pattern: เป็นรูปแบบสลับสี? (true/false)
 * - alternating_sequence_length: ความยาวลำดับสลับสี
 * - is_alternating_trigger: Trigger สัญญาณ (>= 3 แท่ง)
 * - is_alternating_spike: มี Spike ผิดปกติในรูปแบบสลับสี
 * 
 * **11. SMC (Smart Money Concepts):**
 * - smc.swing_trend: เทรนด์หลัก ("bullish" / "bearish")
 * - smc.internal_trend: เทรนด์ภายใน
 * - smc.premium_discount_zone: โซน Premium/Discount
 * - smc.structures, swing_points, order_blocks, fair_value_gaps
 * 
 * 📝 **หมายเหตุ (Notes):**
 * - Class นี้ใช้ Pure JavaScript ไม่ต้องพึ่ง Library ภายนอก
 * - รองรับ EMA, SMA, HMA (Hull Moving Average)
 * - คำนวณ Real-time หรือ Batch Analysis ได้
 * - ผลลัพธ์สามารถนำไปใช้กับ Chart, Trading Bot, หรือ Backtesting
 * 
 * @version 2.0.0
 * @author Converted from Rust turbo-indicators
 * @license MIT
 */

class FullAnalysisEngine {
    constructor() {
        // Default configuration
        this.defaultConfig = {
            ema: {
                short: { period: 9, type: 'ema' },
                medium: { period: 21, type: 'ema' },
                long: { period: 50, type: 'ema' }
            },
            indicators: {
                adxPeriod: 14,
                atrPeriod: 7,
                atrMulti: 1.3,
                bbPeriod: 20,
                ciPeriod: 14,
                smcPeriod: 50
            }
        };
        
        // Threshold values (can be customized per asset)
        this.thresholds = {
            flatThreshold: 0.0,
            macdGapValue: 0.0,
            altCandleAtrMultiplier: 0.5,
            altCandleSpikeMultiplier: 1.5
        };
    }

    /**
     * คำนวณ Simple Moving Average (SMA)
     */
    computeSMA(prices, period) {
        const result = new Array(prices.length).fill(0);
        if (prices.length < period || period === 0) return result;
        
        for (let i = period - 1; i < prices.length; i++) {
            let sum = 0;
            for (let j = 0; j < period; j++) {
                sum += prices[i - j];
            }
            result[i] = sum / period;
        }
        return result;
    }

    /**
     * คำนวณ Exponential Moving Average (EMA)
     */
    computeEMA(prices, period) {
        const result = new Array(prices.length).fill(0);
        if (prices.length < period || period === 0) return result;
        
        // คำนวณ SMA แรก
        let sum = 0;
        for (let i = 0; i < period; i++) {
            sum += prices[i];
        }
        let ema = sum / period;
        result[period - 1] = ema;
        
        // คำนวณ EMA ต่อไป
        const alpha = 2 / (period + 1);
        for (let i = period; i < prices.length; i++) {
            ema = prices[i] * alpha + ema * (1 - alpha);
            result[i] = ema;
        }
        return result;
    }

    /**
     * คำนวณ Hull Moving Average (HMA)
     * HMA = WMA(2*WMA(n/2) - WMA(n), sqrt(n))
     */
    computeHMA(prices, period) {
        const result = new Array(prices.length).fill(0);
        if (prices.length < period) return result;
        
        const halfPeriod = Math.floor(period / 2);
        const sqrtPeriod = Math.floor(Math.sqrt(period));
        
        // WMA ของ n/2
        const wma1 = this.computeWMA(prices, halfPeriod);
        // WMA ของ n
        const wma2 = this.computeWMA(prices, period);
        
        // 2*WMA(n/2) - WMA(n)
        const diff = prices.map((_, i) => 2 * wma1[i] - wma2[i]);
        
        // WMA ของ diff
        return this.computeWMA(diff, sqrtPeriod);
    }

    /**
     * คำนวณ Weighted Moving Average (WMA)
     */
    computeWMA(prices, period) {
        const result = new Array(prices.length).fill(0);
        if (prices.length < period || period === 0) return result;
        
        const weights = [];
        let weightSum = 0;
        for (let i = 1; i <= period; i++) {
            weights.push(i);
            weightSum += i;
        }
        
        for (let i = period - 1; i < prices.length; i++) {
            let sum = 0;
            for (let j = 0; j < period; j++) {
                sum += prices[i - j] * weights[period - 1 - j];
            }
            result[i] = sum / weightSum;
        }
        return result;
    }

    /**
     * คำนวณ Moving Average ตามประเภทที่กำหนด
     */
    computeMA(prices, period, type) {
        const lowerType = type.toLowerCase();
        if (lowerType === 'sma') return this.computeSMA(prices, period);
        if (lowerType === 'hma') return this.computeHMA(prices, period);
        if (lowerType === 'wma') return this.computeWMA(prices, period);
        return this.computeEMA(prices, period); // default
    }

    /**
     * คำนวณ MACD (Moving Average Convergence Divergence)
     * @returns { macdLine, signalLine, histogram }
     */
    computeMACD(prices, fastPeriod = 12, slowPeriod = 26, signalPeriod = 9) {
        const emaFast = this.computeEMA(prices, fastPeriod);
        const emaSlow = this.computeEMA(prices, slowPeriod);
        
        const macdLine = emaFast.map((val, i) => val - emaSlow[i]);
        const signalLine = this.computeEMA(macdLine, signalPeriod);
        const histogram = macdLine.map((val, i) => val - signalLine[i]);
        
        return { macdLine, signalLine, histogram };
    }

    /**
     * คำนวณ RSI (Relative Strength Index)
     */
    computeRSI(prices, period) {
        const result = new Array(prices.length).fill(0);
        if (prices.length < period + 1) return result;
        
        // คำนวณ gains และ losses
        const gains = [];
        const losses = [];
        for (let i = 1; i < prices.length; i++) {
            const change = prices[i] - prices[i - 1];
            gains.push(change > 0 ? change : 0);
            losses.push(change < 0 ? -change : 0);
        }
        
        // Average gain/loss แรก
        let avgGain = 0, avgLoss = 0;
        for (let i = 0; i < period; i++) {
            avgGain += gains[i];
            avgLoss += losses[i];
        }
        avgGain /= period;
        avgLoss /= period;
        
        // คำนวณ RSI
        for (let i = period; i < prices.length; i++) {
            if (avgLoss === 0) {
                result[i] = 100;
            } else {
                const rs = avgGain / avgLoss;
                result[i] = 100 - (100 / (1 + rs));
            }
            
            // Smooth average
            if (i < prices.length - 1) {
                avgGain = (avgGain * (period - 1) + gains[i]) / period;
                avgLoss = (avgLoss * (period - 1) + losses[i]) / period;
            }
        }
        
        return result;
    }

    /**
     * คำนวณ ATR (Average True Range)
     */
    computeATR(ohlcv, period) {
        const result = new Array(ohlcv.length).fill(0);
        if (ohlcv.length < 2) return result;
        
        // คำนวณ True Range
        const tr = [ohlcv[0].high - ohlcv[0].low];
        for (let i = 1; i < ohlcv.length; i++) {
            const high = ohlcv[i].high;
            const low = ohlcv[i].low;
            const prevClose = ohlcv[i - 1].close;
            
            const tr1 = high - low;
            const tr2 = Math.abs(high - prevClose);
            const tr3 = Math.abs(low - prevClose);
            
            tr.push(Math.max(tr1, tr2, tr3));
        }
        
        // คำนวณ ATR แบบ EMA
        let atr = 0;
        for (let i = 0; i < period && i < tr.length; i++) {
            atr += tr[i];
        }
        atr /= Math.min(period, tr.length);
        result[period - 1] = atr;
        
        const alpha = 1 / period;
        for (let i = period; i < ohlcv.length; i++) {
            atr = tr[i] * alpha + atr * (1 - alpha);
            result[i] = atr;
        }
        
        return result;
    }

    /**
     * คำนวณ Bollinger Bands
     */
    computeBollingerBands(prices, period, stdDev = 2.0) {
        const result = [];
        for (let i = 0; i < prices.length; i++) {
            result.push({ upper: 0, middle: 0, lower: 0 });
        }
        
        if (prices.length < period) return result;
        
        for (let i = period - 1; i < prices.length; i++) {
            // คำนวณ Mean
            let sum = 0;
            for (let j = 0; j < period; j++) {
                sum += prices[i - j];
            }
            const mean = sum / period;
            
            // คำนวณ Standard Deviation
            let variance = 0;
            for (let j = 0; j < period; j++) {
                const diff = prices[i - j] - mean;
                variance += diff * diff;
            }
            const std = Math.sqrt(variance / period);
            
            result[i] = {
                middle: mean,
                upper: mean + stdDev * std,
                lower: mean - stdDev * std
            };
        }
        
        return result;
    }

    /**
     * คำนวณ Choppiness Index
     */
    computeChoppinessIndex(ohlcv, period) {
        const result = new Array(ohlcv.length).fill(0);
        if (ohlcv.length < period) return result;
        
        for (let i = period - 1; i < ohlcv.length; i++) {
            let atrSum = 0;
            let maxHigh = -Infinity;
            let minLow = Infinity;
            
            for (let j = 0; j < period; j++) {
                const idx = i - j;
                const high = ohlcv[idx].high;
                const low = ohlcv[idx].low;
                
                maxHigh = Math.max(maxHigh, high);
                minLow = Math.min(minLow, low);
                
                if (idx > 0) {
                    const prevClose = ohlcv[idx - 1].close;
                    const tr = Math.max(
                        high - low,
                        Math.abs(high - prevClose),
                        Math.abs(low - prevClose)
                    );
                    atrSum += tr;
                } else {
                    atrSum += high - low;
                }
            }
            
            const range = maxHigh - minLow;
            if (range > 0) {
                result[i] = 100 * Math.log10(atrSum / range) / Math.log10(period);
            }
        }
        
        return result;
    }

    /**
     * คำนวณ ADX (Average Directional Index)
     */
    computeADX(ohlcv, period) {
        const result = new Array(ohlcv.length).fill(0);
        if (ohlcv.length < period * 2) return result;
        
        let trSum = 0, pdmSum = 0, ndmSum = 0;
        
        // คำนวณ TR, +DM, -DM แรก
        for (let i = 1; i <= period; i++) {
            const high = ohlcv[i].high;
            const low = ohlcv[i].low;
            const prevHigh = ohlcv[i - 1].high;
            const prevLow = ohlcv[i - 1].low;
            const prevClose = ohlcv[i - 1].close;
            
            const tr = Math.max(high - low, Math.abs(high - prevClose), Math.abs(low - prevClose));
            const up = high - prevHigh;
            const down = prevLow - low;
            
            trSum += tr;
            pdmSum += (up > down && up > 0) ? up : 0;
            ndmSum += (down > up && down > 0) ? down : 0;
        }
        
        const dxValues = new Array(ohlcv.length).fill(0);
        
        for (let i = period; i < ohlcv.length; i++) {
            if (i > period) {
                const high = ohlcv[i].high;
                const low = ohlcv[i].low;
                const prevHigh = ohlcv[i - 1].high;
                const prevLow = ohlcv[i - 1].low;
                const prevClose = ohlcv[i - 1].close;
                
                const tr = Math.max(high - low, Math.abs(high - prevClose), Math.abs(low - prevClose));
                const up = high - prevHigh;
                const down = prevLow - low;
                
                trSum = trSum - (trSum / period) + tr;
                pdmSum = pdmSum - (pdmSum / period) + ((up > down && up > 0) ? up : 0);
                ndmSum = ndmSum - (ndmSum / period) + ((down > up && down > 0) ? down : 0);
            }
            
            const pdi = trSum > 0 ? (100 * pdmSum / trSum) : 0;
            const ndi = trSum > 0 ? (100 * ndmSum / trSum) : 0;
            const sum = pdi + ndi;
            dxValues[i] = sum > 0 ? (100 * Math.abs(pdi - ndi) / sum) : 0;
        }
        
        // คำนวณ ADX (EMA ของ DX)
        let adx = 0;
        for (let i = period; i < period * 2; i++) {
            adx += dxValues[i];
        }
        adx /= period;
        result[period * 2 - 1] = adx;
        
        for (let i = period * 2; i < ohlcv.length; i++) {
            adx = (adx * (period - 1) + dxValues[i]) / period;
            result[i] = adx;
        }
        
        return result;
    }

    /**
     * คำนวณ Range Detector (ตรวจจับกรอบ Range)
     */
    computeRangeDetector(ohlcv, closes, length = 20, mult = 1.0, atrLen = 200) {
        const n = closes.length;
        const results = Array(n).fill(null).map(() => ({
            in_range: false,
            range_top: 0,
            range_bottom: 0,
            range_avg: 0,
            range_state: 'none'
        }));
        
        if (n <= length) return results;
        
        const sma = this.computeSMA(closes, length);
        
        // คำนวณ ATR
        const atrArr = new Array(n).fill(0);
        for (let i = 0; i < n; i++) {
            if (i === 0) {
                atrArr[i] = (ohlcv[0].high - ohlcv[0].low) * mult;
            } else {
                const start = Math.max(1, i + 1 - atrLen);
                let sum = 0;
                let count = 0;
                for (let k = start; k <= i; k++) {
                    const tr = Math.max(
                        ohlcv[k].high - ohlcv[k].low,
                        Math.abs(ohlcv[k].high - ohlcv[k - 1].close),
                        Math.abs(ohlcv[k].low - ohlcv[k - 1].close)
                    );
                    sum += tr;
                    count++;
                }
                atrArr[i] = count > 0 ? (sum / count) * mult : 0;
            }
        }
        
        // นับแท่งที่ทะลุกรอบ
        const countArr = new Array(n).fill(0);
        for (let i = length - 1; i < n; i++) {
            if (sma[i] === 0) continue;
            let count = 0;
            for (let k = 0; k < length; k++) {
                if (Math.abs(closes[i - k] - sma[i]) > atrArr[i]) {
                    count++;
                }
            }
            countArr[i] = count;
        }
        
        // ตรวจจับ Range Box
        let boxTop = 0, boxBottom = 0, boxAvg = 0;
        let boxState = 'none';
        let boxRightIdx = 0;
        let hasBox = false;
        
        for (let i = length; i < n; i++) {
            const count = countArr[i];
            const countPrv = countArr[i - 1];
            const ma = sma[i];
            const atrVal = atrArr[i];
            
            if (ma === 0) continue;
            
            // สร้าง Box ใหม่เมื่อ count = 0
            if (count === 0 && countPrv !== 0) {
                const startBar = i - length;
                if (hasBox && startBar <= boxRightIdx) {
                    boxTop = Math.max(ma + atrVal, boxTop);
                    boxBottom = Math.min(ma - atrVal, boxBottom);
                    boxRightIdx = i;
                    boxAvg = (boxTop + boxBottom) / 2;
                } else {
                    boxTop = ma + atrVal;
                    boxBottom = ma - atrVal;
                    boxRightIdx = i;
                    boxAvg = ma;
                    boxState = 'unbroken';
                    hasBox = true;
                }
            } else if (count === 0 && hasBox) {
                boxRightIdx = i;
            }
            
            // ตรวจจับ Breakout
            if (hasBox) {
                if (closes[i] > boxTop) boxState = 'up';
                else if (closes[i] < boxBottom) boxState = 'down';
            }
            
            // บันทึกผล
            if (hasBox) {
                results[i] = {
                    in_range: boxState === 'unbroken',
                    range_top: boxTop,
                    range_bottom: boxBottom,
                    range_avg: boxAvg,
                    range_state: boxState
                };
            }
        }
        
        return results;
    }

    /**
     * ตรวจสอบตำแหน่ง EMA บนแท่งเทียน
     */
    getCandlePosition(ema, open, high, low, close) {
        const maxBody = Math.max(open, close);
        const minBody = Math.min(open, close);
        
        if (ema > high) return 'AboveHigh';
        if (ema <= high && ema > maxBody) return 'UpperWick';
        if (ema <= maxBody && ema >= minBody) return 'Body';
        if (ema < minBody && ema >= low) return 'LowerWick';
        return 'BelowLow';
    }

    /**
     * ฟังก์ชันหลักสำหรับวิเคราะห์แท่งเทียน
     * @param {Array} candles - Array ของแท่งเทียน [{ epoch, open, high, low, close }, ...]
     * @param {Object} config - Configuration (optional)
     * @param {String} assetName - ชื่อ Asset (optional)
     * @returns {Array} ผลการวิเคราะห์แต่ละแท่ง
     */
    performAnalysis(candles, config = null, assetName = null) {
        const n = candles.length;
        if (n === 0) return [];
        
        // ใช้ config ที่ส่งมา หรือ default
        const cfg = config || this.defaultConfig;
        
        // ดึงค่า period และ type
        const emaShortPeriod = cfg.ema.short.period;
        const emaMediumPeriod = cfg.ema.medium.period;
        const emaLongPeriod = cfg.ema.long.period;
        
        const emaShortType = cfg.ema.short.type;
        const emaMediumType = cfg.ema.medium.type;
        const emaLongType = cfg.ema.long.type;
        
        const adxPeriod = cfg.indicators.adxPeriod;
        const atrPeriod = cfg.indicators.atrPeriod;
        const atrMulti = cfg.indicators.atrMulti;
        const bbPeriod = cfg.indicators.bbPeriod;
        const ciPeriod = cfg.indicators.ciPeriod;
        const smcPeriod = cfg.indicators.smcPeriod;
        
        // แปลงเป็น OHLCV
        const ohlcv = candles.map(c => ({
            open: c.open,
            high: c.high,
            low: c.low,
            close: c.close,
            volume: 0,
            timestamp: c.epoch
        }));
        
        const closes = ohlcv.map(c => c.close);
        
        // คำนวณ Indicators
        const emaShort = this.computeMA(closes, emaShortPeriod, emaShortType);
        const emaMedium = this.computeMA(closes, emaMediumPeriod, emaMediumType);
        const emaLong = this.computeMA(closes, emaLongPeriod, emaLongType);
        
        const macd = this.computeMACD(closes, 12, 26, 9);
        const rsi = this.computeRSI(closes, smcPeriod);
        const chop = this.computeChoppinessIndex(ohlcv, ciPeriod);
        const atr = this.computeATR(ohlcv, atrPeriod);
        const atr3 = this.computeATR(ohlcv, 3);
        const bb = this.computeBollingerBands(closes, bbPeriod, 2.0);
        const adx = this.computeADX(ohlcv, adxPeriod);
        const rangeDetector = this.computeRangeDetector(ohlcv, closes, 20, 1.0, 200);
        
        // คำนวณ BB Bandwidth
        const bbBandwidths = bb.map(b => 
            b.middle > 0 ? ((b.upper - b.lower) / b.middle * 100) : 0
        );
        
        // วิเคราะห์แต่ละแท่ง
        const results = [];
        let altSeqLen = 0;
        
        for (let i = 0; i < n; i++) {
            const candle = candles[i];
            const prevIdx = Math.max(0, i - 1);
            
            // Basic Info
            const pipSize = Math.abs(candle.open - candle.close);
            const color = candle.close > candle.open ? 'green' : 
                         candle.close < candle.open ? 'red' : 'equal';
            
            // Candle Anatomy
            const range = candle.high - candle.low;
            const body = Math.abs(candle.open - candle.close);
            const uWick = candle.high - Math.max(candle.open, candle.close);
            const lWick = Math.min(candle.open, candle.close) - candle.low;
            
            const uWickPercent = range > 0 ? (uWick / range) * 100 : 0;
            const bodyPercent = range > 0 ? (body / range) * 100 : 0;
            const lWickPercent = range > 0 ? (lWick / range) * 100 : 0;
            
            // Alternating Pattern Detection
            let isAlternatingPattern = false;
            let isAlternatingSpike = false;
            
            if (i > 0) {
                const prevCandle = candles[i - 1];
                const prevColor = prevCandle.close > prevCandle.open ? 'green' :
                                 prevCandle.close < prevCandle.open ? 'red' : 'equal';
                const prevBody = Math.abs(prevCandle.open - prevCandle.close);
                
                const isAltColor = (color === 'green' && prevColor === 'red') || 
                                  (color === 'red' && prevColor === 'green');
                const bodyDiff = Math.abs(prevBody - body);
                const threshold = atr3[i] * this.thresholds.altCandleAtrMultiplier;
                const isSimilarBody = threshold > 0 && bodyDiff < threshold;
                
                if (isAltColor && isSimilarBody) {
                    altSeqLen = altSeqLen === 0 ? 2 : altSeqLen + 1;
                    isAlternatingPattern = true;
                } else {
                    altSeqLen = 0;
                }
                
                if (isAltColor && atr3[i] > 0 && 
                    bodyDiff > (atr3[i] * this.thresholds.altCandleSpikeMultiplier)) {
                    isAlternatingSpike = true;
                }
            } else {
                altSeqLen = 0;
            }
            
            const isAlternatingTrigger = altSeqLen >= 3;
            
            // Time Display
            const date = new Date(candle.epoch * 1000);
            const candletimeDisplay = date.toISOString().replace('T', ' ').slice(0, 19);
            
            // Previous Values
            const prevEmaShort = emaShort[prevIdx];
            const prevEmaMedium = emaMedium[prevIdx];
            const prevEmaLong = emaLong[prevIdx];
            const prevMacd12 = macd.macdLine[prevIdx];
            const prevMacd23 = macd.signalLine[prevIdx];
            
            // EMA Directions
            const emaShortDir = emaShort[i] >= prevEmaShort ? 'Up' : 'Down';
            const emaMediumDir = emaMedium[i] >= prevEmaMedium ? 'Up' : 'Down';
            const emaLongDir = emaLong[i] >= prevEmaLong ? 'Up' : 'Down';
            
            // EMA Turns
            let emaShortTurn = '-';
            let emaMediumTurn = '-';
            let emaLongTurn = '-';
            
            if (i > 1) {
                const prev2EmaShort = emaShort[i - 2];
                const prev2EmaMedium = emaMedium[i - 2];
                const prev2EmaLong = emaLong[i - 2];
                
                if (prevEmaShort < prev2EmaShort && emaShort[i] > prevEmaShort) emaShortTurn = 'TurnUp';
                else if (prevEmaShort > prev2EmaShort && emaShort[i] < prevEmaShort) emaShortTurn = 'TurnDown';
                
                if (prevEmaMedium < prev2EmaMedium && emaMedium[i] > prevEmaMedium) emaMediumTurn = 'TurnUp';
                else if (prevEmaMedium > prev2EmaMedium && emaMedium[i] < prevEmaMedium) emaMediumTurn = 'TurnDown';
                
                if (prevEmaLong < prev2EmaLong && emaLong[i] > prevEmaLong) emaLongTurn = 'TurnUp';
                else if (prevEmaLong > prev2EmaLong && emaLong[i] < prevEmaLong) emaLongTurn = 'TurnDown';
            }
            
            // EMA Relationships
            const emaAbove = emaShort[i] >= emaMedium[i] ? 'ShortAbove' : 'MediumAbove';
            const emaLongAbove = emaMedium[i] >= emaLong[i] ? 'MediumAbove' : 'LongAbove';
            
            const emaConv = Math.abs(emaShort[i] - emaMedium[i]) < Math.abs(prevEmaShort - prevEmaMedium) 
                ? 'convergence' : 'divergence';
            const emaLongConv = Math.abs(emaMedium[i] - emaLong[i]) < Math.abs(prevEmaMedium - prevEmaLong)
                ? 'C' : 'D';
            
            // EMA Cross Detection
            let emaCutShortMedium = '-';
            if (prevEmaShort < prevEmaMedium && emaShort[i] > emaMedium[i]) emaCutShortMedium = 'CrossUp';
            else if (prevEmaShort > prevEmaMedium && emaShort[i] < emaMedium[i]) emaCutShortMedium = 'CrossDown';
            
            let emaCutMediumLong = '-';
            if (prevEmaMedium < prevEmaLong && emaMedium[i] > emaLong[i]) emaCutMediumLong = 'CrossUp';
            else if (prevEmaMedium > prevEmaLong && emaMedium[i] < emaLong[i]) emaCutMediumLong = 'CrossDown';
            
            let emaCutShortLong = '-';
            if (prevEmaShort < prevEmaLong && emaShort[i] > emaLong[i]) emaCutShortLong = 'CrossUp';
            else if (prevEmaShort > prevEmaLong && emaShort[i] < emaLong[i]) emaCutShortLong = 'CrossDown';
            
            let emaCutAll = '-';
            if (emaShort[i] > emaMedium[i] && emaMedium[i] > emaLong[i] &&
                !(prevEmaShort > prevEmaMedium && prevEmaMedium > prevEmaLong)) {
                emaCutAll = 'AllCrossUp';
            } else if (emaShort[i] < emaMedium[i] && emaMedium[i] < emaLong[i] &&
                      !(prevEmaShort < prevEmaMedium && prevEmaMedium < prevEmaLong)) {
                emaCutAll = 'AllCrossDown';
            }
            
            // BB Position
            let bbPosition = 'NearLower';
            if (candle.close > bb[i].upper) bbPosition = 'AboveUpper';
            else if (candle.close > bb[i].middle) bbPosition = 'NearUpper';
            else if (candle.close < bb[i].lower) bbPosition = 'BelowLower';
            
            // Abnormal Detection
            const isAbnormalCandle = range > atr[i] * atrMulti;
            const isAbnormalAtr = i > 5 && atr[i] > atr[i - 5] * 2.0;
            const isAtr = atr[i] > 0 && range > atr[i] * atrMulti;
            
            // EMA Positions on Candle
            const emaShortPos = this.getCandlePosition(emaShort[i], candle.open, candle.high, candle.low, candle.close);
            const emaMediumPos = this.getCandlePosition(emaMedium[i], candle.open, candle.high, candle.low, candle.close);
            const emaLongPos = this.getCandlePosition(emaLong[i], candle.open, candle.high, candle.low, candle.close);
            
            // BB Squeeze Detection
            let isBbSqueeze = false;
            const lookback = 20;
            if (i >= lookback) {
                let minBw = Infinity;
                for (let j = i - lookback; j < i; j++) {
                    minBw = Math.min(minBw, bbBandwidths[j]);
                }
                isBbSqueeze = bbBandwidths[i] < minBw;
            }
            
            // Slope Calculations
            const shortSlopeAbs = Math.abs(emaShort[i] - prevEmaShort);
            const mediumSlopeAbs = Math.abs(emaMedium[i] - prevEmaMedium);
            const longSlopeAbs = Math.abs(emaLong[i] - prevEmaLong);
            
            // SMC Data (simplified)
            const smc = {
                structures: [],
                swing_points: [],
                order_blocks: [],
                fair_value_gaps: [],
                equal_highs_lows: [],
                premium_discount_zone: {
                    start_time: i >= 20 ? ohlcv[i - 20].timestamp : candle.epoch,
                    end_time: candle.epoch,
                    premium_top: candle.high + atr[i],
                    premium_bottom: candle.close,
                    equilibrium: (candle.high + candle.low) / 2,
                    discount_top: candle.close,
                    discount_bottom: candle.low - atr[i]
                },
                strong_weak_levels: [],
                swing_trend: emaMediumDir === 'Up' ? 'bullish' : 'bearish',
                internal_trend: emaShortDir === 'Up' ? 'bullish' : 'bearish'
            };
            
            // Tick Volatility (placeholder)
            const tickVolatility = {
                tick_count: 0,
                buy_tick_count: 0,
                sell_tick_count: 0,
                buy_sell_ratio: 0.5,
                avg_tick_move: 0,
                max_tick_move: 0,
                sum_tick_move: 0,
                volatility_clustering: 0,
                volatility_level: 'Low'
            };
            
            // สร้างผลลัพธ์
            const result = {
                index: i,
                asset_code: assetName || '',
                candletime: candle.epoch,
                candletime_display: candletimeDisplay,
                open: candle.open,
                high: candle.high,
                low: candle.low,
                close: candle.close,
                color: color,
                next_color: null,
                pip_size: pipSize,
                
                ema_short_value: emaShort[i],
                ema_short_direction: emaShortDir,
                ema_short_turn_type: emaShortTurn,
                ema_short_slope_value: emaShort[i] - prevEmaShort,
                emaslope_thereshold: this.thresholds.flatThreshold,
                diff: shortSlopeAbs - this.thresholds.flatThreshold,
                ema_short_flat: shortSlopeAbs <= this.thresholds.flatThreshold ? 'y' : 'n',
                
                ema_medium_value: emaMedium[i],
                ema_medium_direction: emaMediumDir,
                ema_medium_turn_type: emaMediumTurn,
                ema_medium_slope_value: emaMedium[i] - prevEmaMedium,
                ema_medium_flat: mediumSlopeAbs <= this.thresholds.flatThreshold ? 'y' : 'n',
                
                ema_long_value: emaLong[i],
                ema_long_direction: emaLongDir,
                ema_long_turn_type: emaLongTurn,
                ema_long_slope_value: emaLong[i] - prevEmaLong,
                ema_long_flat: longSlopeAbs <= this.thresholds.flatThreshold ? 'y' : 'n',
                
                short_medium_gap_value: Math.abs(emaShort[i] - emaMedium[i]),
                is_short_medium_gap_occur: i > 0 && emaShort[i] !== 0 && emaMedium[i] !== 0 &&
                    Math.abs(emaShort[i] - emaMedium[i]) <= this.thresholds.macdGapValue ? 'y' : 'n',
                medium_long_gap_value: Math.abs(emaMedium[i] - emaLong[i]),
                is_medium_long_gap_occur: i > 0 && emaMedium[i] !== 0 && emaLong[i] !== 0 &&
                    Math.abs(emaMedium[i] - emaLong[i]) <= this.thresholds.macdGapValue ? 'y' : 'n',
                
                ema_above: emaAbove,
                ema_long_above: emaLongAbove,
                
                macd_12: macd.macdLine[i],
                macd_23: macd.signalLine[i],
                
                previous_ema_short_value: prevEmaShort,
                previous_ema_medium_value: prevEmaMedium,
                previous_ema_long_value: prevEmaLong,
                previous_macd_12: prevMacd12,
                previous_macd_23: prevMacd23,
                
                ema_convergence_type: emaConv,
                ema_long_convergence_type: emaLongConv,
                
                choppy_indicator: chop[i],
                adx_value: adx[i],
                rsi_value: rsi[i],
                
                bb_values: bb[i],
                bb_position: bbPosition,
                
                atr_value: atr[i],
                is_abnormal_candle: isAbnormalCandle,
                is_abnormal_atr: isAbnormalAtr,
                is_atr: isAtr,
                
                u_wick: uWick,
                u_wick_percent: uWickPercent,
                body: body,
                body_percent: bodyPercent,
                l_wick: lWick,
                l_wick_percent: lWickPercent,
                
                ema_cut_position: emaCutShortMedium,
                ema_cut_long_type: emaCutMediumLong,
                ema_cut_short_long_type: emaCutShortLong,
                ema_cut_all_type: emaCutAll,
                candles_since_ema_cut: 0,
                
                ema_short_pos: emaShortPos,
                ema_medium_pos: emaMediumPos,
                ema_long_pos: emaLongPos,
                
                bb_bandwidth: bbBandwidths[i],
                is_bb_squeeze: isBbSqueeze,
                
                up_con_medium_ema: 0,
                down_con_medium_ema: 0,
                up_con_long_ema: 0,
                down_con_long_ema: 0,
                
                is_mark: 'n',
                status_code: '0',
                status_desc: '-',
                status_desc_0: '-',
                hint_status: '',
                suggest_color: '',
                win_status: '',
                win_con: 0,
                loss_con: 0,
                
                smc: smc,
                tick_volatility: tickVolatility,
                range_detector: rangeDetector[i],
                is_alternating_pattern: isAlternatingPattern,
                alternating_sequence_length: altSeqLen,
                is_alternating_trigger: isAlternatingTrigger,
                is_alternating_spike: isAlternatingSpike
            };
            
            results.push(result);
        }
        
        return results;
    }

    /**
     * ตั้งค่า Thresholds สำหรับ Asset เฉพาะ
     */
    setThresholds(flatThreshold = 0.0, macdGapValue = 0.0, altAtrMult = 0.5, altSpikeMult = 1.5) {
        this.thresholds.flatThreshold = flatThreshold;
        this.thresholds.macdGapValue = macdGapValue;
        this.thresholds.altCandleAtrMultiplier = altAtrMult;
        this.thresholds.altCandleSpikeMultiplier = altSpikeMult;
    }

    /**
     * โหลด Thresholds จาก JSON Object
     */
    loadThresholdsFromJSON(thresholdsData, assetCode) {
        if (!Array.isArray(thresholdsData)) return;
        
        const threshold = thresholdsData.find(t => 
            t.asset === assetCode || t.assetCode === assetCode
        );
        
        if (threshold) {
            this.setThresholds(
                threshold.flatTheresholdValue || 0.0,
                threshold.MACDGapValue || 0.0,
                threshold.altCandleAtrMultiplier || 0.5,
                threshold.altCandleSpikeMultiplier || 1.5
            );
        }
    }
}

// Export สำหรับ Node.js / Module
if (typeof module !== 'undefined' && module.exports) {
    module.exports = FullAnalysisEngine;
}

// Export สำหรับ Browser
if (typeof window !== 'undefined') {
    window.FullAnalysisEngine = FullAnalysisEngine;
}

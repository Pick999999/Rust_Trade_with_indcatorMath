/**
 * FixAction.js
 * รวมฟังก์ชันสำหรับแก้ปัญหาโดนลากยาว (loss_con ลึก) จากสภาวะตลาดสลับสีหรือพักตัว
 * สามารถนำไปใช้งานร่วมกับกลยุทธ์หลักเพื่อกรองออเดอร์ที่เสี่ยงขาดทุนได้ทันที (Leading/Fast Reaction)
 */

/**
 * 1. ตรวจจับการทะลุกรอบ Bollinger Bands (BB Extremes)
 * หากแท่ง Spike พุ่งทะลุกรอบบนหรือล่าง มักจะเกิดการเด้งกลับ (Mean Reversion) ทันที
 * 
 * @param {Object} analysis - ข้อมูล full_analysis_data
 * @param {Number} lossCon - จำนวนครั้งที่แพ้ต่อเนื่อง
 * @returns {String|null} - คืนค่า "Wait" ถ้าควรหยุดเทรด หรือ null ถ้าปลอดภัย
 */
function checkBBExtremes(analysis, lossCon) {
    // ถ้าเจอ Spike สีเขียว แต่ราคาทะลุขอบบน (Upper Band) แสดงว่าเป็นยอดดอยชั่วคราว มีโอกาสสวิงลงสูงมาก
    if (analysis.color === "green" && analysis.bb_position === "Above Upper") {
        // แนะนำให้หยุดเทรด (Wait) หรือจะแก้เป็น return "red" เพื่อเทรดสวน (Reversal) ก็ได้
        return "Wait";
    }
    
    // ถ้าเจอ Spike สีแดง แต่ราคาทะลุขอบล่าง (Lower Band) แสดงว่าเป็นก้นเหวชั่วคราว มีโอกาสสวิงกลับขึ้นสูง
    if (analysis.color === "red" && analysis.bb_position === "Below Lower") {
        return "Wait";
    }
    
    return null; // ปลอดภัย ให้ข้ามไปเช็คเงื่อนไขอื่นต่อ
}

/**
 * 2. ตรวจจับแรงต้านจากไส้เทียน (Wick Rejection)
 * อ่านแรงซื้อ/แรงขายที่ซ่อนอยู่ในแท่ง Spike แท่งล่าสุด (จบในแท่งนั้นเลย ไม่ต้องรออินดิเคเตอร์ตัวอื่น)
 * 
 * @param {Object} analysis - ข้อมูล full_analysis_data
 * @param {Number} lossCon - จำนวนครั้งที่แพ้ต่อเนื่อง
 * @returns {String|null} - คืนค่า "Wait" ถ้าเจอไส้เทียนยาวผิดปกติ (โดนตบสวน) หรือ null ถ้าปลอดภัย
 */
function checkWickRejection(analysis, lossCon) {
    // กำหนดเกณฑ์: ถ้าไส้เทียนฝั่งสวนทางยาวเกิน 40% ของทั้งแท่ง ถือว่ามีแรงต้านรุนแรง
    const WICK_THRESHOLD = 40.0;

    // Spike สีเขียว แต่มีไส้บน (Upper Wick) ยาว แปลว่าตอนพุ่งขึ้นไป โดนเทขายสวนลงมาหนักมาก
    if (analysis.color === "green" && analysis.u_wick_percent > WICK_THRESHOLD) {
        return "Wait"; // ห้าม Follow ตามสีเขียวเด็ดขาด
    }

    // Spike สีแดง แต่มีไส้ล่าง (Lower Wick) ยาว แปลว่าตอนเทลงมา มีแรงซื้อดันสวนขึ้นมาหนักมาก
    if (analysis.color === "red" && analysis.l_wick_percent > WICK_THRESHOLD) {
        return "Wait"; // ห้าม Follow ตามสีแดงเด็ดขาด
    }

    return null;
}

/**
 * 3. ตรวจจับตลาดสลับสีระยะสั้น (Micro Alternating Check)
 * ดูสถานะตลาดย้อนหลังว่ากำลังอยู่ในสภาวะ Choppy Market (กราฟสลับสีไปมา เขียว-แดง-เขียว) หรือไม่
 * 
 * @param {Object} analysis - ข้อมูล full_analysis_data
 * @param {Number} lossCon - จำนวนครั้งที่แพ้ต่อเนื่อง
 * @returns {String|null} - คืนค่า "Wait" หรือสีเพื่อเทรดสวน หากอยู่ในตลาดสลับสี
 */
function checkMicroAlternating(analysis, lossCon) {
    // ถ้าระบบ (Rust) คำนวณและส่งค่ามาบอกว่ากราฟตรงนี้กำลังเป็นแพทเทิร์นสลับสี (Alternating)
    if (analysis.is_alternating_pattern === true) {
        
        // ตัวเลือก A: หยุดเทรดชั่วคราว (Pause) ตัดปัญหาการเดาสี
        return "Wait";
        
        // ตัวเลือก B (ถ้าอยากซิ่ง): เมื่อรู้ว่ามันสลับสีแน่นอน ก็เปิดออเดอร์สวนทางกับสีแท่ง Spike เลย
        // return analysis.color === "green" ? "red" : "green";
    }

    return null;
}

/**
 * 4. ไม้ตัดจบความดื้อดึง (Fast Circuit Breaker)
 * หากโดนลากจนแพ้ถึงขีดจำกัดที่รับได้ ให้บังคับหยุดเทรดทันที ตัดไฟแต่ต้นลมเพื่อรักษาทุน
 * 
 * @param {Object} analysis - ข้อมูล full_analysis_data
 * @param {Number} lossCon - จำนวนครั้งที่แพ้ต่อเนื่อง
 * @returns {String|null} - คืนค่า "Wait" ถ้าแพ้ถึงขีดจำกัด หรือ null ถ้ายังอยู่ในลิมิต
 */
function checkCircuitBreaker(analysis, lossCon) {
    // กำหนดจุดยอมแพ้ (Stop Loss) เช่น แพ้ติดกัน 3 ไม้ ให้หยุดการทำงานของลูปนี้ทันที
    const MAX_LOSS = 3; 

    if (lossCon >= MAX_LOSS) {
        // สั่งให้ระบบคืนค่า Wait ทันที ทำให้โปรแกรมไม่สามารถไปเข้าไม้ 4, 5, ..., 10 ได้
        return "Wait";
    }

    return null;
}

/**
 * ============================================================================
 * ฟังก์ชันหลัก (Main Filter) ที่รวบรวมตัวกรองทั้ง 4 ข้อเข้าด้วยกัน
 * สามารถนำฟังก์ชันนี้ไปคั่นไว้ก่อนที่ระบบจะรันกลยุทธ์ V1, V2 ปกติ
 * ============================================================================
 * 
 * @param {Object} analysis - ข้อมูล full_analysis_data
 * @param {Number} lossCon - จำนวนครั้งที่แพ้ต่อเนื่อง
 * @returns {String|null} - คืนค่า "Wait", "green", "red" หรือ null หากผ่านตัวกรองทั้งหมดอย่างปลอดภัย
 */
function applyFixActionFilters(analysis, lossCon) {
    let action = null;

    // 1. เช็ค Circuit Breaker ก่อนเป็นอันดับแรก (ถ้าแพ้ทะลุเป้า ให้หยุดทุกอย่าง)
    action = checkCircuitBreaker(analysis, lossCon);
    if (action) return action;

    // 2. เช็คแรงต้านจากไส้เทียน (Wick Rejection) ที่หน้างานจริง
    action = checkWickRejection(analysis, lossCon);
    if (action) return action;

    // 3. เช็คว่าราคาทะลุกรอบ BB จนอันตรายหรือไม่ (Extremes)
    action = checkBBExtremes(analysis, lossCon);
    if (action) return action;

    // 4. เช็คสภาวะกราฟสลับสี (Choppy / Alternating)
    action = checkMicroAlternating(analysis, lossCon);
    if (action) return action;

    // ถ้ารอดจากเงื่อนไขด้านบนมาได้ทั้งหมด แปลว่า Spike นี้มีความแข็งแกร่งและน่าเชื่อถือ
    // ให้คืนค่า null กลับไป เพื่อให้โปรแกรมทำงานตามกลยุทธ์ V1, V2 (หรืออื่นๆ) ตามปกติต่อไป
    return null;
}

// Export เพื่อนำไปประยุกต์ใช้กับไฟล์อื่นได้ง่าย
if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        checkBBExtremes,
        checkWickRejection,
        checkMicroAlternating,
        checkCircuitBreaker,
        applyFixActionFilters
    };
}

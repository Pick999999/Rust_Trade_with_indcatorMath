// --- TOAST FUNCTION ---
function showToast(message, duration = 3000) {
    let container = document.querySelector('.toast-container');
    if (!container) {
        container = document.createElement('div');
        container.className = 'toast-container';
        document.body.appendChild(container);
    }
    const toast = document.createElement('div');
    toast.className = 'toast-popup';
    toast.innerHTML = `<span style="font-size: 16px;">💬</span><span>${message}</span>`;
    container.appendChild(toast);

    // Trigger animation
    setTimeout(() => {
        toast.classList.add('show');
    }, 10);

    setTimeout(() => {
        toast.classList.remove('show');
        setTimeout(() => {
            toast.remove();
        }, 300);
    }, duration);
}

function playStopSound() {
    const toggle = document.getElementById('playSoundToggle');
    if (toggle && toggle.checked) {
        console.log("Playing stop sound...");
        const stopAudio = new Audio('/sounds/universfield-attention-chime-123107.mp3');
        stopAudio.play().catch(e => console.error("Audio play error:", e));
    }
}

function updateDigitalClock() {
    const now = new Date();
    const hh = String(now.getHours()).padStart(2, '0');
    const mm = String(now.getMinutes()).padStart(2, '0');
    const ss = String(now.getSeconds()).padStart(2, '0');
    const timeStr = `${hh}:${mm}:${ss}`;

    const clockEl = document.getElementById('digitalClock');
    if (clockEl) clockEl.textContent = timeStr;

    const inlineClockEl = document.getElementById('inlineClock');
    if (inlineClockEl) inlineClockEl.textContent = timeStr;

    // Schedule UI update
    const useSchedule = document.getElementById('useSchedule');
    const startDateEl = document.getElementById('startDate');
    const stopDateEl = document.getElementById('stopDate');
    const fetchBtn = document.getElementById('setDerivTimeBtn');
    const scheduleCard = document.getElementById('scheduleSectionCard');

    if (useSchedule && startDateEl && stopDateEl && fetchBtn && scheduleCard) {
        if (useSchedule.checked) {
            const startTime = new Date(startDateEl.value).getTime();
            const stopTime = new Date(stopDateEl.value).getTime();
            const currentTime = now.getTime();

            if (currentTime >= startTime && currentTime <= stopTime) {
                // Trading is active
                if (!fetchBtn.disabled) {
                    fetchBtn.disabled = true;
                    fetchBtn.style.background = '#475569';
                    fetchBtn.style.color = '#94a3b8';
                    fetchBtn.style.cursor = 'not-allowed';

                    scheduleCard.style.background = 'linear-gradient(to bottom, #87e0fd 0%, #53cbf1 40%, #05abe0 100%)';
                    scheduleCard.style.border = '1px solid #05abe0';
                    // To ensure text remains readable on bright blue
                    scheduleCard.style.color = '#000';
                    const title = scheduleCard.querySelector('.section-card-title');
                    if (title) title.style.color = '#000';
                    scheduleCard.querySelectorAll('.date-range-label').forEach(el => el.style.color = '#1e293b');
                    if (inlineClockEl) {
                        inlineClockEl.style.color = '#003366';
                        inlineClockEl.style.borderColor = '#003366';
                        inlineClockEl.style.background = 'rgba(255, 255, 255, 0.3)';
                        inlineClockEl.style.boxShadow = 'inset 0 0 5px rgba(255, 255, 255, 0.5)';
                    }
                }
            } else {
                // Not active
                if (fetchBtn.disabled) {
                    fetchBtn.disabled = false;
                    fetchBtn.style.background = 'var(--orange)';
                    fetchBtn.style.color = '#fff';
                    fetchBtn.style.cursor = 'pointer';

                    scheduleCard.style.background = 'var(--surface)';
                    scheduleCard.style.border = '1px solid var(--border)';
                    scheduleCard.style.color = '';
                    const title = scheduleCard.querySelector('.section-card-title');
                    if (title) title.style.color = '';
                    scheduleCard.querySelectorAll('.date-range-label').forEach(el => el.style.color = '');
                    if (inlineClockEl) {
                        inlineClockEl.style.color = 'var(--orange)';
                        inlineClockEl.style.borderColor = 'var(--orange)';
                        inlineClockEl.style.background = 'rgba(245, 166, 35, 0.1)';
                        inlineClockEl.style.boxShadow = 'inset 0 0 5px rgba(245, 166, 35, 0.2)';
                    }
                }
            }
        } else {
            // Schedule disabled
            if (fetchBtn.disabled) {
                fetchBtn.disabled = false;
                fetchBtn.style.background = 'var(--orange)';
                fetchBtn.style.color = '#fff';
                fetchBtn.style.cursor = 'pointer';

                scheduleCard.style.background = 'var(--surface)';
                scheduleCard.style.border = '1px solid var(--border)';
                scheduleCard.style.color = '';
                const title = scheduleCard.querySelector('.section-card-title');
                if (title) title.style.color = '';
                scheduleCard.querySelectorAll('.date-range-label').forEach(el => el.style.color = '');
                if (inlineClockEl) {
                    inlineClockEl.style.color = 'var(--orange)';
                    inlineClockEl.style.borderColor = 'var(--orange)';
                    inlineClockEl.style.background = 'rgba(245, 166, 35, 0.1)';
                    inlineClockEl.style.boxShadow = 'inset 0 0 5px rgba(245, 166, 35, 0.2)';
                }
            }
        }
    }
}
setInterval(updateDigitalClock, 1000);
updateDigitalClock();

// ══════════════════════════════════════════
//  COLLECT ALL SETTINGS → JSON
// ══════════════════════════════════════════
function collectSettings() {
    const assets = [...document.querySelectorAll('input[name="asset"]:checked')].map(cb => cb.value);
    const martType = document.querySelector('input[name="useMartingale"]:checked')?.value || 'fixed';
    const martList = document.getElementById('martingaleList').value;
    const condIdx = document.getElementById('conditionStopTrade').selectedIndex;
    const condVals = ['targetMoney', 'targetLot', 'stopDate'];

    const activeGranBtn = document.querySelector('.gran-btn.active');
    const activeGranSet = document.querySelector('.gran-setting-btn.active');

    let customGranSecs = 0;
    const customGranInput = document.getElementById('customGranularitySeconds');
    if (customGranInput) {
        customGranSecs = parseInt(customGranInput.value) || 0;
    }

    return {
        meta: {
            generatedAt: new Date().toISOString(),
            theme: document.body.getAttribute('data-theme') || 'lightblue'
        },
        granularity: activeGranBtn ? parseInt(activeGranBtn.dataset.value) : 60,
        granularitySettings: customGranSecs > 0 ? customGranSecs : (activeGranSet ? parseInt(activeGranSet.dataset.value) : 60),
        chart: {
            rightOffset: parseInt(document.getElementById('chartRightOffset').value) || 5
        },
        assets,
        indicators: {
            atrPeriod: document.getElementById('atrPeriod').value === "" ? 14 : parseFloat(document.getElementById('atrPeriod').value),
            atrMulti: document.getElementById('atrMulti').value === "" ? 2 : parseFloat(document.getElementById('atrMulti').value),
            altCandleAtrMultiplier: document.getElementById('altCandleAtrMultiplier') ? (document.getElementById('altCandleAtrMultiplier').value === "" ? 0.5 : parseFloat(document.getElementById('altCandleAtrMultiplier').value)) : 0.5,
            ciPeriod: parseInt(document.getElementById('ciPeriod').value) || 20,
            adxPeriod: parseInt(document.getElementById('adxPeriod').value) || 14,
            bbPeriod: parseInt(document.getElementById('bbPeriod').value) || 20,
            smcPeriod: parseInt(document.getElementById('smcPeriod').value) || 50,
            choppyCombinedMode: document.getElementById('kamaChoppyMode') ? document.getElementById('kamaChoppyMode').value : 'or',
            choppyGroupSize: document.getElementById('kamaGroupSize') ? parseInt(document.getElementById('kamaGroupSize').value) || 15 : 15,
            choppyPeriod: document.getElementById('kamaChopPeriod') ? parseInt(document.getElementById('kamaChopPeriod').value) || 14 : 14,
            choppyThreshold: document.getElementById('kamaChopThreshold') ? parseFloat(document.getElementById('kamaChopThreshold').value) || 61.8 : 61.8,
            kamaErThreshold: document.getElementById('kamaErThreshold') ? parseFloat(document.getElementById('kamaErThreshold').value) || 0.35 : 0.35,
            bodyRatioThreshold: document.getElementById('kamaBodyRatioThreshold') ? parseFloat(document.getElementById('kamaBodyRatioThreshold').value) || 0.30 : 0.30,
            minSwitches: document.getElementById('kamaMinSwitches') ? parseInt(document.getElementById('kamaMinSwitches').value) || 3 : 3
        },
        ema: {
            short: { enabled: document.getElementById('emaShortToggle').checked, period: parseInt(document.getElementById('emaShortPeriod').value) || 9, type: document.getElementById('emaShortType').value, color: document.getElementById('emaShortColor').value },
            medium: { enabled: document.getElementById('emaMediumToggle').checked, period: parseInt(document.getElementById('emaMediumPeriod').value) || 21, type: document.getElementById('emaMediumType').value, color: document.getElementById('emaMediumColor').value },
            long: { enabled: document.getElementById('emaLongToggle').checked, period: parseInt(document.getElementById('emaLongPeriod').value) || 50, type: document.getElementById('emaLongType').value, color: document.getElementById('emaLongColor').value }
        },
        trade: {
            conditionStopTrade: condVals[condIdx],
            targetMoney: parseFloat(document.getElementById('targetMoney').value) || 0,
            targetLot: parseInt(document.getElementById('targetLot').value) || 0,
            notifyTelegram: document.getElementById('telegramToggle') ? document.getElementById('telegramToggle').checked : false,
            playSound: document.getElementById('playSoundToggle') ? document.getElementById('playSoundToggle').checked : true,
            playEmaCutSound: document.getElementById('playEmaCutSoundToggle') ? document.getElementById('playEmaCutSoundToggle').checked : true,
            autoSellTimeout: document.getElementById('autoSellTimeoutToggle') ? document.getElementById('autoSellTimeoutToggle').checked : false,
            autoSellTimeoutSeconds: parseInt(document.getElementById('autoSellTimeoutSeconds').value) || 90,
            autoSellProfit: document.getElementById('autoSellProfitToggle') ? document.getElementById('autoSellProfitToggle').checked : false,
            autoSellProfitTarget: document.getElementById('autoSellProfitTarget') ? (parseFloat(document.getElementById('autoSellProfitTarget').value) || 0.10) : 0.10,
            buyMethod: document.getElementById('buyMethodSelect') ? document.getElementById('buyMethodSelect').value : 'proposal',
            suggestStrategy: document.getElementById('suggestStrategySelect') ? document.getElementById('suggestStrategySelect').value : 'V1',
            borrowSignal: document.getElementById('borrowSignalToggle') ? document.getElementById('borrowSignalToggle').checked : false,
            whipsawZone: document.getElementById('whipsawZoneToggle') ? document.getElementById('whipsawZoneToggle').checked : true,
            saveTrackOrder: document.getElementById('saveTrackOrderToggle') ? document.getElementById('saveTrackOrderToggle').checked : false,
            checkBbFlatChoppy: document.getElementById('checkBbFlatChoppyToggle') ? document.getElementById('checkBbFlatChoppyToggle').checked : true,
            checkBbSqueeze: document.getElementById('checkBbSqueezeToggle') ? document.getElementById('checkBbSqueezeToggle').checked : true,
            checkKarmaChoppy: document.getElementById('checkKarmaChoppyToggle') ? document.getElementById('checkKarmaChoppyToggle').checked : true,
            thbRate: document.getElementById('thb-rate') ? parseFloat(document.getElementById('thb-rate').value) : 35,
            maxLossCon: document.getElementById('maxLossCon') ? parseInt(document.getElementById('maxLossCon').value) : 0,
            martingale: {
                type: martType,
                list: martType === 'martingale'
                    ? martList.split(',').map(v => parseFloat(v.trim())).filter(v => !isNaN(v))
                    : []
            }
        }
    };
}

let isInitLoading = false;
function updateJSON() {
    if (isInitLoading) return;
    const currentSettings = collectSettings();
    document.getElementById('jsonSettingTextarea').value =
        JSON.stringify(currentSettings, null, 2);
}

let hasLoadedSchedule = false;
let scheduleTradeCompleted = false; // 🔧 FIX: ป้องกัน schedule auto-start trade ใหม่เมื่อ trade รอบนี้จบแล้ว
async function fetchTradeControl() {
    try {
        const res = await fetch('/api/tradeControl', { cache: 'no-store' });
        const json = await res.json();
        if (json && json.status === 'success') {
            const tc = json.data;
            const rNo = tc.totalTrade > 0 ? tc.totalTrade : 0;
            
            console.log(`📊 [TradeControl] Status: ${tc.TradeStatus}, totalTrade: ${tc.totalTrade}, isTradeRunning: ${isTradeRunning}`);

            if (!hasLoadedSchedule) {
                if (tc.startTradeTime) {
                    let st = tc.startTradeTime.replace(" ", "T");
                    if (st.length >= 16) st = st.substring(0, 16);
                    document.getElementById('startDate').value = st;
                }
                if (tc.stopTradeTime) {
                    let st = tc.stopTradeTime.replace(" ", "T");
                    if (st.length >= 16) st = st.substring(0, 16);
                    document.getElementById('stopDate').value = st;
                }
                if (document.getElementById('useSchedule') && tc.useSchedule !== undefined) {
                    document.getElementById('useSchedule').checked = tc.useSchedule;
                }
                hasLoadedSchedule = true;
            }

            const el = document.getElementById('scheduleTradeNo');
            if (el) {
                if (tc.TradeStatus === 'กำลังเทรด') {
                    el.value = tc.totalTrade;
                    // ╔══════════════════════════════════════════════════════════════════╗
                    // ║ ⚠️ CRITICAL BUG FIX — ห้ามลบหรือแก้ไขโค้ดส่วนนี้! ⚠️           ║
                    // ║ ป้องกัน "Schedule Trade Round +1 Bug"                           ║
                    // ║ ต้องตั้ง isTradeRunning = true ทันทีเมื่อ backend รายงาน         ║
                    // ║ ว่ากำลังเทรด เพื่อป้องกัน Schedule Timer จาก auto-start ซ้ำ      ║
                    // ║ ก่อนที่ WebSocket จะยืนยัน ถ้าลบส่วนนี้ → เปิดหน้าเว็บ          ║
                    // ║ ระหว่างเทรด → รอบเทรดจะ +1 ซ้ำผิดพลาด                         ║
                    // ╚══════════════════════════════════════════════════════════════════╝
                    if (!isTradeRunning) {
                        isTradeRunning = true;
                        scheduleTradeCompleted = true; // ป้องกัน schedule timer trigger ซ้ำ
                        console.log(`✅ [TradeControl] ตั้ง isTradeRunning = true ทันที (TradeStatus = 'กำลังเทรด', round: ${tc.totalTrade})`);
                    }
                } else {
                    // 🔧 FIX: ตรวจว่ายังอยู่ใน schedule window หรือไม่
                    const useScheduleEl = document.getElementById('useSchedule');
                    const stopVal = document.getElementById('stopDate').value;
                    const now = Date.now();
                    const stop = stopVal ? new Date(stopVal).getTime() : 0;
                    const isInScheduleWindow = useScheduleEl && useScheduleEl.checked && (!stop || now < stop);

                    if (isInScheduleWindow && tc.totalTrade > 0) {
                        // ยังอยู่ใน schedule window — keep round number เดิม (ไม่ +1)
                        if (parseInt(el.value) !== tc.totalTrade) {
                            el.value = tc.totalTrade;
                            console.log(`📌 [TradeControl] Keep scheduleTradeNo = ${tc.totalTrade} (still in schedule window)`);
                        }
                        // 🔧 FIX: ตั้ง flag ว่า trade รอบนี้จบแล้ว ไม่ให้ schedule auto-start ใหม่
                        if (tc.actionStop && tc.actionStop !== '') {
                            scheduleTradeCompleted = true;
                            console.log(`🏁 [TradeControl] scheduleTradeCompleted = true (actionStop: ${tc.actionStop})`);
                        }
                    } else {
                        // Schedule หมดเวลาแล้ว หรือไม่ได้ใช้ schedule — แสดง next round
                        const nextRound = tc.totalTrade + 1;
                        if (parseInt(el.value) !== nextRound) {
                            el.value = nextRound;
                        }
                        scheduleTradeCompleted = false; // reset flag เมื่อ schedule window จบ
                    }
                    if (isTradeRunning) {
                        isTradeRunning = false;
                        if (typeof tradeDurationInterval !== 'undefined' && tradeDurationInterval) clearInterval(tradeDurationInterval);
                        tradeDurationInterval = null;
                        updateTradingDuration();

                        // Play sound when trade stops
                        playStopSound();
                    }
                }
            }
        }
    } catch (e) {
        console.error("fetchTradeControl error:", e);
    }
}

async function loadSetup() {
    try {
        isInitLoading = true;
        const res = await fetch('/api/setup', { cache: 'no-store' });
        const data = await res.json();
        if (data && Object.keys(data).length > 0) {
            if (data.indicators) {
                if (document.getElementById('atrPeriod')) document.getElementById('atrPeriod').value = data.indicators.atrPeriod;
                if (document.getElementById('atrMulti')) document.getElementById('atrMulti').value = data.indicators.atrMulti;
                if (document.getElementById('altCandleAtrMultiplier') && data.indicators.altCandleAtrMultiplier !== undefined) document.getElementById('altCandleAtrMultiplier').value = data.indicators.altCandleAtrMultiplier;
                if (document.getElementById('ciPeriod')) document.getElementById('ciPeriod').value = data.indicators.ciPeriod;
                if (document.getElementById('adxPeriod')) document.getElementById('adxPeriod').value = data.indicators.adxPeriod;
                if (document.getElementById('bbPeriod')) document.getElementById('bbPeriod').value = data.indicators.bbPeriod;
                if (document.getElementById('smcPeriod')) document.getElementById('smcPeriod').value = data.indicators.smcPeriod;
                if (document.getElementById('kamaChoppyMode') && data.indicators.choppyCombinedMode !== undefined) document.getElementById('kamaChoppyMode').value = data.indicators.choppyCombinedMode;
                if (document.getElementById('kamaGroupSize') && data.indicators.choppyGroupSize !== undefined) document.getElementById('kamaGroupSize').value = data.indicators.choppyGroupSize;
                if (document.getElementById('kamaChopPeriod') && data.indicators.choppyPeriod !== undefined) document.getElementById('kamaChopPeriod').value = data.indicators.choppyPeriod;
                if (document.getElementById('kamaChopThreshold') && data.indicators.choppyThreshold !== undefined) document.getElementById('kamaChopThreshold').value = data.indicators.choppyThreshold;
                if (document.getElementById('kamaErThreshold') && data.indicators.kamaErThreshold !== undefined) document.getElementById('kamaErThreshold').value = data.indicators.kamaErThreshold;
                if (document.getElementById('kamaBodyRatioThreshold') && data.indicators.bodyRatioThreshold !== undefined) document.getElementById('kamaBodyRatioThreshold').value = data.indicators.bodyRatioThreshold;
                if (document.getElementById('kamaMinSwitches') && data.indicators.minSwitches !== undefined) document.getElementById('kamaMinSwitches').value = data.indicators.minSwitches;
            }
            // ตรวจสอบค่าล่าสุดจาก /api/settings ด้วยเพื่อซิงค์ตาม Approach 1
            try {
                const sRes = await fetch('/api/settings', { cache: 'no-store' });
                if (sRes.ok) {
                    const sData = await sRes.json();
                    const cc = sData?.settings?.parameters?.choppy_combined;
                    if (cc) {
                        if (document.getElementById('kamaChoppyMode') && cc.mode) document.getElementById('kamaChoppyMode').value = cc.mode;
                        if (document.getElementById('kamaGroupSize') && cc.group_size) document.getElementById('kamaGroupSize').value = cc.group_size;
                        if (document.getElementById('kamaChopPeriod') && cc.chop_period) document.getElementById('kamaChopPeriod').value = cc.chop_period;
                        if (document.getElementById('kamaChopThreshold') && cc.chop_threshold) document.getElementById('kamaChopThreshold').value = cc.chop_threshold;
                        if (document.getElementById('kamaErThreshold') && cc.kama_er_threshold) document.getElementById('kamaErThreshold').value = cc.kama_er_threshold;
                        if (document.getElementById('kamaBodyRatioThreshold') && cc.body_ratio_threshold) document.getElementById('kamaBodyRatioThreshold').value = cc.body_ratio_threshold;
                        if (document.getElementById('kamaMinSwitches') && cc.min_switches) document.getElementById('kamaMinSwitches').value = cc.min_switches;
                    }
                }
            } catch (_) {}
            if (data.trade) {
                const condVals = ['targetMoney', 'targetLot', 'stopDate'];
                if (document.getElementById('conditionStopTrade')) document.getElementById('conditionStopTrade').selectedIndex = condVals.indexOf(data.trade.conditionStopTrade);
                if (document.getElementById('targetMoney')) document.getElementById('targetMoney').value = data.trade.targetMoney;
                if (document.getElementById('targetLot')) document.getElementById('targetLot').value = data.trade.targetLot;
                if (document.getElementById('maxLossCon') && data.trade.maxLossCon !== undefined) document.getElementById('maxLossCon').value = data.trade.maxLossCon;
                if (document.getElementById('telegramToggle')) document.getElementById('telegramToggle').checked = data.trade.notifyTelegram;
                if (document.getElementById('playSoundToggle')) document.getElementById('playSoundToggle').checked = data.trade.playSound !== false;
                if (document.getElementById('playEmaCutSoundToggle')) document.getElementById('playEmaCutSoundToggle').checked = data.trade.playEmaCutSound !== false;
                if (document.getElementById('autoSellTimeoutToggle')) document.getElementById('autoSellTimeoutToggle').checked = data.trade.autoSellTimeout || false;
                if (document.getElementById('autoSellTimeoutSeconds') && data.trade.autoSellTimeoutSeconds) document.getElementById('autoSellTimeoutSeconds').value = data.trade.autoSellTimeoutSeconds;
                if (document.getElementById('autoSellProfitToggle') && data.trade.autoSellProfit !== undefined) document.getElementById('autoSellProfitToggle').checked = data.trade.autoSellProfit;
                if (document.getElementById('autoSellProfitTarget') && data.trade.autoSellProfitTarget !== undefined) document.getElementById('autoSellProfitTarget').value = data.trade.autoSellProfitTarget;
                if (document.getElementById('buyMethodSelect') && data.trade.buyMethod) document.getElementById('buyMethodSelect').value = data.trade.buyMethod;
                if (document.getElementById('suggestStrategySelect') && data.trade.suggestStrategy) document.getElementById('suggestStrategySelect').value = data.trade.suggestStrategy;
                if (document.getElementById('borrowSignalToggle')) document.getElementById('borrowSignalToggle').checked = data.trade.borrowSignal || false;
                if (document.getElementById('whipsawZoneToggle')) document.getElementById('whipsawZoneToggle').checked = data.trade.whipsawZone !== false;
                if (document.getElementById('saveTrackOrderToggle')) document.getElementById('saveTrackOrderToggle').checked = data.trade.saveTrackOrder || false;
                if (document.getElementById('checkBbFlatChoppyToggle') && data.trade.checkBbFlatChoppy !== undefined) document.getElementById('checkBbFlatChoppyToggle').checked = data.trade.checkBbFlatChoppy;
                if (document.getElementById('checkBbSqueezeToggle') && data.trade.checkBbSqueeze !== undefined) document.getElementById('checkBbSqueezeToggle').checked = data.trade.checkBbSqueeze;
                if (document.getElementById('checkKarmaChoppyToggle') && data.trade.checkKarmaChoppy !== undefined) document.getElementById('checkKarmaChoppyToggle').checked = data.trade.checkKarmaChoppy;

                if (data.trade.martingale) {
                    const rb = document.querySelector(`input[name="useMartingale"][value="${data.trade.martingale.type}"]`);
                    if (rb) rb.checked = true;
                    if (document.getElementById('martingaleList') && data.trade.martingale.list) {
                        document.getElementById('martingaleList').value = data.trade.martingale.list.join(', ');
                    }
                }
            }
            if (data.ema) {
                ['Short', 'Medium', 'Long'].forEach(k => {
                    const lk = k.toLowerCase();
                    if (data.ema[lk]) {
                        if (document.getElementById(`ema${k}Toggle`)) document.getElementById(`ema${k}Toggle`).checked = data.ema[lk].enabled;
                        if (document.getElementById(`ema${k}Period`)) document.getElementById(`ema${k}Period`).value = data.ema[lk].period;
                        if (document.getElementById(`ema${k}Type`)) document.getElementById(`ema${k}Type`).value = data.ema[lk].type;
                        if (document.getElementById(`ema${k}Color`)) document.getElementById(`ema${k}Color`).value = data.ema[lk].color;
                    }
                });
            }
            if (data.assets) {
                document.querySelectorAll('input[name="asset"]').forEach(cb => {
                    cb.checked = data.assets.includes(cb.value);
                });
            }
            if (data.granularity) {
                document.querySelectorAll('.gran-btn').forEach(b => {
                    b.classList.toggle('active', parseInt(b.dataset.value) === data.granularity);
                });
            }
            if (data.granularitySettings) {
                let isStandard = false;
                document.querySelectorAll('.gran-setting-btn').forEach(b => {
                    const isMatch = parseInt(b.dataset.value) === data.granularitySettings;
                    b.classList.toggle('active', isMatch);
                    if (isMatch) isStandard = true;
                });
                const customInput = document.getElementById('customGranularitySeconds');
                if (customInput) {
                    customInput.value = isStandard ? 0 : data.granularitySettings;
                }
            }
            if (data.chart) {
                if (document.getElementById('chartRightOffset') && data.chart.rightOffset !== undefined) {
                    document.getElementById('chartRightOffset').value = data.chart.rightOffset;
                }
            }
            // Load and apply theme
            if (data.meta && data.meta.theme) {
                const theme = data.meta.theme;
                document.body.setAttribute('data-theme', theme);
                document.querySelectorAll('.theme-btn').forEach(btn => {
                    btn.classList.toggle('active', btn.dataset.theme === theme);
                });
                // Update chart if it exists
                setTimeout(() => { if (typeof updateChartTheme === 'function') updateChartTheme(); }, 100);
            }
        }
    } catch (e) {
        console.error("Failed to load setup", e);
    } finally {
        isInitLoading = false;
    }
    updateJSON();

    // Initial fetch for trade control
    fetchTradeControl();
    setInterval(fetchTradeControl, 5000); // Fetch every 5 seconds
    if (typeof updateActiveAssetButtons === 'function') updateActiveAssetButtons();
    if (typeof updateChartAssetButtons === 'function') updateChartAssetButtons();
    if (typeof updateManualTradeAssetDropdown === 'function') updateManualTradeAssetDropdown();
    if (typeof applyManualModeUI === 'function') applyManualModeUI();

    // Hide initial loading screen after setup is loaded
    const initLoadingScreen = document.getElementById('initLoadingScreen');
    if (initLoadingScreen) {
        initLoadingScreen.style.opacity = '0';
        setTimeout(() => {
            initLoadingScreen.style.visibility = 'hidden';
            initLoadingScreen.style.display = 'none';
        }, 500);
    }
}

// ══════════════════════════════════════════
//  1. DATE VALIDATION
// ══════════════════════════════════════════
function validateDates() {
    const startEl = document.getElementById('startDate');
    const stopEl = document.getElementById('stopDate');
    const startMsg = document.getElementById('startDateMsg');
    const stopMsg = document.getElementById('stopDateMsg');
    const start = new Date(startEl.value);
    const stop = new Date(stopEl.value);

    // reset
    startEl.classList.remove('error'); stopEl.classList.remove('error');
    startMsg.className = 'validation-msg'; stopMsg.className = 'validation-msg';
    startMsg.textContent = ''; stopMsg.textContent = '';

    if (!startEl.value || !stopEl.value) return true;

    const sameDay =
        start.getFullYear() === stop.getFullYear() &&
        start.getMonth() === stop.getMonth() &&
        start.getDate() === stop.getDate();

    if (!sameDay) {
        stopEl.classList.add('error'); stopMsg.classList.add('show');
        stopMsg.textContent = '⚠ วันสิ้นสุดต้องเป็นวันเดียวกับวันเริ่มต้น';
        return false;
    }
    if (start >= stop) {
        startEl.classList.add('error'); startMsg.classList.add('show');
        startMsg.textContent = '⚠ เวลาเริ่มต้นต้องน้อยกว่าเวลาสิ้นสุด';
        return false;
    }
    return true;
}

document.getElementById('scheduleTradeNo').addEventListener('change', () => { updateJSON(); });
document.getElementById('startDate').addEventListener('change', () => { validateDates(); updateJSON(); });
document.getElementById('stopDate').addEventListener('change', () => { validateDates(); updateJSON(); });
if (document.getElementById('buyMethodSelect')) document.getElementById('buyMethodSelect').addEventListener('change', () => { updateJSON(); });

// ══════════════════════════════════════════
//  MANUAL MODE UI HANDLER
// ══════════════════════════════════════════
function applyManualModeUI() {
    const sel = document.getElementById('suggestStrategySelect');
    if (!sel) return;
    const isManual = sel.value === 'MANUAL';

    // Strategy card highlight
    const card = document.getElementById('strategyCardWrapper');
    if (card) {
        if (isManual) {
            card.style.border = '2px solid #F5A623';
            card.style.background = 'rgba(245,166,35,0.08)';
            card.style.boxShadow = '0 0 12px rgba(245,166,35,0.25)';
        } else {
            card.style.border = '1px solid var(--border)';
            card.style.background = 'var(--bg3)';
            card.style.boxShadow = 'none';
        }
    }

    // Hint text
    const hint = document.getElementById('strategyHintText');
    if (hint) {
        if (isManual) {
            hint.innerHTML = '🖐️ <strong style="color:#F5A623;">Manual Mode ON</strong> — บอทจะหยุด <em>ไม่เข้าเทรดอัตโนมัติ</em> รอรับคำสั่ง CALL/PUT จากคุณ';
        } else if (sel.value === 'PKTrend') {
            hint.innerHTML = '🎯 <strong style="color:#06D6A0;">PKTrend V1 Mode ON</strong> — หา Action จุดเข้าเทรดจาก pkTrend (Trend-following, Traps, Safe Whipsaw Filter)';
        } else if (sel.value === 'PKTrendV5') {
            hint.innerHTML = '🚀 <strong style="color:#00F0FF;">PKTrend V5 Mode ON</strong> — ระบบ Price Action V5 ครบ 27 รูปแบบ, Structure (HH/LL/HL/LH) & Whipsaw Guard';
        } else if (sel.value === 'ForecastSequence') {
            hint.innerHTML = '🔮 <strong style="color:#BD93F9;">Forecast Sequence Mode ON</strong> — ทำนายแท่งถัดไป & แมตช์คู่รหัส 2-Bar Sequence Action (Reversal Bull/Bear Trap)';
        } else if (sel.value === 'PKTrendSelectCaseCode') {
            hint.innerHTML = '📋 <strong style="color:#F39C12;">PKTrend CaseCode Mode ON</strong> — หา Action (Call / Put / Idle) ตามตาราง setup/strategy_case_codes.json ตาม code_no / case_code';
        } else {
            hint.innerHTML = '💡 เลือกกลยุทธ์ที่ใช้ตัดสินใจเมื่อเกิด ATR Spike';
        }
    }

    // Manual Trade section — badge & note
    const badge = document.getElementById('manualModeActiveBadge');
    const note = document.getElementById('manualModeStandbyNote');
    const section = document.getElementById('manual-trade-section');
    if (badge) badge.style.display = isManual ? 'inline-flex' : 'none';
    if (note) note.style.display = isManual ? 'none' : 'inline';
    if (section) {
        if (isManual) {
            section.style.border = '2px solid #F5A623';
            section.style.background = 'rgba(245,166,35,0.06)';
            section.style.boxShadow = '0 0 16px rgba(245,166,35,0.2)';
        } else {
            section.style.border = '1px solid var(--border)';
            section.style.background = 'var(--bg3)';
            section.style.boxShadow = 'none';
        }
    }

    // Enable/disable call&put buttons based on mode
    const callBtn2 = document.getElementById('manualCallBtn');
    const putBtn2 = document.getElementById('manualPutBtn');
    if (callBtn2) callBtn2.style.opacity = isManual ? '1' : '0.5';
    if (putBtn2) putBtn2.style.opacity = isManual ? '1' : '0.5';
}

async function autoSaveSetup() {
    updateJSON();
    const currentSettings = collectSettings();
    try {
        await fetch('/api/setup', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(currentSettings)
        });
    } catch (e) {
        console.error("Could not auto-save setup:", e);
    }
}

if (document.getElementById('suggestStrategySelect')) {
    document.getElementById('suggestStrategySelect').addEventListener('change', async () => {
        updateJSON();
        applyManualModeUI();
        if (typeof updateChartMarkers === 'function') updateChartMarkers();
        await autoSaveSetup();
    });
    // Apply initial state on page load
    setTimeout(applyManualModeUI, 600);
}

document.getElementById('useSchedule').addEventListener('change', function () {
    if (this.checked) {
        const now = new Date();
        const startVal = document.getElementById('startDate').value;
        const stopVal = document.getElementById('stopDate').value;
        if (startVal && stopVal) {
            const start = new Date(startVal);
            const stop = new Date(stopVal);
            if (start < now || stop < now) {
                alert('⚠️ คำเตือน: เวลาที่ตั้งไว้ใน Start Date หรือ Stop Date เป็นเวลาในอดีต! กรุณาตรวจสอบเวลาอีกครั้ง');
                this.checked = false;
                updateJSON();
            }
        }
    }
});

document.getElementById('fetchBtn').addEventListener('click', () => {
    if (!validateDates()) return;
    console.log('Fetching data...', collectSettings());
});

let isTradeRunning = false;
let tradeStartTime = null;
let tradeDurationInterval = null;

function updateTradingDuration() {
    const container = document.getElementById('trading-duration-container');
    if (!isTradeRunning || !tradeStartTime) {
        if (container) container.style.display = 'none';
        return;
    }

    const useSchedule = document.getElementById('useSchedule');
    if (!useSchedule || !useSchedule.checked) {
        if (container) container.style.display = 'none';
        return;
    }

    if (container) container.style.display = 'flex';

    const now = Date.now();
    const elapsedMs = now - tradeStartTime;
    const elapsedMins = Math.floor(elapsedMs / 60000);
    const elapsedSecs = Math.floor((elapsedMs % 60000) / 1000);

    const timeEl = document.getElementById('trading-time-elapsed');
    if (timeEl) timeEl.textContent = `ระยะเวลา: ${elapsedMins} นาที ${elapsedSecs} วินาที`;

    // Calculate percent if stopDate is set
    const stopVal = document.getElementById('stopDate').value;
    const startVal = document.getElementById('startDate').value;

    const bar = document.getElementById('trading-progress-bar');
    const percentEl = document.getElementById('trading-progress-percent');

    if (stopVal && startVal && bar && percentEl) {
        const start = new Date(startVal).getTime();
        const stop = new Date(stopVal).getTime();
        const totalDuration = stop - start;

        if (totalDuration > 0) {
            let percent = ((now - start) / totalDuration) * 100;
            percent = Math.min(100, Math.max(0, percent));

            bar.style.width = `${percent}%`;
            percentEl.textContent = `${Math.round(percent)}%`;

            const remainingMs = Math.max(0, stop - now);
            const remainingMins = Math.floor(remainingMs / 60000);
            const remainingSecs = Math.floor((remainingMs % 60000) / 1000);
            const remainingEl = document.getElementById('trading-time-remaining');
            if (remainingEl) {
                remainingEl.textContent = `เหลือเวลา: ${remainingMins} นาที ${remainingSecs} วินาที`;
            }

            if (percent > 90) {
                bar.style.background = 'var(--red)';
                percentEl.style.color = 'var(--red)';
            } else if (percent > 70) {
                bar.style.background = 'var(--orange)';
                percentEl.style.color = 'var(--orange)';
            } else {
                bar.style.background = 'linear-gradient(90deg, var(--green), #00d2ff)';
                percentEl.style.color = 'var(--green)';
            }
        } else {
            bar.style.width = '0%';
            percentEl.textContent = '0%';
        }
    } else if (bar && percentEl) {
        bar.style.width = '100%';
        bar.style.background = 'var(--accent)';
        percentEl.textContent = 'Active';
        percentEl.style.color = 'var(--accent)';
    }
}
document.getElementById('goTradeBtn').addEventListener('click', async () => {
    console.log('🎯 [Go Trade] Button clicked!');
    console.log('🎯 [Go Trade] validateDates():', validateDates());
    console.log('🎯 [Go Trade] isTradeRunning:', isTradeRunning);
    
    if (!validateDates()) {
        console.warn('❌ [Go Trade] Validation failed - dates invalid');
        return;
    }
    
    if (isTradeRunning) {
        console.warn('❌ [Go Trade] Trade is already running');
        return;
    }
    
    isTradeRunning = true;
    tradeStartTime = Date.now();
    if (tradeDurationInterval) clearInterval(tradeDurationInterval);
    tradeDurationInterval = setInterval(updateTradingDuration, 1000);
    updateTradingDuration();
    const settings = collectSettings();
    console.log('📤 [Go Trade] Sending Trade Config to Server...', settings);

    // Note: scheduleTradeNo is now managed by tradeControl.json on the server

    // ═══ Reconnect WS ก่อนเทรด (ถ้ายังไม่ได้เชื่อมต่อ) ═══
    shouldStayConnected = true;
    if (!localWs || localWs.readyState !== WebSocket.OPEN) {
        console.log('🔌 [Go Trade] Reconnecting WS before Go Trade...');
        connectRustWebSocket('trade');
        await new Promise(resolve => setTimeout(resolve, 1500));
    }

    try {
        console.log('📡 [Go Trade] Fetching /api/trade...');
        const response = await fetch('/api/trade', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(settings)
        });

        const data = await response.json();
        console.log("✅ [Go Trade] Server Response:", data);
        showToast(data.message);

        // อัปเดต UI ทันทีโดยใช้ข้อมูลจาก settings ไม่ต้องรอ fetch สถานะใหม่
        if (settings && settings.assets) {
            const assets = settings.assets;
            const botCount = assets.length;
            // นับจำนวนบอทให้ตรงกับ Backend (รวม 1 bot ต่อ 1 asset)
            const msg = `Trading Active (${botCount} bot${botCount > 1 ? 's' : ''}: ${assets.join(', ')})`;
            const connText = document.getElementById('connText');
            if (connText) connText.textContent = msg;
            const connTextBig = document.getElementById('connTextBig');
            if (connTextBig) connTextBig.textContent = msg;
        }
    } catch (e) {
        console.error("❌ [Go Trade] Error communicating with server:", e);
        isTradeRunning = false;
        if (tradeDurationInterval) clearInterval(tradeDurationInterval);
        tradeDurationInterval = null;
        updateTradingDuration();
        shouldStayConnected = false;
        showToast("Could not connect to the Backend Server.");
    }
});

document.getElementById('stopTradeBtn').addEventListener('click', async () => {
    console.log("Stopping Trade...");
    try {
        const response = await fetch('/api/stop', { method: 'POST' });
        const data = await response.json();
        console.log("Server Response:", data);
        if (data.status === 'success') {
            setConnectionStatus(false);
            document.getElementById('connText').textContent = 'Trade Stopped';
        }
        isTradeRunning = false;
        console.log('✅ [Stop Trade] isTradeRunning set to false');

        // User requested: Stop trade should disable schedule and revert UI
        const scheduleToggle = document.getElementById('useSchedule');
        if (scheduleToggle) scheduleToggle.checked = false;
        scheduleTradeCompleted = false; // 🔧 FIX: reset flag เมื่อ user กด Stop Trade
        if (typeof updateDigitalClock === 'function') updateDigitalClock();

        if (tradeDurationInterval) clearInterval(tradeDurationInterval);
        tradeDurationInterval = null;
        updateTradingDuration();
        // ═══ ปิด WS เพื่อประหยัด traffic หลังหยุดเทรด ═══
        shouldStayConnected = false;
        if (localWs) {
            localWs.onclose = null;
            localWs.close();
            localWs = null;
        }
        const ct = document.getElementById('connText');
        if (ct) ct.textContent = 'หยุดเทรดแล้ว (ประหยัด Traffic)';
        const ctb = document.getElementById('connTextBig');
        if (ctb) ctb.textContent = 'หยุดเทรดแล้ว — กด Go Trade เพื่อเริ่มใหม่';
        showToast(data.message || 'Trade Stopped');
        playStopSound();
    } catch (e) {
        console.error("Error stopping trade:", e);
        showToast("Could not connect to the Backend Server to Stop.");
    }
});

document.getElementById('terminateBtn').addEventListener('click', async () => {
    if (!confirm("⚠️ คำเตือน: คุณต้องการปิดโปรแกรม Backend ทั้งหมดและกลับสู่หน้าต่าง Prompt หรือไม่?")) return;
    isTradeRunning = false;
    if (tradeDurationInterval) clearInterval(tradeDurationInterval);
    tradeDurationInterval = null;
    updateTradingDuration();
    console.log("Terminating Server...");
    try {
        const response = await fetch('/api/terminate', { method: 'POST' });
        const data = await response.json();
        console.log("Server Response:", data);

        // อัปเดต UI ให้รู้ว่าปิดแล้ว
        setConnectionStatus(false);
        document.getElementById('connText').textContent = 'Server Terminated';
        const connTextBig = document.getElementById('connTextBig');
        if (connTextBig) connTextBig.textContent = 'Server Terminated';

        // แจ้งเตือนผู้ใช้
        showToast(data.message || 'Server Terminated');
        playStopSound();

        // ปิด WebSocket Local 
        if (localWs) {
            localWs.onclose = null; // กันไม่ให้มันเด้งพยายามต่อใหม่
            localWs.close();
        }
    } catch (e) {
        console.error("Error terminating server:", e);
        setConnectionStatus(false);
        document.getElementById('connText').textContent = 'Server Offline';
        const connTextBig = document.getElementById('connTextBig');
        if (connTextBig) connTextBig.textContent = 'Server Offline';
    }
});

// ══════════════════════════════════════════
//  2.5 MANUAL TRADE (CALL / PUT)
// ══════════════════════════════════════════
function updateManualTradeAssetDropdown() {
    const select = document.getElementById('manualTradeAsset');
    if (!select) return;
    const checked = Array.from(document.querySelectorAll('input[name="asset"]:checked')).map(el => el.value);
    const currentVal = select.value;
    select.innerHTML = '<option value="">— เลือก Asset —</option>';
    checked.forEach(asset => {
        const opt = document.createElement('option');
        opt.value = asset;
        opt.textContent = asset.toUpperCase();
        select.appendChild(opt);
    });
    // Restore previous selection if still valid
    if (checked.includes(currentVal)) {
        select.value = currentVal;
    } else if (checked.length === 1) {
        select.value = checked[0];
    }
}

// Update dropdown when asset checkboxes change
document.querySelectorAll('input[name="asset"]').forEach(el => {
    el.addEventListener('change', updateManualTradeAssetDropdown);
});
// Initial fill on page load
setTimeout(updateManualTradeAssetDropdown, 500);

// Hover effects
const callBtn = document.getElementById('manualCallBtn');
const putBtn = document.getElementById('manualPutBtn');
if (callBtn) {
    callBtn.addEventListener('mouseenter', () => { callBtn.style.transform = 'translateY(-2px)'; callBtn.style.boxShadow = '0 6px 20px rgba(6,214,160,0.4)'; });
    callBtn.addEventListener('mouseleave', () => { callBtn.style.transform = 'translateY(0)'; callBtn.style.boxShadow = '0 4px 12px rgba(6,214,160,0.3)'; });
}
if (putBtn) {
    putBtn.addEventListener('mouseenter', () => { putBtn.style.transform = 'translateY(-2px)'; putBtn.style.boxShadow = '0 6px 20px rgba(232,48,74,0.4)'; });
    putBtn.addEventListener('mouseleave', () => { putBtn.style.transform = 'translateY(0)'; putBtn.style.boxShadow = '0 4px 12px rgba(232,48,74,0.3)'; });
}

async function sendManualTrade(contractType) {
    const asset = document.getElementById('manualTradeAsset').value;
    const amount = parseFloat(document.getElementById('manualTradeAmount').value) || 1;

    if (!asset) {
        alert('⚠️ กรุณาเลือก Asset ก่อน!');
        return;
    }
    if (amount < 0.35) {
        alert('⚠️ จำนวนเงินต้องไม่น้อยกว่า $0.35!');
        return;
    }

    const emoji = contractType === 'CALL' ? '📈' : '📉';
    if (!confirm(`${emoji} ยืนยันเปิด ${contractType} $${amount.toFixed(2)} บน ${asset.toUpperCase()} ?`)) return;

    // Disable buttons briefly
    if (callBtn) callBtn.disabled = true;
    if (putBtn) putBtn.disabled = true;

    try {
        const response = await fetch('/api/manual_trade', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ asset, contract_type: contractType, amount })
        });
        const data = await response.json();
        console.log('Manual Trade Response:', data);
        if (data.status !== 'success') {
            alert('❌ ' + (data.message || 'เกิดข้อผิดพลาด'));
        }
    } catch (e) {
        console.error('Manual Trade Error:', e);
        alert('❌ ไม่สามารถเชื่อมต่อ Server ได้');
    } finally {
        setTimeout(() => {
            if (callBtn) callBtn.disabled = false;
            if (putBtn) putBtn.disabled = false;
        }, 2000);
    }
}

if (callBtn) callBtn.addEventListener('click', () => sendManualTrade('CALL'));
if (putBtn) putBtn.addEventListener('click', () => sendManualTrade('PUT'));

// ══════════════════════════════════════════
//  3. RUST WEBSOCKET CLIENT & DATA RENDER
// ══════════════════════════════════════════
let localWs;
let shouldStayConnected = false; // true เมื่อกำลังเทรดหรือต้องการ connect ถาวร
let lwChart = null;
let candleSeries = null;
let emaShortSeries = null;
let emaMediumSeries = null;
let emaLongSeries = null;
let shadeSeries = null;
let allAnalysisData = []; // ข้อมูลของ asset ที่กำลังแสดงอยู่
let allDataByAsset = {}; // เก็บข้อมูลแยกตาม asset { 'vol10': [...], 'vol25_1s': [...] }
let currentChartAsset = ''; // asset ที่กำลังแสดงบนกราฟ
window.currentTradeHistory = []; // เก็บข้อมูล trade history ที่โหลดมา
window.assetTradingStates = {}; // เก็บสถานะการเทรดของแต่ละ asset
window.activeOrders = {}; // เก็บข้อมูลออเดอร์ที่กำลังรันอยู่ { contract_id: data }
window.targetProfits = {}; // เก็บ target profit ของแต่ละ contract_id
window.orderProfitStats = {}; // เก็บ min/max profit ที่เกิดขึ้นจริงระหว่าง order { contract_id: {min, max} }

window.handleOrderUpdate = function (data) {
    if (data && data.contract_id) {
        if (data.status === 'won' || data.status === 'lost' || data.status === 'sold' || data.is_sold === 1) {
            delete window.activeOrders[data.contract_id];
            // Keep stats for a moment so last render shows final values, then clean up
            setTimeout(() => { delete window.orderProfitStats[data.contract_id]; }, 3000);
        } else {
            const isNewOrder = !window.activeOrders[data.contract_id];

            // ── Track Min / Max Profit ──────────────────────────
            const cid = data.contract_id;
            const currentProfit = typeof data.profit === 'number' ? data.profit : parseFloat(data.profit) || 0;
            if (!window.orderProfitStats[cid]) {
                // Initialize on first update
                window.orderProfitStats[cid] = { min: currentProfit, max: currentProfit };
            } else {
                if (currentProfit < window.orderProfitStats[cid].min) {
                    window.orderProfitStats[cid].min = currentProfit;
                }
                if (currentProfit > window.orderProfitStats[cid].max) {
                    window.orderProfitStats[cid].max = currentProfit;
                }
            }
            // Inject tracked min/max into the data object so the table can use them
            data.min_profit = window.orderProfitStats[cid].min;
            data.max_profit = window.orderProfitStats[cid].max;
            // ─────────────────────────────────────────────────────

            window.activeOrders[cid] = data;

            // Auto set target profit if enabled
            if (isNewOrder) {
                const autoToggle = document.getElementById('autoSellProfitToggle');
                const autoTargetInput = document.getElementById('autoSellProfitTarget');
                if (autoToggle && autoToggle.checked && autoTargetInput) {
                    const percentVal = parseFloat(autoTargetInput.value) || 0;
                    const buyPrice = parseFloat(data.buy_price) || 0;
                    if (percentVal > 0 && buyPrice > 0) {
                        const tval = buyPrice * percentVal;
                        window.targetProfits[cid] = tval;
                        console.log(`🎯 Auto-set Target Profit for ${cid} to $${tval.toFixed(2)} (${percentVal * 100}%)`);
                    }
                }
            }
        }
        if (typeof updateTrackOrdersTable === 'function') {
            updateTrackOrdersTable();
        }
    }
};

window.syncOpenOrdersFromDeriv = function () {
    let token = localStorage.getItem('API_KEY');
    if (!token) return;

    console.log("🔄 กำลังซิงค์ออเดอร์ที่ค้างอยู่จาก Deriv (portfolio)...");
    
    // เปลี่ยนจาก Public WS เป็นใช้ Local WS ที่ Backend จัดการ OTP ให้
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const ws = new WebSocket(`${protocol}//${window.location.host}/ws`);

    ws.onopen = () => {
        // ส่งคำสั่งผ่าน Local WS ไปยัง Backend
        ws.send(JSON.stringify({ 
            type: 'sync_portfolio',
            token: token 
        }));
    };

    ws.onmessage = (msg) => {
        try {
            const data = JSON.parse(msg.data);

            if (data.error) {
                console.error("Deriv Sync Error:", data.error.message);
                return;
            }

            if (data.msg_type === 'authorize') {
                ws.send(JSON.stringify({ portfolio: 1 }));
            }

            if (data.msg_type === 'portfolio') {
                const contracts = data.portfolio.contracts;
                if (contracts && contracts.length > 0) {
                    console.log(`📋 พบ ${contracts.length} ออเดอร์ค้างอยู่ กำลังติดตามต่อ...`);
                    contracts.forEach(c => {
                        ws.send(JSON.stringify({
                            proposal_open_contract: 1,
                            contract_id: c.contract_id,
                            subscribe: 1
                        }));
                    });
                } else {
                    console.log("🟢 ไม่มีออเดอร์ค้างอยู่ (Deriv Portfolio)");
                    ws.close();
                }
            }

            if (data.msg_type === 'proposal_open_contract') {
                if (data.proposal_open_contract) {
                    window.handleOrderUpdate(data.proposal_open_contract);
                }
            }
        } catch (e) {
            console.error("Error parsing Deriv sync msg:", e);
        }
    };

    ws.onerror = (e) => {
        console.error("Deriv WS Sync error:", e);
    };
};

let initialBalance = null;
let currentBalance = 0;

function updateBalanceSummary(balance) {
    currentBalance = balance;

    const formatCurrency = (val) => val.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });

    // 1. Balance ณ ปัจจุบัน
    document.getElementById('summary-initial-balance').textContent = `$${formatCurrency(currentBalance)}`;

    // คำนวณกำไรจาก Trade History
    const history = window.currentTradeHistory || [];
    const currentScheduleNo = parseInt(document.getElementById('scheduleTradeNo').value) || 1;
    const thbRate = parseFloat(document.getElementById('thb-rate').value) || 35;

    // 2. กำไรรอบนี้ (scheduleTradeNo)
    let scheduleProfitUsd = 0;
    let prevScheduleProfitUsd = 0;
    let currentStreaks = {};
    let maxLossStreak = 0;
    let maxLossAsset = "";

    history.forEach(t => {
        if ((t.scheduleTradeNo || 0) === currentScheduleNo) {
            scheduleProfitUsd += (t.ThisProfit || 0);

            const asset = t.Asset || 'Unknown';
            const profit = t.ThisProfit || 0;
            if (profit < 0) {
                currentStreaks[asset] = (currentStreaks[asset] || 0) + 1;
                if (currentStreaks[asset] > maxLossStreak) {
                    maxLossStreak = currentStreaks[asset];
                    maxLossAsset = asset;
                }
            } else if (profit > 0) {
                currentStreaks[asset] = 0;
            }
        } else if ((t.scheduleTradeNo || 0) === currentScheduleNo - 1) {
            prevScheduleProfitUsd += (t.ThisProfit || 0);
        }
    });

    const schedLabelEl = document.getElementById('summary-schedule-label');
    const schedLabelThbEl = document.getElementById('summary-schedule-label-thb');
    if (schedLabelEl) schedLabelEl.textContent = `#${currentScheduleNo}`;
    if (schedLabelThbEl) schedLabelThbEl.textContent = `#${currentScheduleNo}`;

    const schedEl = document.getElementById('summary-current-balance');
    let profitText = `${scheduleProfitUsd >= 0 ? '+' : ''}$${formatCurrency(scheduleProfitUsd)}`;

    let maxLossHtml = '';
    let colorBorder = "rgba(0,0,0,0.1)";
    if (document.body.getAttribute('data-theme') === 'dark' || document.body.getAttribute('data-theme') === 'midnight') {
        colorBorder = "rgba(255,255,255,0.1)";
    }

    if (maxLossStreak > 0) {
        maxLossHtml = `<hr style="border:none; border-top:1px solid ${colorBorder}; margin: 6px 0;">
                               <div style="font-size: 11px; color: var(--red); font-weight: normal; text-transform: none; line-height: 1.2;">
                                   Max Loss Streak=${maxLossStreak} On ${maxLossAsset.toUpperCase()}
                               </div>`;
    }

    let prevProfitHtml = '';
    if (currentScheduleNo > 1) {
        let prevProfitColor = prevScheduleProfitUsd >= 0 ? 'var(--green)' : 'var(--red)';
        let prevProfitText = `${prevScheduleProfitUsd >= 0 ? '+' : ''}$${formatCurrency(prevScheduleProfitUsd)}`;
        prevProfitHtml = `<hr style="border:none; border-top:1px dashed ${colorBorder}; margin: 6px 0;">
                                  <div style="font-size: 12px; color: var(--text2); font-weight: normal; text-transform: none; line-height: 1.2;">
                                      กำไรรอบที่ ${currentScheduleNo - 1} :: <span style="color: ${prevProfitColor}; font-weight: bold;">${prevProfitText}</span>
                                  </div>`;
    }

    schedEl.innerHTML = `${profitText}${maxLossHtml}${prevProfitHtml}`;
    schedEl.style.color = scheduleProfitUsd >= 0 ? 'var(--green)' : 'var(--red)';

    const scheduleProfitThb = scheduleProfitUsd * thbRate;
    const schedThbEl = document.getElementById('summary-schedule-thb');
    let thbProfitText = `${scheduleProfitThb >= 0 ? '+' : ''}฿${formatCurrency(scheduleProfitThb)}`;

    let prevThbProfitHtml = '';
    if (currentScheduleNo > 1) {
        let prevScheduleProfitThb = prevScheduleProfitUsd * thbRate;
        let prevProfitColor = prevScheduleProfitThb >= 0 ? 'var(--green)' : 'var(--red)';
        let prevProfitText = `${prevScheduleProfitThb >= 0 ? '+' : ''}฿${formatCurrency(prevScheduleProfitThb)}`;
        prevThbProfitHtml = `<hr style="border:none; border-top:1px dashed ${colorBorder}; margin: 6px 0;">
                                  <div style="font-size: 12px; color: var(--text2); font-weight: normal; text-transform: none; line-height: 1.2;">
                                      กำไรรอบที่ ${currentScheduleNo - 1} :: <span style="color: ${prevProfitColor}; font-weight: bold;">${prevProfitText}</span>
                                  </div>`;
    }

    schedThbEl.innerHTML = `${thbProfitText}${prevThbProfitHtml}`;
    schedThbEl.style.color = scheduleProfitThb >= 0 ? 'var(--green)' : 'var(--red)';

    // 3. กำไรวันนี้ (รวมทุกรอบ)
    let todayProfitUsd = 0;
    history.forEach(t => {
        todayProfitUsd += (t.ThisProfit || 0);
    });

    const profitEl = document.getElementById('summary-profit-usd');
    profitEl.textContent = `${todayProfitUsd >= 0 ? '+' : ''}$${formatCurrency(todayProfitUsd)}`;
    profitEl.style.color = todayProfitUsd >= 0 ? 'var(--green)' : 'var(--red)';

    const todayProfitThb = todayProfitUsd * thbRate;
    const thbEl = document.getElementById('summary-profit-thb');
    thbEl.textContent = `${todayProfitThb >= 0 ? '+' : ''}฿${formatCurrency(todayProfitThb)}`;
    thbEl.style.color = todayProfitThb >= 0 ? 'var(--green)' : 'var(--red)';
}

document.addEventListener('DOMContentLoaded', () => {
    const thbInput = document.getElementById('thb-rate');
    if (thbInput) {
        thbInput.addEventListener('input', () => {
            if (currentBalance !== 0) updateBalanceSummary(currentBalance);
        });
    }

    // Listen to checkbox changes to update the active asset buttons
    document.querySelectorAll('input[name="asset"]').forEach(el => {
        el.addEventListener('change', () => {
            updateActiveAssetButtons();
            if (typeof updateChartAssetButtons === 'function') updateChartAssetButtons();
        });
    });

    // Fetch initial USD to THB rate
    fetchExchangeRate();
});

async function fetchExchangeRate() {
    try {
        const res = await fetch('https://api.exchangerate-api.com/v4/latest/USD');
        const data = await res.json();
        if (data && data.rates && data.rates.THB) {
            const rate = data.rates.THB;
            const thbRateEl = document.getElementById('thb-rate');
            if (thbRateEl) {
                thbRateEl.value = rate.toFixed(2);
                console.log(`💱 Updated THB Rate: ${rate.toFixed(2)} ฿/$`);
                if (currentBalance !== 0) updateBalanceSummary(currentBalance);
            }
        }
    } catch (e) {
        console.error("Failed to fetch exchange rate", e);
    }
}

window.fetchGlobalStrategyData = async function () {
    try {
        const now = new Date();
        const yyyy = now.getFullYear();
        const mm = String(now.getMonth() + 1).padStart(2, '0');
        const dd = String(now.getDate()).padStart(2, '0');
        const dayTrade = `${yyyy}-${mm}-${dd}`;

        const response = await fetch(`/api/getStrategyComparison?dayTrade=${dayTrade}`);
        if (response.ok) {
            const data = await response.json();
            window.strategyComparisonData = data;
        }
    } catch (e) {
        console.error("Failed to fetch global strategy data", e);
    }
};

function updateChartAssetButtons() {
    const select = document.getElementById('chartAssetSelect');
    if (!select) return;

    const checked = Array.from(document.querySelectorAll('input[name="asset"]:checked')).map(el => el.value);

    if (checked.length === 0) {
        select.innerHTML = '<span style="color:var(--text3); font-size:12px; padding-top:4px;">— รอข้อมูล —</span>';
        return;
    }

    select.innerHTML = '';
    let html = '';
    checked.forEach(asset => {
        let displayName = asset.toUpperCase();
        if (displayName.includes('_1S')) displayName = displayName.replace('_1S', '');

        let isActive = (asset === currentChartAsset);
        let bg = isActive ? 'var(--accent)' : 'var(--bg3)';
        let color = isActive ? '#fff' : 'var(--text)';
        let border = isActive ? 'var(--accent)' : 'var(--border)';

        html += `<button id="chart-btn-${asset}" class="chart-asset-btn" data-asset="${asset}" onclick="window.switchChartAsset('${asset}')" style="padding:4px 14px; border-radius:12px; border:1px solid ${border}; background:${bg}; color:${color}; font-size:12px; cursor:pointer; font-weight:600; transition:all 0.2s; letter-spacing:0.5px;">${displayName}</button>`;
    });

    select.innerHTML = html;

    if (!currentChartAsset && checked.length > 0) {
        window.switchChartAsset(checked[0]);
    }
}

function updateActiveAssetButtons() {
    const container = document.getElementById('active-assets-container');
    if (!container) return;

    const checked = Array.from(document.querySelectorAll('input[name="asset"]:checked')).map(el => el.value);

    if (checked.length === 0) {
        container.innerHTML = '<div style="font-size:12px; color:var(--text3);">ไม่มี Asset ที่ถูกเลือก</div>';
        return;
    }

    let html = '';
    checked.forEach(asset => {
        if (!window.assetTradingStates[asset]) {
            window.assetTradingStates[asset] = { isTrading: false, lossCon: 0 };
        }

        let state = window.assetTradingStates[asset];
        let bg = state.isTrading ? 'var(--green)' : 'var(--bg3)';
        let color = state.isTrading ? '#fff' : 'var(--text)';
        let text = state.isTrading ? `${asset.toUpperCase()} [LC:${state.lossCon}]` : asset.toUpperCase();
        let border = state.isTrading ? '1px solid var(--green)' : '1px solid var(--border)';

        html += `<div id="btn-asset-${asset}" style="padding:6px 12px; border-radius:20px; font-size:12px; font-weight:bold; background:${bg}; color:${color}; border:${border}; transition:all 0.3s; display:flex; align-items:center; justify-content:center; min-width:80px;">
                    ${text}
                </div>`;
    });

    container.innerHTML = html;
}

function getChartThemeColors() {
    const cs = getComputedStyle(document.body);
    const theme = document.body.getAttribute('data-theme') || 'lightblue';
    const isDark = (theme === 'dark' || theme === 'midnight');
    return {
        background: cs.getPropertyValue('--bg2').trim() || (isDark ? '#172130' : '#ffffff'),
        textColor: cs.getPropertyValue('--text').trim() || (isDark ? '#e0eaf8' : '#1a2535'),
        gridColor: cs.getPropertyValue('--border').trim() || (isDark ? '#2e4055' : '#c8d8eb'),
        isDark
    };
}

let chartResizeObserver = null;

function initChart() {
    const container = document.getElementById('chart');

    // ตรวจว่า chart ยังอยู่ใน DOM จริงๆ (มี canvas ข้างใน)
    const chartStillInDom = lwChart && candleSeries && container.querySelector('canvas');
    if (chartStillInDom) {
        console.log('📊 initChart: chart exists, skip recreate');
        return;
    }

    // ถ้า chart เก่ามีอยู่แต่ DOM หายไป → ทำลายก่อน
    if (lwChart) {
        try { lwChart.remove(); } catch (e) { }
        lwChart = null;
        candleSeries = null;
        emaShortSeries = null;
        emaMediumSeries = null;
        emaLongSeries = null;
        shadeSeries = null;
    }

    container.innerHTML = '';
    console.log('📊 initChart: creating new chart, container width =', container.clientWidth);

    const colors = getChartThemeColors();
    const TH_OFFSET = 7 * 60 * 60;
    const chartWidth = container.clientWidth > 0 ? container.clientWidth : 800;

    try {
        lwChart = LightweightCharts.createChart(container, {
            width: chartWidth,
            height: 420,
            layout: {
                background: { type: 'solid', color: colors.background },
                textColor: colors.textColor,
                fontFamily: "'Sarabun', sans-serif",
                fontSize: 12,
            },
            grid: {
                vertLines: { color: colors.gridColor, style: 1 },
                horzLines: { color: colors.gridColor, style: 1 },
            },
            crosshair: { mode: LightweightCharts.CrosshairMode.Normal },
            rightPriceScale: { borderColor: colors.gridColor },
            timeScale: {
                borderColor: colors.gridColor,
                timeVisible: true,
                secondsVisible: false,
                rightOffset: parseInt(document.getElementById('chartRightOffset').value) || 5,
            },
            handleScroll: { vertTouchDrag: false },
            localization: {
                timeFormatter: (timestamp) => {
                    const d = new Date((timestamp + TH_OFFSET) * 1000);
                    const hh = String(d.getUTCHours()).padStart(2, '0');
                    const mm = String(d.getUTCMinutes()).padStart(2, '0');
                    return `${hh}:${mm}`;
                },
            },
        });

        candleSeries = lwChart.addCandlestickSeries({
            upColor: '#06D6A0',
            downColor: '#E8304A',
            borderDownColor: '#E8304A',
            borderUpColor: '#06D6A0',
            wickDownColor: '#E8304A',
            wickUpColor: '#06D6A0',
        });

        shadeSeries = lwChart.addHistogramSeries({
            color: 'rgba(255, 193, 7, 0.15)',
            priceFormat: { type: 'volume' },
            priceScaleId: '', // Overlay
            scaleMargins: { top: 0, bottom: 0 },
        });

        emaShortSeries = lwChart.addLineSeries({
            color: document.getElementById('emaShortColor') ? document.getElementById('emaShortColor').value : '#06D6A0',
            lineWidth: 2,
            crosshairMarkerVisible: false,
            lastValueVisible: false,
            priceLineVisible: false,
        });

        emaMediumSeries = lwChart.addLineSeries({
            color: document.getElementById('emaMediumColor') ? document.getElementById('emaMediumColor').value : '#F5A623',
            lineWidth: 2,
            crosshairMarkerVisible: false,
            lastValueVisible: false,
            priceLineVisible: false,
        });

        emaLongSeries = lwChart.addLineSeries({
            color: document.getElementById('emaLongColor') ? document.getElementById('emaLongColor').value : '#E8304A',
            lineWidth: 2,
            crosshairMarkerVisible: false,
            lastValueVisible: false,
            priceLineVisible: false,
        });

        // Responsive resize — สร้าง ResizeObserver แค่ครั้งเดียว
        if (chartResizeObserver) chartResizeObserver.disconnect();
        chartResizeObserver = new ResizeObserver(() => {
            if (lwChart && container.clientWidth > 0) {
                lwChart.applyOptions({ width: container.clientWidth });
            }
        });
        chartResizeObserver.observe(container);

        // Tooltip setup
        let chartTooltip = container.querySelector('.chart-tooltip-panel');
        if (!chartTooltip) {
            chartTooltip = document.createElement('div');
            chartTooltip.className = 'chart-tooltip-panel';
            chartTooltip.style.display = 'none';
            chartTooltip.style.position = 'absolute';
            chartTooltip.style.zIndex = '1000';
            chartTooltip.style.background = 'var(--surface)';
            chartTooltip.style.border = '1px solid var(--border)';
            chartTooltip.style.padding = '12px';
            chartTooltip.style.borderRadius = 'var(--radius)';
            chartTooltip.style.pointerEvents = 'none';
            chartTooltip.style.fontSize = '12px';
            chartTooltip.style.color = 'var(--text)';
            chartTooltip.style.boxShadow = '0 4px 12px rgba(0,0,0,0.15)';
            chartTooltip.style.width = '320px';
            container.style.position = 'relative'; // Ensure container is relative
            container.appendChild(chartTooltip);
        }

        lwChart.subscribeCrosshairMove(param => {
            const tooltipToggle = document.getElementById('showStrategyTooltipToggle');
            if (!tooltipToggle || !tooltipToggle.checked || !param.time || param.point.x < 0 || param.point.y < 0 || param.point.x > container.clientWidth || param.point.y > container.clientHeight) {
                chartTooltip.style.display = 'none';
                return;
            }

            const candleTime = param.time - TH_OFFSET_SEC;
            const strategyData = window.strategyComparisonData || [];

            const getCleanAsset = (a) => {
                if (!a) return "";
                let s = a.toLowerCase();
                if (s.startsWith('r_')) return s.replace('r_', 'vol');
                if (s.startsWith('1hz') && s.endsWith('v')) return s.replace('1hz', 'vol').replace('v', '_1s');
                return s;
            };
            const cAsset = getCleanAsset(currentChartAsset);

            const row = strategyData.find(d => d.candleTimestamp === candleTime && getCleanAsset(d.assetCode) === cAsset);

            if (row) {
                const formatStrategy = (reason, loss, winStatus, suggest, code) => {
                    let winColor = winStatus === 'Win' ? 'var(--green)' : (winStatus === 'Loss' ? 'var(--red)' : 'var(--text)');
                    let emoji = suggest === 'green' ? '🟢' : (suggest === 'red' ? '🔴' : (suggest || '-'));
                    let reasonText = reason || '-';
                    let codeHtml = code ? `<span style="color:#f5a623; font-weight:bold; margin-right:6px;">${code}</span>` : '';
                    return `<div style="color:var(--text2); font-size:11px; margin-bottom:2px; line-height:1.2;">${reasonText}</div>
                                    <div>${codeHtml}${emoji} / ${loss || 0} / <span style="font-weight:bold; color:${winColor}">${winStatus || '-'}</span></div>`;
                };

                const thisEmoji = row.thisColor === 'green' ? '🟢' : (row.thisColor === 'red' ? '🔴' : '-');
                const actEmoji = row.actualNextColor === 'green' ? '🟢' : (row.actualNextColor === 'red' ? '🔴' : '-');

                const getStrategyLabel = (strat) => {
                    const style = row.activeStrategy === strat ? 'color:var(--green); font-weight:bold;' : 'font-weight:bold;';
                    return `<span style="${style}">${strat}:</span>`;
                };

                chartTooltip.innerHTML = `
                            <table style="width:100%; border-collapse:collapse; margin:0; font-size:12px;">
                                <tr>
                                    <td colspan="2" style="font-weight:bold; font-size:13px; color:var(--accent); border-bottom:1px solid var(--border); padding-bottom:6px;">Strategy Data (${row.candleTimeDisp})</td>
                                </tr>
                                <tr>
                                    <td colspan="2" style="padding:6px 0;"><strong>Color Path:</strong> <span style="font-size:14px;">${thisEmoji} ➜ ${actEmoji}</span></td>
                                </tr>
                                <tr style="border-bottom:1px solid rgba(255,255,255,0.05);">
                                    <td style="padding:4px 0; width:45px; vertical-align:top;">${getStrategyLabel('V1')}</td>
                                    <td style="padding:4px 0;">${formatStrategy(row.v1Reason, row.lossConByV1, row.winStatusByV1, row.suggestColorByV1, row.v1Code)}</td>
                                </tr>
                                <tr style="border-bottom:1px solid rgba(255,255,255,0.05);">
                                    <td style="padding:4px 0; vertical-align:top;">${getStrategyLabel('V2')}</td>
                                    <td style="padding:4px 0;">${formatStrategy(row.v2Reason, row.lossConByV2, row.winStatusByV2, row.suggestColorByV2, row.v2Code)}</td>
                                </tr>
                                <tr style="border-bottom:1px solid rgba(255,255,255,0.05);">
                                    <td style="padding:4px 0; vertical-align:top;">${getStrategyLabel('V3A')}</td>
                                    <td style="padding:4px 0;">${formatStrategy(row.v3aReason, row.lossConByV3A, row.winStatusByV3A, row.suggestColorByV3A, row.v3aCode)}</td>
                                </tr>
                                <tr style="border-bottom:1px solid rgba(255,255,255,0.05);">
                                    <td style="padding:4px 0; vertical-align:top;">${getStrategyLabel('V3B')}</td>
                                    <td style="padding:4px 0;">${formatStrategy(row.v3bReason, row.lossConByV3B, row.winStatusByV3B, row.suggestColorByV3B, row.v3bCode)}</td>
                                </tr>
                                <tr>
                                    <td style="padding:4px 0; vertical-align:top;">${getStrategyLabel('V3C')}</td>
                                    <td style="padding:4px 0;">${formatStrategy(row.v3cReason, row.lossConByV3C, row.winStatusByV3C, row.suggestColorByV3C, row.v3cCode)}</td>
                                </tr>
                                <tr style="border-bottom:1px solid rgba(255,255,255,0.05);">
                                    <td style="padding:4px 0; vertical-align:top;">${getStrategyLabel('FTA')}</td>
                                    <td style="padding:4px 0;">${formatStrategy(row.ftaReason, row.lossConByFTA, row.winStatusByFTA, row.suggestColorByFTA, row.ftaCode)}</td>
                                </tr>
                                <tr>
                                    <td style="padding:4px 0; vertical-align:top;">${getStrategyLabel('FTB')}</td>
                                    <td style="padding:4px 0;">${formatStrategy(row.ftbReason, row.lossConByFTB, row.winStatusByFTB, row.suggestColorByFTB, row.ftbCode)}</td>
                                </tr>
                            </table>
                        `;
                chartTooltip.style.display = 'block';

                const tooltipHeight = chartTooltip.offsetHeight;

                let y = param.point.y;
                let x = param.point.x + 15;

                // Prevent overflow on the right
                if (x + 320 > container.clientWidth) {
                    x = param.point.x - 335;
                }

                // Prevent overflow on the bottom
                if (y + tooltipHeight > container.clientHeight) {
                    y = y - tooltipHeight - 15; // Move above the cursor
                    // Ensure it doesn't go off the top edge
                    if (y < 0) y = 10;
                }

                chartTooltip.style.left = x + 'px';
                chartTooltip.style.top = y + 'px';
            } else {
                chartTooltip.style.display = 'none';
            }
        });

        console.log('📊 initChart: chart created successfully');
    } catch (err) {
        console.error('❌ initChart error:', err);
        lwChart = null;
        candleSeries = null;
    }
}

// แปลง epoch UTC → epoch ที่ shift +7 ชม. สำหรับ Lightweight Charts
const TH_OFFSET_SEC = 7 * 60 * 60;
function toThaiTime(epoch) {
    return epoch + TH_OFFSET_SEC;
}

window.activeJsonMarkerMode = null;

function showJsonFlatMarkers() {
    window.activeJsonMarkerMode = 'flat';
    if (typeof updateChartMarkers === 'function') updateChartMarkers();
}

function showJsonGapMarkers() {
    window.activeJsonMarkerMode = 'gap';
    if (typeof updateChartMarkers === 'function') updateChartMarkers();
}

function showAlternatingPatternMarkers() {
    window.activeJsonMarkerMode = 'alt';
    if (typeof updateChartMarkers === 'function') updateChartMarkers();
}

function showAllJsonMarkers() {
    window.activeJsonMarkerMode = 'all';
    if (typeof updateChartMarkers === 'function') updateChartMarkers();
}

function updateChartMarkers() {
    if (!candleSeries) return;
    const showTradeMarkers = document.getElementById('showTradeMarkersToggle') ? document.getElementById('showTradeMarkersToggle').checked : false;
    
    console.log(`🔍 [updateChartMarkers] showTradeMarkers: ${showTradeMarkers}, has currentTradeHistory: ${!!window.currentTradeHistory}, count: ${window.currentTradeHistory ? window.currentTradeHistory.length : 0}`);
    
    let markers = [];
    if (showTradeMarkers && window.currentTradeHistory && window.currentTradeHistory.length > 0) {
        const getCleanAsset = (a) => {
            if (!a) return "";
            let s = a.toLowerCase();
            // แปลง R_10, R_50, R_75 → vol10, vol50, vol75
            if (s.startsWith('r_')) return s.replace('r_', 'vol');
            // แปลง 1HZ10V, 1HZ50V, 1HZ75V → vol10_1s, vol50_1s, vol75_1s
            if (s.startsWith('1hz') && s.endsWith('v')) {
                const num = s.replace('1hz', '').replace('v', '');
                return `vol${num}_1s`;
            }
            return s;
        };
        const cAsset = getCleanAsset(currentChartAsset);
        
        console.log(`🔍 [Asset Name] currentChartAsset: ${currentChartAsset} → cleaned: ${cAsset}`);

        const relevantTrades = window.currentTradeHistory.filter(t => {
            const tradeAsset = t.Asset || t.assetCode || '';
            const cleanTradeAsset = getCleanAsset(tradeAsset);
            return cleanTradeAsset === cAsset;
        });
        
        // Debug: แสดง asset names ใน trade history
        if (window.currentTradeHistory.length > 0 && relevantTrades.length === 0) {
            const sampleAssets = window.currentTradeHistory.slice(0, 5).map(t => {
                const orig = t.Asset || t.assetCode || 'unknown';
                return `${orig} → ${getCleanAsset(orig)}`;
            });
            console.log(`🔍 [Asset Mismatch] Looking for: ${cAsset}, Sample assets in history:`, sampleAssets);
        }
        
        console.log(`📊 [Trade History] currentAsset: ${cAsset}, relevantTrades: ${relevantTrades.length}, total history: ${window.currentTradeHistory.length}`);

        // เรียงตามเวลา (อดีต → ปัจจุบัน)
        const sorted = relevantTrades.slice().sort((a, b) => (a.timeCandle || 0) - (b.timeCandle || 0));

        // จัดกลุ่มเป็น Martingale sequence:
        // sequence ใหม่เริ่มเมื่อ lossCon = 0, ต่อเนื่องเมื่อ lossCon > 0
        const sequences = [];
        let currentSeq = [];

        sorted.forEach(trade => {
            const lossCon = trade.lossCon !== undefined ? trade.lossCon : 0;
            if (lossCon === 0 && currentSeq.length > 0) {
                sequences.push(currentSeq);
                currentSeq = [trade];
            } else {
                currentSeq.push(trade);
            }
        });
        if (currentSeq.length > 0) sequences.push(currentSeq);

        // แต่ละ sequence → แสดง marker ทุกตัวใน sequence เพื่อแสดงข้อมูล loss ด้วย
        sequences.forEach(seq => {
            seq.forEach(trade => {
                const timeEpoch = trade.timeCandle ? toThaiTime(trade.timeCandle) : 0;
                if (timeEpoch <= 0) return;

                const isWin = (trade.ThisProfit || 0) > 0;
                const isLoss = (trade.ThisProfit || 0) < 0;
                const lossCon = trade.lossCon !== undefined ? trade.lossCon : 0;
                const subTradeno = trade.subTradeno !== undefined ? trade.subTradeno : 1;

                // ชนะไม้แรก (lossCon=0) = เรื่องปกติ → ไม่ต้องแสดง marker
                if (isWin && lossCon === 0) return;

                let mColor = isWin ? '#06D6A0' : (isLoss ? '#E8304A' : '#F5A623');
                let mShape = isWin ? 'arrowUp' : 'arrowDown';
                let mPos = isWin ? 'belowBar' : 'aboveBar';

                // 🔧 NEW: เปลี่ยน marker text เป็น C/P + ✅/❌ + winCon/lossCon
                const contractType = (trade.thisAction || 'CALL').toUpperCase();
                const typeSymbol = contractType === 'CALL' ? 'C' : 'P';
                
                console.log(`📍 [Marker Debug] trade.thisAction: ${trade.thisAction}, typeSymbol: ${typeSymbol}, isWin: ${isWin}, lossCon: ${lossCon}, subTradeno: ${subTradeno}`);
                
                let mText;
                if (isWin) {
                    // Win: แสดง winCon (= จำนวนครั้งที่แพ้มาก่อนหน้า + 1 = ชนะในไม้ที่เท่าไหร่)
                    const winCon = lossCon + 1;
                    mText = `${typeSymbol}✅${winCon}`;
                } else {
                    // Loss: แสดง lossCon (= จำนวนครั้งที่แพ้ติดกัน)
                    const displayLossCon = subTradeno;
                    const warningIcon = displayLossCon > 4 ? '⚠️' : '';
                    mText = `${warningIcon}${typeSymbol}❌${displayLossCon}`;
                }

                markers.push({
                    time: timeEpoch,
                    position: mPos,
                    color: mColor,
                    shape: mShape,
                    text: mText
                });
            });
        });
    }

    // Add JSON UI Markers
    if (window.activeJsonMarkerMode && typeof allAnalysisData !== 'undefined' && allAnalysisData && allAnalysisData.length > 0) {
        const mode = window.activeJsonMarkerMode;
        let flatCount = 0;
        let gapCount = 0;
        let altCount = 0;

        // Flat Check
        if (mode === 'flat' || mode === 'all') {
            const flatSelect = document.getElementById('jsonFlatSelect');
            if (flatSelect) {
                const flatType = flatSelect.value;
                allAnalysisData.forEach(d => {
                    if (d[flatType] === 'y' || d[flatType] === 'yes' || d[flatType] === true) {
                        flatCount++;
                        markers.push({
                            time: toThaiTime(d.candletime),
                            position: 'aboveBar',
                            color: '#FFD700',
                            shape: 'circle',
                            text: 'FLAT'
                        });
                    }
                });
            }
        }

        // Gap Check
        if (mode === 'gap' || mode === 'all') {
            const gapSelect = document.getElementById('jsonGapSelect');
            if (gapSelect) {
                const gapType = gapSelect.value;
                allAnalysisData.forEach(d => {
                    if (d[gapType] === 'y' || d[gapType] === 'yes' || d[gapType] === true) {
                        gapCount++;
                        markers.push({
                            time: toThaiTime(d.candletime),
                            position: 'aboveBar',
                            color: '#00FFFF',
                            shape: 'circle',
                            text: 'GAP'
                        });
                    }
                });
            }
        }

        // Alt Pattern Check
        if (mode === 'alt' || mode === 'all') {
            allAnalysisData.forEach(item => {
                const isAltPattern = item.is_alternating_pattern === true || item.is_alternating_pattern === 'y' || String(item.is_alternating_pattern).toLowerCase() === 'yes';
                const isAltSpike = item.is_alternating_spike === true || item.is_alternating_spike === 'y' || String(item.is_alternating_spike).toLowerCase() === 'yes';
                const isAltTrigger = item.is_alternating_trigger === true || item.is_alternating_trigger === 'y' || String(item.is_alternating_trigger).toLowerCase() === 'yes';

                if (isAltPattern && !isAltSpike && !isAltTrigger) {
                    altCount++;
                    markers.push({
                        time: toThaiTime(item.candletime),
                        position: 'aboveBar',
                        color: '#ff3c6e',
                        shape: 'circle',
                        text: 'ALT',
                        size: 1
                    });
                }
            });
        }

        const totalCandles = allAnalysisData.length;
        if (document.getElementById('flatCountText')) {
            document.getElementById('flatCountText').innerText = (mode === 'flat' || mode === 'all') ? `พบ Flat ${flatCount} รายการ จาก ${totalCandles} แท่ง` : '';
        }
        if (document.getElementById('gapCountText')) {
            document.getElementById('gapCountText').innerText = (mode === 'gap' || mode === 'all') ? `พบ Gap ${gapCount} รายการ จาก ${totalCandles} แท่ง` : '';
        }
        if (document.getElementById('altCountText')) {
            document.getElementById('altCountText').innerText = (mode === 'alt' || mode === 'all') ? `พบ Alt ${altCount} รายการ จาก ${totalCandles} แท่ง` : '';
        }
        if (document.getElementById('totalCountText')) {
            document.getElementById('totalCountText').innerText = (mode === 'all') ? `พบรวมทั้งหมด ${flatCount + gapCount + altCount} รายการ` : '';
        }
    } else {
        if (document.getElementById('flatCountText')) document.getElementById('flatCountText').innerText = '';
        if (document.getElementById('gapCountText')) document.getElementById('gapCountText').innerText = '';
        if (document.getElementById('altCountText')) document.getElementById('altCountText').innerText = '';
        if (document.getElementById('totalCountText')) document.getElementById('totalCountText').innerText = '';
    }

    const strategySelect = document.getElementById('suggestStrategySelect');
    const isPKTrend = strategySelect && strategySelect.value === 'PKTrend';

    if (isPKTrend) {
        // เมื่อเลือกใช้กลยุทธ์ PKTrend: วาด Marker Call / Put (Up arrow / Down arrow) และไม่แสดง EMA Cut
        if (typeof allAnalysisData !== 'undefined' && allAnalysisData && allAnalysisData.length > 0) {
            const opt = {
                minTrendScoreStrong: 40,
                minCloseConvictionUp: 0.70,
                maxCloseConvictionDown: 0.30,
                allowTrapReversal: true,
                allowSpikeContinuation: true
            };

            allAnalysisData.forEach(candle => {
                if (!candle || !candle.pkTrend) return;
                let decision = null;
                if (typeof evaluatePkTrendEntry === 'function') {
                    decision = evaluatePkTrendEntry(candle.pkTrend, opt);
                } else if (typeof window.StrategyEngine !== 'undefined' && window.StrategyEngine.getSuggestColorPKTrendWithReason) {
                    const res = window.StrategyEngine.getSuggestColorPKTrendWithReason(candle, 0, allAnalysisData);
                    if (res.suggest_color === 'green') decision = { signal: 'BUY', reason: res.reason };
                    else if (res.suggest_color === 'red') decision = { signal: 'SELL', reason: res.reason };
                }

                if (decision && decision.signal && decision.signal !== 'WAIT') {
                    const isBuy = decision.signal === 'BUY';
                    // 🔧 เปลี่ยน signal marker เป็น C/P แทน Call/Put
                    const signalText = isBuy ? 'C' : 'P';
                    markers.push({
                        time: toThaiTime(candle.candletime),
                        position: isBuy ? 'belowBar' : 'aboveBar',
                        color: isBuy ? '#06D6A0' : '#E8304A',
                        shape: isBuy ? 'arrowUp' : 'arrowDown',
                        text: signalText,
                        size: 2
                    });
                }
            });
        }
    } else {
        // กลยุทธ์อื่นๆ แสดง EMA Cut Markers ตามปกติ
        if (window.emaCutCustomMarkers && window.emaCutCustomMarkers[currentChartAsset]) {
            markers.push(...window.emaCutCustomMarkers[currentChartAsset]);
        }
    }

    markers.sort((a, b) => a.time - b.time);
    candleSeries.setMarkers(markers);
}

function renderChart(analysisList) {
    if (!lwChart || !candleSeries) initChart();

    const showSpike = document.getElementById('spikeShowToggle').checked;
    const greenSpikeColor = document.getElementById('spikeGreenColor').value;
    const redSpikeColor = document.getElementById('spikeRedColor').value;

    let greenCount = 0;
    let redCount = 0;

    const chartData = analysisList.map(d => {
        let colorProps = {};
        if (showSpike && d.is_atr) {
            if (d.color === 'green') {
                colorProps = { color: greenSpikeColor, wickColor: greenSpikeColor, borderColor: greenSpikeColor };
                greenCount++;
            } else if (d.color === 'red') {
                colorProps = { color: redSpikeColor, wickColor: redSpikeColor, borderColor: redSpikeColor };
                redCount++;
            }
        }

        return {
            time: toThaiTime(d.candletime),
            open: d.open,
            high: d.high,
            low: d.low,
            close: d.close,
            ...colorProps
        };
    });

    document.getElementById('spike-green-count').textContent = greenCount;
    document.getElementById('spike-red-count').textContent = redCount;

    candleSeries.setData(chartData);

    const showShade = document.getElementById('shadeAdxChoppyToggle') ? document.getElementById('shadeAdxChoppyToggle').checked : false;
    const shadeAdxVal = document.getElementById('shadeAdxVal') ? parseFloat(document.getElementById('shadeAdxVal').value) : 25;
    const shadeChoppyVal = document.getElementById('shadeChoppyVal') ? parseFloat(document.getElementById('shadeChoppyVal').value) : 38;

    if (shadeSeries) {
        if (showShade) {
            const shadeData = analysisList.map(d => {
                const actualAdx = (d.adx !== undefined) ? d.adx : 0;
                const actualChoppy = (d.choppy_indicator !== undefined) ? d.choppy_indicator : 0;
                const meetsCondition = (actualAdx > shadeAdxVal && actualChoppy > 0 && actualChoppy < shadeChoppyVal);
                return {
                    time: toThaiTime(d.candletime),
                    value: meetsCondition ? 1 : 0,
                    color: meetsCondition ? 'rgba(255, 193, 7, 0.15)' : 'transparent'
                };
            });
            const uniqueShadeData = shadeData.filter((v, i, a) => a.findIndex(t => t.time === v.time) === i).sort((a, b) => a.time - b.time);
            shadeSeries.setData(uniqueShadeData);
        } else {
            shadeSeries.setData([]);
        }
    }

    const emaShortData = [];
    const emaMediumData = [];
    const emaLongData = [];

    analysisList.forEach(d => {
        const time = toThaiTime(d.candletime);
        const shortVal = d.ema_short_value !== undefined ? d.ema_short_value : d.ema_short_val;
        const mediumVal = d.ema_medium_value !== undefined ? d.ema_medium_value : d.ema_medium_val;
        const longVal = d.ema_long_value !== undefined ? d.ema_long_value : d.ema_long_val;
        if (shortVal) emaShortData.push({ time, value: shortVal });
        if (mediumVal) emaMediumData.push({ time, value: mediumVal });
        if (longVal) emaLongData.push({ time, value: longVal });
    });

    if (emaShortSeries) {
        if (document.getElementById('emaShortToggle') && document.getElementById('emaShortToggle').checked) {
            emaShortSeries.setData(emaShortData);
        } else {
            emaShortSeries.setData([]);
        }
    }
    if (emaMediumSeries) {
        if (document.getElementById('emaMediumToggle') && document.getElementById('emaMediumToggle').checked) {
            emaMediumSeries.setData(emaMediumData);
        } else {
            emaMediumSeries.setData([]);
        }
    }
    if (emaLongSeries) {
        if (document.getElementById('emaLongToggle') && document.getElementById('emaLongToggle').checked) {
            emaLongSeries.setData(emaLongData);
        } else {
            emaLongSeries.setData([]);
        }
    }

    updateChartMarkers();

    // Only fit content on the very first load to avoid zooming out unexpectedly
    if (!lwChart.hasInitiallyFitContent) {
        lwChart.timeScale().fitContent();
        lwChart.hasInitiallyFitContent = true;
    }
}

function updateChartTheme() {
    if (!lwChart) return;
    const colors = getChartThemeColors();
    lwChart.applyOptions({
        layout: {
            background: { type: 'solid', color: colors.background },
            textColor: colors.textColor,
        },
        grid: {
            vertLines: { color: colors.gridColor },
            horzLines: { color: colors.gridColor },
        },
        rightPriceScale: { borderColor: colors.gridColor },
        timeScale: { borderColor: colors.gridColor },
    });
}

function setConnectionStatus(connected) {
    const badge = document.getElementById('connBadge');
    const text = document.getElementById('connText');
    const badgeBig = document.getElementById('connBadgeBig');
    const textBig = document.getElementById('connTextBig');
    if (connected) {
        if (badge) badge.className = 'conn-badge connected';
        if (text) text.textContent = 'Connected';
        if (badgeBig) badgeBig.className = 'conn-badge connected';
        if (textBig) textBig.textContent = 'Connected';
    } else {
        if (badge) badge.className = 'conn-badge disconnected';
        if (text) text.textContent = 'Disconnected';
        if (badgeBig) badgeBig.className = 'conn-badge disconnected';
        if (textBig) textBig.textContent = 'Disconnected';
    }
}

function connectRustWebSocket(mode) {
    mode = mode || 'trade';
    if (mode === 'trade') shouldStayConnected = true;

    if (localWs) {
        localWs.onclose = null; // กันไม่ให้ trigger reconnect ขณะปิดตัวเก่า
        localWs.close();
    }
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    let token = localStorage.getItem('API_KEY') || '';
    localWs = new WebSocket(`${protocol}//${window.location.host}/ws?token=${token}`);

    localWs.onopen = () => {
        console.log(`🟢 Connected to Rust Backend WebSocket (mode: ${mode})`);
        setConnectionStatus(true);

        // ═══ ดึงสถานะปัจจุบัน: บอทกำลังทำงานอยู่หรือไม่? ═══
        fetch('/api/status').then(r => r.json()).then(status => {
            console.log("📊 Server Status:", status);
            
            // 🔧 FIX: ตรวจสอบว่าเป็น Short Term Trade จริงๆ
            // ใช้ bot_count > 0 + active_assets เป็นเงื่อนไขหลัก
            const isShortTermActive = status.bot_count > 0 && 
                                      status.active_assets && 
                                      status.active_assets.length > 0;
            
            console.log(`🔍 [Status Check] isShortTermActive: ${isShortTermActive}, bot_count: ${status.bot_count}, active_assets: ${JSON.stringify(status.active_assets)}`);
            
            if (isShortTermActive) {
                isTradeRunning = true;
                console.log('✅ [Status Check] Setting isTradeRunning = true (Short Term Trade is active)');
                shouldStayConnected = true; // ถ้าบอทกำลังทำงาน ให้ connect ถาวร

                // ฟื้นฟู progress bar และเวลาที่รันไปแล้ว หาก reload หน้าจอใหม่
                if (!tradeStartTime) {
                    // ลองหา actualStartTime จาก tradeControl ก่อน
                    fetch('/api/tradeControl', { cache: 'no-store' })
                        .then(r => r.json())
                        .then(json => {
                            if (json && json.status === 'success' && json.data.actualStartTime) {
                                tradeStartTime = new Date(json.data.actualStartTime.replace(" ", "T")).getTime();
                                console.log('📅 [Status Check] Restored tradeStartTime from tradeControl:', new Date(tradeStartTime));
                            } else {
                                const useScheduleEl = document.getElementById('useSchedule');
                                if (useScheduleEl && useScheduleEl.checked) {
                                    const startVal = document.getElementById('startDate').value;
                                    if (startVal) tradeStartTime = new Date(startVal).getTime();
                                }
                                if (!tradeStartTime) tradeStartTime = Date.now();
                            }
                            if (tradeDurationInterval) clearInterval(tradeDurationInterval);
                            tradeDurationInterval = setInterval(updateTradingDuration, 1000);
                            updateTradingDuration();
                        })
                        .catch(e => {
                            console.error('❌ [Status Check] Error fetching tradeControl:', e);
                            tradeStartTime = Date.now();
                            if (tradeDurationInterval) clearInterval(tradeDurationInterval);
                            tradeDurationInterval = setInterval(updateTradingDuration, 1000);
                            updateTradingDuration();
                        });
                }

                const msg = `Trading Active (${status.bot_count} bot${status.bot_count > 1 ? 's' : ''}: ${status.active_assets.join(', ')})`;
                const connText = document.getElementById('connText');
                if (connText) connText.textContent = msg;
                const connTextBig = document.getElementById('connTextBig');
                if (connTextBig) connTextBig.textContent = msg;
                appendBotLog(`🔄 เชื่อมต่อสำเร็จ — พบบอทกำลังทำงาน ${status.bot_count} ตัว: ${status.active_assets.join(', ')}`, 'system');
            } else {
                console.log('ℹ️ [Status Check] isTradeRunning remains false (no active Short Term Trade)');
                appendBotLog('🔌 เชื่อมต่อสำเร็จ — ยังไม่มีบอททำงาน (กด Go Trade เพื่อเริ่ม)', 'system');
            }
            if (status.account_name) {
                const accNameEl = document.getElementById('account-name-badge');
                if (accNameEl) accNameEl.textContent = status.account_name;
            }
            if (status.balance !== undefined) {
                document.getElementById('status-balance').textContent = `Balance: $${status.balance.toFixed(2)} 🔄`;
                currentBalance = status.balance;
                // เรียก updateBalanceSummary เฉพาะเมื่อ trade history ถูกโหลดแล้ว
                // เพื่อป้องกันไม่ให้กำไรรอบนี้กลายเป็น 0 จาก race condition
                if (window.currentTradeHistory && window.currentTradeHistory.length > 0) {
                    updateBalanceSummary(status.balance);
                } else {
                    // แสดงเฉพาะยอด Balance เบื้องต้นไปก่อน รอ Trade History โหลดเสร็จค่อยคำนวณกำไรใหม่
                    const el = document.getElementById('summary-initial-balance');
                    if (el) {
                        el.textContent = `$${currentBalance.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
                    }
                }
            }

            // ═══ ตรวจสอบ Schedule Status ═══
            checkScheduleStatus();

            // ═══ ถ้าเป็น initial mode และยังไม่ได้เทรด Short Term → ปิด WS เพื่อประหยัด traffic ═══
            if (mode === 'initial' && !isShortTermActive) {
                console.log('📴 Balance fetched. จะปิด WS เพื่อประหยัด Traffic ใน 2 วินาที...');
                setTimeout(() => {
                    if (localWs && !shouldStayConnected) {
                        localWs.onclose = null;
                        localWs.close();
                        localWs = null;
                        setConnectionStatus(false);
                        const ct = document.getElementById('connText');
                        if (ct) ct.textContent = 'ยังไม่เทรด (ประหยัด Traffic)';
                        const ctb = document.getElementById('connTextBig');
                        if (ctb) ctb.textContent = 'ยังไม่เทรด — รอ Schedule หรือกด Go Trade';
                        console.log('📴 WS ปิดแล้ว — รอ Schedule หรือ Go Trade เพื่อ reconnect');
                    }
                }, 2000);
            }
        }).catch(e => console.error("Error fetching status:", e));
    };

    localWs.onmessage = (event) => {
        try {
            const payload = JSON.parse(event.data);

            if (payload.type === 'candles_history') {
                const asset = payload.asset || 'unknown';
                console.log(`📊 [${asset}] ได้รับประวัติแท่งเทียน: ${payload.data.length} แท่ง`);

                // เก็บข้อมูลแยกตาม asset
                allDataByAsset[asset] = payload.data;

                // อัปเดตปุ่ม asset
                const select = document.getElementById('chartAssetSelect');
                if (!document.getElementById(`chart-btn-${asset}`)) {
                    if (select.innerHTML.includes('รอข้อมูล')) select.innerHTML = '';
                    const btn = document.createElement('button');
                    btn.id = `chart-btn-${asset}`;
                    btn.className = 'chart-asset-btn';
                    btn.dataset.asset = asset;
                    btn.textContent = asset.toUpperCase();
                    btn.style.cssText = "padding:4px 12px; border-radius:16px; border:1px solid var(--border); background:var(--bg3); color:var(--text); font-size:12px; cursor:pointer; font-weight:600; transition:all 0.2s;";
                    btn.onclick = () => window.switchChartAsset(asset);
                    select.appendChild(btn);
                }

                // ถ้ายังไม่ได้เลือก asset → เลือก asset แรกที่เข้ามา
                if (!currentChartAsset) {
                    window.switchChartAsset(asset);
                }

                // วาดกราฟเฉพาะ asset ที่กำลังแสดงอยู่
                if (asset === currentChartAsset) {
                    allAnalysisData = allDataByAsset[asset];
                    document.getElementById('status-count').textContent = `Candles: ${allAnalysisData.length}`;
                    initChart();
                    const prevLogicalRange = lwChart ? lwChart.timeScale().getVisibleLogicalRange() : null;
                    renderChart(allAnalysisData);
                    if (prevLogicalRange && lwChart) {
                        try { lwChart.timeScale().setVisibleLogicalRange(prevLogicalRange); } catch (e) { }
                    }
                    renderTable(allAnalysisData);
                }
            }
            else if (payload.type === 'ohlc_update') {
                const newData = payload.data;
                const asset = payload.asset || 'unknown';
                if (!newData || !newData.candletime || !newData.close) return;
                if (newData.open <= 0 || newData.high <= 0 || newData.low <= 0 || newData.close <= 0) return;

                // เก็บข้อมูลแยกตาม asset
                if (!allDataByAsset[asset]) allDataByAsset[asset] = [];
                const assetData = allDataByAsset[asset];

                if (assetData.length > 0) {
                    const lastIdx = assetData.length - 1;
                    if (assetData[lastIdx].candletime === newData.candletime) {
                        assetData[lastIdx] = newData;
                    } else if (newData.candletime > assetData[lastIdx].candletime) {
                        // ─── NEW CANDLE DETECTED ───
                        const closedCandle = assetData[lastIdx];
                        const prevCandle = assetData.length > 1 ? assetData[lastIdx - 1] : null;

                        if (prevCandle && closedCandle) {
                            const prevShort = prevCandle.ema_short_value !== undefined ? prevCandle.ema_short_value : prevCandle.ema_short_val;
                            const prevMed = prevCandle.ema_medium_value !== undefined ? prevCandle.ema_medium_value : prevCandle.ema_medium_val;

                            const currShort = closedCandle.ema_short_value !== undefined ? closedCandle.ema_short_value : closedCandle.ema_short_val;
                            const currMed = closedCandle.ema_medium_value !== undefined ? closedCandle.ema_medium_value : closedCandle.ema_medium_val;

                            if (prevShort !== undefined && prevMed !== undefined && currShort !== undefined && currMed !== undefined) {
                                let isCrossUp = prevShort <= prevMed && currShort > currMed;
                                let isCrossDown = prevShort >= prevMed && currShort < currMed;

                                if (isCrossUp || isCrossDown) {
                                    if (document.getElementById('playEmaCutSoundToggle') && document.getElementById('playEmaCutSoundToggle').checked) {
                                        try {
                                            const audio = new Audio('/sounds/mp3/Jungle exclamation.mp3');
                                            audio.play().catch(e => console.log('Audio play failed:', e));
                                        } catch (e) { }
                                    }

                                    if (!window.emaCutCustomMarkers) window.emaCutCustomMarkers = {};
                                    if (!window.emaCutCustomMarkers[asset]) window.emaCutCustomMarkers[asset] = [];

                                    window.emaCutCustomMarkers[asset].push({
                                        time: toThaiTime(closedCandle.candletime),
                                        position: isCrossUp ? 'belowBar' : 'aboveBar',
                                        color: isCrossUp ? '#06D6A0' : '#E8304A',
                                        shape: isCrossUp ? 'arrowUp' : 'arrowDown',
                                        text: isCrossUp ? 'EMA Cut Up' : 'EMA Cut Down'
                                    });

                                    if (typeof updateChartMarkers === 'function') updateChartMarkers();
                                }
                            }
                        }

                        if (typeof updateChartMarkers === 'function') updateChartMarkers();
                        assetData.push(newData);
                    } else {
                        return;
                    }
                } else {
                    assetData.push(newData);
                }

                // อัปเดตปุ่มถ้า asset ใหม่
                const select = document.getElementById('chartAssetSelect');
                if (!document.getElementById(`chart-btn-${asset}`)) {
                    if (select.innerHTML.includes('รอข้อมูล')) select.innerHTML = '';
                    const btn = document.createElement('button');
                    btn.id = `chart-btn-${asset}`;
                    btn.className = 'chart-asset-btn';
                    btn.dataset.asset = asset;
                    btn.textContent = asset.toUpperCase();
                    btn.style.cssText = "padding:4px 12px; border-radius:16px; border:1px solid var(--border); background:var(--bg3); color:var(--text); font-size:12px; cursor:pointer; font-weight:600; transition:all 0.2s;";
                    btn.onclick = () => window.switchChartAsset(asset);
                    select.appendChild(btn);
                }
                if (!currentChartAsset) {
                    window.switchChartAsset(asset);
                }

                // อัปเดตกราฟเฉพาะ asset ที่กำลังแสดงอยู่
                if (asset !== currentChartAsset) return;

                allAnalysisData = assetData;
                document.getElementById('status-count').textContent = `Candles: ${allAnalysisData.length}`;

                if (candleSeries) {
                    const showSpike = document.getElementById('spikeShowToggle').checked;
                    const greenSpikeColor = document.getElementById('spikeGreenColor').value;
                    const redSpikeColor = document.getElementById('spikeRedColor').value;

                    let colorProps = {};
                    if (showSpike && newData.is_atr) {
                        if (newData.color === 'green') {
                            colorProps = { color: greenSpikeColor, wickColor: greenSpikeColor, borderColor: greenSpikeColor };
                        } else if (newData.color === 'red') {
                            colorProps = { color: redSpikeColor, wickColor: redSpikeColor, borderColor: redSpikeColor };
                        }
                    }

                    candleSeries.update({
                        time: toThaiTime(newData.candletime),
                        open: newData.open,
                        high: newData.high,
                        low: newData.low,
                        close: newData.close,
                        ...colorProps
                    });

                    const tTime = toThaiTime(newData.candletime);
                    const shortVal = newData.ema_short_value !== undefined ? newData.ema_short_value : newData.ema_short_val;
                    const mediumVal = newData.ema_medium_value !== undefined ? newData.ema_medium_value : newData.ema_medium_val;
                    const longVal = newData.ema_long_value !== undefined ? newData.ema_long_value : newData.ema_long_val;
                    if (emaShortSeries && document.getElementById('emaShortToggle') && document.getElementById('emaShortToggle').checked && shortVal) {
                        emaShortSeries.update({ time: tTime, value: shortVal });
                    }
                    if (emaMediumSeries && document.getElementById('emaMediumToggle') && document.getElementById('emaMediumToggle').checked && mediumVal) {
                        emaMediumSeries.update({ time: tTime, value: mediumVal });
                    }
                    if (emaLongSeries && document.getElementById('emaLongToggle') && document.getElementById('emaLongToggle').checked && longVal) {
                        emaLongSeries.update({ time: tTime, value: longVal });
                    }

                    // Remove the heavy renderChart(allAnalysisData) call
                    // Markers are now updated separately in reloadTradeHistory

                    // Update Spike counts for real-time
                    let gc = 0, rc = 0;
                    allAnalysisData.forEach(d => {
                        if (d.is_atr) {
                            if (d.color === 'green') gc++;
                            else if (d.color === 'red') rc++;
                        }
                    });
                    document.getElementById('spike-green-count').textContent = gc;
                    document.getElementById('spike-red-count').textContent = rc;
                }

                document.getElementById('status-count').textContent = `Candles: ${allAnalysisData.length}`;
            }
            else if (payload.type === 'order_update') {
                window.handleOrderUpdate(payload.data);
            }
            else if (payload.type === 'trade_result') {
                console.log("💰 ข้อมูลการตบท้ายไม้:", payload.data);
                // Refresh strategy data globally to update tooltips
                if (document.getElementById('showStrategyTooltipToggle') && document.getElementById('showStrategyTooltipToggle').checked) {
                    window.fetchGlobalStrategyData();
                }
                // alert(`Trade Finished: ${payload.data.win_status} | Profit: $${payload.data.profit} | Balance: $${payload.data.balance}`);
                document.getElementById('status-balance').textContent = `Balance: $${payload.data.balance.toFixed(2)} 🔄`;
                updateBalanceSummary(payload.data.balance);

                // อัปเดต History ทันทีเพื่อให้วาด Marker อัตโนมัติ
                if (document.getElementById('showTradeMarkersToggle') && document.getElementById('showTradeMarkersToggle').checked) {
                    if (typeof reloadTradeHistory === 'function') {
                        reloadTradeHistory(true);
                    }
                }

                // อัปเดตปุ่ม Asset ว่าเทรดจบแล้ว
                const assetName = payload.asset || '';
                if (assetName && window.assetTradingStates[assetName]) {
                    window.assetTradingStates[assetName].isTrading = false;
                    updateActiveAssetButtons();
                }
            }
            else if (payload.type === 'trade_start') {
                const assetName = payload.asset || '';
                const lossCon = payload.data?.loss_con || 0;
                if (assetName) {
                    if (!window.assetTradingStates[assetName]) {
                        window.assetTradingStates[assetName] = { isTrading: false, lossCon: 0 };
                    }
                    window.assetTradingStates[assetName].isTrading = true;
                    window.assetTradingStates[assetName].lossCon = lossCon;
                    updateActiveAssetButtons();
                }

                // Play sound when entering a trade
                if (document.getElementById('playSoundToggle') && document.getElementById('playSoundToggle').checked) {
                    try {
                        const audio = new Audio('/sounds/icq_uh_oh.mp3');
                        audio.play().catch(e => console.log('Audio play failed:', e));
                    } catch (e) { }
                }
            }
            else if (payload.type === 'balance_update') {
                if (payload.data && payload.data.balance !== undefined) {
                    document.getElementById('status-balance').textContent = `Balance: $${payload.data.balance.toFixed(2)} 🔄`;
                    updateBalanceSummary(payload.data.balance);
                }
            }
            else if (payload.type === 'multi_node_summary') {
                if (payload.data) {
                    renderMultiNodeSummary(payload.data);
                }
            }
            else if (payload.type === 'trade_control_update') {
                // ═══ Trade Control Update — Whipsaw Zone ═══
                const tcData = payload.data;
                const asset = payload.asset || '';
                if (tcData) {
                    // Store per-asset trade control state
                    if (!window.tradeControlByAsset) window.tradeControlByAsset = {};
                    window.tradeControlByAsset[asset] = tcData;
                    updateTradeControlTable();
                }
            }
            else if (payload.type === 'bot_log') {
                const msg = payload.data?.message || '';
                appendBotLog(msg, payload.asset || '');
                if (msg.includes('AUTO SELL TIMEOUT!')) {
                    playAutoSellSound(); // เล่นเสียงเมื่อ backend สั่ง auto sale timeout
                }
            }
        } catch (e) {
            console.error("Error parsing message:", e);
        }
    };

    localWs.onerror = (err) => {
        console.error("WebSocket error:", err);
        setConnectionStatus(false);
    };

    localWs.onclose = () => {
        console.log('🔴 Disconnected from Rust Backend');
        setConnectionStatus(false);
        // Auto-reconnect เฉพาะเมื่ออยู่ในโหมดเทรด (ประหยัด traffic)
        if (shouldStayConnected) {
            console.log('🔄 Auto-reconnecting in 3s (trade mode)...');
            setTimeout(() => connectRustWebSocket('trade'), 3000);
        } else {
            console.log('📴 WS ปิด (traffic saving mode — ไม่ reconnect)');
            const ct = document.getElementById('connText');
            if (ct) ct.textContent = 'ยังไม่เทรด (ประหยัด Traffic)';
            const ctb = document.getElementById('connTextBig');
            if (ctb) ctb.textContent = 'ยังไม่เทรด — รอ Schedule หรือกด Go Trade';
        }
    };
}

function updateTrackOrdersTable() {
    const orders = Object.values(window.activeOrders).sort((a, b) => (b.purchase_time || 0) - (a.purchase_time || 0));

    // Update counts
    const countStr = `${orders.length} active orders`;
    const c1 = document.getElementById('track-orders-count');
    if (c1) c1.textContent = countStr;
    const c2 = document.getElementById('manual-track-orders-count');
    if (c2) c2.textContent = countStr;

    const tables = [
        { body: document.querySelector('#trackOrdersTable tbody'), prefix: 'main' },
        { body: document.querySelector('#manualTrackOrdersTable tbody'), prefix: 'manual' }
    ];

    tables.forEach(({ body, prefix }) => {
        if (!body) return;

        if (orders.length === 0) {
            body.innerHTML = '<tr class="empty-row"><td colspan="15">ไม่มีออเดอร์ที่กำลังทำงานอยู่</td></tr>';
            return;
        }

        // Remove empty row if it exists
        const emptyRow = body.querySelector('.empty-row');
        if (emptyRow) emptyRow.remove();

        // Track active IDs to remove stale rows
        const activeIds = new Set(orders.map(o => `row_${prefix}_${o.contract_id}`));
        Array.from(body.children).forEach(tr => {
            if (!activeIds.has(tr.id)) {
                tr.remove();
            }
        });

        orders.forEach((order, index) => {
            const profit = order.profit || 0;
            const buyTime = order.purchase_time ? new Date(order.purchase_time * 1000).toLocaleTimeString('th-TH') : '-';
            const expiryTime = order.date_expiry ? new Date(order.date_expiry * 1000).toLocaleTimeString('th-TH') : '-';

            // Calculate Time Left and Progress
            const now = Math.floor(Date.now() / 1000);
            let timeLeftStr = "-";
            let isOvertime = false;
            let progressHtml = "";
            let timeLeftColor = "inherit";

            if (order.date_expiry && order.purchase_time) {
                const totalDuration = order.date_expiry - order.purchase_time;
                const elapsed = Math.max(0, now - order.purchase_time);

                if (now > order.date_expiry) {
                    isOvertime = true;
                    const overTimeSecs = now - order.date_expiry;
                    timeLeftStr = `Over ${overTimeSecs}s ⚠️`;
                    timeLeftColor = "var(--red)";
                } else {
                    const timeLeft = order.date_expiry - now;
                    timeLeftStr = timeLeft > 60 ? `${Math.floor(timeLeft / 60)}m ${timeLeft % 60}s` : `${timeLeft}s`;
                    if (timeLeft < 10) timeLeftColor = "var(--red)";
                }

                if (totalDuration > 0) {
                    let percent = (elapsed / totalDuration) * 100;
                    percent = Math.min(100, Math.max(0, percent));

                    let barColor = 'var(--green)';
                    if (percent > 90 || isOvertime) barColor = 'var(--red)';
                    else if (percent > 70) barColor = 'var(--orange)';

                    progressHtml = `
                                <div style="width:100%; min-width:60px; max-width:100px; background:var(--bg3); border-radius:4px; height:6px; margin-top:4px; overflow:hidden;">
                                    <div style="width:${percent}%; height:100%; background:${barColor}; transition:width 1s linear;"></div>
                                </div>
                            `;
                }
            } else if (order.date_expiry) {
                const timeLeft = Math.max(0, order.date_expiry - now);
                timeLeftStr = `${timeLeft}s`;
                if (timeLeft < 10) timeLeftColor = "var(--red)";
            }

            // Auto Sell Logic (Frontend side simple check)
            const targetProfit = window.targetProfits[order.contract_id] || 0;
            if (prefix === 'main') {
                // Only run auto-sell once per order, not per table
                if (targetProfit > 0 && profit >= targetProfit) {
                    console.log(`🎯 Target profit reached for ${order.contract_id}! Selling...`);
                    sellContract(order.contract_id, true);
                    window.targetProfits[order.contract_id] = 0; // Prevent duplicate sell calls
                }
            }

            const rowId = `row_${prefix}_${order.contract_id}`;
            const inputId = `target_${prefix}_${order.contract_id}`;
            let tr = document.getElementById(rowId);

            if (!tr) {
                // Create new row
                tr = document.createElement('tr');
                tr.id = rowId;
                tr.innerHTML = `
                            <td>${index + 1}</td>
                            <td>${order.contract_id}</td>
                            <td>${order.display_name || order.symbol || order.asset || order.underlying_symbol || "Unknown"}</td>
                            <td style="color:${order.contract_type === 'CALL' ? 'var(--green)' : 'var(--red)'}">${order.contract_type}</td>
                            <td>$${(order.buy_price || 0).toFixed(2)}</td>
                            <td style="color:var(--accent); font-weight:bold;" id="entry_${prefix}_${order.contract_id}">${(order.entry_spot > 0) ? order.entry_spot : ((order.entry_tick > 0) ? order.entry_tick : '-')}</td>
                            <td>$${(order.payout || 0).toFixed(2)}</td>
                            <td style="color:${profit >= 0 ? 'var(--green)' : 'var(--red)'}; font-weight:bold;">
                                ${profit >= 0 ? '+' : ''}${profit.toFixed(2)}
                            </td>
                            <td>${buyTime}</td>
                            <td>${expiryTime}</td>
                            <td style="font-weight:bold; color:${timeLeftColor}">
                                <div style="display:flex; flex-direction:column; align-items:center;">
                                    <span>${timeLeftStr}</span>
                                    ${progressHtml}
                                </div>
                            </td>
                            <td style="color:var(--red);">$${(order.min_profit || 0).toFixed(2)}</td>
                            <td style="color:var(--green);">$${(order.max_profit || 0).toFixed(2)}</td>
                            <td>
                                <input type="number" class="target-profit-input" id="${inputId}" value="${targetProfit || ''}" step="0.01" placeholder="0.00">
                                <button class="set-btn" onclick="setTargetProfit(${order.contract_id}, '${inputId}')">Set</button>
                            </td>
                            <td>
                                <div style="display:flex; gap:4px; align-items:center;">
                                    <button class="sell-btn" onclick="sellContract(${order.contract_id})">ขายทันที</button>
                                    <button class="draw-line-btn" id="drawbtn_${prefix}_${order.contract_id}" style="padding:4px 8px; border-radius:4px; background:var(--bg3); border:1px solid var(--border); color:var(--text); cursor:pointer;" onclick="drawPriceLine(${(order.entry_spot > 0) ? order.entry_spot : ((order.entry_tick > 0) ? order.entry_tick : order.buy_price)}, '${order.contract_type}')" title="วาดเส้นราคา Entry Spot">📌</button>
                                </div>
                            </td>
                        `;
                body.appendChild(tr);
            } else {
                // Update existing row (only volatile columns)
                tr.cells[0].innerHTML = index + 1;

                // Update entry_spot when it becomes available (initially 0 from Deriv API)
                const entrySpotVal = (order.entry_spot > 0) ? order.entry_spot : ((order.entry_tick > 0) ? order.entry_tick : 0);
                const entryCell = document.getElementById(`entry_${prefix}_${order.contract_id}`);
                if (entryCell) {
                    entryCell.innerHTML = entrySpotVal > 0 ? entrySpotVal : '-';
                }

                // Update draw-line-btn onclick when entry_spot becomes available
                const drawBtn = document.getElementById(`drawbtn_${prefix}_${order.contract_id}`);
                if (drawBtn && entrySpotVal > 0) {
                    drawBtn.setAttribute('onclick', `drawPriceLine(${entrySpotVal}, '${order.contract_type}')`);
                    drawBtn.style.background = 'rgba(74,158,255,0.15)';
                    drawBtn.style.borderColor = 'var(--accent)';
                    drawBtn.title = `วาดเส้น Entry Spot: ${entrySpotVal}`;
                }

                tr.cells[7].style.color = profit >= 0 ? 'var(--green)' : 'var(--red)';
                tr.cells[7].innerHTML = `${profit >= 0 ? '+' : ''}${profit.toFixed(2)}`;

                tr.cells[10].style.color = timeLeftColor;
                tr.cells[10].innerHTML = `
                            <div style="display:flex; flex-direction:column; align-items:center;">
                                <span>${timeLeftStr}</span>
                                ${progressHtml}
                            </div>
                        `;

                tr.cells[11].innerHTML = `$${(order.min_profit || 0).toFixed(2)}`;
                tr.cells[12].innerHTML = `$${(order.max_profit || 0).toFixed(2)}`;
            }
        });
    });
}

function setTargetProfit(contractId, inputId) {
    const actualInputId = inputId || `target-${contractId}`; // fallback for older code if any
    const inp = document.getElementById(actualInputId);
    const val = inp ? (parseFloat(inp.value) || 0) : 0;
    window.targetProfits[contractId] = val;
    console.log(`Set Target Profit for ${contractId}: ${val}`);
}

function playAutoSellSound() {
    try {
        const autoSellAudio = new Audio('/sounds/mp3/Jungle default sound.mp3');
        autoSellAudio.play().catch(e => console.log('Auto-sell audio play failed:', e));
    } catch (err) {
        console.log('Audio error:', err);
    }
}

async function sellContract(contractId, skipConfirm) {
    if (!skipConfirm) {
        if (!confirm(`คุณต้องการขายสัญญา ID ${contractId} ทันทีใช่หรือไม่?`)) return;
    } else {
        playAutoSellSound(); // เล่นเสียงเมื่อเป็นการขายแบบ Auto Sale
    }

    try {
        const res = await fetch('/api/sell', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ contract_id: contractId })
        });
        const data = await res.json();
        console.log("Sell Response:", data);
    } catch (e) {
        console.error("Error selling contract:", e);
    }
}

window.activePriceLines = window.activePriceLines || [];
function drawPriceLine(price, type) {
    const numPrice = parseFloat(price);
    if (!numPrice || numPrice <= 0) {
        alert('⚠️ ยังไม่มีข้อมูล Entry Spot — รอสักครู่แล้วลองใหม่');
        return;
    }
    if (!candleSeries) {
        alert('⚠️ กราฟยังไม่ถูกโหลด — กรุณาเลือก Asset และโหลดข้อมูลก่อน');
        return;
    }
    const color = type === 'CALL' ? '#26a69a' : '#ef5350';
    const priceLine = candleSeries.createPriceLine({
        price: numPrice,
        color: color,
        lineWidth: 2,
        lineStyle: LightweightCharts.LineStyle.Dashed,
        axisLabelVisible: true,
        title: `${type} Entry: ${numPrice}`,
    });
    window.activePriceLines.push(priceLine);
    console.log(`📌 Drew price line at ${numPrice} for ${type}`);
}

// Update time left every second
setInterval(() => {
    if (Object.keys(window.activeOrders).length > 0) {
        updateTrackOrdersTable();
    }
}, 1000);
// ═══ Trade Control Table (Whipsaw Zone) ═══
function updateTradeControlTable() {
    const container = document.getElementById('tradeControlContainer');
    const statusEl = document.getElementById('trade-control-status');
    if (!container || !window.tradeControlByAsset) return;

    const assets = Object.keys(window.tradeControlByAsset);
    if (assets.length === 0) {
        container.innerHTML = '<div style="color:var(--text3); text-align:center; padding:20px; font-size:13px;">📡 รอข้อมูลจากบอท...</div>';
        if (statusEl) statusEl.textContent = 'ยังไม่มีข้อมูล';
        return;
    }

    let html = '';
    assets.forEach(assetKey => {
        const tc = window.tradeControlByAsset[assetKey];
        const isWS = tc.whipsawZoneActive;
        const badgeColor = isWS ? '#ff4444' : '#4CAF50';
        const badgeText = isWS ? '🎯 WHIPSAW ZONE' : '✅ NORMAL';

        html += `<div style="background:var(--bg2); border:1px solid ${isWS ? '#ff4444' : 'var(--border)'}; border-radius:12px; padding:14px; ${isWS ? 'box-shadow: 0 0 12px rgba(255,68,68,0.3);' : ''}">`;
        html += `<div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:10px; flex-wrap:wrap; gap:8px;">`;
        html += `<div style="display:flex; align-items:center; gap:8px;">`;
        html += `<span style="font-weight:700; font-size:14px; color:var(--text);">${tc.assetCode || assetKey}</span>`;
        html += `<span style="font-size:11px; padding:2px 10px; border-radius:20px; background:${badgeColor}; color:#fff; font-weight:600;">${badgeText}</span>`;
        html += `</div>`;
        html += `<div style="display:flex; gap:12px; font-size:12px; color:var(--text3);">`;
        html += `<span>LossCon: <b style="color:${tc.lossCon >= 4 ? '#ff4444' : 'var(--text)'}">${tc.lossCon}</b></span>`;
        html += `<span>Colors: <b style="color:var(--text)">${tc.colorList || '-'}</b></span>`;
        html += `</div>`;
        html += `</div>`;

        // Trade list table
        if (tc.tradeList && tc.tradeList.length > 0) {
            html += `<div style="overflow-x:auto;"><table style="width:100%; border-collapse:collapse; font-size:12px;">`;
            html += `<thead><tr style="background:var(--bg3); color:var(--text3);">`;
            html += `<th style="padding:6px 10px; text-align:center;">No.</th>`;
            html += `<th style="padding:6px 10px; text-align:center;">Symbol</th>`;
            html += `<th style="padding:6px 10px; text-align:center;">Color</th>`;
            html += `<th style="padding:6px 10px; text-align:center;">Action</th>`;
            html += `<th style="padding:6px 10px; text-align:center;">Win Status</th>`;
            html += `<th style="padding:6px 10px; text-align:center;">LossCon</th>`;
            html += `<th style="padding:6px 10px; text-align:center;">Whipsaw</th>`;
            html += `</tr></thead><tbody>`;

            tc.tradeList.forEach(item => {
                const isIdle = item.action === 'IDLE';
                const rowBg = item.whipsawZone ? 'rgba(255,68,68,0.08)' : 'transparent';
                const actionColor = item.action === 'CALL' ? 'var(--green)' : item.action === 'PUT' ? 'var(--red)' : '#999';
                const colorDot = item.candleColor === 'green' ? '🟢' : '🔴';
                const winIcon = item.winStatus === 'Win' ? '✅' : item.winStatus === 'Loss' ? '❌' : item.winStatus === '-' ? '⏸️' : '⏳';

                html += `<tr style="background:${rowBg}; border-bottom:1px solid var(--border); ${isIdle ? 'opacity:0.6;' : ''}">`;
                html += `<td style="padding:6px 10px; text-align:center;">${item.tradeno}</td>`;
                html += `<td style="padding:6px 10px; text-align:center; font-weight:700;">${item.candleSymbol}</td>`;
                html += `<td style="padding:6px 10px; text-align:center;">${colorDot}</td>`;
                html += `<td style="padding:6px 10px; text-align:center; font-weight:700; color:${actionColor};">${item.action}</td>`;
                html += `<td style="padding:6px 10px; text-align:center;">${winIcon} ${item.winStatus || ''}</td>`;
                html += `<td style="padding:6px 10px; text-align:center; font-weight:600;">${item.lossCon}</td>`;
                html += `<td style="padding:6px 10px; text-align:center; font-size:16px;">${item.whipsawZone ? '🎯' : ''}</td>`;
                html += `</tr>`;
            });

            html += `</tbody></table></div>`;
        } else {
            html += `<div style="color:var(--text3); text-align:center; padding:10px; font-size:12px;">ยังไม่มีไม้เทรด</div>`;
        }

        html += `</div>`;
    });

    container.innerHTML = html;

    // Update status badge
    const anyActive = assets.some(k => window.tradeControlByAsset[k].whipsawZoneActive);
    if (statusEl) {
        if (anyActive) {
            statusEl.textContent = '🎯 Whipsaw Zone Active!';
            statusEl.style.background = '#ff4444';
            statusEl.style.color = '#fff';
        } else {
            statusEl.textContent = `${assets.length} asset(s) monitored`;
            statusEl.style.background = 'var(--bg3)';
            statusEl.style.color = 'var(--text3)';
        }
    }
}

function renderTable(analysisList) {
    const tbody = document.querySelector('.data-table tbody');
    document.getElementById('table-count').textContent = `${analysisList.length} records`;

    if (!analysisList || analysisList.length === 0) return;

    tbody.innerHTML = '';
    // โชว์แค่ 50 แท่งล่าสุดเอาไว้บนสุดเพื่อไม่ให้หนักเบราว์เซอร์
    const displayAnalysis = analysisList.slice(-50).reverse();

    displayAnalysis.forEach(data => {
        const tr = document.createElement('tr');
        const timeStr = new Date(data.candletime * 1000).toLocaleTimeString('th-TH');

        tr.innerHTML = `
                    <td>${timeStr}</td>
                    <td>${data.open.toFixed(4)}</td>
                    <td>${data.high.toFixed(4)}</td>
                    <td>${data.low.toFixed(4)}</td>
                    <td>${data.close.toFixed(4)}</td>
                    <td>${data.color === 'green' ? '🟢 เขียว' : '🔴 แดง'}</td>
                    <td>${data.is_atr ? '💥 ใช่' : 'ไม่ใช่'} (${data.atrValue ? data.atrValue.toFixed(2) : 'N/A'})</td>
                    <td>${data.ema_short_direction === 'Up' ? '<span style="color:var(--green); font-weight:bold;">▲ ขึ้น</span>' : '<span style="color:var(--red); font-weight:bold;">▼ ลง</span>'}</td>
                    <td>${data.ema_medium_direction === 'Up' ? '<span style="color:var(--green); font-weight:bold;">▲ ขึ้น</span>' : '<span style="color:var(--red); font-weight:bold;">▼ ลง</span>'}</td>
                `;
        tbody.appendChild(tr);
    });
}

// ══════════════════════════════════════════
//  VPS QUICK NAVIGATION TOOLBAR (vpsMaster)
// ══════════════════════════════════════════
function renderVpsButtons() {
    const container = document.getElementById('vps-buttons-container');
    if (!container) return;

    // ข้อมูล Master ของเครื่อง VPS ตามที่บันทึกไว้ในตาราง vpsMaster
    const vpsMasterList = [
        {
            vpsName: "AWS",
            publicIP: "pkderiv.online",
            portno: null,
            url: "pkderiv.online"
        },
        {
            vpsName: "Google Could",
            publicIP: "gpkderiv.shop",
            portno: null,
            url: "gpkderiv.shop"
        },
        {
            vpsName: "Oracle-Free-Tier-3",
            publicIP: "161.118.203.228",
            portno: "3000",
            url: null
        },
        {
            vpsName: "Oracle-Free-Tier-4",
            publicIP: "161.118.217.177",
            portno: "3000",
            url: null
        }
    ];

    const currentHost = (window.location.host || '').toLowerCase();
    const currentHostname = (window.location.hostname || '').toLowerCase();
    const currentPort = window.location.port ? String(window.location.port) : '';

    container.innerHTML = '';

    vpsMasterList.forEach(vps => {
        const domain = (vps.url || '').trim();
        const ip = (vps.publicIP || '').trim();
        const port = vps.portno ? String(vps.portno).trim() : '';

        // หา host ที่จะใช้เชื่อมต่อ
        const hostOnly = domain || ip;
        let hostWithPort = hostOnly;
        if (port) {
            hostWithPort += `:${port}`;
        }

        // กำหนดโปรโตคอล (มี port ปกติเป็น http, โดเมนไม่มี port เป็น https)
        let protocol = 'http:';
        if (!port && (hostOnly.includes('.shop') || hostOnly.includes('.online') || hostOnly.includes('.com'))) {
            protocol = 'https:';
        }

        const targetUrl = `${protocol}//${hostWithPort}/index_short_term.html`;

        // ตรวจสอบว่าตรงกับ Current URL ของเครื่องที่เปิดอยู่หรือไม่
        const isCurrent = (currentHost === hostWithPort.toLowerCase()) ||
                          (port ? (currentHostname === hostOnly.toLowerCase() && currentPort === port)
                                : (currentHostname === domain.toLowerCase() || currentHostname === ip.toLowerCase()));

        const btn = document.createElement('button');
        btn.type = 'button';
        btn.className = 'vps-nav-btn ' + (isCurrent ? 'active-vps' : 'inactive-vps');
        
        if (isCurrent) {
            btn.innerHTML = `🟢 <b>${vps.vpsName}</b>`;
            btn.title = `เครื่องปัจจุบันที่คุณกำลังใช้งานอยู่ (${targetUrl})`;
        } else {
            btn.innerHTML = `🌐 ${vps.vpsName}`;
            btn.title = `คลิกเพื่อเปิดไปยัง ${vps.vpsName} (${targetUrl})`;
        }

        btn.addEventListener('click', function (e) {
            e.preventDefault();
            const switchEl = document.getElementById('vpsOpenNewTabSwitch');
            const openInNewTab = switchEl ? switchEl.checked : true;
            if (openInNewTab) {
                window.open(targetUrl, '_blank');
            } else {
                window.location.href = targetUrl;
            }
        });

        container.appendChild(btn);
    });

    // จำค่า Switchbox จาก LocalStorage
    const switchEl = document.getElementById('vpsOpenNewTabSwitch');
    if (switchEl) {
        const savedPref = localStorage.getItem('vps_open_new_tab_pref');
        if (savedPref !== null) {
            switchEl.checked = (savedPref === 'true');
        }
        switchEl.addEventListener('change', function () {
            localStorage.setItem('vps_open_new_tab_pref', switchEl.checked);
        });
    }
}

// เรียกสร้างปุ่ม VPS ทันที
renderVpsButtons();

// ═══════════════════════════════════════════════════════════
//  MULTI-VPS LIVE AGGREGATOR RENDERER
// ═══════════════════════════════════════════════════════════
function renderMultiNodeSummary(summary) {
    if (!summary) return;

    // 1. Update Grand Totals
    const grandBalanceEl = document.getElementById('grand-total-balance');
    if (grandBalanceEl && summary.total_balance !== undefined) {
        grandBalanceEl.textContent = `$${summary.total_balance.toFixed(2)}`;
    }

    const grandRoundEl = document.getElementById('grand-round-profit');
    const grandRoundThbEl = document.getElementById('grand-round-profit-thb');
    if (grandRoundEl && summary.total_round_profit_usd !== undefined) {
        const sign = summary.total_round_profit_usd >= 0 ? '+' : '';
        const color = summary.total_round_profit_usd >= 0 ? '#10b981' : '#ef4444';
        grandRoundEl.innerHTML = `<span style="color:${color}">${sign}$${summary.total_round_profit_usd.toFixed(2)}</span>`;
        if (grandRoundThbEl) {
            grandRoundThbEl.innerHTML = `<span style="color:${color}">${sign}฿${(summary.total_round_profit_thb || 0).toFixed(2)}</span>`;
        }
    }

    const grandTodayEl = document.getElementById('grand-today-profit');
    const grandTodayThbEl = document.getElementById('grand-today-profit-thb');
    if (grandTodayEl && summary.total_today_profit_usd !== undefined) {
        const sign = summary.total_today_profit_usd >= 0 ? '+' : '';
        const color = summary.total_today_profit_usd >= 0 ? '#10b981' : '#ef4444';
        grandTodayEl.innerHTML = `<span style="color:${color}">${sign}$${summary.total_today_profit_usd.toFixed(2)}</span>`;
        if (grandTodayThbEl) {
            grandTodayThbEl.innerHTML = `<span style="color:${color}">${sign}฿${(summary.total_today_profit_thb || 0).toFixed(2)}</span>`;
        }
    }

    const grandWinLossEl = document.getElementById('grand-win-loss');
    const grandWinRateEl = document.getElementById('grand-win-rate');
    if (grandWinLossEl) {
        const totalTrades = (summary.total_win || 0) + (summary.total_loss || 0);
        grandWinLossEl.innerHTML = `<span style="color:#10b981">${summary.total_win || 0} W</span> / <span style="color:#ef4444">${summary.total_loss || 0} L</span>`;
        if (grandWinRateEl) {
            const wr = totalTrades > 0 ? ((summary.total_win / totalTrades) * 100).toFixed(1) : '0.0';
            grandWinRateEl.textContent = `Win Rate: ${wr}% (${totalTrades} ไม้)`;
        }
    }

    const activeCountEl = document.getElementById('multi-vps-active-count');
    if (activeCountEl) {
        activeCountEl.textContent = `${summary.active_nodes_count || 0}/${summary.total_nodes_count || 0} Nodes Online`;
        if ((summary.active_nodes_count || 0) > 0) {
            activeCountEl.style.color = '#10b981';
            activeCountEl.style.background = 'rgba(16, 185, 129, 0.15)';
        } else {
            activeCountEl.style.color = '#ef4444';
            activeCountEl.style.background = 'rgba(239, 68, 68, 0.15)';
        }
    }

    const lastSyncEl = document.getElementById('multi-vps-last-sync');
    if (lastSyncEl) {
        const d = new Date();
        lastSyncEl.textContent = `🕒 ซิงค์ล่าสุด: ${d.toLocaleTimeString()}`;
    }

    // 2. Render Node Cards
    const nodesContainer = document.getElementById('multi-vps-nodes-container');
    if (nodesContainer && Array.isArray(summary.nodes)) {
        nodesContainer.innerHTML = '';
        summary.nodes.forEach(node => {
            const card = document.createElement('div');
            card.className = 'node-badge-item';

            const isOnline = node.is_online;
            const statusDotClass = isOnline ? 'node-status-dot online' : 'node-status-dot offline';
            
            const todayProfit = node.today_profit_usd || 0;
            const todaySign = todayProfit >= 0 ? '+' : '';
            const todayColor = todayProfit >= 0 ? '#10b981' : '#ef4444';

            const roundProfit = node.round_profit_usd || 0;
            const roundSign = roundProfit >= 0 ? '+' : '';
            const roundColor = roundProfit >= 0 ? '#10b981' : '#ef4444';

            const tradingBadge = node.is_trading 
                ? `<span style="font-size:10px; background:rgba(245,158,11,0.2); color:#f59e0b; padding:1px 6px; border-radius:4px; font-weight:600;">⚡ กำลังเทรด</span>` 
                : `<span style="font-size:10px; background:rgba(148,163,184,0.15); color:#94a3b8; padding:1px 6px; border-radius:4px; font-weight:500;">💤 สแตนด์บาย</span>`;

            const strategyBadge = node.trading_strategy 
                ? `<span style="font-size:10px; background:rgba(59,130,246,0.15); color:#60a5fa; padding:1px 6px; border-radius:4px; font-weight:600; font-family:'Share Tech Mono',monospace;">${node.trading_strategy}</span>`
                : '';

            const maxLossStreak = node.max_loss_streak || 0;
            const maxLossAsset = (node.max_loss_streak_asset || '').toUpperCase();
            const maxLossHtml = maxLossStreak > 0
                ? `<span style="color:#ef4444; font-weight:700; font-family:'Share Tech Mono',monospace;">${maxLossStreak}${maxLossAsset ? ` On ${maxLossAsset}` : ''}</span>`
                : `<span style="color:var(--text3); font-size:11px;">0</span>`;

            card.innerHTML = `
                <div style="display:flex; align-items:center; justify-content:space-between;">
                    <div style="display:flex; align-items:center; gap:6px; font-weight:700; font-size:12.5px; color:var(--text);">
                        <span class="${statusDotClass}"></span>
                        <span>${node.node_name}</span>
                    </div>
                    ${tradingBadge}
                </div>
                ${strategyBadge ? `
                <div style="display:flex; align-items:center; justify-content:space-between; margin-top:2px;">
                    <span style="font-size:11px; color:var(--text3);">Strategy:</span>
                    ${strategyBadge}
                </div>` : ''}
                <div style="display:flex; align-items:baseline; justify-content:space-between; margin-top:2px;">
                    <span style="font-size:11px; color:var(--text3);">Balance:</span>
                    <span style="font-family:'Share Tech Mono',monospace; font-weight:700; font-size:14px; color:${isOnline ? 'var(--text)' : '#64748b'};">$${(node.balance || 0).toFixed(2)}</span>
                </div>
                <div style="display:flex; align-items:center; justify-content:space-between; font-size:11px;">
                    <span style="color:var(--text3);">กำไรรอบนี้:</span>
                    <span style="font-family:'Share Tech Mono',monospace; font-weight:700; color:${roundColor};">${roundSign}$${roundProfit.toFixed(2)}</span>
                </div>
                <div style="display:flex; align-items:center; justify-content:space-between; font-size:11px;">
                    <span style="color:var(--text3);">วันนี้:</span>
                    <span style="font-family:'Share Tech Mono',monospace; font-weight:700; color:${todayColor};">${todaySign}$${todayProfit.toFixed(2)}</span>
                </div>
                <div style="display:flex; align-items:center; justify-content:space-between; font-size:11px;">
                    <span style="color:var(--text3);">Max Loss:</span>
                    ${maxLossHtml}
                </div>
                <div style="display:flex; align-items:center; justify-content:space-between; font-size:10px; color:var(--text3); border-top:1px dashed var(--border); padding-top:4px; margin-top:2px;">
                    <span>Account: ${node.account_id || '-'}</span>
                    <span>${node.win_count || 0}W / ${node.loss_count || 0}L</span>
                </div>
            `;
            nodesContainer.appendChild(card);
        });
    }
}

// Initial fetch from REST API in case websocket hasn't delivered yet
function fetchInitialMultiNodeSummary() {
    fetch('/api/multi_node_summary')
        .then(res => res.json())
        .then(res => {
            if (res.status === 'success' && res.data) {
                renderMultiNodeSummary(res.data);
            }
        })
        .catch(err => console.debug('Initial multi_node_summary fetch:', err));
}

// Attach manual refresh button
const refreshNodesBtn = document.getElementById('btn-manual-sync-nodes');
if (refreshNodesBtn) {
    refreshNodesBtn.addEventListener('click', fetchInitialMultiNodeSummary);
}

// Trigger initial fetch
fetchInitialMultiNodeSummary();

// ดึง account_name และ status เริ่มต้นทันที
fetch('/api/status').then(r => r.json()).then(st => {
    if (st && st.account_name) {
        const el = document.getElementById('account-name-badge');
        if (el) el.textContent = st.account_name;
    }
}).catch(() => {});

// เชื่อมต่อแบบ initial: ดึง balance แล้วปิด WS เพื่อประหยัด traffic
connectRustWebSocket('initial');

// ดึงออเดอร์ที่เปิดค้างอยู่จาก Deriv ในกรณีที่เพิ่งโหลดหน้าเว็บ
window.syncOpenOrdersFromDeriv();

// ══════════════════════════════════════════
//  4. CONDITION STOP TRADE → enable/disable
// ══════════════════════════════════════════
const DISABLED_STYLE = 'background:darkgray;color:#444;cursor:not-allowed;opacity:.7;';

function applyConditionUI() {
    const idx = document.getElementById('conditionStopTrade').selectedIndex;
    const targetMoney = document.getElementById('targetMoney');
    const targetLot = document.getElementById('targetLot');

    const setField = (el, enabled) => {
        el.disabled = !enabled;
        el.style.cssText = enabled ? '' : DISABLED_STYLE;
    };

    if (idx === 0) { setField(targetMoney, true); setField(targetLot, false); }
    else if (idx === 1) { setField(targetMoney, false); setField(targetLot, true); }
    else { setField(targetMoney, false); setField(targetLot, false); }
}

document.getElementById('conditionStopTrade').addEventListener('change', () => {
    applyConditionUI();
    updateJSON();
});
applyConditionUI();

// Martingale list — enable only when martingale selected
document.querySelectorAll('input[name="useMartingale"]').forEach(rb => {
    rb.addEventListener('change', () => {
        const isMart = rb.value === 'martingale' && rb.checked;
        const listEl = document.getElementById('martingaleList');
        listEl.disabled = !isMart;
        listEl.style.cssText = isMart ? '' : DISABLED_STYLE;
        updateJSON();
    });
});
// init martingale list state
(function () {
    const isMart = document.querySelector('input[name="useMartingale"]:checked')?.value === 'martingale';
    const listEl = document.getElementById('martingaleList');
    listEl.disabled = !isMart;
    listEl.style.cssText = isMart ? '' : DISABLED_STYLE;
})();

// ══════════════════════════════════════════
//  THEME SWITCHER
// ══════════════════════════════════════════
document.querySelectorAll('.theme-btn').forEach(btn => {
    btn.addEventListener('click', () => {
        document.body.setAttribute('data-theme', btn.dataset.theme);
        document.querySelectorAll('.theme-btn').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        updateJSON();
        updateChartTheme();
    });
});

// ══════════════════════════════════════════
//  MODAL
// ══════════════════════════════════════════
const modal = document.getElementById('settingsModal');
document.getElementById('openSettingsBtn').addEventListener('click', () => modal.classList.add('active'));
document.getElementById('closeModalBtn').addEventListener('click', () => modal.classList.remove('active'));

const tradeSettingsModal = document.getElementById('tradeSettingsModal');
const openTradeSettingsBtn = document.getElementById('openTradeSettingsBtn');
if (openTradeSettingsBtn) {
    openTradeSettingsBtn.addEventListener('click', () => tradeSettingsModal?.classList.add('active'));
}
const closeTradeSettingsBtn = document.getElementById('closeTradeSettingsBtn');
if (closeTradeSettingsBtn) {
    closeTradeSettingsBtn.addEventListener('click', () => tradeSettingsModal?.classList.remove('active'));
}
if (tradeSettingsModal) {
    tradeSettingsModal.addEventListener('click', e => { if (e.target === tradeSettingsModal) tradeSettingsModal.classList.remove('active'); });
}

document.getElementById('cancelBtn').addEventListener('click', () => modal.classList.remove('active'));
const loadDefaultBtn = document.getElementById('loadDefaultBtn');
if (loadDefaultBtn) {
    loadDefaultBtn.addEventListener('click', async () => {
        const initLoadingScreen = document.getElementById('initLoadingScreen');
        if (initLoadingScreen) {
            initLoadingScreen.style.display = 'flex';
            initLoadingScreen.style.visibility = 'visible';
            initLoadingScreen.style.opacity = '1';
        }

        try {
            const res = await fetch('/api/setup/default', { cache: 'no-store' });
            const data = await res.json();

            if (data && Object.keys(data).length > 0) {
                await fetch('/api/setup', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(data)
                });
                await loadSetup();
            }
        } catch (e) {
            console.error("Failed to load default setup", e);
            alert("ไม่สามารถโหลดค่าเริ่มต้นได้");
            if (initLoadingScreen) {
                initLoadingScreen.style.opacity = '0';
                setTimeout(() => {
                    initLoadingScreen.style.visibility = 'hidden';
                    initLoadingScreen.style.display = 'none';
                }, 500);
            }
        }
    });
}
document.getElementById('saveBtn').addEventListener('click', async () => {
    updateJSON();

    const currentSettings = collectSettings();
    try {
        await fetch('/api/setup', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(currentSettings)
        });

        let startRaw = document.getElementById('startDate').value;
        let stopRaw = document.getElementById('stopDate').value;
        let useSch = document.getElementById('useSchedule') ? document.getElementById('useSchedule').checked : false;

        let startFormat = startRaw ? startRaw.replace("T", " ") + ":00" : null;
        let stopFormat = stopRaw ? stopRaw.replace("T", " ") + ":00" : null;

        // ซิงค์การตั้งค่า KAMA ไปยัง /api/settings ด้วย (Approach 1)
        try {
            const settingsRes = await fetch('/api/settings', { cache: 'no-store' });
            if (settingsRes.ok) {
                let settingsData = await settingsRes.json();
                if (!settingsData.settings) settingsData.settings = {};
                if (!settingsData.settings.parameters) settingsData.settings.parameters = {};
                settingsData.settings.parameters.choppy_combined = {
                    mode: document.getElementById('kamaChoppyMode') ? document.getElementById('kamaChoppyMode').value : 'or',
                    group_size: document.getElementById('kamaGroupSize') ? parseInt(document.getElementById('kamaGroupSize').value) || 15 : 15,
                    chop_period: document.getElementById('kamaChopPeriod') ? parseInt(document.getElementById('kamaChopPeriod').value) || 14 : 14,
                    chop_threshold: document.getElementById('kamaChopThreshold') ? parseFloat(document.getElementById('kamaChopThreshold').value) || 61.8 : 61.8,
                    kama_er_threshold: document.getElementById('kamaErThreshold') ? parseFloat(document.getElementById('kamaErThreshold').value) || 0.35 : 0.35,
                    body_ratio_threshold: document.getElementById('kamaBodyRatioThreshold') ? parseFloat(document.getElementById('kamaBodyRatioThreshold').value) || 0.30 : 0.30,
                    min_switches: document.getElementById('kamaMinSwitches') ? parseInt(document.getElementById('kamaMinSwitches').value) || 3 : 3
                };
                await fetch('/api/settings', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(settingsData)
                });
            }
        } catch (errSettings) {
            console.warn("Could not sync /api/settings:", errSettings);
        }

        await fetch('/api/tradeControl', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                startTradeTime: startFormat,
                stopTradeTime: stopFormat,
                useSchedule: useSch
            })
        });
    } catch (e) {
        console.error("Could not save settings:", e);
    }

    modal.classList.remove('active');
});
modal.addEventListener('click', e => { if (e.target === modal) modal.classList.remove('active'); });


// ══════════════════════════════════════════
//  THERESHOLD MODAL
// ══════════════════════════════════════════
const modalThereshold = document.getElementById('settingsTheresholdModal');
document.getElementById('closeModalTheresholdBtn').addEventListener('click', () => modalThereshold.classList.remove('active'));
document.getElementById('cancelTheresholdBtn').addEventListener('click', () => modalThereshold.classList.remove('active'));
document.getElementById('saveTheresholdBtn').addEventListener('click', () => {
    updateTheresholdJSON(); modalThereshold.classList.remove('active');
});
modalThereshold.addEventListener('click', e => { if (e.target === modal) modalThereshold.classList.remove('active'); });

let globalThresholds = [];
let globalNoiseConfig = {};
async function loadAllThresholds() {
    try {
        let res = await fetch('/api/thereshold', { cache: 'no-store' });
        let data = await res.json();
        if (Array.isArray(data)) {
            globalThresholds = data;
        } else if (data && data.thresholds) {
            globalThresholds = data.thresholds;
            globalNoiseConfig = data.noiseConfig || {};
        }
        console.log("Loaded globalThresholds:", globalThresholds);
    } catch (e) {
        console.error("Failed to load thresholds", e);
    }
}

function populateThresholdInputs() {
    const selectedAssetRadio = document.querySelector('input[name="asset_threshold"]:checked');
    if (!selectedAssetRadio) return;
    const asset = selectedAssetRadio.value;
    const t = globalThresholds.find(x => x.asset === asset);
    console.log("Populating threshold for", asset, "Found data:", t);
    if (t) {
        document.getElementById('flatTheresholdValue').value = t.flatTheresholdValue !== undefined ? t.flatTheresholdValue : 14;
        document.getElementById('MACDGapValue').value = t.MACDGapValue !== undefined ? t.MACDGapValue : 2;
        document.getElementById('th_altCandleAtrMultiplier').value = t.altCandleAtrMultiplier !== undefined ? t.altCandleAtrMultiplier : 0.5;
    } else {
        // Default if not found
        document.getElementById('flatTheresholdValue').value = 14;
        document.getElementById('MACDGapValue').value = 2;
        document.getElementById('th_altCandleAtrMultiplier').value = 0.5;
    }

    const nCfg = globalNoiseConfig || {};
    if (nCfg.isCheckNoise) {
        const r = document.querySelector(`input[name="th_isCheckNoise"][value="${nCfg.isCheckNoise}"]`);
        if (r) r.checked = true;
    } else {
        const rNoise = document.querySelector(`input[name="th_isCheckNoise"][value="no"]`);
        if (rNoise) rNoise.checked = true;
    }

    document.querySelectorAll('input[name="th_noiseTypeCheck"]').forEach(cb => cb.checked = false);
    if (nCfg.noiseTypeCheck && Array.isArray(nCfg.noiseTypeCheck)) {
        nCfg.noiseTypeCheck.forEach(v => {
            const cb = document.querySelector(`input[name="th_noiseTypeCheck"][value="${v}"]`);
            if (cb) cb.checked = true;
        });
    }

    if (nCfg.flatCondition) {
        const r = document.querySelector(`input[name="th_flatCondition"][value="${nCfg.flatCondition}"]`);
        if (r) r.checked = true;
    } else {
        const rFlat = document.querySelector(`input[name="th_flatCondition"][value="case1"]`);
        if (rFlat) rFlat.checked = true;
    }

    // ═══ Load New Noise Filters ═══
    if (document.getElementById('th_checkAdx')) document.getElementById('th_checkAdx').checked = (nCfg.checkAdx === 'yes');
    if (document.getElementById('th_adxThreshold')) document.getElementById('th_adxThreshold').value = nCfg.adxThreshold !== undefined ? nCfg.adxThreshold : 25;
    if (document.getElementById('th_checkChoppy')) document.getElementById('th_checkChoppy').checked = (nCfg.checkChoppy === 'yes');
    if (document.getElementById('th_choppyThreshold')) document.getElementById('th_choppyThreshold').value = nCfg.choppyThreshold !== undefined ? nCfg.choppyThreshold : 50;
    if (document.getElementById('th_checkRange')) document.getElementById('th_checkRange').checked = (nCfg.checkRange === 'yes');
    if (document.getElementById('th_checkBbSqueeze')) document.getElementById('th_checkBbSqueeze').checked = (nCfg.checkBbSqueeze === 'yes');
    if (document.getElementById('th_bbBandwidthThreshold')) document.getElementById('th_bbBandwidthThreshold').value = nCfg.bbBandwidthThreshold !== undefined ? nCfg.bbBandwidthThreshold : 0.5;
}

document.querySelectorAll('input[name="asset_threshold"]').forEach(radio => {
    radio.addEventListener('change', populateThresholdInputs);
});

document.getElementById('openSettingsTheresholdBtn').addEventListener('click', async () => {
    await loadAllThresholds();
    // check if any radio is selected, if not select first one
    const selected = document.querySelector('input[name="asset_threshold"]:checked');
    if (!selected) {
        const first = document.querySelector('input[name="asset_threshold"]');
        if (first) first.checked = true;
    }
    populateThresholdInputs();
    modalThereshold.classList.add('active');
});

async function updateTheresholdJSON() {
    const selectedAssetRadio = document.querySelector('input[name="asset_threshold"]:checked');
    if (!selectedAssetRadio) {
        alert("Please select an asset first");
        return;
    }
    const asset = selectedAssetRadio.value;
    const pVal = document.getElementById('flatTheresholdValue').value;
    const flatTheresholdValue = pVal === "" ? 14 : parseFloat(pVal);

    const mVal = document.getElementById('MACDGapValue').value;
    const MACDGapValue = mVal === "" ? 2 : parseFloat(mVal);

    const aVal = document.getElementById('th_altCandleAtrMultiplier').value;
    const altCandleAtrMultiplier = aVal === "" ? 0.5 : parseFloat(aVal);

    const map = {
        "vol10": "R_10", "vol10_1s": "1HZ10V", "vol15_1s": "1HZ15V",
        "vol25": "R_25", "vol25_1s": "1HZ25V", "vol30_1s": "1HZ30V",
        "vol50": "R_50", "vol50_1s": "1HZ50V", "vol75": "R_75",
        "vol75_1s": "1HZ75V", "vol90_1s": "1HZ90V", "vol100": "R_100",
        "vol100_1s": "1HZ100V"
    };
    const assetCode = map[asset] || asset;

    const isCheckNoiseNode = document.querySelector('input[name="th_isCheckNoise"]:checked');
    const isCheckNoise = isCheckNoiseNode ? isCheckNoiseNode.value : "no";

    const noiseTypeCheck = [];
    document.querySelectorAll('input[name="th_noiseTypeCheck"]:checked').forEach(cb => {
        noiseTypeCheck.push(cb.value);
    });

    const flatConditionNode = document.querySelector('input[name="th_flatCondition"]:checked');
    const flatCondition = flatConditionNode ? flatConditionNode.value : "case1";
    const flatConditionDesc = flatConditionNode ? (flatConditionNode.getAttribute('Desc') || '') : '';

    // ═══ Collect New Noise Filters ═══
    const checkAdx = document.getElementById('th_checkAdx') && document.getElementById('th_checkAdx').checked ? 'yes' : 'no';
    const adxThreshold = parseFloat(document.getElementById('th_adxThreshold')?.value) || 25;
    const checkChoppy = document.getElementById('th_checkChoppy') && document.getElementById('th_checkChoppy').checked ? 'yes' : 'no';
    const choppyThreshold = parseFloat(document.getElementById('th_choppyThreshold')?.value) || 50;
    const checkRange = document.getElementById('th_checkRange') && document.getElementById('th_checkRange').checked ? 'yes' : 'no';
    const checkBbSqueeze = document.getElementById('th_checkBbSqueeze') && document.getElementById('th_checkBbSqueeze').checked ? 'yes' : 'no';
    const bbBandwidthThreshold = parseFloat(document.getElementById('th_bbBandwidthThreshold')?.value) || 0.5;

    const payload = {
        asset,
        assetCode,
        flatTheresholdValue,
        MACDGapValue,
        altCandleAtrMultiplier,
        isCheckNoise,
        noiseTypeCheck,
        flatCondition,
        flatConditionDesc,
        checkAdx,
        adxThreshold,
        checkChoppy,
        choppyThreshold,
        checkRange,
        checkBbSqueeze,
        bbBandwidthThreshold
    };

    try {
        const res = await fetch('/api/thereshold', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(payload)
        });
        const data = await res.json();
        if (data.status === 'success') {
            // Update local copy
            let found = false;
            for (let t of globalThresholds) {
                if (t.asset === asset) {
                    t.assetCode = assetCode;
                    t.flatTheresholdValue = flatTheresholdValue;
                    t.MACDGapValue = MACDGapValue;
                    t.altCandleAtrMultiplier = altCandleAtrMultiplier;
                    found = true;
                    break;
                }
            }
            if (!found) {
                globalThresholds.push({
                    asset, assetCode, flatTheresholdValue, MACDGapValue, altCandleAtrMultiplier
                });
            }

            globalNoiseConfig = {
                isCheckNoise,
                noiseTypeCheck,
                flatCondition,
                flatConditionDesc,
                checkAdx,
                adxThreshold,
                checkChoppy,
                choppyThreshold,
                checkRange,
                checkBbSqueeze,
                bbBandwidthThreshold
            };

            alert("บันทึกการตั้งค่า Threshold เรียบร้อยแล้ว (อัพเดต Backend)");
        }
    } catch (e) {
        console.error("Failed to save threshold", e);
    }
}

// ══════════════════════════════════════════
//  TABS
// ══════════════════════════════════════════
document.querySelectorAll('.tab-btn').forEach(btn => {
    btn.addEventListener('click', () => {
        document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
        document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
        btn.classList.add('active');
        document.getElementById(btn.dataset.tab + '-tab').classList.add('active');
        if (btn.dataset.tab === 'jsonsetting') updateJSON();
    });
});

// ══════════════════════════════════════════
//  GRANULARITY BUTTONS
// ══════════════════════════════════════════
document.querySelectorAll('.gran-btn').forEach(btn => {
    btn.addEventListener('click', () => {
        document.querySelectorAll('.gran-btn').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        document.getElementById('status-gran').textContent = 'Granularity: ' + btn.textContent;
        updateJSON();
    });
});
document.querySelectorAll('.gran-setting-btn').forEach(btn => {
    btn.addEventListener('click', () => {
        document.querySelectorAll('.gran-setting-btn').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        updateJSON();
    });
});

// ══════════════════════════════════════════
//  ATTACH CHANGE LISTENERS → AUTO UPDATE JSON
// ══════════════════════════════════════════
const watchIds = [
    'useSchedule', 'scheduleTradeNo', 'atrPeriod', 'atrMulti', 'ciPeriod', 'adxPeriod', 'bbPeriod', 'smcPeriod',
    'emaShortPeriod', 'emaShortType', 'emaShortColor', 'emaShortToggle',
    'emaMediumPeriod', 'emaMediumType', 'emaMediumColor', 'emaMediumToggle',
    'emaLongPeriod', 'emaLongType', 'emaLongColor', 'emaLongToggle',
    'martingaleList', 'maxLossCon', 'targetMoney', 'targetLot', 'telegramToggle', 'playSoundToggle',
    'spikeShowToggle', 'spikeGreenColor', 'spikeRedColor', 'showTradeMarkersToggle',
    'autoSellTimeoutToggle', 'autoSellTimeoutSeconds',
    'borrowSignalToggle',
    'whipsawZoneToggle',
    'saveTrackOrderToggle',
    'checkBbFlatChoppyToggle',
    'checkBbSqueezeToggle',
    'checkKarmaChoppyToggle',
    'chartRightOffset',
    'kamaChoppyMode',
    'kamaGroupSize',
    'kamaChopPeriod',
    'kamaChopThreshold',
    'kamaErThreshold',
    'kamaBodyRatioThreshold',
    'kamaMinSwitches'
];
watchIds.forEach(id => {
    const el = document.getElementById(id);
    if (el) {
        el.addEventListener('change', () => {
            if (id !== 'showTradeMarkersToggle') {
                updateJSON();
                if (['borrowSignalToggle', 'whipsawZoneToggle', 'buyMethodSelect', 'checkBbFlatChoppyToggle', 'checkBbSqueezeToggle', 'checkKarmaChoppyToggle', 'saveTrackOrderToggle'].includes(id)) {
                    autoSaveSetup();
                }
            }
            // ถ้าเปลี่ยนการตั้งค่า Spike หรือ Markers หรือ EMA ให้วาดกราฟใหม่
            if ((id.startsWith('spike') || id === 'showTradeMarkersToggle' || id.startsWith('ema')) && allAnalysisData && allAnalysisData.length > 0) {
                if (id.includes('Color')) {
                    if (id === 'emaShortColor' && emaShortSeries) emaShortSeries.applyOptions({ color: document.getElementById('emaShortColor').value });
                    if (id === 'emaMediumColor' && emaMediumSeries) emaMediumSeries.applyOptions({ color: document.getElementById('emaMediumColor').value });
                    if (id === 'emaLongColor' && emaLongSeries) emaLongSeries.applyOptions({ color: document.getElementById('emaLongColor').value });
                }
                const prevLogicalRange = lwChart && lwChart.timeScale() ? lwChart.timeScale().getVisibleLogicalRange() : null;
                renderChart(allAnalysisData);
                if (prevLogicalRange && lwChart) {
                    try { lwChart.timeScale().setVisibleLogicalRange(prevLogicalRange); } catch (e) { }
                }
            } else if (id === 'chartRightOffset' && lwChart) {
                lwChart.timeScale().applyOptions({ rightOffset: parseInt(el.value) || 5 });
            }
        });
    }
});
document.querySelectorAll('input[name="asset"]').forEach(cb =>
    cb.addEventListener('change', () => {
        const checked = [...document.querySelectorAll('input[name="asset"]:checked')].map(c => c.value);
        document.getElementById('status-asset').textContent = 'Asset: ' + (checked.length ? checked.join(', ') : '—');
        updateJSON();
    })
);

const tooltipToggleEl = document.getElementById('showStrategyTooltipToggle');
if (tooltipToggleEl) {
    tooltipToggleEl.addEventListener('change', () => {
        if (tooltipToggleEl.checked) {
            window.fetchGlobalStrategyData();
        } else {
            const chartTooltip = document.querySelector('.chart-tooltip-panel');
            if (chartTooltip) chartTooltip.style.display = 'none';
        }
    });
}

// ══════════════════════════════════════════
//  CHART ASSET SELECTOR — สลับ asset ที่แสดงบนกราฟ
// ══════════════════════════════════════════
window.switchChartAsset = function (selected) {
    if (!selected || !allDataByAsset[selected]) return;

    currentChartAsset = selected;
    allAnalysisData = allDataByAsset[selected];
    document.getElementById('status-count').textContent = `Candles: ${allAnalysisData.length}`;
    document.getElementById('status-asset').textContent = `Asset: ${selected.toUpperCase()}`;

    // อัปเดตปุ่ม active
    document.querySelectorAll('.chart-asset-btn').forEach(btn => {
        if (btn.dataset.asset === selected) {
            btn.style.background = 'var(--accent)';
            btn.style.color = '#fff';
            btn.style.borderColor = 'var(--accent)';
        } else {
            btn.style.background = 'var(--bg3)';
            btn.style.color = 'var(--text)';
            btn.style.borderColor = 'var(--border)';
        }
    });

    // วาดกราฟใหม่ด้วยข้อมูลของ asset ที่เลือก
    initChart();
    renderChart(allAnalysisData);
    lwChart.timeScale().fitContent();
    renderTable(allAnalysisData);

    // อัปเดต Dropdown ของ Manual Trade ให้ตรงกับกราฟ
    const manualTradeAssetSelect = document.getElementById('manualTradeAsset');
    if (manualTradeAssetSelect) {
        manualTradeAssetSelect.value = selected;
    }

    console.log(`📊 Switched chart to: ${selected}`);
};

// ══════════════════════════════════════════
//  COPY JSON BUTTON
// ══════════════════════════════════════════
document.getElementById('copyJsonBtn').addEventListener('click', () => {
    const ta = document.getElementById('jsonSettingTextarea');
    ta.select();
    document.execCommand('copy');
    const btn = document.getElementById('copyJsonBtn');
    btn.textContent = '✅ คัดลอกแล้ว!';
    setTimeout(() => btn.textContent = '📋 คัดลอก JSON', 2000);
});

// ══════════════════════════════════════════
//  INITIAL RENDER
// ══════════════════════════════════════════
// ⚡ แก้ Race Condition: ต้อง await loadSetup() ให้เสร็จก่อน
//    เพื่อให้ scheduleTradeNo ถูกโหลดจาก setup.json ก่อนคำนวณกำไร
//    (บน e2.micro ที่ช้ากว่า loadSetup อาจเสร็จหลัง reloadTradeHistory
//     ทำให้ scheduleTradeNo ผิด → กำไรรอบนี้กลายเป็น 0)
(async () => {
    await loadSetup();
    reloadTradeHistory(true); // โหลดประวัติและคำนวณกำไรทันทีหลัง setup พร้อม
})();

// ══════════════════════════════════════════
//  SCHEDULE TIMER
// ══════════════════════════════════════════
setInterval(() => {
    // อัปเดต Schedule Status ทุกวินาที
    checkScheduleStatus();

    const useScheduleEl = document.getElementById('useSchedule');
    if (!useScheduleEl || !useScheduleEl.checked) return;

    const startVal = document.getElementById('startDate').value;
    const stopVal = document.getElementById('stopDate').value;
    if (!startVal) return;

    const start = new Date(startVal).getTime();
    const now = Date.now();

    if (!isTradeRunning && now >= start && !scheduleTradeCompleted) {
        const stop = stopVal ? new Date(stopVal).getTime() : 0;
        if (!stop || now < stop) {
            // ╔══════════════════════════════════════════════════════════════════╗
            // ║ ⚠️ CRITICAL BUG FIX — ห้ามลบหรือแก้ไขโค้ดส่วนนี้! ⚠️           ║
            // ║ ป้องกัน "Schedule Trade Round +1 Bug"                           ║
            // ║ ต้อง fetch /api/status ตรวจสอบก่อนว่าบอทรันอยู่แล้วหรือไม่      ║
            // ║ ก่อน auto-start เทรด ถ้าลบส่วนนี้ → เปิดหน้าเว็บระหว่างเทรด    ║
            // ║ → goTradeBtn.click() ซ้ำ → รอบเทรดจะ +1 ซ้ำผิดพลาด            ║
            // ╚══════════════════════════════════════════════════════════════════╝
            scheduleTradeCompleted = true; // ล็อคไว้ก่อน ป้องกัน trigger ซ้ำระหว่างรอ fetch
            console.log("⏰ Schedule reached! Checking server status before auto-start...");
            fetch('/api/status').then(r => r.json()).then(status => {
                const alreadyRunning = status.bot_count > 0 && status.active_assets && status.active_assets.length > 0;
                if (alreadyRunning) {
                    // บอทรันอยู่แล้ว (Backend Scheduler เริ่มไปแล้ว) — ไม่ต้อง auto-start ซ้ำ
                    isTradeRunning = true;
                    console.log(`🛡️ [Schedule] บอทรันอยู่แล้ว (${status.bot_count} bots: ${status.active_assets.join(', ')}) — ข้ามการ auto-start`);
                } else {
                    // บอทยังไม่ได้รัน — เริ่มเทรดอัตโนมัติ
                    scheduleTradeCompleted = false; // ปลดล็อคเพื่อให้ goTradeBtn ทำงานได้
                    console.log("✅ [Schedule] Server ไม่มีบอทรันอยู่ — Auto starting trade...");
                    shouldStayConnected = true;
                    if (!localWs || localWs.readyState !== WebSocket.OPEN) {
                        console.log('🔌 Schedule: Reconnecting WS before auto-trade...');
                        connectRustWebSocket('trade');
                        setTimeout(() => {
                            document.getElementById('goTradeBtn').click();
                        }, 1500);
                    } else {
                        document.getElementById('goTradeBtn').click();
                    }
                }
            }).catch(e => {
                console.error('❌ [Schedule] Error checking server status:', e);
                scheduleTradeCompleted = false; // ปลดล็อคเมื่อ fetch ล้มเหลว
            });
        }
    }

    if (isTradeRunning && stopVal) {
        const stop = new Date(stopVal).getTime();
        const cond = document.getElementById('conditionStopTrade').value;
        if (now >= stop && cond === 'stopDate') {
            console.log("⏰ StopDate reached! Auto stopping trade...");
            document.getElementById('stopTradeBtn').click();
            document.getElementById('useSchedule').checked = false;
            isTradeRunning = false;
            if (tradeDurationInterval) clearInterval(tradeDurationInterval);
            tradeDurationInterval = null;
            updateTradingDuration();
            updateJSON();
        }
    }
}, 1000);

// ══════════════════════════════════════════
//  LOAD TRADE HISTORY
// ══════════════════════════════════════════
async function reloadTradeHistory(isAuto = false) {
    const today = new Date();
    const d = String(today.getDate()).padStart(2, '0');
    const m = String(today.getMonth() + 1).padStart(2, '0');
    const y = today.getFullYear();
    const dateStr = `${y}-${m}-${d}`;

    const loadHistoryBtn = document.getElementById('loadHistoryBtn');
    if (loadHistoryBtn && !isAuto) {
        loadHistoryBtn.textContent = 'กำลังโหลด...';
    }

    try {
        const res = await fetch(`/api/history?date=${dateStr}`);
        const historyData = await res.json();

        window.currentTradeHistory = historyData; // Store globally for chart markers
        updateBalanceSummary(currentBalance); // อัปเดตการแสดงผลกำไรทันที

        // Trigger chart markers update
        if (typeof updateChartMarkers === 'function') {
            updateChartMarkers();
        }

        const tbody = document.querySelector('#historyTable tbody');

        const titleEl = document.getElementById('historyStrategyTitle');
        if (titleEl) {
            if (historyData && historyData.length > 0) {
                const tradeWithStrategy = historyData.find(t => t.tradeStrategy);
                titleEl.textContent = tradeWithStrategy ? `(${tradeWithStrategy.tradeStrategy})` : '';
            } else {
                titleEl.textContent = '';
            }
        }

        if (tbody) {
            const filterContainer = document.getElementById('history-asset-filters');
            if (filterContainer) filterContainer.innerHTML = '';

            const uniqueAssets = [...new Set((historyData || []).map(item => item.Asset).filter(Boolean))];

            const renderHistoryTable = (filterAsset) => {
                tbody.innerHTML = '';
                if (!historyData || historyData.length === 0) {
                    tbody.innerHTML = '<tr class="empty-row"><td colspan="15">ไม่พบประวัติการเทรดในวันนี้</td></tr>';
                    return;
                }

                let filteredData = historyData;
                if (filterAsset !== 'All') {
                    filteredData = historyData.filter(item => item.Asset === filterAsset);
                }

                const toggleEl = document.getElementById('toggleOnlyTrades');
                if (toggleEl && toggleEl.checked) {
                    filteredData = filteredData.filter(item => {
                        const act = String(item.thisAction || item.Action || '').toLowerCase();
                        return act.includes('call') || act.includes('put');
                    });
                }

                if (filteredData.length === 0) {
                    tbody.innerHTML = '<tr class="empty-row"><td colspan="15">ไม่พบประวัติสำหรับ Asset นี้ หรือ เงื่อนไขที่เลือก</td></tr>';
                    return;
                }

                const scheduleColors = [
                    'rgba(40, 167, 69, 0.15)',  // green
                    'rgba(0, 123, 255, 0.15)',  // blue
                    'rgba(255, 193, 7, 0.15)',  // yellow/orange
                    'rgba(23, 162, 184, 0.15)', // cyan
                    'rgba(102, 16, 242, 0.15)', // purple
                    'rgba(253, 126, 20, 0.15)'  // orange
                ];
                // Calculate running schedule balance (from oldest to newest)
                const scheduleBalances = {};
                const scheduleStats = {}; // สำหรับเก็บยอดรวมและ Max Loss Streak

                for (let i = filteredData.length - 1; i >= 0; i--) {
                    const t = filteredData[i];
                    const rNo = t.scheduleTradeNo || 0;
                    const p = t.ThisProfit || 0;

                    if (rNo > 0) {
                        if (!scheduleBalances[rNo]) scheduleBalances[rNo] = 0;
                        scheduleBalances[rNo] += p;
                        t.scheduleRunningBalance = scheduleBalances[rNo];

                        // เก็บข้อมูลสำหรับ Summary Row
                        if (!scheduleStats[rNo]) {
                            scheduleStats[rNo] = {
                                totalProfit: 0,
                                maxSubTradeByAsset: {}
                            };
                        }
                        scheduleStats[rNo].totalProfit += p;
                        const asset = t.Asset || 'Unknown';
                        const sub = t.subTradeno || 1;
                        if (!scheduleStats[rNo].maxSubTradeByAsset[asset] || sub > scheduleStats[rNo].maxSubTradeByAsset[asset]) {
                            scheduleStats[rNo].maxSubTradeByAsset[asset] = sub;
                        }
                    } else {
                        t.scheduleRunningBalance = p;
                    }
                }

                const thbRate = parseFloat(document.getElementById('thb-rate') ? document.getElementById('thb-rate').value : 35) || 35;

                let prevRunNo = filteredData.length > 0 ? filteredData[0].scheduleTradeNo || 0 : 0;

                filteredData.forEach((trade, index) => {
                    const tr = document.createElement('tr');
                    const runNo = trade.scheduleTradeNo || 0;

                    // แทรก Summary Row ถ้ารอบ schedule เปลี่ยน
                    if (prevRunNo !== 0 && prevRunNo !== runNo) {
                        const stats = scheduleStats[prevRunNo];
                        if (stats) {
                            const trSum = document.createElement('tr');
                            trSum.style.backgroundColor = '#ffffff';
                            trSum.style.color = '#000000';
                            const profitColor = stats.totalProfit > 0 ? '#10b981' : (stats.totalProfit < 0 ? '#ef4444' : '#000000');
                            const thb = stats.totalProfit * thbRate;
                            const maxStreaks = Object.entries(stats.maxSubTradeByAsset)
                                .map(([a, m]) => `${a}: ${m}`)
                                .join(', ');

                            trSum.innerHTML = `
                                        <td colspan="15" style="text-align: center; padding: 10px; font-size: 13px; font-weight: bold; border-bottom: 3px solid var(--border);">
                                            🏁 สรุปรอบที่ #${prevRunNo} &nbsp;|&nbsp; 
                                            <span style="color: ${profitColor};">
                                                ยอดรวม: ${stats.totalProfit > 0 ? '+' : ''}${stats.totalProfit.toFixed(2)} USD 
                                                (${thb > 0 ? '+' : (thb < 0 ? '-' : '')}฿${Math.abs(thb).toFixed(2)})
                                            </span> &nbsp;|&nbsp; 
                                            <span style="color: #4b5563;">Max Loss Streak: ${maxStreaks || '-'}</span>
                                        </td>
                                    `;
                            tbody.appendChild(trSum);
                        }

                        // แทรกแถวรอบที่ไม่มีการเทรด
                        if (runNo !== 0 && prevRunNo > runNo) {
                            for (let gapNo = prevRunNo - 1; gapNo > runNo; gapNo--) {
                                const trGap = document.createElement('tr');
                                trGap.style.backgroundColor = 'rgba(255, 255, 255, 0.02)';
                                trGap.innerHTML = `
                                            <td colspan="15" style="text-align: center; padding: 10px; font-style: italic; color: #888;">
                                                รอบที่ #${gapNo} : ไม่มีการเทรดเกิดขึ้น
                                            </td>
                                        `;
                                tbody.appendChild(trGap);
                            }
                        }
                    }
                    if (trade.isAnomaly) {
                        tr.style.backgroundColor = 'rgba(232, 48, 74, 0.2)'; // Highlight red for anomaly
                        tr.title = 'Anomaly: ใช้เวลาปิดไม้นานกว่าปกติ (' + (trade.actualDuration || 0) + ' วินาที)';
                    } else if (runNo > 0) {
                        const colorIndex = (runNo - 1) % scheduleColors.length;
                        tr.style.backgroundColor = scheduleColors[colorIndex];
                        tr.title = `Schedule Trade No: ${runNo}`;
                    }

                    const runningBal = trade.scheduleRunningBalance || 0;
                    const balColor = runningBal > 0 ? '#06D6A0' : (runningBal < 0 ? '#E8304A' : 'inherit');

                    const balThb = runningBal * thbRate;

                    tr.innerHTML = `
                                <td>${new Date((trade.timeCandle || 0) * 1000).toLocaleString('th-TH')}</td>
                                <td>${trade.sellTimeDisplay || '-'}</td>
                                <td><span style="font-size:11px; background:var(--accent-bg, var(--bg3)); padding:2px 8px; border-radius:12px; font-weight:600; color:var(--accent, var(--text));">#${runNo || '-'}</span></td>
                                <td>${trade.subTradeno || '-'}${(trade.subTradeno || 0) > 5 ? ' 🔴' : (trade.subTradeno || 0) >= 4 ? ' 🟠' : ''}</td>
                                <td>${trade.Asset || '-'}</td>
                                <td>${trade.thisColor === 'green' ? '🟢' : (trade.thisColor === 'red' ? '🔴' : (trade.thisColor || '-'))}</td>
                                <td>${trade.thisAction || '-'}${trade.thisAction === 'Put' ? ' 🔴' : (trade.thisAction === 'Call' ? ' 🟢' : '')}</td>
                                <td style="color:#f5a623; font-weight:bold;">${trade.tradeStrategy || '-'}${trade.codeStrategy === 'V2-W' ? ' 🧯' : ''}${trade.codeStrategy ? '<br><span style="font-size:11px; color:var(--text2); font-weight:normal;">' + trade.codeStrategy + '</span>' : ''}</td>
                                <td>${trade.noiseCode ? '<span style="font-size:11px; background:' + (trade.noiseCode === "none" ? "var(--surface2)" : "var(--red)") + '; padding:2px 6px; border-radius:4px; color:' + (trade.noiseCode === "none" ? "var(--text)" : "#ffffff") + ';">' + (trade.noiseCode === "nochecknoise" ? "no" : trade.noiseCode) + '</span>' : '-'}</td>
                                <td style="color:${trade.DiffSpot > 0 ? '#06D6A0' : (trade.DiffSpot < 0 ? '#E8304A' : 'inherit')}; font-weight:bold;">${trade.DiffSpot !== undefined && trade.DiffSpot !== 0 ? trade.DiffSpot.toFixed(2) : '-'}</td>
                                <td>${trade.MoneyTrade !== undefined ? trade.MoneyTrade.toFixed(2) : '-'}</td>
                                <td style="color:${(trade.ThisProfit || 0) > 0 ? '#06D6A0' : ((trade.ThisProfit || 0) < 0 ? '#E8304A' : 'inherit')}">
                                    ${(trade.ThisProfit || 0) > 0 ? '+' : ''}${trade.ThisProfit !== undefined ? trade.ThisProfit.toFixed(2) : '0.00'}
                                    ${trade.gaveUp ? '<br><span style="font-size:11px;color:#E8304A;font-weight:bold;">(ยอมแพ้)</span>' : ''}
                                </td>
                                <td>${trade.GrandBalance !== undefined ? trade.GrandBalance.toFixed(2) : '-'}</td>
                                <td style="color:${balColor}; font-weight:bold;">
                                    ${runningBal > 0 ? '+' : ''}${runningBal.toFixed(2)}
                                </td>
                                <td style="color:white; font-weight:bold;">
                                    ${balThb > 0 ? '+' : (balThb < 0 ? '-' : '')}฿${Math.abs(balThb).toFixed(2)}
                                </td>
                            `;
                    tbody.appendChild(tr);
                    prevRunNo = runNo;
                });

                // แทรก Summary Row สำหรับรอบ schedule สุดท้ายในตาราง
                if (prevRunNo !== 0) {
                    const stats = scheduleStats[prevRunNo];
                    if (stats) {
                        const trSum = document.createElement('tr');
                        trSum.style.backgroundColor = '#ffffff';
                        trSum.style.color = '#000000';
                        const profitColor = stats.totalProfit > 0 ? '#10b981' : (stats.totalProfit < 0 ? '#ef4444' : '#000000');
                        const thb = stats.totalProfit * thbRate;
                        const maxStreaks = Object.entries(stats.maxSubTradeByAsset)
                            .map(([a, m]) => `${a}: ${m}`)
                            .join(', ');

                        trSum.innerHTML = `
                                    <td colspan="15" style="text-align: center; padding: 10px; font-size: 13px; font-weight: bold; border-bottom: 3px solid var(--border);">
                                        🏁 สรุปรอบที่ #${prevRunNo} &nbsp;|&nbsp; 
                                        <span style="color: ${profitColor};">
                                            ยอดรวม: ${stats.totalProfit > 0 ? '+' : ''}${stats.totalProfit.toFixed(2)} USD 
                                            (${thb > 0 ? '+' : (thb < 0 ? '-' : '')}฿${Math.abs(thb).toFixed(2)})
                                        </span> &nbsp;|&nbsp; 
                                        <span style="color: #4b5563;">Max Loss Streak: ${maxStreaks || '-'}</span>
                                    </td>
                                `;
                        tbody.appendChild(trSum);
                    }

                    // แทรกแถวรอบที่ไม่มีการเทรดลงไปจนถึงรอบที่ 1
                    for (let gapNo = prevRunNo - 1; gapNo >= 1; gapNo--) {
                        const trGap = document.createElement('tr');
                        trGap.style.backgroundColor = 'rgba(255, 255, 255, 0.02)';
                        trGap.innerHTML = `
                                    <td colspan="15" style="text-align: center; padding: 10px; font-style: italic; color: var(--text3, #888);">
                                        รอบที่ #${gapNo} : ไม่มีการเทรดเกิดขึ้น
                                    </td>
                                `;
                        tbody.appendChild(trGap);
                    }
                }
            };

            const createFilterBtn = (assetName) => {
                const btn = document.createElement('button');
                btn.textContent = assetName;
                btn.style.padding = '4px 12px';
                btn.style.fontSize = '12px';
                btn.style.borderRadius = 'var(--radius)';
                btn.style.cursor = 'pointer';
                btn.style.border = '1px solid var(--border)';
                btn.style.background = assetName === 'All' ? 'var(--accent)' : 'transparent';
                btn.style.color = assetName === 'All' ? '#fff' : 'var(--text)';

                btn.addEventListener('click', () => {
                    // reset all buttons
                    Array.from(filterContainer.children).forEach(child => {
                        child.style.background = 'transparent';
                        child.style.color = 'var(--text)';
                    });
                    // set active button
                    btn.style.background = 'var(--accent)';
                    btn.style.color = '#fff';

                    renderHistoryTable(assetName);
                });
                return btn;
            };

            if (filterContainer && historyData && historyData.length > 0) {
                filterContainer.appendChild(createFilterBtn('All'));
                uniqueAssets.forEach(asset => {
                    filterContainer.appendChild(createFilterBtn(asset));
                });

                const toggleWrapper = document.createElement('div');
                toggleWrapper.style.display = 'flex';
                toggleWrapper.style.alignItems = 'center';
                toggleWrapper.style.marginLeft = 'auto'; // Push to the right

                // Add CSS for switch if not exists
                if (!document.getElementById('switch-style')) {
                    const style = document.createElement('style');
                    style.id = 'switch-style';
                    style.textContent = `
                                .switch-history { position: relative; display: inline-block; width: 34px; height: 20px; flex-shrink: 0; }
                                .switch-history input { opacity: 0; width: 0; height: 0; }
                                .switch-history .slider { position: absolute; cursor: pointer; top: 0; left: 0; right: 0; bottom: 0; background-color: var(--border2); transition: .4s; border-radius: 20px; }
                                .switch-history .slider:before { position: absolute; content: ""; height: 14px; width: 14px; left: 3px; bottom: 3px; background-color: white; transition: .4s; border-radius: 50%; }
                                .switch-history input:checked + .slider { background-color: var(--accent); }
                                .switch-history input:checked + .slider:before { transform: translateX(14px); }
                            `;
                    document.head.appendChild(style);
                }

                const toggleLabel = document.createElement('label');
                toggleLabel.className = 'switch-history';

                const toggleCheckbox = document.createElement('input');
                toggleCheckbox.type = 'checkbox';
                toggleCheckbox.id = 'toggleOnlyTrades';
                toggleCheckbox.addEventListener('change', () => {
                    let activeAsset = 'All';
                    Array.from(filterContainer.children).forEach(child => {
                        if (child.tagName === 'BUTTON' && child.style.background === 'var(--accent)') {
                            activeAsset = child.textContent;
                        }
                    });
                    renderHistoryTable(activeAsset);
                });

                const slider = document.createElement('span');
                slider.className = 'slider';

                toggleLabel.appendChild(toggleCheckbox);
                toggleLabel.appendChild(slider);

                const textLabel = document.createElement('span');
                textLabel.textContent = 'แสดงเฉพาะเทรดจริง (ซ่อน Flat, Alt, Gap)';
                textLabel.style.marginLeft = '8px';
                textLabel.style.fontSize = '12px';
                textLabel.style.color = 'var(--text)';
                textLabel.style.cursor = 'pointer';
                textLabel.addEventListener('click', () => {
                    toggleCheckbox.checked = !toggleCheckbox.checked;
                    toggleCheckbox.dispatchEvent(new Event('change'));
                });

                toggleWrapper.appendChild(toggleLabel);
                toggleWrapper.appendChild(textLabel);
                filterContainer.appendChild(toggleWrapper);
            }

            renderHistoryTable('All');
            updateScheduleSummaryTable(historyData);
        }

    } catch (e) {
        console.error("Failed to load history", e);
        const tbody = document.querySelector('#historyTable tbody');
        if (tbody && !isAuto) {
            tbody.innerHTML = '<tr class="empty-row"><td colspan="15" style="color:var(--red);">โหลดข้อมูลผิดพลาด</td></tr>';
        }
    } finally {
        if (loadHistoryBtn && !isAuto) {
            loadHistoryBtn.textContent = 'โหลดประวัติวันนี้';
        }
    }
}

function updateScheduleSummaryTable(historyData) {
    const tbody = document.querySelector('#scheduleSummaryTable tbody');
    if (!tbody) return;

    if (!historyData || historyData.length === 0) {
        tbody.innerHTML = '<tr class="empty-row"><td colspan="8">ไม่มีข้อมูลรอบการเทรด</td></tr>';
        document.getElementById('schedule-summary-count').textContent = '0 รอบ';
        return;
    }

    const summary = {};
    let grandProfitUsd = 0;

    historyData.forEach(t => {
        const rNo = t.scheduleTradeNo || 0;
        if (rNo === 0) return; // ข้ามไม้เทรดมือเปล่าที่ไม่มีรอบ

        if (!summary[rNo]) {
            summary[rNo] = { 
                trades: 0, 
                wins: 0, 
                losses: 0, 
                profit: 0, 
                startTime: Infinity, 
                endTime: 0,
                maxSubTradeByAsset: {} 
            };
        }

        summary[rNo].trades++;
        const profit = t.ThisProfit || 0;
        summary[rNo].profit += profit;

        if (profit > 0) summary[rNo].wins++;
        else if (profit < 0) summary[rNo].losses++;

        // บันทึก Max Loss Streak (subTradeno) ของแต่ละ Asset ในรอบนี้
        const asset = t.Asset || 'Unknown';
        const sub = Number(t.subTradeno) || (t.LossCon !== undefined ? (Number(t.LossCon) + 1) : 1);
        if (!summary[rNo].maxSubTradeByAsset[asset] || sub > summary[rNo].maxSubTradeByAsset[asset]) {
            summary[rNo].maxSubTradeByAsset[asset] = sub;
        }

        if (t.timeCandle) {
            if (t.timeCandle < summary[rNo].startTime) summary[rNo].startTime = t.timeCandle;
            if (t.timeCandle > summary[rNo].endTime) summary[rNo].endTime = t.timeCandle;
        }
    });

    const rounds = Object.keys(summary).map(Number).sort((a, b) => b - a); // เรียงจากรอบล่าสุดไปเก่าสุด
    document.getElementById('schedule-summary-count').textContent = `${rounds.length} รอบ`;

    if (rounds.length === 0) {
        tbody.innerHTML = '<tr class="empty-row"><td colspan="8">ไม่มีข้อมูลรอบการเทรด</td></tr>';
        return;
    }

    tbody.innerHTML = '';
    const thbRate = parseFloat(document.getElementById('thb-rate') ? document.getElementById('thb-rate').value : 35) || 35;
    const currentScheduleNo = parseInt(document.getElementById('scheduleTradeNo') ? document.getElementById('scheduleTradeNo').value : '0');

    function formatTimeOnly(epochSec) {
        if (!epochSec || epochSec === Infinity) return '-';
        return new Date(epochSec * 1000).toLocaleTimeString('th-TH', { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    }

    rounds.forEach(rNo => {
        const data = summary[rNo];
        grandProfitUsd += data.profit;
        const tr = document.createElement('tr');
        const profitColor = data.profit > 0 ? 'var(--green)' : (data.profit < 0 ? 'var(--red)' : 'inherit');
        const thbProfit = data.profit * thbRate;

        const startTimeStr = formatTimeOnly(data.startTime);
        const endTimeStr = formatTimeOnly(data.endTime);

        if (rNo === currentScheduleNo) {
            tr.style.background = 'rgba(6, 214, 160, 0.15)'; // Green tint
            tr.style.borderLeft = '3px solid var(--green)';
        }

        // จัดรูปแบบการแสดงผล Max Loss Streak แยกตาม Asset
        const maxStreakEntries = Object.entries(data.maxSubTradeByAsset || {});
        let maxStreakHtml = '<span style="color:var(--text3);">-</span>';
        if (maxStreakEntries.length > 0) {
            maxStreakHtml = maxStreakEntries.map(([ast, m]) => {
                let badgeBg = 'var(--surface2, #2a2e39)';
                let badgeColor = 'var(--text, #e1e7ec)';
                let icon = '';
                if (m >= 5) {
                    badgeBg = 'rgba(239, 68, 68, 0.2)';
                    badgeColor = '#ef4444';
                    icon = ' 🔴';
                } else if (m >= 4) {
                    badgeBg = 'rgba(245, 158, 11, 0.2)';
                    badgeColor = '#f59e0b';
                    icon = ' 🟠';
                } else if (m >= 3) {
                    badgeBg = 'rgba(234, 179, 8, 0.15)';
                    badgeColor = '#eab308';
                }
                return `<span style="display:inline-block; font-size:11px; padding:2px 6px; margin:2px 3px; border-radius:6px; background:${badgeBg}; color:${badgeColor}; font-weight:600; font-family:monospace; border:1px solid rgba(255,255,255,0.06);">${ast}: ${m}${icon}</span>`;
            }).join('');
        }

        tr.innerHTML = `
                    <td><span style="font-size:12px; background:var(--accent-bg, var(--bg3)); padding:4px 12px; border-radius:12px; font-weight:600; color:var(--accent, var(--text));">#${rNo}</span></td>
                    <td style="font-family:monospace; font-size:12px; color:var(--text2);">${startTimeStr} - ${endTimeStr}</td>
                    <td>${data.trades}</td>
                    <td style="color:var(--green)">${data.wins}</td>
                    <td style="color:var(--red)">${data.losses}</td>
                    <td style="text-align:center;">${maxStreakHtml}</td>
                    <td style="color:${profitColor}; font-weight:bold;">${data.profit > 0 ? '+' : ''}${data.profit.toFixed(2)}</td>
                    <td style="color:white; font-weight:bold;">${thbProfit > 0 ? '+' : (thbProfit < 0 ? '-' : '')}฿${Math.abs(thbProfit).toFixed(2)}</td>
                `;
        tbody.appendChild(tr);
    });

    const grandThb = grandProfitUsd * thbRate;
    const usdEl = document.getElementById('schedule-grand-usd');
    const thbEl = document.getElementById('schedule-grand-thb');

    if (usdEl) {
        usdEl.style.color = grandProfitUsd > 0 ? 'var(--green)' : (grandProfitUsd < 0 ? 'var(--red)' : 'var(--text)');
        usdEl.textContent = `${grandProfitUsd > 0 ? '+' : ''}${grandProfitUsd.toFixed(2)}`;
    }
    if (thbEl) {
        thbEl.style.color = grandThb > 0 ? 'var(--green)' : (grandThb < 0 ? 'var(--red)' : 'var(--text)');
        thbEl.textContent = `${grandThb > 0 ? '+' : ''}฿${Math.abs(grandThb).toFixed(2)}`;
    }
}

const loadHistoryBtn = document.getElementById('loadHistoryBtn');
if (loadHistoryBtn) {
    loadHistoryBtn.addEventListener('click', () => reloadTradeHistory(false));
}

// ══════════════════════════════════════════
//  CHECK VERSION (non-blocking — ไม่ใช้ alert เพื่อไม่บล็อก UI/chart)
// ══════════════════════════════════════════
fetch('/api/version').then(res => res.json()).then(data => {
    const htmlVer = "1.0.2";
    console.log(`✅ Server OK | Backend: ${data.main_rs_version} | Frontend: ${htmlVer} | Compiled: ${data.compiled_at}`);
    // แสดงใน status bar แทน alert
    const connText = document.getElementById('connText');
    if (connText && connText.textContent.includes('Connected')) {
        connText.textContent = `Connected (v${data.main_rs_version})`;
    }
    const connTextBig = document.getElementById('connTextBig');
    if (connTextBig && connTextBig.textContent.includes('Connected')) {
        connTextBig.textContent = `Connected (v${data.main_rs_version})`;
    }
}).catch(err => {
    console.error("Error fetching version", err);
});

// ══════════════════════════════════════════
//  DERIV TIME SYNC & DURATION CALCULATOR
// ══════════════════════════════════════════
function updateStopDateFromDuration() {
    const startVal = document.getElementById('startDate').value;
    if (!startVal) return;
    const start = new Date(startVal);
    const addMins = parseInt(document.getElementById('addMinutes').value) || 0;
    const stopDate = new Date(start.getTime() + addMins * 60000);

    // ใช้ timezone offset ของเครื่องเพื่อให้ตรงกับฟอร์แมต datetime-local
    const tzoffset = (new Date()).getTimezoneOffset() * 60000;
    document.getElementById('stopDate').value = (new Date(stopDate - tzoffset)).toISOString().slice(0, 16);
    updateJSON();
}

const addMinsEl = document.getElementById('addMinutes');
if (addMinsEl) {
    addMinsEl.addEventListener('input', updateStopDateFromDuration);
}

const startDateEl = document.getElementById('startDate');
if (startDateEl) {
    startDateEl.addEventListener('change', updateStopDateFromDuration);
}

const setDerivTimeBtn = document.getElementById('setDerivTimeBtn');
if (setDerivTimeBtn) {
    setDerivTimeBtn.addEventListener('click', () => {
        const btn = setDerivTimeBtn;
        const originalText = btn.textContent;
        btn.textContent = "⏳ กำลังตั้งเวลา...";
        btn.disabled = true;

        try {
            // ดึงเวลาจาก clock ที่แสดงอยู่
            const clockElement = document.getElementById('inlineClock');
            const clockTime = clockElement ? clockElement.textContent : null;
            
            if (clockTime) {
                // สร้าง Date object จากเวลาปัจจุบัน
                const now = new Date();
                const [hours, minutes, seconds] = clockTime.split(':').map(s => parseInt(s.trim()));
                
                // ตั้งค่าเวลาให้ตรงกับ clock
                now.setHours(hours, minutes, seconds || 0, 0);
                
                // แปลงเป็นรูปแบบ datetime-local
                const tzoffset = now.getTimezoneOffset() * 60000;
                const localISOTime = (new Date(now - tzoffset)).toISOString().slice(0, 16);
                
                // อัปเดต startDate
                document.getElementById('startDate').value = localISOTime;
                updateStopDateFromDuration();
                
                btn.textContent = "✅ อัปเดตเวลาแล้ว!";
                console.log(`✅ ตั้งเวลาจาก Clock: ${clockTime} → ${localISOTime}`);
            } else {
                btn.textContent = "❌ ไม่พบ Clock";
                console.error('❌ ไม่พบ element #inlineClock');
            }
        } catch (e) {
            console.error('Error setting time from clock:', e);
            btn.textContent = "❌ ล้มเหลว";
        }
        
        setTimeout(() => {
            btn.textContent = originalText;
            btn.disabled = false;
        }, 2000);
    });
}

const saveTradeTimeBtn = document.getElementById('saveTradeTimeBtn');
if (saveTradeTimeBtn) {
    saveTradeTimeBtn.addEventListener('click', async () => {
        const btn = saveTradeTimeBtn;
        const originalText = btn.textContent;
        btn.textContent = "⏳ กำลังบันทึก...";
        btn.disabled = true;

        let startRaw = document.getElementById('startDate').value;
        let stopRaw = document.getElementById('stopDate').value;
        let useSch = document.getElementById('useSchedule') ? document.getElementById('useSchedule').checked : false;

        let startFormat = startRaw ? startRaw.replace("T", " ") + ":00" : null;
        let stopFormat = stopRaw ? stopRaw.replace("T", " ") + ":00" : null;

        try {
            const res = await fetch('/api/tradeControl', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    startTradeTime: startFormat,
                    stopTradeTime: stopFormat,
                    useSchedule: useSch
                })
            });

            if (res.ok) {
                btn.textContent = "✅ บันทึกแล้ว!";
                scheduleTradeCompleted = false; // 🔧 FIX: reset flag เมื่อ user เปลี่ยน schedule ใหม่
                console.log('🔄 [SaveTradeTime] Reset scheduleTradeCompleted = false');
            } else {
                btn.textContent = "❌ บันทึกล้มเหลว";
            }
        } catch (e) {
            console.error("Could not save trade time:", e);
            btn.textContent = "❌ บันทึกล้มเหลว";
        }

        setTimeout(() => { btn.textContent = originalText; btn.disabled = false; }, 2000);
    });
}


// ══════════════════════════════════════════
//  BOT LOG — แสดงสถานะบอทแบบ real-time
// ══════════════════════════════════════════
window.botLogCount = window.botLogCount || 0;
const MAX_LOG_LINES = 500;

function appendBotLog(message, asset) {
    const body = document.getElementById('botLogBody');
    if (!body || !message) return;

    window.botLogCount++;
    const botLogCountEl = document.getElementById('botLogCount');
    if (botLogCountEl) botLogCountEl.textContent = `(${window.botLogCount})`;

    // กำหนดสีตามประเภทข้อความ
    let cssClass = 'log-wait';
    if (message.includes('🚀') || message.includes('เข้าเทรด')) cssClass = 'log-trade';
    else if (message.includes('🎉') || message.includes('ชนะ')) cssClass = 'log-win';
    else if (message.includes('😢') || message.includes('แพ้')) cssClass = 'log-loss';
    else if (message.includes('✅') || message.includes('Spike: ✅')) cssClass = 'log-spike';
    else if (message.includes('🟢') || message.includes('🔴') || asset === 'system') cssClass = 'log-system';

    const now = new Date();
    const timeStr = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}:${String(now.getSeconds()).padStart(2, '0')}`;

    const div = document.createElement('div');
    div.className = `log-line ${cssClass}`;
    div.textContent = `[${timeStr}] ${message}`;
    body.appendChild(div);

    // จำกัดจำนวน log ไม่ให้มากเกินไป
    while (body.children.length > MAX_LOG_LINES) {
        body.removeChild(body.firstChild);
    }

    // Auto scroll ลงล่าง
    body.scrollTop = body.scrollHeight;
}

document.getElementById('clearLogBtn').addEventListener('click', () => {
    const body = document.getElementById('botLogBody');
    body.innerHTML = '<div class="log-line log-system">🗑 ล้าง Log แล้ว</div>';
    window.botLogCount = 0;
    const botLogCountEl = document.getElementById('botLogCount');
    if (botLogCountEl) botLogCountEl.textContent = '(0)';
});

// ══════════════════════════════════════════
//  MANUAL BALANCE SYNC
// ══════════════════════════════════════════
document.getElementById('status-balance').addEventListener('click', async () => {
    const el = document.getElementById('status-balance');
    const originalText = el.textContent;
    el.style.opacity = '0.5';
    el.style.pointerEvents = 'none';

    try {
        const res = await fetch('/api/balance/sync', { method: 'POST' });
        const data = await res.json();
        if (data.status === 'success') {
            el.textContent = `Balance: $${data.balance.toFixed(2)} 🔄`;
            updateBalanceSummary(data.balance);
            console.log("💰 Balance synced successfully:", data.balance);
        } else {
            console.error("❌ Balance sync failed:", data.message);
            alert("Sync Balance ล้มเหลว: " + data.message);
        }
    } catch (e) {
        console.error("❌ Balance sync error:", e);
        alert("ไม่สามารถเชื่อมต่อกับ Server เพื่อ Sync Balance");
    } finally {
        el.style.opacity = '1';
        el.style.pointerEvents = 'auto';
    }
});

// ══════════════════════════════════════════
//  SCHEDULE STATUS CHECK — ตรวจสอบว่าอยู่ในช่วงเวลาเทรดหรือไม่
// ══════════════════════════════════════════
function checkScheduleStatus() {
    const statusEl = document.getElementById('schedule-status');
    if (!statusEl) return;

    const useScheduleEl = document.getElementById('useSchedule');
    if (!useScheduleEl || !useScheduleEl.checked) {
        statusEl.innerHTML = '<span style="color:var(--text3);">📅 Schedule: ปิดอยู่</span>';
        return;
    }

    const startVal = document.getElementById('startDate').value;
    const stopVal = document.getElementById('stopDate').value;
    if (!startVal) {
        statusEl.innerHTML = '<span style="color:var(--text3);">📅 ยังไม่ได้ตั้งเวลา</span>';
        return;
    }

    const now = Date.now();
    const start = new Date(startVal).getTime();
    const stop = stopVal ? new Date(stopVal).getTime() : 0;

    if (now < start) {
        // ยังไม่ถึงเวลาเทรด — แสดง countdown
        const diff = start - now;
        const h = Math.floor(diff / 3600000);
        const m = Math.floor((diff % 3600000) / 60000);
        const s = Math.floor((diff % 60000) / 1000);
        const timeStr = `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
        statusEl.innerHTML = `<span style="color:var(--orange); font-weight:600;">⏳ รอ Schedule อีก ${timeStr}</span>`;
    } else if (!stop || now < stop) {
        // อยู่ในช่วงเวลาเทรด
        statusEl.innerHTML = '<span style="color:var(--green); font-weight:700;">🟢 อยู่ในช่วงเวลาเทรด</span>';
    } else {
        // เลยเวลาเทรดไปแล้ว
        statusEl.innerHTML = '<span style="color:var(--red); font-weight:600;">🔴 เลยช่วงเวลาเทรดแล้ว</span>';
    }
}
// ══════════════════════════════════════════
//  ANALYSIS DATA GENERATOR
// ══════════════════════════════════════════

// Load saved dates from localStorage if available
const savedStartDateAnalysis = localStorage.getItem('savedStartDateAnalysis');
const savedStopDateAnalysis = localStorage.getItem('savedStopDateAnalysis');
if (savedStartDateAnalysis) document.getElementById('startDateAnalysis').value = savedStartDateAnalysis;
if (savedStopDateAnalysis) document.getElementById('stopDateAnalysis').value = savedStopDateAnalysis;

document.getElementById('btnCreateAnalysis').addEventListener('click', async () => {
    const asset = document.getElementById('analysisAssetList').value;
    const startDate = document.getElementById('startDateAnalysis').value;
    const stopDate = document.getElementById('stopDateAnalysis').value;

    // Save selected dates to localStorage
    localStorage.setItem('savedStartDateAnalysis', startDate);
    localStorage.setItem('savedStopDateAnalysis', stopDate);

    if (!asset) {
        alert('กรุณาเลือก Asset จาก Listbox ก่อนครับ');
        return;
    }
    if (!startDate || !stopDate) {
        alert('กรุณาเลือกวันเวลาเริ่มต้นและสิ้นสุดให้ครบถ้วน');
        return;
    }

    const btn = document.getElementById('btnCreateAnalysis');
    const originalText = btn.innerHTML;
    btn.innerHTML = '⏳ กำลังดึงข้อมูลและประมวลผล...';
    btn.disabled = true;
    document.getElementById('analysisOutput').value = 'กำลังโหลด...';

    try {
        const response = await fetch('/api/generate_analysis', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                asset: asset,
                start_date: startDate,
                stop_date: stopDate
            })
        });

        const result = await response.json();

        if (result.status === 'success') {
            document.getElementById('analysisAssetDisplay').textContent = result.asset;
            document.getElementById('analysisCountDisplay').textContent = result.count;
            document.getElementById('analysisStartDisplay').textContent = result.start.replace('T', ' ');
            document.getElementById('analysisStopDisplay').textContent = result.stop.replace('T', ' ');

            document.getElementById('analysisOutput').value = JSON.stringify(result.data, null, 2);
        } else {
            alert('เกิดข้อผิดพลาดจาก Server: ' + result.message);
            document.getElementById('analysisOutput').value = 'Error: ' + result.message;
        }
    } catch (error) {
        console.error('Error fetching analysis data:', error);
        alert('เกิดข้อผิดพลาดในการเชื่อมต่อ (อาจเกิดปัญหา Timeout หรือ Network)');
        document.getElementById('analysisOutput').value = 'Connection Error: ' + error.message;
    } finally {
        btn.innerHTML = originalText;
        btn.disabled = false;
    }
});

document.getElementById('btnPostFullAnalysisData').addEventListener('click', async () => {
    const dataStr = document.getElementById('analysisOutput').value;
    if (!dataStr || dataStr === 'กำลังโหลด...' || dataStr.startsWith('Error:')) {
        alert('ยังไม่มีข้อมูลสำหรับส่ง กรุณาสร้างข้อมูลก่อนครับ');
        return;
    }

    let jsonData;
    try {
        jsonData = JSON.parse(dataStr);
    } catch (e) {
        alert('รูปแบบข้อมูลไม่ถูกต้อง (ไม่ใช่ JSON)');
        return;
    }

    const btn = document.getElementById('btnPostFullAnalysisData');
    const originalText = btn.innerHTML;
    btn.innerHTML = '⏳ กำลังส่งข้อมูล...';
    btn.disabled = true;

    try {
        // Fetch the URL from our Rust backend
        const configRes = await fetch('/api/config/url_ajax_post');
        const configData = await configRes.json();
        const postUrl = configData.url;

        if (!postUrl) {
            alert('ไม่พบ URLAJAXPOST_FullAnalysis ในไฟล์ .env หรือยังไม่ได้ตั้งค่า');
            return;
        }

        // Send the actual POST request
        const response = await fetch(postUrl, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(jsonData)
        });

        if (response.ok) {
            alert('ส่งข้อมูลสำเร็จ!');
        } else {
            const errorText = await response.text();
            alert('เกิดข้อผิดพลาดในการส่งข้อมูล (Status ' + response.status + '): ' + errorText);
        }
    } catch (error) {
        console.error('Error posting analysis data:', error);
        alert('เกิดข้อผิดพลาดในการเชื่อมต่อ (อาจเกิดปัญหา CORS หรือ Network Error)');
    } finally {
        btn.innerHTML = originalText;
        btn.disabled = false;
    }
});

//  STRATEGY COMPARISON FETCH
document.getElementById('loadStrategyComparisonBtn').addEventListener('click', async () => {
    const btn = document.getElementById('loadStrategyComparisonBtn');
    const originalText = btn.innerHTML;
    btn.innerHTML = 'กำลังโหลด...';
    btn.disabled = true;

    try {
        const now = new Date();
        const yyyy = now.getFullYear();
        const mm = String(now.getMonth() + 1).padStart(2, '0');
        const dd = String(now.getDate()).padStart(2, '0');
        const dayTrade = `${yyyy}-${mm}-${dd}`;

        const response = await fetch(`/api/getStrategyComparison?dayTrade=${dayTrade}`);
        const data = await response.json();

        window.strategyComparisonData = data; // Store globally for chart tooltips

        const filterContainer = document.getElementById('strategy-asset-filters');
        if (filterContainer) filterContainer.innerHTML = '';

        if (!Array.isArray(data) || data.length === 0) {
            const tbody = document.querySelector('#strategyComparisonTable tbody');
            document.getElementById('strategy-comparison-count').textContent = '0 records';
            tbody.innerHTML = '<tr class="empty-row"><td colspan="8">ไม่พบข้อมูลของวันนี้</td></tr>';
            return;
        }

        // Extract unique assets
        const uniqueAssets = [...new Set(data.map(item => item.assetCode).filter(Boolean))];

        const renderTable = (filterAsset) => {
            const tbody = document.querySelector('#strategyComparisonTable tbody');
            tbody.innerHTML = '';

            let filteredData = data;
            if (filterAsset !== 'All') {
                filteredData = data.filter(item => item.assetCode === filterAsset);
            }

            document.getElementById('strategy-comparison-count').textContent = `${filteredData.length} records`;

            if (filteredData.length === 0) {
                tbody.innerHTML = '<tr class="empty-row"><td colspan="10">ไม่พบข้อมูลสำหรับ Asset นี้</td></tr>';
                return;
            }

            // Sort chronologically (oldest to newest) to accurately calculate consecutive loss streaks
            filteredData.sort((a, b) => (a.candleTimestamp || 0) - (b.candleTimestamp || 0));

            // Calculate Max Loss Streak by counting consecutive "Loss" entries over the day PER ASSET
            let currentStreaksV1 = {};
            let currentStreaksV2 = {};
            let currentStreaksV3A = {};
            let currentStreaksV3B = {};
            let currentStreaksV3C = {};
            let currentStreaksFTA = {};
            let currentStreaksFTB = {};
            let currentStreaksPKT = {};
            let currentStreaksPKT5 = {};
            let currentStreaksFC = {};
            let currentStreaksPKTCaseCode = {};
            let maxLossPKTCaseCode = 0;
            let maxLossAssetPKTCaseCode = "";
            let maxLossV1 = 0;
            let maxLossV2 = 0;
            let maxLossV3A = 0;
            let maxLossV3B = 0;
            let maxLossV3C = 0;
            let maxLossFTA = 0;
            let maxLossFTB = 0;
            let maxLossPKT = 0;
            let maxLossPKT5 = 0;
            let maxLossFC = 0;
            let maxLossAssetV1 = '';
            let maxLossAssetV2 = '';
            let maxLossAssetV3A = '';
            let maxLossAssetV3B = '';
            let maxLossAssetV3C = '';
            let maxLossAssetFTA = '';
            let maxLossAssetFTB = '';
            let maxLossAssetPKT = '';
            let maxLossAssetPKT5 = '';
            let maxLossAssetFC = '';

            filteredData.forEach(row => {
                const asset = row.assetCode;

                if (row.winStatusByV1 === 'Loss') {
                    currentStreaksV1[asset] = (currentStreaksV1[asset] || 0) + 1;
                    if (currentStreaksV1[asset] > maxLossV1) {
                        maxLossV1 = currentStreaksV1[asset];
                        maxLossAssetV1 = asset;
                    }
                } else if (row.winStatusByV1 === 'Win') {
                    currentStreaksV1[asset] = 0;
                }

                if (row.winStatusByV2 === 'Loss') {
                    currentStreaksV2[asset] = (currentStreaksV2[asset] || 0) + 1;
                    if (currentStreaksV2[asset] > maxLossV2) {
                        maxLossV2 = currentStreaksV2[asset];
                        maxLossAssetV2 = asset;
                    }
                } else if (row.winStatusByV2 === 'Win') {
                    currentStreaksV2[asset] = 0;
                }

                if (row.winStatusByV3A === 'Loss') {
                    currentStreaksV3A[asset] = (currentStreaksV3A[asset] || 0) + 1;
                    if (currentStreaksV3A[asset] > maxLossV3A) {
                        maxLossV3A = currentStreaksV3A[asset];
                        maxLossAssetV3A = asset;
                    }
                } else if (row.winStatusByV3A === 'Win') {
                    currentStreaksV3A[asset] = 0;
                }

                if (row.winStatusByV3B === 'Loss') {
                    currentStreaksV3B[asset] = (currentStreaksV3B[asset] || 0) + 1;
                    if (currentStreaksV3B[asset] > maxLossV3B) {
                        maxLossV3B = currentStreaksV3B[asset];
                        maxLossAssetV3B = asset;
                    }
                } else if (row.winStatusByV3B === 'Win') {
                    currentStreaksV3B[asset] = 0;
                }

                if (row.winStatusByV3C === 'Loss') {
                    currentStreaksV3C[asset] = (currentStreaksV3C[asset] || 0) + 1;
                    if (currentStreaksV3C[asset] > maxLossV3C) {
                        maxLossV3C = currentStreaksV3C[asset];
                        maxLossAssetV3C = asset;
                    }
                } else if (row.winStatusByV3C === 'Win') {
                    currentStreaksV3C[asset] = 0;
                }

                if (row.winStatusByFTA === 'Loss') {
                    currentStreaksFTA[asset] = (currentStreaksFTA[asset] || 0) + 1;
                    if (currentStreaksFTA[asset] > maxLossFTA) {
                        maxLossFTA = currentStreaksFTA[asset];
                        maxLossAssetFTA = asset;
                    }
                } else if (row.winStatusByFTA === 'Win') {
                    currentStreaksFTA[asset] = 0;
                }

                if (row.winStatusByFTB === 'Loss') {
                    currentStreaksFTB[asset] = (currentStreaksFTB[asset] || 0) + 1;
                    if (currentStreaksFTB[asset] > maxLossFTB) {
                        maxLossFTB = currentStreaksFTB[asset];
                        maxLossAssetFTB = asset;
                    }
                } else if (row.winStatusByFTB === 'Win') {
                    currentStreaksFTB[asset] = 0;
                }

                if (row.winStatusByPKTrend === 'Loss') {
                    currentStreaksPKT[asset] = (currentStreaksPKT[asset] || 0) + 1;
                    if (currentStreaksPKT[asset] > maxLossPKT) {
                        maxLossPKT = currentStreaksPKT[asset];
                        maxLossAssetPKT = asset;
                    }
                } else if (row.winStatusByPKTrend === 'Win') {
                    currentStreaksPKT[asset] = 0;
                }

                if (row.winStatusByPKTrendV5 === 'Loss') {
                    currentStreaksPKT5[asset] = (currentStreaksPKT5[asset] || 0) + 1;
                    if (currentStreaksPKT5[asset] > maxLossPKT5) {
                        maxLossPKT5 = currentStreaksPKT5[asset];
                        maxLossAssetPKT5 = asset;
                    }
                } else if (row.winStatusByPKTrendV5 === 'Win') {
                    currentStreaksPKT5[asset] = 0;
                }

                if (row.winStatusByForecast === 'Loss') {
                    currentStreaksFC[asset] = (currentStreaksFC[asset] || 0) + 1;
                    if (currentStreaksFC[asset] > maxLossFC) {
                        maxLossFC = currentStreaksFC[asset];
                        maxLossAssetFC = asset;
                    }
                } else if (row.winStatusByForecast === 'Win') {
                    currentStreaksFC[asset] = 0;
                }

                if (row.winStatusByPKTrendCaseCode === 'Loss') {
                    currentStreaksPKTCaseCode[asset] = (currentStreaksPKTCaseCode[asset] || 0) + 1;
                    if (currentStreaksPKTCaseCode[asset] > maxLossPKTCaseCode) {
                        maxLossPKTCaseCode = currentStreaksPKTCaseCode[asset];
                        maxLossAssetPKTCaseCode = asset;
                    }
                } else if (row.winStatusByPKTrendCaseCode === 'Win') {
                    currentStreaksPKTCaseCode[asset] = 0;
                }
            });

            // Sort descending for display
            filteredData.sort((a, b) => (b.candleTimestamp || 0) - (a.candleTimestamp || 0));

            // Add Max Loss Streak Row
            const summaryRow = document.createElement('tr');
            summaryRow.style.backgroundColor = 'rgba(245, 166, 35, 0.1)';
            const formatMaxLoss = (val, asset) => val > 0 ? `${val} <span style="font-size:11px; color:#ffffff; font-weight:normal;">(${asset})</span>` : '0';
            summaryRow.innerHTML = `
                        <td colspan="4" style="text-align:right; font-weight:bold; color:var(--orange);">🏆 Max Loss Streak:</td>
                        <td style="font-weight:bold; color:var(--orange); font-size:13px;">${formatMaxLoss(maxLossV1, maxLossAssetV1)}</td>
                        <td style="font-weight:bold; color:var(--orange); font-size:13px;">${formatMaxLoss(maxLossV2, maxLossAssetV2)}</td>
                        <td style="font-weight:bold; color:var(--orange); font-size:13px;">${formatMaxLoss(maxLossV3A, maxLossAssetV3A)}</td>
                        <td style="font-weight:bold; color:var(--orange); font-size:13px;">${formatMaxLoss(maxLossV3B, maxLossAssetV3B)}</td>
                        <td style="font-weight:bold; color:var(--orange); font-size:13px;">${formatMaxLoss(maxLossV3C, maxLossAssetV3C)}</td>
                        <td style="font-weight:bold; color:var(--orange); font-size:13px;">${formatMaxLoss(maxLossFTA, maxLossAssetFTA)}</td>
                        <td style="font-weight:bold; color:var(--orange); font-size:13px;">${formatMaxLoss(maxLossFTB, maxLossAssetFTB)}</td>
                        <td style="font-weight:bold; color:var(--orange); font-size:13px;">${formatMaxLoss(maxLossPKT, maxLossAssetPKT)}</td>
                        <td style="font-weight:bold; color:var(--orange); font-size:13px;">${formatMaxLoss(maxLossPKT5, maxLossAssetPKT5)}</td>
                        <td style="font-weight:bold; color:var(--orange); font-size:13px;">${formatMaxLoss(maxLossFC, maxLossAssetFC)}</td>
                        <td style="font-weight:bold; color:var(--orange); font-size:13px;">${formatMaxLoss(maxLossPKTCaseCode, maxLossAssetPKTCaseCode)}</td>
                        <td></td>
                    `;
            tbody.appendChild(summaryRow);

            // Render table
            filteredData.forEach(row => {
                const tr = document.createElement('tr');

                const formatStrategyCell = (suggest, lossCon, winStatus) => {
                    let winColor = winStatus === 'Win' ? 'var(--green)' : (winStatus === 'Loss' ? 'var(--red)' : 'var(--text)');
                    let suggestEmoji = suggest === 'green' ? '🟢' : (suggest === 'red' ? '🔴' : (suggest || '-'));
                    return `<span style="font-size:12px;" title="${suggest || ''}">${suggestEmoji}</span> / 
                                    ${lossCon || 0} / 
                                    <span style="font-weight:bold; color:${winColor}">${winStatus || '-'}</span>`;
                };

                let actualEmoji = row.actualNextColor === 'green' ? '🟢' : (row.actualNextColor === 'red' ? '🔴' : (row.actualNextColor || '-'));

                tr.innerHTML = `
                            <td>${row.runno || '-'}</td>
                            <td>${row.candleTimeDisp || '-'}</td>
                            <td>${row.assetCode || '-'}</td>
                            <td><span style="font-size:12px;" title="${row.actualNextColor || ''}">${actualEmoji}</span></td>
                            <td>${formatStrategyCell(row.suggestColorByV1, row.lossConByV1, row.winStatusByV1)}</td>
                            <td>${formatStrategyCell(row.suggestColorByV2, row.lossConByV2, row.winStatusByV2)}</td>
                            <td>${formatStrategyCell(row.suggestColorByV3A, row.lossConByV3A, row.winStatusByV3A)}</td>
                            <td>${formatStrategyCell(row.suggestColorByV3B, row.lossConByV3B, row.winStatusByV3B)}</td>
                            <td>${formatStrategyCell(row.suggestColorByV3C, row.lossConByV3C, row.winStatusByV3C)}</td>
                            <td>${formatStrategyCell(row.suggestColorByFTA, row.lossConByFTA, row.winStatusByFTA)}</td>
                            <td>${formatStrategyCell(row.suggestColorByFTB, row.lossConByFTB, row.winStatusByFTB)}</td>
                            <td>${formatStrategyCell(row.suggestColorByPKTrend, row.lossConByPKTrend, row.winStatusByPKTrend)}</td>
                            <td>${formatStrategyCell(row.suggestColorByPKTrendV5, row.lossConByPKTrendV5, row.winStatusByPKTrendV5)}</td>
                            <td>${formatStrategyCell(row.suggestColorByForecast, row.lossConByForecast, row.winStatusByForecast)}</td>
                            <td>${formatStrategyCell(row.suggestColorByPKTrendCaseCode, row.lossConByPKTrendCaseCode, row.winStatusByPKTrendCaseCode)}</td>
                            <td style="text-align: center;">
                                <button class="fetch-btn" style="padding: 2px 6px; font-size: 10px;" onclick="showDiagnosticModal(this)" data-diag='${JSON.stringify({
                    v1Reason: row.v1Reason || "-", v1Loss: row.v1LossFactor || "-",
                    v2Reason: row.v2Reason || "-", v2Loss: row.v2LossFactor || "-",
                    v3aReason: row.v3aReason || "-", v3aLoss: row.v3aLossFactor || "-",
                    v3bReason: row.v3bReason || "-", v3bLoss: row.v3bLossFactor || "-",
                    v3cReason: row.v3cReason || "-", v3cLoss: row.v3cLossFactor || "-",
                    ftaReason: row.ftaReason || "-", ftaLoss: row.ftaLossFactor || "-",
                    ftbReason: row.ftbReason || "-", ftbLoss: row.ftbLossFactor || "-",
                    pktrendReason: row.pktrendReason || "-", pktrendLoss: row.pktrendLossFactor || "-",
                    pktrendV5Reason: row.pktrendV5Reason || "-", pktrendV5Loss: row.pktrendV5LossFactor || "-",
                    forecastReason: row.forecastReason || "-", forecastLoss: row.forecastLossFactor || "-",
                    pktrendCaseCodeReason: row.pktrendCaseCodeReason || "-", pktrendCaseCodeLoss: row.pktrendCaseCodeLossFactor || "-"
                }).replace(/'/g, "&#39;")}'>🔍</button>
                            </td>
                        `;
                tbody.appendChild(tr);
            });
        };

        const createFilterBtn = (assetName) => {
            const btn = document.createElement('button');
            btn.textContent = assetName;
            btn.style.padding = '4px 12px';
            btn.style.fontSize = '12px';
            btn.style.borderRadius = 'var(--radius)';
            btn.style.cursor = 'pointer';
            btn.style.border = '1px solid var(--border)';
            btn.style.background = assetName === 'All' ? 'var(--accent)' : 'transparent';
            btn.style.color = assetName === 'All' ? '#fff' : 'var(--text)';

            btn.addEventListener('click', () => {
                Array.from(filterContainer.children).forEach(child => {
                    child.style.background = 'transparent';
                    child.style.color = 'var(--text)';
                });
                btn.style.background = 'var(--accent)';
                btn.style.color = '#fff';

                renderTable(assetName);
            });
            return btn;
        };

        if (filterContainer) {
            filterContainer.appendChild(createFilterBtn('All'));
            uniqueAssets.forEach(asset => {
                filterContainer.appendChild(createFilterBtn(asset));
            });
        }

        // Initial render
        renderTable('All');
    } catch (error) {
        console.error('Error fetching strategy comparison:', error);
        alert('เกิดข้อผิดพลาดในการโหลดข้อมูล: ' + error.message);
    } finally {
        btn.innerHTML = originalText;
        btn.disabled = false;
    }
});


// --- Diagnostic Modal ---
function showDiagnosticModal(btn) {
    try {
        const data = JSON.parse(btn.getAttribute('data-diag'));
        let html = '';
        const strats = ['v1', 'v2', 'v3a', 'v3b', 'v3c', 'fta', 'ftb', 'pktrend', 'pktrendV5', 'forecast', 'pktrendCaseCode'];
        strats.forEach(s => {
            const name = s.toUpperCase();
            html += `<div style="margin-bottom:15px; background:var(--surface2); padding:10px; border-radius:8px; border:1px solid var(--border2);">
                        <strong style="color:var(--accent); font-size:15px;">${name} Strategy</strong><br>
                        <span style="color:var(--text2);">Reason:</span> ${data[s + 'Reason'] || '-'}<br>
                        <span style="color:var(--red);">Loss Factor:</span> ${data[s + 'Loss'] || '-'}
                    </div>`;
        });
        document.getElementById('diagnosticContent').innerHTML = html;
        document.getElementById('diagnosticModal').style.display = 'block';
    } catch (e) {
        console.error(e);
    }
}

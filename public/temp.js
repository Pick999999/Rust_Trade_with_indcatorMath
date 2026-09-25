

        const TH_OFFSET_SEC = 7 * 60 * 60;
        function toThaiTime(epoch) {
            return epoch + TH_OFFSET_SEC;
        }

        // ── Theme Switching ──────────────────────────────────────
        document.querySelectorAll('.theme-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                document.body.setAttribute('data-theme', btn.dataset.theme);
                document.querySelectorAll('.theme-btn').forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
                localStorage.setItem('lt-theme', btn.dataset.theme);
            });
        });
        // Restore theme
        const savedTheme = localStorage.getItem('lt-theme') || 'dark';
        document.body.setAttribute('data-theme', savedTheme);
        document.querySelectorAll('.theme-btn').forEach(b => {
            b.classList.toggle('active', b.dataset.theme === savedTheme);
        });

        // ── Tab Switching ────────────────────────────────────────
        document.querySelectorAll('#ltTabsRow .tab-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                document.querySelectorAll('#ltTabsRow .tab-btn').forEach(b => b.classList.remove('active'));
                document.querySelectorAll('.tab-content').forEach(tc => tc.classList.remove('active'));
                btn.classList.add('active');
                document.getElementById(btn.dataset.tab + '-tab').classList.add('active');
            });
        });

        // ── Exit Strategy Card Selection ─────────────────────────
        document.querySelectorAll('input[name="ltExitStrategy"]').forEach(radio => {
            radio.addEventListener('change', () => {
                document.querySelectorAll('.exit-card').forEach(c => c.classList.remove('selected'));
                radio.closest('.exit-card').classList.add('selected');
            });
        });

        // ── Condition Card Toggle ────────────────────────────────
        document.querySelectorAll('.condition-card input[type="checkbox"]').forEach(cb => {
            cb.addEventListener('change', () => {
                cb.closest('.condition-card').classList.toggle('active-condition', cb.checked);
            });
        });

        // ── Chart Timeframe Buttons ──────────────────────────────
        document.querySelectorAll('.chart-toolbar .gran-btn').forEach(btn => {
            btn.addEventListener('click', async () => {
                if (btn.dataset.value === 'tick') return; // Not supporting tick for now

                // Clear any existing auto-refresh timer when user manually changes timeframe

                document.querySelectorAll('.chart-toolbar .gran-btn').forEach(b => b.classList.remove('active'));
                btn.classList.add('active');

                const granularity = parseInt(btn.dataset.value, 10);
                const currentAsset = document.getElementById('ltChartAssetSelect').value;

                if (!currentAsset) return;

                // Show loading
                btn.textContent = '⏳';

                try {
                    const res = await fetch(`/api/longterm/candles?asset=${currentAsset}&granularity=${granularity}`);
                    const json = await res.json();

                    if (json.status === 'success' && json.data && window.ltCandleSeries) {
                        const chartData = json.data.map(d => ({
                            time: toThaiTime(d.candletime),
                            open: d.open,
                            high: d.high,
                            low: d.low,
                            close: d.close
                        }));
                        const emaShortData = json.data.filter(d => d.ema_short_value > 0).map(d => ({ time: toThaiTime(d.candletime), value: d.ema_short_value }));
                        const emaMediumData = json.data.filter(d => d.ema_medium_value > 0).map(d => ({ time: toThaiTime(d.candletime), value: d.ema_medium_value }));
                        const emaLongData = json.data.filter(d => d.ema_long_value > 0).map(d => ({ time: toThaiTime(d.candletime), value: d.ema_long_value }));

                        window.ltCandleSeries.setData(chartData);
                        window.ltEmaShortSeries.setData(emaShortData);
                        window.ltEmaMediumSeries.setData(emaMediumData);
                        window.ltEmaLongSeries.setData(emaLongData);

                        window.ltCurrentChartGranularity = granularity;
                        window.ltCurrentChartData = chartData;

                        // We DO NOT overwrite ltCandlesData[currentAsset] here because that is used by the WS for EMA cuts
                        // and must remain the bot's original timeframe data.

                        // Clear markers
                        window.ltCandleSeries.setMarkers([]);

                        if (json.data.length > 0) {
                            updateSignalDisplay(json.data[json.data.length - 1], currentAsset);
                        }
                    }
                } catch (err) {
                    console.error('Failed to fetch chart candles', err);
                } finally {
                    // Restore text based on dataset value
                    const labels = {
                        "60": "1M", "300": "5M", "900": "15M", "1800": "30M", "3600": "1H", "7200": "2H", "86400": "1D"
                    };
                    btn.textContent = labels[btn.dataset.value] || btn.dataset.value;
                }
            });
        });

        // ── Digital Clock ────────────────────────────────────────
        function updateClock() {
            const now = new Date();
            const h = String(now.getHours()).padStart(2, '0');
            const m = String(now.getMinutes()).padStart(2, '0');
            const s = String(now.getSeconds()).padStart(2, '0');
            document.getElementById('ltDigitalClock').textContent = `${h}:${m}:${s}`;
        }
        setInterval(updateClock, 1000);
        updateClock();

        // ── Initialize Default Schedule Times ────────────────────
        (function initSchedule() {
            const now = new Date();
            const pad = (n) => String(n).padStart(2, '0');
            const fmt = (d) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;

            document.getElementById('ltStartTime').value = fmt(now);

            const addMin = parseInt(document.getElementById('ltAddMinutes').value) || 120;
            const end = new Date(now.getTime() + addMin * 60000);
            document.getElementById('ltStopTime').value = fmt(end);

            // Auto update stop time when addMinutes changes
            document.getElementById('ltAddMinutes').addEventListener('input', () => {
                const start = new Date(document.getElementById('ltStartTime').value);
                const mins = parseInt(document.getElementById('ltAddMinutes').value) || 0;
                const newEnd = new Date(start.getTime() + mins * 60000);
                document.getElementById('ltStopTime').value = fmt(newEnd);
            });
        })();

        // ── Duration Selector → Update Timeframe Display ─────────
        document.getElementById('ltDurationSelect').addEventListener('change', (e) => {
            const text = e.target.options[e.target.selectedIndex].text;
            document.getElementById('lt-timeframe-display').textContent = `Timeframe: ${text}`;
        });


        // ── Initialize Chart (Placeholder) ───────────────────────
        (function initChart() {
            const chartContainer = document.getElementById('ltChart');
            if (typeof LightweightCharts !== 'undefined') {
                const chart = LightweightCharts.createChart(chartContainer, {
                    layout: {
                        background: { type: 'solid', color: 'transparent' },
                        textColor: getComputedStyle(document.body).getPropertyValue('--text2').trim() || '#8aaac8',
                    },
                    grid: {
                        vertLines: { color: 'rgba(255,255,255,0.04)' },
                        horzLines: { color: 'rgba(255,255,255,0.04)' },
                    },
                    localization: {
                        timeFormatter: (time) => {
                            const date = new Date(time * 1000);
                            const d = String(date.getUTCDate()).padStart(2, '0');
                            const m = String(date.getUTCMonth() + 1).padStart(2, '0');
                            const y = date.getUTCFullYear();
                            const h = String(date.getUTCHours()).padStart(2, '0');
                            const min = String(date.getUTCMinutes()).padStart(2, '0');
                            const sec = String(date.getUTCSeconds()).padStart(2, '0');
                            return `${d}/${m}/${y} ${h}:${min}:${sec}`;
                        }
                    },
                    timeScale: {
                        timeVisible: true,
                        secondsVisible: true,
                        tickMarkFormatter: (time, tickMarkType, locale) => {
                            const date = new Date(time * 1000);
                            const d = String(date.getUTCDate()).padStart(2, '0');
                            const m = String(date.getUTCMonth() + 1).padStart(2, '0');
                            const h = String(date.getUTCHours()).padStart(2, '0');
                            const min = String(date.getUTCMinutes()).padStart(2, '0');
                            return `${h}:${min}`;
                        }
                    },
                    crosshair: { mode: 0 },
                });

                const candleSeries = chart.addCandlestickSeries({
                    upColor: '#06D6A0', downColor: '#E8304A',
                    borderUpColor: '#06D6A0', borderDownColor: '#E8304A',
                    wickUpColor: '#06D6A0', wickDownColor: '#E8304A',
                });

                const emaShortSeries = chart.addLineSeries({
                    color: '#06D6A0',
                    lineWidth: 2,
                    crosshairMarkerVisible: false,
                    lastValueVisible: false,
                    priceLineVisible: false,
                });
                const emaMediumSeries = chart.addLineSeries({
                    color: '#ffc107',
                    lineWidth: 2,
                    crosshairMarkerVisible: false,
                    lastValueVisible: false,
                    priceLineVisible: false,
                });
                const emaLongSeries = chart.addLineSeries({
                    color: '#ff1744',
                    lineWidth: 2,
                    crosshairMarkerVisible: false,
                    lastValueVisible: false,
                    priceLineVisible: false,
                });

                // Store references globally for later updates
                window.ltChart = chart;
                window.ltCandleSeries = candleSeries;
                window.ltEmaShortSeries = emaShortSeries;
                window.ltEmaMediumSeries = emaMediumSeries;
                window.ltEmaLongSeries = emaLongSeries;
                window.ltCurrentChartGranularity = 60; // Default to 1M

                chartContainer.style.display = 'flex';
                chartContainer.style.alignItems = 'center';
                chartContainer.style.justifyContent = 'center';
                chartContainer.innerHTML = '';
                chartContainer.appendChild(chart.chartElement());

                // Resize handling
                const ro = new ResizeObserver(() => {
                    chart.applyOptions({ width: chartContainer.clientWidth });
                });
                ro.observe(chartContainer);

                // (Remove simulation interval)
                /*
                let lastPrice = 100;
                let currentCandleTime = Math.floor(Date.now() / 1000);
                
                setInterval(() => {
                    ...
                }, 2000);
                */
            }
        })();

        // ── Bot Log Helper ───────────────────────────────────────
        let ltLogCount = 0;
        function ltAddLog(message, type = 'system') {
            const logBody = document.getElementById('ltBotLogBody');
            const line = document.createElement('div');
            line.className = `log-line log-${type}`;
            const now = new Date();
            const ts = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}:${String(now.getSeconds()).padStart(2, '0')}`;
            line.textContent = `[${ts}] ${message}`;
            logBody.appendChild(line);
            logBody.scrollTop = logBody.scrollHeight;
            ltLogCount++;
            document.getElementById('ltLogCount').textContent = `(${ltLogCount})`;
        }

        // ── Clear Log ────────────────────────────────────────────
        document.getElementById('ltClearLogBtn').addEventListener('click', () => {
            document.getElementById('ltBotLogBody').innerHTML = '<div class="log-line log-system">🗑 Log cleared</div>';
            ltLogCount = 0;
            document.getElementById('ltLogCount').textContent = '(0)';
        });

        // ── Asset Checkboxes → Update Chart Asset Dropdown ───────
        document.getElementById('ltAssetCheckboxes').addEventListener('change', (e) => {
            if (e.target.tagName === 'INPUT' && e.target.type === 'checkbox') {
                const select = document.getElementById('ltChartAssetSelect');
                const checkedBoxes = Array.from(document.querySelectorAll('#ltAssetCheckboxes input:checked'));

                // Update dropdown options
                select.innerHTML = '<option value="">— เลือก Asset —</option>';
                checkedBoxes.forEach(box => {
                    const option = document.createElement('option');
                    option.value = box.value;
                    option.textContent = box.nextSibling.textContent.trim();
                    select.appendChild(option);
                });

                const assetNames = checkedBoxes.map(b => b.nextSibling.textContent.trim());
                document.getElementById('lt-asset-name').textContent = `Asset: ${assetNames.length > 0 ? assetNames.join(', ') : '—'}`;
            }
        });

        // ── Schedule Monitor ─────────────────────────────────────
        let scheduleCheckInterval = null;
        let scheduleStarted = false;

        function updateCountdownDisplay(startTime, stopTime) {
            const now = new Date();
            const countdownEl = document.getElementById('ltCountdownDisplay');
            const statusTextEl = document.getElementById('ltScheduleStatusText');

            if (now < startTime) {
                // รอเวลาเริ่มต้น
                const diff = startTime - now;
                const hours = Math.floor(diff / 3600000);
                const minutes = Math.floor((diff % 3600000) / 60000);
                const seconds = Math.floor((diff % 60000) / 1000);

                statusTextEl.textContent = 'กำลังรอถึงเวลาเริ่มต้น...';
                countdownEl.textContent = `เหลืออีก: ${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
            } else if (stopTime && now < stopTime) {
                // กำลังทำงาน
                const diff = stopTime - now;
                const hours = Math.floor(diff / 3600000);
                const minutes = Math.floor((diff % 3600000) / 60000);
                const seconds = Math.floor((diff % 60000) / 1000);

                statusTextEl.textContent = '✅ กำลังเทรด - จะหยุดในอีก...';
                countdownEl.textContent = `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
            } else {
                statusTextEl.textContent = '⏱️ รอคำสั่ง...';
                countdownEl.textContent = '';
            }
        }

        function startScheduleMonitor() {
            if (scheduleCheckInterval) return;

            const scheduleStatusEl = document.getElementById('ltScheduleStatus');
            scheduleStatusEl.style.display = 'block';

            scheduleCheckInterval = setInterval(() => {
                const useSchedule = document.getElementById('ltUseSchedule').checked;
                if (!useSchedule) return;

                const startTimeRaw = document.getElementById('ltStartTime').value;
                const stopTimeRaw = document.getElementById('ltStopTime').value;

                if (!startTimeRaw) return;

                const now = new Date();
                const startTime = new Date(startTimeRaw);
                const stopTime = stopTimeRaw ? new Date(stopTimeRaw) : null;

                // อัปเดต countdown display
                updateCountdownDisplay(startTime, stopTime);

                // ตรวจสอบว่าเวลาปัจจุบันถึงหรือเกินเวลาเริ่มต้น และยังไม่ได้เริ่มงาน
                if (now >= startTime && !scheduleStarted) {
                    scheduleStarted = true;
                    ltAddLog('⏰ ถึงเวลาที่กำหนด - เริ่มดึงข้อมูลแท่งเทียนอัตโนมัติ', 'signal');
                    showLtToast('🚀 เริ่มดึงข้อมูลตามตารางเวลา', 'success');

                    // คลิกปุ่ม Go Long Term Trade โดยอัตโนมัติ
                    document.getElementById('ltGoTradeBtn').click();
                }

                // ตรวจสอบเวลาหยุด (Stop Time)
                if (stopTime && scheduleStarted) {
                    if (now >= stopTime) {
                        ltAddLog('⏰ ถึงเวลาหยุดที่กำหนด - หยุดการเทรดอัตโนมัติ', 'exit');
                        showLtToast('⏹️ หยุดการเทรดตามตารางเวลา', 'error');
                        document.getElementById('ltStopTradeBtn').click();
                        stopScheduleMonitor();
                    }
                }
            }, 1000); // ตรวจสอบทุก 1 วินาที

            ltAddLog('⏱️ เปิดการตรวจสอบตารางเวลา (Schedule Monitor)', 'system');
            showLtToast('⏱️ เปิดการตรวจสอบตารางเวลา', 'success');
        }

        function stopScheduleMonitor() {
            if (scheduleCheckInterval) {
                clearInterval(scheduleCheckInterval);
                scheduleCheckInterval = null;
                scheduleStarted = false;

                const scheduleStatusEl = document.getElementById('ltScheduleStatus');
                scheduleStatusEl.style.display = 'none';

                ltAddLog('⏹️ หยุดการตรวจสอบตารางเวลา', 'system');
                showLtToast('⏹️ หยุดการตรวจสอบตารางเวลา', 'error');
            }
        }

        // เริ่มตรวจสอบตารางเวลาเมื่อ checkbox ถูกเลือก
        document.getElementById('ltUseSchedule').addEventListener('change', (e) => {
            if (e.target.checked) {
                const startTimeRaw = document.getElementById('ltStartTime').value;
                if (!startTimeRaw) {
                    showLtToast('⚠️ กรุณาระบุเวลาเริ่มต้น (Start Time)', 'error');
                    e.target.checked = false;
                    return;
                }
                startScheduleMonitor();
            } else {
                stopScheduleMonitor();
            }
        });

        // ── Pause Order State ────────────────────────────────────
        let ltOrdersPaused = false;

        // ── Save Schedule Time Button ────────────────────────────
        document.getElementById('saveTradeLongTimeBtn').addEventListener('click', async () => {
            const checkedBoxes = Array.from(document.querySelectorAll('#ltAssetCheckboxes input:checked'));
            const exitStrategy = document.querySelector('input[name="ltExitStrategy"]:checked') ? document.querySelector('input[name="ltExitStrategy"]:checked').value : 'targetProfit';
            
            const conditionArr = [];
            if (document.getElementById('ltCondShortMedium') && document.getElementById('ltCondShortMedium').checked) conditionArr.push('condShortMedium');
            if (document.getElementById('ltCondLongCross') && document.getElementById('ltCondLongCross').checked) conditionArr.push('condLongCross');
            if (document.getElementById('ltCondShortLong') && document.getElementById('ltCondShortLong').checked) conditionArr.push('condShortLong');

            const granularity = 60;
            const duration = parseInt(document.getElementById('ltDurationSelect').value, 10);
            const assets = checkedBoxes.map(b => b.value);
            const amount = parseFloat(document.getElementById('ltStakeAmount').value) || 1.0;
            const target_profit = parseFloat(document.getElementById('ltTargetProfitPercent').value) || 30.0;
            const max_orders = parseInt(document.getElementById('ltMaxOrders').value) || 1;

            const use_schedule = document.getElementById('ltUseSchedule').checked;
            const start_time_raw = document.getElementById('ltStartTime').value;
            const stop_time_raw = document.getElementById('ltStopTime').value;
            let start_time = "";
            let stop_time = "";
            
            if (use_schedule) {
                if (!start_time_raw || !stop_time_raw) {
                    alert('กรุณาระบุเวลา Start Time และ Stop Time ให้ครบถ้วน');
                    return;
                }
                start_time = start_time_raw.replace('T', ' ') + ':00';
                stop_time = stop_time_raw.replace('T', ' ') + ':00';
            }

            const schedulePayload = {
                assets,
                granularity,
                duration,
                exit_strategy: exitStrategy,
                conditions: conditionArr,
                start_time,
                stop_time,
                use_schedule,
                amount,
                target_profit,
                max_orders
            };

            const btn = document.getElementById('saveTradeLongTimeBtn');
            const originalText = btn.textContent;
            btn.textContent = "⏳ กำลังบันทึก...";
            btn.disabled = true;

            try {
                const res = await fetch('/api/longterm/schedule', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(schedulePayload)
                });
                const data = await res.json();
                
                if (data.status === 'success') {
                    btn.textContent = "✅ บันทึกแล้ว!";
                    ltAddLog('💾 บันทึกเวลาเทรดและการตั้งค่า Long Term เรียบร้อย', 'system');
                } else {
                    btn.textContent = "❌ บันทึกล้มเหลว";
                }
            } catch (e) {
                console.error("Could not save long term schedule:", e);
                btn.textContent = "❌ บันทึกล้มเหลว";
            }
            
            setTimeout(() => { btn.textContent = originalText; btn.disabled = false; }, 2000);
        });

        // ── Go Trade Button ──────────────────────────────────────
        document.getElementById('ltGoTradeBtn').addEventListener('click', async () => {
            const checkedBoxes = Array.from(document.querySelectorAll('#ltAssetCheckboxes input:checked'));
            if (checkedBoxes.length === 0) {
                alert('กรุณาเลือก Asset อย่างน้อย 1 รายการ');
                return;
            }

            // Auto-select the first asset for the chart if none is selected
            const chartSelect = document.getElementById('ltChartAssetSelect');
            if (!chartSelect.value && checkedBoxes.length > 0) {
                chartSelect.value = checkedBoxes[0].value;
                document.getElementById('lt-asset-name').textContent = `Asset: ${checkedBoxes[0].nextSibling.textContent.trim()}`;
            }

            const assetVals = checkedBoxes.map(b => b.value).join(', ');
            ltAddLog(`🚀 เริ่มตั้งค่า Long Term Trade — Asset: ${assetVals}`, 'signal');
            ltAddLog(`📊 Duration: ${document.getElementById('ltDurationSelect').options[document.getElementById('ltDurationSelect').selectedIndex].text}`, 'system');

            const exitStrategy = document.querySelector('input[name="ltExitStrategy"]:checked').value;
            ltAddLog(`🚪 Exit Strategy: ${exitStrategy}`, 'system');

            // Check which entry conditions are active
            const conditionArr = [];
            if (document.getElementById('ltCondShortMedium').checked) conditionArr.push('condShortMedium');
            if (document.getElementById('ltCondLongCross').checked) conditionArr.push('condLongCross');
            if (document.getElementById('ltCondShortLong').checked) conditionArr.push('condShortLong');

            document.getElementById('lt-timeframe-display').textContent = `Timeframe: 1M (60s)`;

            const granularity = 60; // Always use 1M for the bot/chart
            const duration = parseInt(document.getElementById('ltDurationSelect').value, 10);
            const assets = checkedBoxes.map(b => b.value);
            const amount = parseFloat(document.getElementById('ltStakeAmount').value) || 1.0;
            const target_profit = parseFloat(document.getElementById('ltTargetProfitPercent').value) || 30.0;

            const use_schedule = document.getElementById('ltUseSchedule').checked;
            const start_time_raw = document.getElementById('ltStartTime').value;
            const stop_time_raw = document.getElementById('ltStopTime').value;
            let start_time = "";
            let stop_time = "";
            if (use_schedule) {
                if (!start_time_raw || !stop_time_raw) {
                    alert('กรุณาระบุเวลา Start Time และ Stop Time ให้ครบถ้วน');
                    return;
                }
                start_time = start_time_raw.replace('T', ' ') + ':00';
                stop_time = stop_time_raw.replace('T', ' ') + ':00';
            }

            const max_orders = parseInt(document.getElementById('ltMaxOrders').value) || 1;
            const schedulePayload = {
                assets,
                granularity,
                duration,
                exit_strategy: exitStrategy,
                conditions: conditionArr,
                start_time,
                stop_time,
                use_schedule,
                amount,
                target_profit,
                max_orders
            };

            try {
                const res = await fetch('/api/longterm/schedule', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(schedulePayload)
                });
                const data = await res.json();
                console.log('📋 Schedule API response:', data);
                console.log('🔘 use_schedule:', use_schedule);

                if (data.status === 'success') {
                    if (!use_schedule) {
                        console.log('🚀 Starting immediately (no schedule)');
                        // Start immediately
                        const startRes = await fetch('/api/longterm/start', {
                            method: 'POST',
                            headers: { 'Content-Type': 'application/json' },
                            body: JSON.stringify({ assets, granularity })
                        });
                        const startData = await startRes.json();
                        console.log('📋 Start API response:', startData);

                        if (startData.status === 'success') {
                            ltAddLog(startData.message, 'system');

                            // ดึงข้อมูลแท่งเทียนย้อนหลังสำหรับ assets ทั้งหมด
                            ltAddLog('📊 กำลังดึงข้อมูลแท่งเทียนย้อนหลัง...', 'system');
                            console.log('🔍 Fetching candles for assets:', assets, 'granularity:', granularity);

                            for (const asset of assets) {
                                try {
                                    const candlesUrl = `/api/longterm/candles?asset=${asset}&granularity=${granularity}`;
                                    console.log('🌐 Fetching:', candlesUrl);

                                    const candlesRes = await fetch(candlesUrl);
                                    const candlesJson = await candlesRes.json();

                                    console.log(`📦 Response for ${asset}:`, candlesJson);

                                    if (candlesJson.status === 'success' && candlesJson.data) {
                                        console.log(`✅ Got ${candlesJson.data.length} candles for ${asset}`);
                                        ltAddLog(`✅ ดึงข้อมูล ${asset} สำเร็จ (${candlesJson.data.length} แท่ง)`, 'system');

                                        // จำลองการส่งข้อมูลผ่าน WebSocket
                                        const fakeMsg = {
                                            type: "lt_candles_history",
                                            asset: asset,
                                            data: candlesJson.data
                                        };

                                        // ใช้ code เดียวกันกับที่จัดการ WebSocket message
                                        ltCandlesData[asset] = candlesJson.data;
                                        const currentAsset = document.getElementById('ltChartAssetSelect').value;
                                        if (asset === currentAsset && window.ltCandleSeries) {
                                            const chartData = candlesJson.data.map(d => ({
                                                time: toThaiTime(d.candletime),
                                                open: d.open,
                                                high: d.high,
                                                low: d.low,
                                                close: d.close
                                            }));

                                            const emaShortData = candlesJson.data.filter(d => d.ema_short_value > 0).map(d => ({
                                                time: toThaiTime(d.candletime),
                                                value: d.ema_short_value
                                            }));
                                            const emaMediumData = candlesJson.data.filter(d => d.ema_medium_value > 0).map(d => ({
                                                time: toThaiTime(d.candletime),
                                                value: d.ema_medium_value
                                            }));
                                            const emaLongData = candlesJson.data.filter(d => d.ema_long_value > 0).map(d => ({
                                                time: toThaiTime(d.candletime),
                                                value: d.ema_long_value
                                            }));

                                            window.ltCandleSeries.setData(chartData);
                                            window.ltEmaShortSeries.setData(emaShortData);
                                            window.ltEmaMediumSeries.setData(emaMediumData);
                                            window.ltEmaLongSeries.setData(emaLongData);
                                            window.ltCandleSeries.setMarkers([]);

                                            ltAddLog(`📈 กราฟแสดง ${chartData.length} แท่งเทียนแล้ว`, 'system');
                                        }

                                        if (candlesJson.data.length > 0) {
                                            updateSignalDisplay(candlesJson.data[candlesJson.data.length - 1], asset);
                                        }
                                    }
                                } catch (candlesErr) {
                                    console.error(`Failed to fetch candles for ${asset}:`, candlesErr);
                                    ltAddLog(`⚠️ ดึงข้อมูล ${asset} ล้มเหลว`, 'exit');
                                }
                            }
                        } else {
                            ltAddLog('Error: ' + startData.message, 'exit');
                            console.error('❌ Start failed:', startData);
                        }
                    } else {
                        console.log('⏰ Schedule mode - waiting for start time');
                        ltAddLog('⏰ ตั้งตารางเวลาสำเร็จ - รอเวลาเริ่มต้น', 'system');

                        // ดึงข้อมูลแท่งเทียนย้อนหลังเพื่อแสดงกราฟก่อนรอเวลาเริ่มต้น
                        ltAddLog('📊 กำลังดึงข้อมูลแท่งเทียนย้อนหลัง...', 'system');
                        console.log('🔍 Fetching candles for schedule mode - assets:', assets, 'granularity:', granularity);

                        for (const asset of assets) {
                            try {
                                const candlesUrl = `/api/longterm/candles?asset=${asset}&granularity=${granularity}`;
                                console.log('🌐 Fetching:', candlesUrl);

                                const candlesRes = await fetch(candlesUrl);
                                const candlesJson = await candlesRes.json();

                                console.log(`📦 Response for ${asset}:`, candlesJson);

                                if (candlesJson.status === 'success' && candlesJson.data) {
                                    console.log(`✅ Got ${candlesJson.data.length} candles for ${asset}`);
                                    ltAddLog(`✅ ดึงข้อมูล ${asset} สำเร็จ (${candlesJson.data.length} แท่ง)`, 'system');

                                    // Cache data
                                    ltCandlesData[asset] = candlesJson.data;
                                    const currentAsset = document.getElementById('ltChartAssetSelect').value;
                                    if (asset === currentAsset && window.ltCandleSeries) {
                                        const chartData = candlesJson.data.map(d => ({
                                            time: toThaiTime(d.candletime),
                                            open: d.open,
                                            high: d.high,
                                            low: d.low,
                                            close: d.close
                                        }));

                                        const emaShortData = candlesJson.data.filter(d => d.ema_short_value > 0).map(d => ({
                                            time: toThaiTime(d.candletime),
                                            value: d.ema_short_value
                                        }));
                                        const emaMediumData = candlesJson.data.filter(d => d.ema_medium_value > 0).map(d => ({
                                            time: toThaiTime(d.candletime),
                                            value: d.ema_medium_value
                                        }));
                                        const emaLongData = candlesJson.data.filter(d => d.ema_long_value > 0).map(d => ({
                                            time: toThaiTime(d.candletime),
                                            value: d.ema_long_value
                                        }));

                                        window.ltCandleSeries.setData(chartData);
                                        window.ltEmaShortSeries.setData(emaShortData);
                                        window.ltEmaMediumSeries.setData(emaMediumData);
                                        window.ltEmaLongSeries.setData(emaLongData);
                                        window.ltCandleSeries.setMarkers([]);

                                        ltAddLog(`📈 กราฟแสดง ${chartData.length} แท่งเทียนแล้ว`, 'system');
                                    }

                                    if (candlesJson.data.length > 0) {
                                        updateSignalDisplay(candlesJson.data[candlesJson.data.length - 1], asset);
                                    }
                                } else {
                                    console.warn(`⚠️ No data or failed for ${asset}:`, candlesJson);
                                    ltAddLog(`⚠️ ไม่มีข้อมูล ${asset}`, 'system');
                                }
                            } catch (candlesErr) {
                                console.error(`Failed to fetch candles for ${asset}:`, candlesErr);
                                ltAddLog(`⚠️ ดึงข้อมูล ${asset} ล้มเหลว`, 'exit');
                            }
                        }

                        // Show Pause button after trade started
                        document.getElementById('ltPauseOrderBtn').style.display = 'inline-flex';
                    }
                } else {
                    ltAddLog('Error: ' + data.message, 'exit');
                    console.error('❌ Schedule failed:', data);
                }
            } catch (err) {
                ltAddLog('Fetch Error: ' + err, 'exit');
            }
        });

        // ── Pause Order Button ────────────────────────────────────
        document.getElementById('ltPauseOrderBtn').addEventListener('click', () => {
            ltOrdersPaused = !ltOrdersPaused;
            const btn = document.getElementById('ltPauseOrderBtn');

            if (ltOrdersPaused) {
                btn.innerHTML = '▶️ Resume Orders';
                btn.style.background = 'var(--green)';
                ltAddLog('⏸️ หยุดรับ Order ชั่วคราว (Paused)', 'system');
                showLtToast('⏸️ หยุดรับ Order ชั่วคราว', 'success');
            } else {
                btn.innerHTML = '⏸️ Pause Orders';
                btn.style.background = 'var(--orange)';
                ltAddLog('▶️ กลับมารับ Order ใหม่ (Resumed)', 'system');
                showLtToast('▶️ กลับมารับ Order ใหม่', 'success');
            }
        });

        // ── Stop Trade Button ────────────────────────────────────
        document.getElementById('ltStopTradeBtn').addEventListener('click', () => {
            ltAddLog('⛔ Long Term Trade หยุดทำงาน', 'exit');
            scheduleStarted = false; // รีเซ็ตสถานะเพื่อให้สามารถเริ่มใหม่ได้

            // Reset pause state
            ltOrdersPaused = false;
            const pauseBtn = document.getElementById('ltPauseOrderBtn');
            pauseBtn.style.display = 'none';
            pauseBtn.innerHTML = '⏸️ Pause Orders';
            pauseBtn.style.background = 'var(--orange)';

            fetch('/api/longterm/stop', { method: 'POST' })
                .then(res => res.json())
                .then(data => ltAddLog(data.message, 'system'))
                .catch(err => ltAddLog('Fetch Error: ' + err, 'exit'));
        });

        // ── Real-time WebSocket Data ─────────────────────────────
        let ltWs = null;
        let ltCandlesData = {}; // Cache for switching assets

        // When user changes chart asset dropdown
        document.getElementById('ltChartAssetSelect').addEventListener('change', (e) => {
            const selectedAsset = e.target.value;
            const selectedName = e.target.options[e.target.selectedIndex]?.text || '—';
            document.getElementById('lt-asset-name').textContent = `Asset: ${selectedName}`;

            if (window.ltCandleSeries && ltCandlesData[selectedAsset]) {
                const chartData = ltCandlesData[selectedAsset].map(d => ({
                    time: toThaiTime(d.candletime),
                    open: d.open,
                    high: d.high,
                    low: d.low,
                    close: d.close
                }));
                const emaShortData = ltCandlesData[selectedAsset].filter(d => d.ema_short_value > 0).map(d => ({ time: toThaiTime(d.candletime), value: d.ema_short_value }));
                const emaMediumData = ltCandlesData[selectedAsset].filter(d => d.ema_medium_value > 0).map(d => ({ time: toThaiTime(d.candletime), value: d.ema_medium_value }));
                const emaLongData = ltCandlesData[selectedAsset].filter(d => d.ema_long_value > 0).map(d => ({ time: toThaiTime(d.candletime), value: d.ema_long_value }));

                window.ltCandleSeries.setData(chartData);
                window.ltEmaShortSeries.setData(emaShortData);
                window.ltEmaMediumSeries.setData(emaMediumData);
                window.ltEmaLongSeries.setData(emaLongData);

                const activeBtn = document.querySelector('.chart-toolbar .gran-btn.active');
                window.ltCurrentChartGranularity = activeBtn ? parseInt(activeBtn.dataset.value, 10) : parseInt(document.getElementById('ltDurationSelect').value, 10);
                window.ltCurrentChartData = chartData;

                const dataArr = ltCandlesData[selectedAsset];
                if (dataArr.length > 0) {
                    updateSignalDisplay(dataArr[dataArr.length - 1], selectedAsset);
                }

                // Update order entry markers
                updateOrderEntryMarkers(selectedAsset);
            } else if (window.ltCandleSeries) {
                window.ltCandleSeries.setData([]);
                window.ltEmaShortSeries.setData([]);
                window.ltEmaMediumSeries.setData([]);
                window.ltEmaLongSeries.setData([]);
                updateSignalDisplay({}, selectedAsset);
            }
        });

        function connectLtWebSocket() {
            if (ltWs) return;
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            ltWs = new WebSocket(`${protocol}//${window.location.host}/ws`);

            ltWs.onopen = () => {
                console.log("[LongTerm] Connected to Rust WebSocket");
                const badge = document.getElementById('ltConnBadge');
                badge.classList.remove('disconnected');
                badge.classList.add('connected');
                document.getElementById('ltConnText').textContent = 'เชื่อมต่อแล้ว';
            };
            ltWs.onclose = () => {
                console.log("[LongTerm] Disconnected. Reconnecting...");
                const badge = document.getElementById('ltConnBadge');
                badge.classList.remove('connected');
                badge.classList.add('disconnected');
                document.getElementById('ltConnText').textContent = 'ขาดการเชื่อมต่อ';
                ltWs = null;
                setTimeout(connectLtWebSocket, 3000);
            };

            ltWs.onmessage = (event) => {
                try {
                    const msg = JSON.parse(event.data);

                    if (msg.type === "lt_candles_history") {
                        console.log(`📊 [lt_candles_history] Received ${msg.data.length} candles for ${msg.asset}`);

                        // Log sample of first few candles
                        if (msg.data.length > 0) {
                            console.log('First candle:', msg.data[0]);
                            console.log('Last candle:', msg.data[msg.data.length - 1]);
                        }

                        ltCandlesData[msg.asset] = msg.data; // Cache data
                        const currentAsset = document.getElementById('ltChartAssetSelect').value;
                        if (msg.asset === currentAsset && window.ltCandleSeries && msg.data) {
                            const chartData = msg.data.map(d => ({
                                time: toThaiTime(d.candletime),
                                open: d.open,
                                high: d.high,
                                low: d.low,
                                close: d.close
                            }));

                            console.log(`📈 Chart data prepared: ${chartData.length} candles`);
                            if (chartData.length > 0) {
                                console.log('First chart point:', chartData[0]);
                                console.log('Last chart point:', chartData[chartData.length - 1]);
                            }

                            const emaShortData = msg.data.filter(d => d.ema_short_value > 0).map(d => ({ time: toThaiTime(d.candletime), value: d.ema_short_value }));
                            const emaMediumData = msg.data.filter(d => d.ema_medium_value > 0).map(d => ({ time: toThaiTime(d.candletime), value: d.ema_medium_value }));
                            const emaLongData = msg.data.filter(d => d.ema_long_value > 0).map(d => ({ time: toThaiTime(d.candletime), value: d.ema_long_value }));

                            window.ltCandleSeries.setData(chartData);
                            window.ltEmaShortSeries.setData(emaShortData);
                            window.ltEmaMediumSeries.setData(emaMediumData);
                            window.ltEmaLongSeries.setData(emaLongData);

                            console.log('✅ Chart updated successfully');

                            // Re-apply markers if exist
                            if (window.emaCutCustomMarkers && window.emaCutCustomMarkers[msg.asset]) {
                                const markers = [...window.emaCutCustomMarkers[msg.asset]];
                                markers.sort((a, b) => a.time - b.time);
                                window.ltCandleSeries.setMarkers(markers);
                            } else {
                                window.ltCandleSeries.setMarkers([]);
                            }
                        }
                        if (msg.data && msg.data.length > 0) {
                            updateSignalDisplay(msg.data[msg.data.length - 1], msg.asset);
                        }
                    } else if (msg.type === "lt_orders_update") {
                        console.log("🔥 [lt_orders_update] Raw Data from Rust:", msg.data);
                        ltOpenOrders = msg.data;
                        ltUpdateOpenOrdersTable();

                        // Update order entry markers
                        const currentAsset = document.getElementById('ltChartAssetSelect').value;
                        if (currentAsset) {
                            updateOrderEntryMarkers(currentAsset);
                        }
                    } else if (msg.type === "lt_candle_update") {
                        // Log candle updates for debugging
                        const now = new Date().toLocaleTimeString('th-TH');
                        console.log(`🕐 [${now}] lt_candle_update for ${msg.asset}:`, msg.data.candletime, 'close:', msg.data.close);

                        // Update last received timestamp
                        window.ltLastCandleUpdate = Date.now();

                        // Update "Last Update" display in UI
                        const lastUpdateEl = document.getElementById('lt-last-update');
                        if (lastUpdateEl) {
                            lastUpdateEl.textContent = `Last: ${now}`;
                            lastUpdateEl.style.color = 'var(--green)';
                            // Reset color after 2 seconds
                            setTimeout(() => {
                                if (lastUpdateEl) lastUpdateEl.style.color = 'var(--text3)';
                            }, 2000);
                        }

                        if (ltCandlesData[msg.asset]) {
                            // Update cache
                            const arr = ltCandlesData[msg.asset];
                            if (arr.length > 0 && arr[arr.length - 1].candletime === msg.data.candletime) {
                                arr[arr.length - 1] = msg.data;
                            } else {
                                // ─── NEW CANDLE DETECTED ───
                                const closedCandle = arr[arr.length - 1];

                                // Use backend's pre-computed ema_cut_position (from full_analysis_ver2)
                                const cutPosition = closedCandle.ema_cut_position;
                                const isCrossUp = cutPosition === 'CrossUp';
                                const isCrossDown = cutPosition === 'CrossDown';

                                if (isCrossUp || isCrossDown) {
                                    ltAddLog(`⚡ EMA Cut Detected: ${cutPosition} (${msg.asset})`, isCrossUp ? 'signal' : 'exit');

                                    if (document.getElementById('playEmaCutSoundToggle') && document.getElementById('playEmaCutSoundToggle').checked) {
                                        try {
                                            const audio = new Audio('/sounds/mp3/Jungle exclamation.mp3');
                                            audio.play().catch(e => console.log('Audio play failed:', e));
                                        } catch (e) { }
                                    }

                                    if (!window.emaCutCustomMarkers) window.emaCutCustomMarkers = {};
                                    if (!window.emaCutCustomMarkers[msg.asset]) window.emaCutCustomMarkers[msg.asset] = [];

                                    window.emaCutCustomMarkers[msg.asset].push({
                                        time: toThaiTime(closedCandle.candletime),
                                        position: isCrossUp ? 'belowBar' : 'aboveBar',
                                        color: isCrossUp ? '#06D6A0' : '#E8304A',
                                        shape: isCrossUp ? 'arrowUp' : 'arrowDown',
                                        text: isCrossUp ? 'EMA Cut Up' : 'EMA Cut Down'
                                    });

                                    // If current chart is this asset, add markers
                                    const currentAsset = document.getElementById('ltChartAssetSelect').value;
                                    if (currentAsset === msg.asset && window.ltCandleSeries) {
                                        const markers = [...(window.emaCutCustomMarkers[msg.asset] || [])];
                                        markers.sort((a, b) => a.time - b.time);
                                        window.ltCandleSeries.setMarkers(markers);
                                    }

                                    // Client-side Trigger logic removed, now handled by Backend.
                                }
                                arr.push(msg.data);
                            }
                        } else {
                            ltCandlesData[msg.asset] = [msg.data];
                        }

                        const currentAsset = document.getElementById('ltChartAssetSelect').value;
                        if (msg.asset === currentAsset && window.ltCandleSeries && msg.data) {
                            const d = msg.data;
                            try {
                                window.ltCandleSeries.update({
                                    time: toThaiTime(d.candletime),
                                    open: d.open,
                                    high: d.high,
                                    low: d.low,
                                    close: d.close
                                });
                                window.ltEmaShortSeries.update({ time: toThaiTime(d.candletime), value: d.ema_short_value });
                                window.ltEmaMediumSeries.update({ time: toThaiTime(d.candletime), value: d.ema_medium_value });
                                window.ltEmaLongSeries.update({ time: toThaiTime(d.candletime), value: d.ema_long_value });
                            } catch (updateErr) { }
                        }
                        updateSignalDisplay(msg.data, msg.asset);
                    } else if (msg.type === "bot_log") {
                        if (msg.data && msg.data.message) {
                            ltAddLog(msg.data.message, 'system');
                        }
                    } else if (msg.type === "lt_bot_started") {
                        ltAddLog("🟢 เริ่ม Long Term Trade (Schedule)", 'system');
                        if (msg.assets && msg.assets.length > 0) {
                            const assetSelect = document.getElementById('ltChartAssetSelect');
                            assetSelect.value = msg.assets[0];
                            assetSelect.dispatchEvent(new Event('change'));
                        }
                    }
                } catch (err) {
                    console.error("WS Parse error:", err);
                }
            };
        }
        connectLtWebSocket();

        // ── Heartbeat Monitor ────────────────────────────────────
        window.ltLastCandleUpdate = Date.now();
        let ltHeartbeatCheckInterval = null;

        function startHeartbeatMonitor() {
            if (ltHeartbeatCheckInterval) return;

            ltHeartbeatCheckInterval = setInterval(() => {
                const now = Date.now();
                const timeSinceLastUpdate = now - window.ltLastCandleUpdate;
                const secondsSinceUpdate = Math.floor(timeSinceLastUpdate / 1000);

                // ถ้าไม่ได้รับข้อมูลมานานกว่า 90 วินาที แสดงเตือน
                if (secondsSinceUpdate > 90) {
                    console.warn(`⚠️ No candle update for ${secondsSinceUpdate} seconds`);

                    // แสดงเตือนใน UI
                    const connBadge = document.getElementById('ltConnBadge');
                    if (connBadge && connBadge.classList.contains('connected')) {
                        ltAddLog(`⚠️ ไม่ได้รับข้อมูลแท่งเทียนมา ${secondsSinceUpdate} วินาทีแล้ว - Backend อาจหยุดทำงาน`, 'system');
                    }
                }

                // ถ้าไม่ได้รับมานานกว่า 2 นาที ลอง reconnect
                if (secondsSinceUpdate > 120 && ltWs) {
                    console.error('🔴 No data for 2 minutes - attempting reconnect');
                    ltAddLog('🔄 ไม่ได้รับข้อมูลนานเกินไป - ลองเชื่อมต่อใหม่...', 'exit');
                    ltWs.close();
                }
            }, 30000); // ตรวจสอบทุก 30 วินาที
        }

        startHeartbeatMonitor();

        // ── Order Entry Markers ──────────────────────────────────
        window.ltOrderEntryMarkers = {}; // Store order entry markers by asset

        function updateOrderEntryMarkers(asset) {
            if (!window.ltCandleSeries) return;

            const currentAsset = document.getElementById('ltChartAssetSelect').value;
            if (currentAsset !== asset) return;

            // สร้าง markers สำหรับ orders ที่เปิดอยู่
            const orderMarkers = [];
            ltOpenOrders.forEach(order => {
                const orderAsset = order.underlying || order.symbol || order.display_name || '';
                if (orderAsset === asset && order.purchase_time) {
                    const isCall = order.contract_type === 'CALL';
                    orderMarkers.push({
                        time: order.purchase_time, // purchase_time เป็น epoch seconds แล้ว
                        position: isCall ? 'belowBar' : 'aboveBar',
                        color: isCall ? '#26a69a' : '#ef5350',
                        shape: isCall ? 'arrowUp' : 'arrowDown',
                        text: `${order.contract_type} $${order.buy_price.toFixed(2)}`
                    });
                }
            });

            // รวม markers ทั้ง EMA cut และ order entry
            const emaCutMarkers = window.emaCutCustomMarkers && window.emaCutCustomMarkers[asset]
                ? [...window.emaCutCustomMarkers[asset]]
                : [];

            const allMarkers = [...emaCutMarkers, ...orderMarkers];
            allMarkers.sort((a, b) => a.time - b.time);

            window.ltCandleSeries.setMarkers(allMarkers);
            console.log(`📍 Updated ${orderMarkers.length} order entry markers for ${asset}`);
        }

        function updateSignalDisplay(analysis, assetName) {
            if (!analysis) return;

            // Only update display if it's the currently selected asset
            const currentAsset = document.getElementById('ltChartAssetSelect').value;
            if (assetName && assetName !== currentAsset) return;

            // Update EMA values
            document.getElementById('ltEmaShortVal').textContent = analysis.ema_short_value?.toFixed(4) || '—';
            document.getElementById('ltEmaMediumVal').textContent = analysis.ema_medium_value?.toFixed(4) || '—';
            document.getElementById('ltEmaLongVal').textContent = analysis.ema_long_value?.toFixed(4) || '—';

            // Update EMA directions
            document.getElementById('ltEmaShortDir').textContent = analysis.ema_short_direction || '—';
            document.getElementById('ltEmaMediumDir').textContent = analysis.ema_medium_direction || '—';
            document.getElementById('ltEmaLongDir').textContent = analysis.ema_long_direction || '—';

            // Update cross statuses
            document.getElementById('ltCutShortMediumStatus').textContent = analysis.ema_cut_position || '—';
            document.getElementById('ltCutAllStatus').textContent = analysis.ema_cut_all_type || '—';
            document.getElementById('ltCutShortLongStatus').textContent = analysis.ema_cut_short_long_type || '—';

            // Update indicator gauges
            const adxEl = document.getElementById('ltAdxValue');
            adxEl.textContent = analysis.adx_value?.toFixed(2) || '—';
            const adxStatus = document.getElementById('ltAdxStatus');
            if (analysis.adx_value > 25) {
                adxEl.style.color = 'var(--green)';
                adxStatus.textContent = 'Trending';
                adxStatus.style.color = 'var(--green)';
            } else {
                adxEl.style.color = 'var(--orange)';
                adxStatus.textContent = 'Weak/Ranging';
                adxStatus.style.color = 'var(--orange)';
            }

            document.getElementById('ltChoppyValue').textContent = analysis.choppy_indicator?.toFixed(2) || '—';
            const choppyStatus = document.getElementById('ltChoppyStatus');
            if (analysis.choppy_indicator < 38) {
                choppyStatus.textContent = 'Trending';
                choppyStatus.style.color = 'var(--green)';
            } else {
                choppyStatus.textContent = 'Choppy';
                choppyStatus.style.color = 'var(--red)';
            }

            const rsiEl = document.getElementById('ltRsiValue');
            rsiEl.textContent = analysis.rsi_value?.toFixed(2) || '—';
            const rsiStatus = document.getElementById('ltRsiStatus');
            if (analysis.rsi_value > 70) {
                rsiStatus.textContent = 'Overbought';
                rsiStatus.style.color = 'var(--red)';
            } else if (analysis.rsi_value < 30) {
                rsiStatus.textContent = 'Oversold';
                rsiStatus.style.color = 'var(--green)';
            } else {
                rsiStatus.textContent = 'Neutral';
                rsiStatus.style.color = 'var(--text3)';
            }

            document.getElementById('ltEmaAbove').textContent = analysis.ema_above || '—';
            document.getElementById('ltEmaLongAbove').textContent = analysis.ema_long_above || '—';
            document.getElementById('ltConvergence').textContent = analysis.ema_convergence_type || '—';
            document.getElementById('ltLongConvergence').textContent = analysis.ema_long_convergence_type || '—';

            // Determine signal
            let signalType = 'wait';
            let signalText = 'WAITING — รอสัญญาณ...';

            const condShortMedium = document.getElementById('ltCondShortMedium').checked;
            const condAllCross = document.getElementById('ltCondLongCross').checked;
            const condShortLong = document.getElementById('ltCondShortLong').checked;

            if (condShortMedium && (analysis.ema_cut_position === 'CrossUp' || analysis.ema_cut_position === 'CrossDown')) {
                signalType = analysis.ema_cut_position === 'CrossUp' ? 'call' : 'put';
                signalText = `${analysis.ema_cut_position === 'CrossUp' ? '🟢 CALL' : '🔴 PUT'} — EMA Short ✕ Medium ${analysis.ema_cut_position}`;
            }
            if (condAllCross && (analysis.ema_cut_all_type === 'AllCrossUp' || analysis.ema_cut_all_type === 'AllCrossDown')) {
                signalType = analysis.ema_cut_all_type === 'AllCrossUp' ? 'call' : 'put';
                signalText = `${analysis.ema_cut_all_type === 'AllCrossUp' ? '🟢 CALL' : '🔴 PUT'} — All EMA ${analysis.ema_cut_all_type}`;
            }
            if (condShortLong && (analysis.ema_cut_short_long_type === 'CrossUp' || analysis.ema_cut_short_long_type === 'CrossDown')) {
                signalType = analysis.ema_cut_short_long_type === 'CrossUp' ? 'call' : 'put';
                signalText = `${analysis.ema_cut_short_long_type === 'CrossUp' ? '🟢 CALL' : '🔴 PUT'} — EMA Short ✕ Long ${analysis.ema_cut_short_long_type}`;
            }

            const signalBox = document.getElementById('ltSignalBox');
            signalBox.className = `signal-box signal-${signalType === 'call' ? 'call' : signalType === 'put' ? 'put' : 'wait'}`;
            document.getElementById('ltSignalText').textContent = signalText;
        }

        // ── Save and Load Input Values ───────────────────────────
        function saveInputValues() {
            const inputs = document.querySelectorAll('input:not(#ltStartTime):not(#ltStopTime), select, textarea');
            const values = {};
            inputs.forEach(input => {
                if (input.id || input.name) {
                    const key = input.id || input.name;
                    if (input.type === 'checkbox') {
                        if (input.name === 'ltAsset') {
                            if (!values[key]) values[key] = [];
                            if (input.checked) values[key].push(input.value);
                        } else {
                            values[key] = input.checked;
                        }
                    } else if (input.type === 'radio') {
                        if (input.checked) values[key] = input.value;
                    } else {
                        values[key] = input.value;
                    }
                }
            });
            localStorage.setItem('lt-input-values', JSON.stringify(values));
        }

        function loadInputValues() {
            const saved = localStorage.getItem('lt-input-values');
            if (saved) {
                try {
                    const values = JSON.parse(saved);
                    const inputs = document.querySelectorAll('input:not(#ltStartTime):not(#ltStopTime), select, textarea');
                    inputs.forEach(input => {
                        const key = input.id || input.name;
                        if (values.hasOwnProperty(key)) {
                            if (input.type === 'checkbox') {
                                if (input.name === 'ltAsset') {
                                    input.checked = Array.isArray(values[key]) && values[key].includes(input.value);
                                } else {
                                    input.checked = values[key];
                                }
                                input.dispatchEvent(new Event('change', { bubbles: true }));
                            } else if (input.type === 'radio') {
                                if (input.value === values[key]) {
                                    input.checked = true;
                                    input.dispatchEvent(new Event('change', { bubbles: true }));
                                }
                            } else {
                                input.value = values[key];
                                input.dispatchEvent(new Event('change', { bubbles: true }));
                            }
                        }
                    });
                } catch (e) {
                    console.error('Error loading input values', e);
                }
            }
        }

        // Initialize and attach events
        setTimeout(() => {
            loadInputValues();


            // Save input values on change
            document.querySelectorAll('input:not(#ltStartTime):not(#ltStopTime), select, textarea').forEach(input => {
                input.addEventListener('change', saveInputValues);
                if (input.type === 'number' || input.type === 'text') {
                    input.addEventListener('input', saveInputValues);
                }
            });
        }, 100);

        // ── Deriv API WebSocket (Phase 4) ───────────────────────────
        let lastOpenOrdersIds = '';
        let ltOpenOrders = [];
        let ltKnownOrderIds = new Set();

        function ltUpdateOpenOrdersTable() {
            const tbodyMain = document.querySelector('#ltOrdersTable tbody');
            const summaryMain = document.getElementById('lt-orders-summary');
            const tbodyChart = document.querySelector('#ltOrdersTableChart tbody');
            const summaryChart = document.getElementById('lt-orders-summary-chart');
            const orderCount = document.getElementById('ltOrderCount');

            if (!tbodyMain && !tbodyChart) return;

            // Check if user is focused inside either table — skip all DOM updates to preserve focus
            const activeEl = document.activeElement;
            const userEditing = (tbodyMain && tbodyMain.contains(activeEl)) || (tbodyChart && tbodyChart.contains(activeEl));

            const nowTime = Math.floor(Date.now() / 1000);
            let totalProfit = 0;

            if (ltOpenOrders.length === 0) {
                if (!userEditing) {
                    if (lastOpenOrdersIds !== 'empty') {
                        const emptyHtml = '<tr class="empty-row"><td colspan="17">ไม่มีออเดอร์ที่เปิดอยู่</td></tr>';
                        if (tbodyMain) tbodyMain.innerHTML = emptyHtml;
                        if (tbodyChart) tbodyChart.innerHTML = emptyHtml;
                        lastOpenOrdersIds = 'empty';
                        ltKnownOrderIds.clear();
                    }
                }
                if (summaryMain) summaryMain.textContent = '0 active orders | P/L: $0.00';
                if (summaryChart) summaryChart.textContent = '0 active orders | P/L: $0.00';
                if (orderCount) orderCount.textContent = '(0)';
                return;
            }

            const incomingIds = new Set(ltOpenOrders.map(o => o.contract_id));

            // Remove stale rows (only when user is not editing)
            if (!userEditing) {
                ltKnownOrderIds.forEach(cid => {
                    if (!incomingIds.has(cid)) {
                        // Order was closed/sold - play sound
                        if (!ltPlayedCloseSound.has(cid)) {
                            ltPlayedCloseSound.add(cid);

                            // Play close order sound if enabled
                            if (document.getElementById('playCloseOrderSoundToggle') && document.getElementById('playCloseOrderSoundToggle').checked) {
                                try {
                                    const audio = new Audio('/sounds/mp3/Jungle default sound.mp3');
                                    audio.play().catch(e => console.log('Close order sound play failed:', e));
                                    console.log('🔔 Playing close order sound for contract:', cid);
                                } catch (e) {
                                    console.error('Close order sound error:', e);
                                }
                            }

                            ltAddLog(`✅ Order ${cid} ถูกปิด/ขายแล้ว`, 'exit');
                        }

                        ['main', 'chart'].forEach(prefix => {
                            const row = document.getElementById(`row-${prefix}-${cid}`);
                            if (row) row.remove();
                        });
                        ltKnownOrderIds.delete(cid);

                        // Clean up sound tracking after 5 seconds
                        setTimeout(() => ltPlayedCloseSound.delete(cid), 5000);
                    }
                });
            }

            ltOpenOrders.forEach((order, index) => {
                const profit = parseFloat(order.profit || 0);
                totalProfit += profit;
                const profitColor = profit >= 0 ? '#06D6A0' : '#E8304A';
                const profitSign = profit >= 0 ? '+' : '-';

                const timeStr = order.purchase_time ? new Date(order.purchase_time * 1000).toLocaleTimeString('th-TH') : (order.time ? new Date(order.time).toLocaleTimeString('th-TH') : '-');
                const expiryStr = order.date_expiry ? new Date(order.date_expiry * 1000).toLocaleTimeString('th-TH') : '-';

                let remainingTime = '-';
                if (order.date_expiry) {
                    const diff = Math.max(0, order.date_expiry - nowTime);
                    const mm = Math.floor(diff / 60).toString().padStart(2, '0');
                    const ss = (diff % 60).toString().padStart(2, '0');
                    remainingTime = `${mm}:${ss}`;
                }

                let durationStr = '-';
                if (order.date_start && order.date_expiry) {
                    const dur = order.date_expiry - order.date_start;
                    durationStr = dur >= 60 ? `${Math.floor(dur / 60)}m` : `${dur}s`;
                }

                if (order.target_profit === undefined) {
                    order.target_profit = parseFloat(order.payout || 0);
                }

                const minProfit = order.min_profit !== undefined ? order.min_profit : profit;
                const maxProfit = order.max_profit !== undefined ? order.max_profit : profit;
                const minColor = minProfit >= 0 ? '#06D6A0' : '#E8304A';
                const maxColor = maxProfit >= 0 ? '#06D6A0' : '#E8304A';
                const minSign = minProfit >= 0 ? '+' : '-';
                const maxSign = maxProfit >= 0 ? '+' : '-';
                const targetPercent = parseFloat(order.target_profit) || 0;
                const entrySpotVal = (order.entry_spot > 0) ? order.entry_spot : ((order.entry_tick > 0) ? order.entry_tick : 0);

                ['main', 'chart'].forEach(prefix => {
                    const tbody = prefix === 'main' ? tbodyMain : tbodyChart;
                    if (!tbody) return;

                    // ลบแถว empty-row ก่อนเพิ่ม order แรก
                    if (index === 0) {
                        const emptyRow = tbody.querySelector('tr.empty-row');
                        if (emptyRow) {
                            emptyRow.remove();
                            console.log(`🗑️ Removed empty row from ${prefix} table`);
                        }
                    }

                    // Create row only once when order first appears
                    if (!document.getElementById(`row-${prefix}-${order.contract_id}`)) {
                        const tr = document.createElement('tr');
                        tr.id = `row-${prefix}-${order.contract_id}`;

                        // สร้างปุ่ม Sell ตามสถานะ
                        const sellButtonHtml = order.is_selling
                            ? `<button class="fetch-btn" id="sell-btn-${prefix}-${order.contract_id}" style="background:#6c757d;color:#fff;padding:2px 8px;font-size:10px;border-radius:4px;cursor:not-allowed;border:none;" disabled>ขายแล้ว</button>`
                            : `<button class="fetch-btn" id="sell-btn-${prefix}-${order.contract_id}" style="background:#E8304A;color:#fff;padding:2px 8px;font-size:10px;border-radius:4px;cursor:pointer;border:none;" onclick="ltSellContract(${order.contract_id}, this)">Sell</button>`;

                        tr.innerHTML = `
                            <td>${index + 1}</td>
                            <td style="font-family:monospace;font-size:11px;">${order.contract_id || '-'}</td>
                            <td id="td-asset-${prefix}-${order.contract_id}"><span class="chip-dot" style="background:#06D6A0;"></span>${order.display_name || order.underlying || order.symbol || '-'}</td>
                            <td id="td-type-${prefix}-${order.contract_id}" style="font-weight:bold;">${order.contract_type || '-'}</td>
                            <td>$${parseFloat(order.buy_price || 0).toFixed(2)}</td>
                            <td id="td-entry-${prefix}-${order.contract_id}">${order.entry_spot || order.entry_tick || '-'}</td>
                            <td id="td-spot-${prefix}-${order.contract_id}">${order.current_spot || '-'}</td>
                            <td id="td-profit-${prefix}-${order.contract_id}" style="color:${profitColor};font-weight:bold;">${profitSign}$${Math.abs(profit).toFixed(2)}</td>
                            <td id="td-minprofit-${prefix}-${order.contract_id}" style="color:${minColor};">${minSign}$${Math.abs(minProfit).toFixed(2)}</td>
                            <td id="td-maxprofit-${prefix}-${order.contract_id}" style="color:${maxColor};">${maxSign}$${Math.abs(maxProfit).toFixed(2)}</td>
                            <td>${timeStr}</td>
                            <td id="td-dur-${prefix}-${order.contract_id}">${durationStr}</td>
                            <td id="td-exp-${prefix}-${order.contract_id}">${expiryStr}</td>
                            <td id="td-rem-${prefix}-${order.contract_id}" style="font-family:monospace;font-weight:bold;color:var(--text);">${remainingTime}</td>
                            <td>Manual</td>
                            <td><input type="number" step="0.01" id="td-target-${prefix}-${order.contract_id}" style="width:60px; background:var(--bg3); color:var(--text); border:1px solid var(--border); padding:2px; border-radius:4px; font-size:11px;" value="${targetPercent.toFixed(2)}" onchange="ltUpdateTargetProfit(${order.contract_id}, this.value, ${order.buy_price})"></td>
                            <td>
                                <div style="display:flex; gap:4px; align-items:center;">
                                    ${sellButtonHtml}
                                    <button class="draw-line-btn" id="drawbtn_${prefix}_lt_${order.contract_id}" style="padding:2px 8px; font-size:10px; border-radius:4px; background:var(--bg3); border:1px solid var(--border); color:var(--text); cursor:pointer;" onclick="drawPriceLine(${entrySpotVal > 0 ? entrySpotVal : order.buy_price}, '${order.contract_type}')" title="วาดเส้นราคา Entry Spot">📌</button>
                                </div>
                            </td>
                        `;
                        tbody.appendChild(tr);
                        ltKnownOrderIds.add(order.contract_id);
                        ltPendingOrders = Math.max(0, ltPendingOrders - 1);
                        setManualButtonsEnabled(true);
                    }

                    // Skip field updates when user is editing inside the table
                    if (userEditing) return;

                    // Update only the dynamic fields — no row recreation
                    const el = (id) => document.getElementById(id);

                    const eSpot = el(`td-spot-${prefix}-${order.contract_id}`);
                    if (eSpot) eSpot.textContent = order.current_spot || '-';

                    const eEntry = el(`td-entry-${prefix}-${order.contract_id}`);
                    if (eEntry) eEntry.textContent = order.entry_spot || order.entry_tick || '-';

                    const eProfit = el(`td-profit-${prefix}-${order.contract_id}`);
                    if (eProfit) { eProfit.textContent = `${profitSign}$${Math.abs(profit).toFixed(2)}`; eProfit.style.color = profitColor; }

                    const eMin = el(`td-minprofit-${prefix}-${order.contract_id}`);
                    if (eMin) { eMin.textContent = `${minSign}$${Math.abs(minProfit).toFixed(2)}`; eMin.style.color = minColor; }

                    const eMax = el(`td-maxprofit-${prefix}-${order.contract_id}`);
                    if (eMax) { eMax.textContent = `${maxSign}$${Math.abs(maxProfit).toFixed(2)}`; eMax.style.color = maxColor; }

                    const eRem = el(`td-rem-${prefix}-${order.contract_id}`);
                    if (eRem) eRem.textContent = remainingTime;

                    // Update draw line button only when entry_spot value changes
                    if (entrySpotVal > 0) {
                        const drawBtn = el(`drawbtn_${prefix}_lt_${order.contract_id}`);
                        if (drawBtn) {
                            const newOnclick = `drawPriceLine(${entrySpotVal}, '${order.contract_type}')`;
                            if (drawBtn.getAttribute('onclick') !== newOnclick) {
                                drawBtn.setAttribute('onclick', newOnclick);
                                drawBtn.style.background = 'rgba(74,158,255,0.15)';
                                drawBtn.style.borderColor = 'var(--accent)';
                                drawBtn.title = `วาดเส้น Entry Spot: ${entrySpotVal}`;
                            }
                        }
                    }

                    // Update Sell button state based on is_selling
                    const sellBtn = el(`sell-btn-${prefix}-${order.contract_id}`);
                    if (sellBtn && order.is_selling) {
                        sellBtn.textContent = 'ขายแล้ว';
                        sellBtn.disabled = true;
                        sellBtn.style.background = '#6c757d';
                        sellBtn.style.cursor = 'not-allowed';
                        sellBtn.onclick = null;
                    }
                });
            });

            const totalProfitColor = totalProfit >= 0 ? '#06D6A0' : '#E8304A';
            const totalProfitSign = totalProfit >= 0 ? '+' : '-';
            const summaryHtml = `${ltOpenOrders.length} active orders | P/L: <span style="color:${totalProfitColor};font-weight:bold;">${totalProfitSign}$${Math.abs(totalProfit).toFixed(2)}</span>`;
            if (summaryMain) summaryMain.innerHTML = summaryHtml;
            if (summaryChart) summaryChart.innerHTML = summaryHtml;
            if (orderCount) orderCount.textContent = `(${ltOpenOrders.length})`;
        }

        function ltUpdateTargetProfit(contractId, value) {
            const order = ltOpenOrders.find(o => o.contract_id == contractId);
            if (order) {
                order.target_profit = parseFloat(value) || 0;
                ltAddLog(`🎯 ตั้งค่า Target Profit Contract ${contractId} เป็น $${order.target_profit}`, 'system');
            }
        }

        let ltTradeHistory = [];

        async function ltReloadTradeHistory() {
            const tbody = document.querySelector('#ltHistoryTable tbody');
            if (!tbody) return;

            const today = new Date();
            const d = String(today.getDate()).padStart(2, '0');
            const m = String(today.getMonth() + 1).padStart(2, '0');
            const y = today.getFullYear();
            const dateStr = `${y}-${m}-${d}`;

            try {
                const res = await fetch(`/api/history?date=${dateStr}`);
                const historyData = await res.json();

                // Filter Long Term trades only
                ltTradeHistory = historyData.filter(t => t.tradeStrategy === 'LONGTERM');

                if (ltTradeHistory.length === 0) {
                    tbody.innerHTML = '<tr class="empty-row"><td colspan="11">ยังไม่มีประวัติการเทรด</td></tr>';
                    return;
                }

                // Sort by purchaseTime or sellTime descending
                ltTradeHistory.sort((a, b) => {
                    const timeA = a.sellTime || a.purchaseTime || 0;
                    const timeB = b.sellTime || b.purchaseTime || 0;
                    return timeB - timeA;
                });

                let html = '';
                ltTradeHistory.forEach(order => {
                    const profit = parseFloat(order.ThisProfit || 0);
                    const profitColor = profit >= 0 ? '#06D6A0' : '#E8304A';
                    const profitSign = profit >= 0 ? '+' : '';
                    const winStatus = order.WinStatus || '-';
                    const statusColor = winStatus === 'Win' ? '#06D6A0' : (winStatus === 'Loss' ? '#E8304A' : 'var(--text3)');

                    html += `
                        <tr>
                            <td>${order.purchaseTimeDisplay || '-'}</td>
                            <td style="font-family:monospace;font-size:10px;">${order.contractId || '-'}</td>
                            <td><span class="chip-dot" style="background:${profitColor};"></span>${order.assetCode || '-'}</td>
                            <td style="font-weight:bold;">${order.thisAction || '-'}</td>
                            <td>${order.actualDuration || 0}s</td>
                            <td>${order.entrySpot || '-'}</td>
                            <td>${order.exitSpot || '-'}</td>
                            <td>${order.sellTimeDisplay || '-'}</td>
                            <td style="color:${statusColor};font-weight:bold;">${winStatus}</td>
                            <td style="color:${profitColor};font-weight:bold;">${profitSign}$${Math.abs(profit).toFixed(2)}</td>
                            <td>$${parseFloat(order.MoneyTrade || 0).toFixed(2)}</td>
                        </tr>
                    `;
                });
                tbody.innerHTML = html;
            } catch (e) {
                console.error("Failed to load history from backend:", e);
                tbody.innerHTML = '<tr class="empty-row"><td colspan="11">โหลดประวัติล้มเหลว</td></tr>';
            }
        }

        // Initialize history table on load
        setTimeout(() => ltReloadTradeHistory(), 500);

        document.getElementById('ltLoadHistoryBtn')?.addEventListener('click', () => {
            ltReloadTradeHistory();
            ltAddLog('รีเฟรชประวัติการเทรดสำเร็จ', 'system');
        });

        // Run UI table update every 1 second
        setInterval(ltUpdateOpenOrdersTable, 1000);

        let ltPlayedCloseSound = new Set(); // Prevent playing sound multiple times for same contract

        function ltSellContract(contractId, buttonElement) {
            // ปิดปุ่มทันทีเพื่อป้องกันการกดซ้ำ
            if (buttonElement) {
                buttonElement.disabled = true;
                buttonElement.textContent = 'Selling...';
                buttonElement.style.background = '#6c757d';
                buttonElement.style.cursor = 'not-allowed';
            }

            // อัปเดตปุ่มในทั้ง 2 ตาราง (main + chart)
            ['main', 'chart'].forEach(prefix => {
                const btn = document.getElementById(`sell-btn-${prefix}-${contractId}`);
                if (btn) {
                    btn.disabled = true;
                    btn.textContent = 'Selling...';
                    btn.style.background = '#6c757d';
                    btn.style.cursor = 'not-allowed';
                    btn.onclick = null;
                }
            });

            fetch('/api/longterm/sell', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ contract_id: contractId })
            }).then(() => {
                ltAddLog(`📤 ส่งคำสั่งขาย Contract ${contractId}`, 'system');

                // เปลี่ยนเป็น "ขายแล้ว" หลังส่งคำสั่งสำเร็จ
                ['main', 'chart'].forEach(prefix => {
                    const btn = document.getElementById(`sell-btn-${prefix}-${contractId}`);
                    if (btn) {
                        btn.textContent = 'ขายแล้ว';
                    }
                });
            }).catch(err => {
                console.error('Sell error:', err);
                // ถ้า error ให้เปิดปุ่มกลับ
                ['main', 'chart'].forEach(prefix => {
                    const btn = document.getElementById(`sell-btn-${prefix}-${contractId}`);
                    if (btn) {
                        btn.disabled = false;
                        btn.textContent = 'Sell';
                        btn.style.background = '#E8304A';
                        btn.style.cursor = 'pointer';
                    }
                });
            });
        }

        function ltUpdateTargetProfit(contractId, percentValue, buyPrice) {
            const targetPercent = parseFloat(percentValue) || 0;
            const targetProfitDollar = (buyPrice * targetPercent) / 100;
            fetch('/api/longterm/target', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ contract_id: contractId, target_profit: targetPercent })
            });
            ltAddLog(`🎯 ตั้งค่า Target Profit Contract ${contractId} เป็น ${percentValue}% ($${targetProfitDollar.toFixed(2)})`, 'system');
        }

        function showLtToast(message, type = 'success') {
            const container = document.getElementById('ltToastContainer');
            if (!container) return;
            const toast = document.createElement('div');
            toast.className = `lt-toast ${type}`;
            const icon = type === 'success' ? '✅' : '❌';
            toast.innerHTML = `<span>${icon}</span> <span>${message}</span>`;
            container.appendChild(toast);

            // Trigger animation
            setTimeout(() => toast.classList.add('show'), 10);

            // Remove after 3s
            setTimeout(() => {
                toast.classList.remove('show');
                setTimeout(() => toast.remove(), 300);
            }, 3000);
        }

        let ltPendingOrders = 0;

        function setManualButtonsEnabled(enabled) {
            const btnCall = document.getElementById('btn-manual-call');
            const btnPut = document.getElementById('btn-manual-put');
            const visible = enabled && ltPendingOrders <= 0;
            if (btnCall) {
                btnCall.style.visibility = visible ? 'visible' : 'hidden';
            }
            if (btnPut) {
                btnPut.style.visibility = visible ? 'visible' : 'hidden';
            }
        }

        function manualTrade(contractType) {
            // Check if orders are paused
            if (ltOrdersPaused) {
                ltAddLog(`⏸️ ไม่สามารถเปิด Order ได้ เนื่องจากกำลัง Pause อยู่`, 'system');
                showLtToast('⏸️ Orders ถูก Pause อยู่ กด Resume ก่อน', 'error');
                return;
            }

            const chartAssetSelect = document.getElementById('ltChartAssetSelect');
            const asset = chartAssetSelect.value;

            if (!asset) {
                ltAddLog(`⚠️ กรุณาเลือก Chart Asset ก่อนกดปุ่ม ${contractType}`, 'system');
                return;
            }

            const stakeInput = document.getElementById('ltStakeAmount');
            const amount = parseFloat(stakeInput ? stakeInput.value : 0);
            if (isNaN(amount) || amount < 0.35) {
                ltAddLog(`❌ Stake Amount ไม่ถูกต้อง: $${amount}`, 'system');
                return;
            }

            const targetInput = document.getElementById('ltTargetProfitPercent');
            const targetPercent = parseFloat(targetInput ? targetInput.value : 0) || 0;

            // Get exit strategy
            const exitStrategy = document.querySelector('input[name="ltExitStrategy"]:checked')?.value || 'targetProfit';

            // Get checkbox state
            const checkDuplicate = document.getElementById('ltCheckDuplicateAsset').checked;

            // Hide buttons immediately — will restore when order row appears
            ltPendingOrders++;
            setManualButtonsEnabled(false);

            ltAddLog(`🖐️ Manual ${contractType} | ${asset} | Stake: $${amount} | Target: ${targetPercent}% | Exit: ${exitStrategy}`, 'entry');

            fetch('/api/longterm/manual_trade', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    asset,
                    contract_type: contractType,
                    amount,
                    target_profit: targetPercent,
                    check_duplicate_asset: checkDuplicate,
                    exit_strategy: exitStrategy,
                    entry_signal: ''  // Manual trade ไม่มี entry signal
                })
            });

            // Failsafe: restore after 10s if order never appears
            setTimeout(() => { ltPendingOrders = Math.max(0, ltPendingOrders - 1); setManualButtonsEnabled(true); }, 10000);
        }

        function ltExecuteBuyOrder(asset, contractType) {
            // Check if orders are paused
            if (ltOrdersPaused) {
                ltAddLog(`⏸️ Orders ถูก Pause — ข้ามสัญญาณ ${asset} ${contractType}`, 'system');
                return;
            }

            // Check Max Open Orders limit
            const maxOrders = parseInt(document.getElementById('ltMaxOrders').value) || 1;
            if (ltOpenOrders.length >= maxOrders) {
                ltAddLog(`⚠️ Max Orders ครบ ${maxOrders} แล้ว — ข้ามสัญญาณ ${asset} ${contractType}`, 'system');
                return;
            }

            // Check Duplicate Asset (if checkbox enabled)
            const checkDuplicate = document.getElementById('ltCheckDuplicateAsset').checked;
            if (checkDuplicate) {
                // ตรวจสอบว่ามี order ของ asset นี้เปิดอยู่แล้วหรือไม่
                const existingAsset = ltOpenOrders.find(order => {
                    const orderAsset = order.underlying || order.symbol || order.display_name || '';
                    return orderAsset === asset;
                });

                if (existingAsset) {
                    ltAddLog(`⚠️ Asset ${asset} มี Order เปิดอยู่แล้ว (ID: ${existingAsset.contract_id}) — ข้ามสัญญาณ`, 'system');
                    console.log(`🚫 Duplicate asset blocked: ${asset}`, existingAsset);
                    return;
                }
            }

            // Check Schedule
            const scheduleStatus = document.getElementById('lt-schedule-status');
            if (scheduleStatus && scheduleStatus.textContent.includes('หยุด')) {
                ltAddLog(`⚠️ Schedule หยุดอยู่ — ข้ามสัญญาณ ${asset} ${contractType}`, 'system');
                return;
            }

            const stakeInput = document.getElementById('ltStakeAmount');
            const amount = parseFloat(stakeInput.value);

            if (isNaN(amount) || amount < 0.35) {
                ltAddLog(`❌ Stake Amount ไม่ถูกต้อง: $${amount}`, 'system');
                return;
            }

            const targetInput = document.getElementById('ltTargetProfitPercent');
            const targetPercent = parseFloat(targetInput ? targetInput.value : 0) || 0;
            const targetProfitDollar = (amount * targetPercent) / 100;

            // Get exit strategy
            const exitStrategy = document.querySelector('input[name="ltExitStrategy"]:checked')?.value || 'targetProfit';

            ltAddLog(`🚀 Sending Buy Request: ${asset} | ${contractType} | Stake: $${amount} | Target: ${targetPercent}% | Exit: ${exitStrategy}`, 'entry');

            fetch('/api/longterm/manual_trade', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    asset,
                    contract_type: contractType,
                    amount,
                    target_profit: targetPercent,
                    check_duplicate_asset: checkDuplicate,
                    exit_strategy: exitStrategy,
                    entry_signal: ''  // Will be filled by backend for auto trade
                })
            });
        }
        window.activePriceLines = window.activePriceLines || [];
        function drawPriceLine(price, type) {
            const numPrice = parseFloat(price);
            if (!numPrice || numPrice <= 0) {
                alert('⚠️ ยังไม่มีข้อมูล Entry Spot — รอสักครู่แล้วลองใหม่');
                return;
            }
            if (!window.ltCandleSeries) {
                alert('⚠️ กราฟยังไม่ถูกโหลด — กรุณาเลือก Asset และโหลดข้อมูลก่อน');
                return;
            }
            const color = type === 'CALL' ? '#26a69a' : '#ef5350';
            const priceLine = window.ltCandleSeries.createPriceLine({
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

        document.getElementById('ltAutoTradeToggle').addEventListener('change', async (e) => {
            const isAuto = e.target.checked;
            try {
                const res = await fetch('/api/longterm/toggle_auto', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ enabled: isAuto })
                });
            } catch (err) {
                console.error("Failed to toggle auto trade", err);
            }
        });

        // ── Save/Load Settings ──────────────────────────────────
        async function ltSaveSettings() {
            const settings = {
                // Assets
                assets: Array.from(document.getElementById('ltAssetListBox').options).map(opt => opt.value),

                // Trading Parameters
                stakeAmount: parseFloat(document.getElementById('ltStakeAmount').value) || 1,
                targetProfitPercent: parseFloat(document.getElementById('ltTargetProfitPercent').value) || 10,
                duration: parseInt(document.getElementById('ltDurationSelect').value) || 3600,

                // Exit Strategy
                exitStrategy: document.querySelector('input[name="ltExitStrategy"]:checked')?.value || 'targetProfit',

                // Entry Conditions
                conditions: {
                    condShortMedium: document.getElementById('condShortMedium')?.checked || false,
                    condLongCross: document.getElementById('condLongCross')?.checked || false,
                    condShortLong: document.getElementById('condShortLong')?.checked || false
                },

                // Options
                checkDuplicateAsset: document.getElementById('ltCheckDuplicateAsset')?.checked || true,
                maxOrders: parseInt(document.getElementById('ltMaxOrders')?.value) || 1,
                autoTrade: document.getElementById('ltAutoTradeToggle')?.checked || false,

                // Schedule
                useSchedule: document.getElementById('ltUseSchedule')?.checked || false,
                startTime: document.getElementById('ltStartTime')?.value || '',
                stopTime: document.getElementById('ltStopTime')?.value || '',

                // Sound
                playEmaCutSound: document.getElementById('playEmaCutSoundToggle')?.checked || true,
                playCloseOrderSound: document.getElementById('playCloseOrderSoundToggle')?.checked || true,

                // Chart
                chartAsset: document.getElementById('ltChartAssetSelect')?.value || '',

                savedAt: new Date().toISOString()
            };

            try {
                const res = await fetch('/api/longterm/settings', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(settings)
                });

                if (res.ok) {
                    showLtToast('✅ บันทึก Settings สำเร็จ', 'success');
                    ltAddLog('💾 บันทึก Settings สำเร็จ', 'system');
                } else {
                    showLtToast('❌ บันทึก Settings ล้มเหลว', 'error');
                }
            } catch (err) {
                console.error('Save settings error:', err);
                showLtToast('❌ บันทึก Settings ล้มเหลว', 'error');
            }
        }

        async function ltLoadSettings() {
            try {
                const res = await fetch('/api/longterm/settings');
                if (!res.ok) {
                    console.log('No saved settings found');
                    return;
                }

                const settings = await res.json();
                console.log('📥 Loading settings:', settings);

                // Assets
                if (settings.assets && settings.assets.length > 0) {
                    const listBox = document.getElementById('ltAssetListBox');
                    listBox.innerHTML = '';
                    settings.assets.forEach(asset => {
                        const opt = document.createElement('option');
                        opt.value = asset;
                        opt.textContent = asset;
                        listBox.appendChild(opt);
                    });
                }

                // Trading Parameters
                if (settings.stakeAmount) document.getElementById('ltStakeAmount').value = settings.stakeAmount;
                if (settings.targetProfitPercent) document.getElementById('ltTargetProfitPercent').value = settings.targetProfitPercent;
                if (settings.duration) document.getElementById('ltDurationSelect').value = settings.duration;

                // Exit Strategy
                if (settings.exitStrategy) {
                    const radio = document.querySelector(`input[name="ltExitStrategy"][value="${settings.exitStrategy}"]`);
                    if (radio) {
                        radio.checked = true;
                        radio.dispatchEvent(new Event('change'));
                    }
                }

                // Entry Conditions
                if (settings.conditions) {
                    if (settings.conditions.condShortMedium !== undefined)
                        document.getElementById('ltCondShortMedium').checked = settings.conditions.condShortMedium;
                    if (settings.conditions.condLongCross !== undefined)
                        document.getElementById('ltCondLongCross').checked = settings.conditions.condLongCross;
                    if (settings.conditions.condShortLong !== undefined)
                        document.getElementById('ltCondShortLong').checked = settings.conditions.condShortLong;
                }

                // Options
                if (settings.checkDuplicateAsset !== undefined)
                    document.getElementById('ltCheckDuplicateAsset').checked = settings.checkDuplicateAsset;
                if (settings.maxOrders) document.getElementById('ltMaxOrders').value = settings.maxOrders;
                if (settings.autoTrade !== undefined)
                    document.getElementById('ltAutoTradeToggle').checked = settings.autoTrade;

                // Load Schedule and Trading Setup from /api/longterm/schedule
                try {
                    const schRes = await fetch('/api/longterm/schedule');
                    if (schRes.ok) {
                        const schData = await schRes.json();
                        if (schData.status === 'success' && schData.data) {
                            const d = schData.data;
                            
                            if (d.amount) document.getElementById('ltStakeAmount').value = d.amount;
                            if (d.target_profit) document.getElementById('ltTargetProfitPercent').value = d.target_profit;
                            if (d.duration) document.getElementById('ltDurationSelect').value = d.duration;
                            if (d.max_orders) document.getElementById('ltMaxOrders').value = d.max_orders;
                            
                            if (d.exit_strategy) {
                                const radio = document.querySelector(`input[name="ltExitStrategy"][value="${d.exit_strategy}"]`);
                                if (radio) {
                                    radio.checked = true;
                                    radio.closest('.exit-card')?.classList.add('selected');
                                }
                            }
                            
                            if (d.conditions) {
                                document.getElementById('ltCondShortMedium').checked = d.conditions.includes('condShortMedium');
                                document.getElementById('ltCondLongCross').checked = d.conditions.includes('condLongCross');
                                document.getElementById('ltCondShortLong').checked = d.conditions.includes('condShortLong');
                                document.querySelectorAll('.condition-card input[type="checkbox"]').forEach(cb => {
                                    cb.closest('.condition-card').classList.toggle('active-condition', cb.checked);
                                });
                            }

                            if (d.use_schedule !== undefined) {
                                const useSchCheckbox = document.getElementById('ltUseSchedule');
                                useSchCheckbox.checked = d.use_schedule;
                                if (d.use_schedule) {
                                    useSchCheckbox.dispatchEvent(new Event('change'));
                                }
                            }
                            if (d.start_time) {
                                document.getElementById('ltStartTime').value = d.start_time.replace(' ', 'T').slice(0, 16);
                            }
                            if (d.stop_time) {
                                document.getElementById('ltStopTime').value = d.stop_time.replace(' ', 'T').slice(0, 16);
                            }
                        }
                    }
                } catch (schErr) {
                    console.error('Load schedule error:', schErr);
                }

                // Sound
                if (settings.playEmaCutSound !== undefined)
                    document.getElementById('playEmaCutSoundToggle').checked = settings.playEmaCutSound;
                if (settings.playCloseOrderSound !== undefined)
                    document.getElementById('playCloseOrderSoundToggle').checked = settings.playCloseOrderSound;

                // Chart
                if (settings.chartAsset) {
                    document.getElementById('ltChartAssetSelect').value = settings.chartAsset;
                }

                ltAddLog(`📥 โหลด Settings สำเร็จ (บันทึกเมื่อ: ${settings.savedAt ? new Date(settings.savedAt).toLocaleString('th-TH') : '-'})`, 'system');
                showLtToast('✅ โหลด Settings สำเร็จ', 'success');
            } catch (err) {
                console.error('Load settings error:', err);
            }
        }

        // Settings Button Event (use setTimeout to ensure DOM is ready)
        setTimeout(() => {
            document.getElementById('ltOpenSettingsBtn')?.addEventListener('click', () => {
                document.getElementById('ltSettingsModal').classList.add('active');
                ltLoadSettingsToModal();
            });

            // Modal Close Events
            document.getElementById('ltCloseModalBtn')?.addEventListener('click', () => {
                document.getElementById('ltSettingsModal').classList.remove('active');
            });

            document.getElementById('lt-cancelBtn')?.addEventListener('click', () => {
                document.getElementById('ltSettingsModal').classList.remove('active');
            });

            // Close modal when clicking backdrop
            document.getElementById('ltSettingsModal')?.addEventListener('click', (e) => {
                if (e.target.id === 'ltSettingsModal') {
                    document.getElementById('ltSettingsModal').classList.remove('active');
                }
            });

            // Save Settings from Modal
            document.getElementById('lt-saveBtn')?.addEventListener('click', () => {
                ltSaveSettingsFromModal();
                document.getElementById('ltSettingsModal').classList.remove('active');
            });

            // Load Default
            document.getElementById('lt-loadDefaultBtn')?.addEventListener('click', () => {
                if (confirm('โหลดค่า Default?\n\nค่าปัจจุบันจะถูกแทนที่')) {
                    ltLoadDefaultSettings();
                }
            });
        }, 100);

        function ltLoadSettingsToModal() {
            // Load current values from longterm_settings.json (via fetch)
            fetch('/api/longterm/settings').then(res => res.json()).then(data => {
                // Check if we got valid settings
                if (data.status === 'error') {
                    console.log('No longterm settings found, using defaults');
                    return;
                }

                // Assets - load from current list box
                const listBox = document.getElementById('ltAssetListBox');
                const currentAssets = Array.from(listBox.options).map(opt => opt.value);
                document.querySelectorAll('.lt-asset-check').forEach(cb => {
                    cb.checked = currentAssets.includes(cb.value);
                });

                // If settings has assets field, use that instead
                if (data.assets && Array.isArray(data.assets)) {
                    document.querySelectorAll('.lt-asset-check').forEach(cb => {
                        cb.checked = data.assets.includes(cb.value);
                    });
                }

                // Indicators
                if (data.indicators) {
                    document.getElementById('lt-atrPeriod').value = data.indicators.atrPeriod || 7;
                    document.getElementById('lt-atrMulti').value = data.indicators.atrMulti || 1.3;
                    document.getElementById('lt-ciPeriod').value = data.indicators.ciPeriod || 20;
                    document.getElementById('lt-adxPeriod').value = data.indicators.adxPeriod || 14;
                    document.getElementById('lt-bbPeriod').value = data.indicators.bbPeriod || 20;
                    document.getElementById('lt-smcPeriod').value = data.indicators.smcPeriod || 14;
                    document.getElementById('lt-rsiPeriod').value = data.indicators.rsiPeriod || 14;
                    document.getElementById('lt-stochPeriod').value = data.indicators.stochPeriod || 14;
                }

                // EMA
                if (data.ema) {
                    document.getElementById('lt-emaShortPeriod').value = data.ema.shortPeriod || 9;
                    document.getElementById('lt-emaShortType').value = data.ema.shortType || 'ema';
                    document.getElementById('lt-emaMediumPeriod').value = data.ema.mediumPeriod || 21;
                    document.getElementById('lt-emaMediumType').value = data.ema.mediumType || 'ema';
                    document.getElementById('lt-emaLongPeriod').value = data.ema.longPeriod || 50;
                    document.getElementById('lt-emaLongType').value = data.ema.longType || 'ema';
                }
            }).catch(err => console.error('Load longterm settings error:', err));
        }

        function ltSaveSettingsFromModal() {
            // Collect selected assets
            const selectedAssets = Array.from(document.querySelectorAll('.lt-asset-check:checked')).map(cb => cb.value);

            // Update asset list box
            const listBox = document.getElementById('ltAssetListBox');
            listBox.innerHTML = '';
            selectedAssets.forEach(asset => {
                const opt = document.createElement('option');
                opt.value = asset;
                opt.textContent = asset;
                listBox.appendChild(opt);
            });

            // Build settings object
            const settings = {
                indicators: {
                    atrPeriod: parseInt(document.getElementById('lt-atrPeriod').value) || 7,
                    atrMulti: parseFloat(document.getElementById('lt-atrMulti').value) || 1.3,
                    ciPeriod: parseInt(document.getElementById('lt-ciPeriod').value) || 20,
                    adxPeriod: parseInt(document.getElementById('lt-adxPeriod').value) || 14,
                    bbPeriod: parseInt(document.getElementById('lt-bbPeriod').value) || 20,
                    smcPeriod: parseInt(document.getElementById('lt-smcPeriod').value) || 14,
                    rsiPeriod: parseInt(document.getElementById('lt-rsiPeriod').value) || 14,
                    stochPeriod: parseInt(document.getElementById('lt-stochPeriod').value) || 14
                },
                ema: {
                    shortPeriod: parseInt(document.getElementById('lt-emaShortPeriod').value) || 9,
                    shortType: document.getElementById('lt-emaShortType').value || 'ema',
                    mediumPeriod: parseInt(document.getElementById('lt-emaMediumPeriod').value) || 21,
                    mediumType: document.getElementById('lt-emaMediumType').value || 'ema',
                    longPeriod: parseInt(document.getElementById('lt-emaLongPeriod').value) || 50,
                    longType: document.getElementById('lt-emaLongType').value || 'ema'
                },
                assets: selectedAssets
            };

            // Save to backend longterm_settings.json
            fetch('/api/longterm/settings', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(settings)
            }).then(res => res.json()).then(() => {
                showLtToast('✅ บันทึก Settings สำเร็จ', 'success');
                ltAddLog('💾 บันทึก Settings และ Assets สำเร็จ', 'system');
                // Reload settings into UI
                ltLoadSettings();
            }).catch(err => {
                console.error('Save error:', err);
                showLtToast('❌ บันทึก Settings ล้มเหลว', 'error');
            });
        }

        function ltLoadDefaultSettings() {
            // Reset to default values
            document.getElementById('lt-atrPeriod').value = 7;
            document.getElementById('lt-atrMulti').value = 1.3;
            document.getElementById('lt-ciPeriod').value = 20;
            document.getElementById('lt-adxPeriod').value = 14;
            document.getElementById('lt-bbPeriod').value = 20;
            document.getElementById('lt-smcPeriod').value = 14;
            document.getElementById('lt-rsiPeriod').value = 14;
            document.getElementById('lt-stochPeriod').value = 14;

            document.getElementById('lt-emaShortPeriod').value = 9;
            document.getElementById('lt-emaShortType').value = 'ema';
            document.getElementById('lt-emaMediumPeriod').value = 21;
            document.getElementById('lt-emaMediumType').value = 'ema';
            document.getElementById('lt-emaLongPeriod').value = 50;
            document.getElementById('lt-emaLongType').value = 'ema';

            showLtToast('✅ โหลดค่า Default สำเร็จ', 'success');
        }

        // Auto-load settings on page load
        setTimeout(() => {
            ltLoadSettings();
        }, 1000);

        console.log('✅ Long Term Trade UI loaded successfully');
    
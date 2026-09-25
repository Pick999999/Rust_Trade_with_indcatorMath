# -*- coding: utf-8 -*-
import re

path = r'd:\Rust\turbo-indicators\turbo-indicators_v2\indicators_Multiplex_Ver1\public\index.html'
with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. HTML inputs
html_target = '''                    <label class="toggle-switch" style="margin-left: 8px;">
                        <input type="checkbox" id="showStrategyTooltipToggle">
                        <span class="toggle-slider"></span>
                    </label>
                    <span class="chart-toolbar-label" style="margin-left: 6px;">Strategy Tooltip</span>
                </div>'''

html_replacement = '''                    <label class="toggle-switch" style="margin-left: 8px;">
                        <input type="checkbox" id="showStrategyTooltipToggle">
                        <span class="toggle-slider"></span>
                    </label>
                    <span class="chart-toolbar-label" style="margin-left: 6px;">Strategy Tooltip</span>
                    
                    <div class="sep" style="height:16px; margin-left:8px;"></div>
                    <label class="toggle-switch" style="margin-left: 8px;">
                        <input type="checkbox" id="shadeAdxChoppyToggle" onchange="if(window.allAnalysisData && window.allAnalysisData.length > 0) renderChart(window.allAnalysisData)">
                        <span class="toggle-slider"></span>
                    </label>
                    <span class="chart-toolbar-label" style="margin-left: 6px;">Shade ADX/Choppy</span>
                    <span class="chart-toolbar-label" style="margin-left: 8px;">ADX ></span>
                    <input type="number" id="shadeAdxVal" value="25" style="width:50px; padding:2px; font-size:11px; background:var(--bg2); color:var(--text); border:1px solid var(--border); border-radius:4px; text-align:center;" title="ADX Threshold" oninput="if(window.allAnalysisData && window.allAnalysisData.length > 0) renderChart(window.allAnalysisData)">
                    <span class="chart-toolbar-label" style="margin-left: 8px;">Choppy <</span>
                    <input type="number" id="shadeChoppyVal" value="38" style="width:50px; padding:2px; font-size:11px; background:var(--bg2); color:var(--text); border:1px solid var(--border); border-radius:4px; text-align:center;" title="Choppy Threshold" oninput="if(window.allAnalysisData && window.allAnalysisData.length > 0) renderChart(window.allAnalysisData)">
                </div>'''

if html_target in content:
    content = content.replace(html_target, html_replacement)
else:
    print("HTML Target not found")

# 2. Global vars
vars_target = '''        let emaShortSeries = null;
        let emaMediumSeries = null;
        let emaLongSeries = null;'''

vars_replacement = '''        let emaShortSeries = null;
        let emaMediumSeries = null;
        let emaLongSeries = null;
        let shadeSeries = null;'''

if vars_target in content:
    content = content.replace(vars_target, vars_replacement)
else:
    print("Vars Target not found")

# 3. Destroy logic
destroy_target = '''                emaShortSeries = null;
                emaMediumSeries = null;
                emaLongSeries = null;
            }'''

destroy_replacement = '''                emaShortSeries = null;
                emaMediumSeries = null;
                emaLongSeries = null;
                shadeSeries = null;
            }'''

if destroy_target in content:
    content = content.replace(destroy_target, destroy_replacement)
else:
    print("Destroy Target not found")

# 4. Create series
create_target = '''                candleSeries = lwChart.addCandlestickSeries({
                    upColor: '#06D6A0', downColor: '#E8304A',
                    borderVisible: true,
                    wickUpColor: '#06D6A0', wickDownColor: '#E8304A',
                    borderColor: '#000000', borderUpColor: '#06D6A0', borderDownColor: '#E8304A'
                });'''

create_replacement = '''                candleSeries = lwChart.addCandlestickSeries({
                    upColor: '#06D6A0', downColor: '#E8304A',
                    borderVisible: true,
                    wickUpColor: '#06D6A0', wickDownColor: '#E8304A',
                    borderColor: '#000000', borderUpColor: '#06D6A0', borderDownColor: '#E8304A'
                });

                shadeSeries = lwChart.addHistogramSeries({
                    color: 'rgba(255, 193, 7, 0.15)',
                    priceFormat: { type: 'volume' },
                    priceScaleId: '',
                    scaleMargins: { top: 0, bottom: 0 },
                });'''

if create_target in content:
    content = content.replace(create_target, create_replacement)
else:
    print("Create Target not found")

# 5. Render Chart logic
render_target = '''            candleSeries.setData(chartData);

            // เตรียมข้อมูล EMA'''

render_replacement = '''            candleSeries.setData(chartData);

            // คำนวณและวาดแรเงา ADX & Choppy
            const showShade = document.getElementById('shadeAdxChoppyToggle') ? document.getElementById('shadeAdxChoppyToggle').checked : false;
            const shadeAdxVal = document.getElementById('shadeAdxVal') ? parseFloat(document.getElementById('shadeAdxVal').value) : 25;
            const shadeChoppyVal = document.getElementById('shadeChoppyVal') ? parseFloat(document.getElementById('shadeChoppyVal').value) : 38;
            
            if (shadeSeries) {
                if (showShade) {
                    const shadeData = analysisList.map(d => {
                        const actualAdx = (d.adx !== undefined) ? d.adx : 0;
                        const actualChoppy = (d.choppiNessIndex !== undefined) ? d.choppiNessIndex : (d.choppiness_index !== undefined ? d.choppiness_index : 0);

                        const meetsCondition = (actualAdx > shadeAdxVal && actualChoppy > 0 && actualChoppy < shadeChoppyVal);
                        
                        return { 
                            time: toThaiTime(d.epoch), 
                            value: meetsCondition ? 1 : 0, 
                            color: meetsCondition ? 'rgba(255, 193, 7, 0.15)' : 'transparent' 
                        };
                    });
                    
                    const uniqueShadeData = shadeData.filter((v, i, a) => a.findIndex(t => t.time === v.time) === i).sort((a,b) => a.time - b.time);
                    shadeSeries.setData(uniqueShadeData);
                } else {
                    shadeSeries.setData([]);
                }
            }

            // เตรียมข้อมูล EMA'''

if render_target in content:
    content = content.replace(render_target, render_replacement)
else:
    print("Render Target not found")

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
print("Done")

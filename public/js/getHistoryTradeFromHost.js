/**
 * Get History Trade Data from Host API
 * 
 * Features:
 * - Download ZIP file from API
 * - Unzip in browser using JSZip
 * - Parse JSON files
 * - Return structured data object
 * 
 * Dependencies:
 * - JSZip library (https://cdnjs.cloudflare.com/ajax/libs/jszip/3.10.1/jszip.min.js)
 */

/**
 * ดึงข้อมูล trade history จาก API และ unzip
 * @param {string} startdatetime - วันที่ในรูปแบบ YYYY-MM-DD (เช่น "2026-07-29")
 * @returns {Promise<Object>} - Object ที่มี structure: { assetCode: { trades: [...], trackOrders: [...] } }
 */
async function getHistoryTradeFromHost(startdatetime) {
    console.log(`📦 [Export] Fetching trade data for ${startdatetime}...`);
    
    try {
        // 1. ดาวน์โหลด ZIP file จาก API
        const response = await fetch(`/api/export_trade_data?startdatetime=${startdatetime}`);
        
        if (!response.ok) {
            const errorText = await response.text();
            throw new Error(`API error (${response.status}): ${errorText}`);
        }
        
        console.log(`✅ [Export] ZIP file downloaded successfully`);
        
        // 2. อ่าน response เป็น ArrayBuffer
        const arrayBuffer = await response.arrayBuffer();
        
        // 3. Unzip ด้วย JSZip
        const zip = await JSZip.loadAsync(arrayBuffer);
        console.log(`📂 [Export] ZIP extracted, files:`, Object.keys(zip.files).length);
        
        // 4. อ่านไฟล์ทั้งหมดและจัดโครงสร้าง
        const tradeData = {};
        let tradeHead = null;
        
        for (const [filename, zipEntry] of Object.entries(zip.files)) {
            // ข้าม directories
            if (zipEntry.dir) {
                continue;
            }
            
            console.log(`📄 [Export] Processing: ${filename}`);
            
            // อ่านเนื้อหาไฟล์
            const content = await zipEntry.async('text');
            
            // Parse JSON
            let jsonData;
            try {
                jsonData = JSON.parse(content);
            } catch (parseError) {
                console.error(`❌ [Export] Failed to parse JSON for ${filename}:`, parseError);
                continue;
            }

            const cleanPath = filename.replace(/^\.\//, '');
            const pathParts = cleanPath.split('/');
            
            if (pathParts.length === 1) {
                // ไฟล์ที่อยู่ root ของวัน เช่น tradeHead.json, strategy_comparison.json
                const jsonFilename = pathParts[0];
                if (jsonFilename === 'tradeHead.json') {
                    tradeHead = jsonData;
                    console.log(`✅ [Export] Root tradeHead.json: loaded`, Array.isArray(jsonData) ? `(${jsonData.length} rounds)` : '');
                } else {
                    console.log(`ℹ️ [Export] Root file: ${jsonFilename}`);
                }
            } else if (pathParts.length === 2) {
                const [assetCode, jsonFilename] = pathParts;
                
                // สร้าง object สำหรับ asset นี้ถ้ายังไม่มี
                if (!tradeData[assetCode]) {
                    tradeData[assetCode] = {
                        assetCode: assetCode,
                        trades: null,
                        trackOrders: null,
                        tradeHead: null
                    };
                }
                
                // กำหนดข้อมูลตามชื่อไฟล์
                if (jsonFilename === 'trades.json') {
                    tradeData[assetCode].trades = jsonData;
                    console.log(`✅ [Export] ${assetCode}/trades.json: ${jsonData.length} records`);
                } else if (jsonFilename === 'track_orders.json') {
                    tradeData[assetCode].trackOrders = jsonData;
                    console.log(`✅ [Export] ${assetCode}/track_orders.json: ${jsonData.length} records`);
                } else if (jsonFilename === 'tradeHead.json') {
                    tradeData[assetCode].tradeHead = jsonData;
                    if (!tradeHead) tradeHead = jsonData;
                    console.log(`✅ [Export] ${assetCode}/tradeHead.json loaded`);
                } else {
                    console.warn(`⚠️ [Export] Unknown file type: ${jsonFilename}`);
                }
            } else {
                console.warn(`⚠️ [Export] Skipping unexpected path: ${filename}`);
            }
        }
        
        // 5. สรุปผลลัพธ์
        const assetCount = Object.keys(tradeData).length;
        let totalTrades = 0;
        let totalTrackOrders = 0;
        
        for (const asset of Object.values(tradeData)) {
            if (asset.trades) totalTrades += asset.trades.length;
            if (asset.trackOrders) totalTrackOrders += asset.trackOrders.length;
        }
        
        console.log(`🎉 [Export] Success!`);
        console.log(`   - Assets: ${assetCount}`);
        console.log(`   - Total Trades: ${totalTrades}`);
        console.log(`   - Total Track Orders: ${totalTrackOrders}`);
        if (tradeHead) {
            console.log(`   - Trade Head: Loaded`);
        }
        
        return {
            success: true,
            date: startdatetime,
            assetCount: assetCount,
            totalTrades: totalTrades,
            totalTrackOrders: totalTrackOrders,
            tradeHead: tradeHead,
            data: tradeData
        };
        
    } catch (error) {
        console.error(`❌ [Export] Error:`, error);
        return {
            success: false,
            error: error.message,
            date: startdatetime,
            data: null
        };
    }
}

/**
 * ดึงข้อมูล trade history สำหรับหลายวัน
 * @param {string[]} dates - Array ของวันที่ (เช่น ["2026-07-29", "2026-07-30"])
 * @returns {Promise<Object[]>} - Array ของ results
 */
async function getHistoryTradeMultipleDates(dates) {
    console.log(`📦 [Export] Fetching trade data for ${dates.length} dates...`);
    
    const results = [];
    
    for (const date of dates) {
        const result = await getHistoryTradeFromHost(date);
        results.push(result);
        
        // รอ 500ms ระหว่างการ request เพื่อไม่ให้ spam server
        await new Promise(resolve => setTimeout(resolve, 500));
    }
    
    console.log(`✅ [Export] All dates processed`);
    return results;
}

/**
 * ดึง trades จาก asset เฉพาะจากวันที่ระบุ
 * @param {string} startdatetime - วันที่ (YYYY-MM-DD)
 * @param {string} assetCode - Asset code (เช่น "1HZ10V")
 * @returns {Promise<Object>} - { trades: [...], trackOrders: [...] }
 */
async function getTradesByAsset(startdatetime, assetCode) {
    console.log(`📦 [Export] Fetching ${assetCode} data for ${startdatetime}...`);
    
    const result = await getHistoryTradeFromHost(startdatetime);
    
    if (!result.success) {
        console.error(`❌ [Export] Failed to fetch data:`, result.error);
        return null;
    }
    
    if (!result.data[assetCode]) {
        console.warn(`⚠️ [Export] Asset ${assetCode} not found in data`);
        return null;
    }
    
    return result.data[assetCode];
}

/**
 * วิเคราะห์ข้อมูล trade และสร้าง summary
 * @param {Object} tradeData - ข้อมูลจาก getHistoryTradeFromHost()
 * @returns {Object} - Summary statistics
 */
function analyzeTradeData(tradeData) {
    if (!tradeData.success || !tradeData.data) {
        return null;
    }
    
    const summary = {
        date: tradeData.date,
        assets: [],
        totalProfit: 0,
        totalWins: 0,
        totalLosses: 0,
        totalTrades: 0,
        winRate: 0
    };
    
    for (const [assetCode, assetData] of Object.entries(tradeData.data)) {
        if (!assetData.trades || assetData.trades.length === 0) {
            continue;
        }
        
        let assetProfit = 0;
        let assetWins = 0;
        let assetLosses = 0;
        
        for (const trade of assetData.trades) {
            const profit = parseFloat(trade.ThisProfit || 0);
            assetProfit += profit;
            summary.totalProfit += profit;
            summary.totalTrades++;
            
            if (profit >= 0) {
                assetWins++;
                summary.totalWins++;
            } else {
                assetLosses++;
                summary.totalLosses++;
            }
        }
        
        summary.assets.push({
            assetCode: assetCode,
            trades: assetData.trades.length,
            trackOrders: assetData.trackOrders ? assetData.trackOrders.length : 0,
            profit: assetProfit,
            wins: assetWins,
            losses: assetLosses,
            winRate: assetData.trades.length > 0 ? (assetWins / assetData.trades.length * 100).toFixed(2) : 0
        });
    }
    
    summary.winRate = summary.totalTrades > 0 
        ? (summary.totalWins / summary.totalTrades * 100).toFixed(2) 
        : 0;
    
    return summary;
}

/**
 * ดาวน์โหลด ZIP file แทนการ unzip (สำหรับ backup)
 * @param {string} startdatetime - วันที่ (YYYY-MM-DD)
 */
async function downloadTradeDataZip(startdatetime) {
    console.log(`💾 [Export] Downloading ZIP for ${startdatetime}...`);
    
    try {
        const response = await fetch(`/api/export_trade_data?startdatetime=${startdatetime}`);
        
        if (!response.ok) {
            const errorText = await response.text();
            throw new Error(`API error (${response.status}): ${errorText}`);
        }
        
        const blob = await response.blob();
        const url = window.URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `trade_data_${startdatetime}.zip`;
        document.body.appendChild(a);
        a.click();
        
        window.URL.revokeObjectURL(url);
        document.body.removeChild(a);
        
        console.log(`✅ [Export] ZIP downloaded: trade_data_${startdatetime}.zip`);
    } catch (error) {
        console.error(`❌ [Export] Download failed:`, error);
        alert(`ดาวน์โหลดล้มเหลว: ${error.message}`);
    }
}

// Export functions for use in other scripts
if (typeof window !== 'undefined') {
    window.getHistoryTradeFromHost = getHistoryTradeFromHost;
    window.getHistoryTradeMultipleDates = getHistoryTradeMultipleDates;
    window.getTradesByAsset = getTradesByAsset;
    window.analyzeTradeData = analyzeTradeData;
    window.downloadTradeDataZip = downloadTradeDataZip;
}

// ============================================
// EXAMPLE USAGE
// ============================================

/*

// Example 1: ดึงข้อมูลวันเดียว
const result = await getHistoryTradeFromHost('2026-07-29');
console.log(result);
// Output:
// {
//   success: true,
//   date: "2026-07-29",
//   assetCount: 4,
//   totalTrades: 15,
//   totalTrackOrders: 150,
//   data: {
//     "1HZ10V": {
//       assetCode: "1HZ10V",
//       trades: [...],
//       trackOrders: [...]
//     },
//     "1HZ25V": { ... },
//     ...
//   }
// }

// Example 2: เข้าถึงข้อมูล trade ของ asset เฉพาะ
const vol10Data = result.data["1HZ10V"];
console.log(vol10Data.trades);      // Array of trade records
console.log(vol10Data.trackOrders); // Array of track order snapshots

// Example 3: ดึงข้อมูลหลายวัน
const results = await getHistoryTradeMultipleDates([
    '2026-07-27',
    '2026-07-28',
    '2026-07-29'
]);

// Example 4: ดึงข้อมูล asset เฉพาะ
const vol25Data = await getTradesByAsset('2026-07-29', '1HZ25V');
console.log(vol25Data.trades);

// Example 5: วิเคราะห์ข้อมูล
const summary = analyzeTradeData(result);
console.log(summary);
// Output:
// {
//   date: "2026-07-29",
//   assets: [
//     { assetCode: "1HZ10V", trades: 5, profit: 2.5, wins: 3, losses: 2, winRate: "60.00" },
//     ...
//   ],
//   totalProfit: 10.5,
//   totalWins: 12,
//   totalLosses: 3,
//   totalTrades: 15,
//   winRate: "80.00"
// }

// Example 6: ดาวน์โหลด ZIP แทนการ unzip
await downloadTradeDataZip('2026-07-29');

*/

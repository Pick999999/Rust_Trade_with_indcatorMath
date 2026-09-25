<?php

class ATRCandleAnalyzer {
    
    private $numFound = 0;
    private $grandBalance = 0;
    private $numTradeArray = array();
    private $AllArray = array();
    private $tradeNo = 1;
    private $martinGaleMoney = array();
    private $strategyType = 2; // 1 = Red->Green, Green->Red | 2 = Red->Green, Green->Green
    
    public function __construct($martinGaleStakes = null, $strategyType = 2) {
        // Default martingale stakes
        $this->martinGaleMoney = $martinGaleStakes ?? array(1, 2, 6, 18, 17, 36, 70, 140, 280, 500);
        $this->numTradeArray = array_fill(0, 14, 0);
        $this->strategyType = $strategyType;
    }
    
    /**
     * โหลดข้อมูลจาก JSON file
     */
    public function loadDataFromFile($fileName = 'data.json') {
        if (!file_exists($fileName)) {
            throw new Exception("File not found: {$fileName}");
        }
        
        $jsonContent = file_get_contents($fileName);
        $data = json_decode($jsonContent, true);
        
        if ($data === null) {
            throw new Exception("Failed to parse JSON: " . json_last_error_msg());
        }
        
        return $data;
    }
    
    /**
     * โหลดข้อมูลจาก JSON string
     */
    public function loadDataFromString($jsonString) {
        $data = json_decode($jsonString, true);
        
        if ($data === null) {
            throw new Exception("Failed to parse JSON: " . json_last_error_msg());
        }
        
        return $data;
    }
    
    /**
     * วิเคราะห์ข้อมูลหลัก
     */
    public function analyze($candleData) {
        echo "<h1>จำนวนข้อมูลทั้งสิ้น = " . count($candleData) . '</h1>';
        
        for ($i = 0; $i < count($candleData); $i++) {
            // ค้นหาแท่ง ATR (Spike)
            if (isset($candleData[$i]['isAtr']) && $candleData[$i]['isAtr'] === true) {
                $thisColor = $candleData[$i]['color'];
                list($targetColor, $targetColor2) = $this->getSuggestColor($thisColor, 0);
                
                $parentColor = $thisColor;
                $subBalance = 0;
                $candleIndex = $i;
                $numTrade = 0;
                
                $labTradeArray = $this->findWinOnColor(
                    $candleData, 
                    $candleIndex, 
                    $parentColor, 
                    $targetColor, 
                    $subBalance, 
                    $numTrade, 
                    $this->tradeNo
                );
                
                $this->tradeNo++;
                $this->AllArray[] = $labTradeArray;
                $this->numTradeArray[$numTrade - 1]++;
                $this->grandBalance += $subBalance;
                $this->numFound++;
            }
        }
        
        return $this->AllArray;
    }
    
    /**
     * หาการชนะตามสีที่กำหนด
     */
    private function findWinOnColor(&$candleData, &$thisIndex, $parentColor, $targetColor, &$balance, &$numTrade, $tradeNo) {
        $numTrade = 0;
        $lossCon = 0;
        $balance = 0;
        $sArray = array();
        
        for ($i = $thisIndex + 1; $i < count($candleData); $i++) {
            $moneyTrade = $this->martinGaleMoney[$lossCon] * 1;
            $thisColor = $candleData[$i]['color'];
            
            // ถ้าแพ้ติดต่อกัน >= 3 ให้เปลี่ยนกลยุทธ์
            if ($lossCon >= 3) {
                list($targetColor, $targetColor2) = $this->getSuggestColor($thisColor, $lossCon);
            }
            
            // คำนวณผล Win/Loss
            if ($thisColor === $targetColor) {
                $winStatus = 'Win';
                $thisProfit = $moneyTrade * 0.94; // 94% payout
            } else {
                $winStatus = 'Loss';
                $thisProfit = $moneyTrade * -1;
                $lossCon++;
            }
            
            $numTrade++;
            $balance += $thisProfit;
            $candleData[$i]['winStatus'] = $winStatus;
            
            // สร้างข้อมูลการเทรด
            $tradeRecord = new stdClass();
            $tradeRecord->tradeNo = $tradeNo;
            $tradeRecord->i = $i;
            $tradeRecord->timeDisplay = $candleData[$i]['timeDisplay'] ?? '';
            $tradeRecord->color = $candleData[$i]['color'];
            $tradeRecord->targetColor = $targetColor;
            $tradeRecord->nextColor = $candleData[$i]['nextColor'] ?? '';
            $tradeRecord->emaShortDirection = $candleData[$i]['emaShortDirection'] ?? '';
            $tradeRecord->emaMediumDirection = $candleData[$i]['emaMeduimDirection'] ?? '';
            $tradeRecord->WinStatus = $winStatus;
			$tradeRecord->thisProfit = $thisProfit;
			$tradeRecord->balance = $balance ;
            $tradeRecord->LossCon = $lossCon;
            
            if ($lossCon >= 3) {
                $tradeRecord->ShouldFix = 'y';
                $tradeRecord->ChangeTargetTo = $thisColor;
            } else {
                $tradeRecord->ShouldFix = 'n';
                $tradeRecord->ChangeTargetTo = '';
            }
            
            $sArray[] = $tradeRecord;
            
            // ถ้าชนะให้หยุด
            if ($winStatus === 'Win') {
                $thisIndex = $i;
                return $sArray;
            }
        }
        
        return $sArray;
    }
    
    /**
     * แนะนำสีที่ควรเทรด
     */
    private function getSuggestColor($thisColor, $lossCon) {
        // Type 1: Red->Green, Green->Red (opposite)
        // Type 2: Red->Green, Green->Green (follow green)
        
        if ($this->strategyType == 1) {
            if ($thisColor === 'red') {
                $suggestColor = 'green';
                $suggestColor2 = '🟢';
            } else {
                $suggestColor = 'red';
                $suggestColor2 = '🔴';
            }
        }
        
        if ($this->strategyType == 2) {
            if ($thisColor === 'red') {
                $suggestColor = 'green';
                $suggestColor2 = '🟢';
            } else {
                $suggestColor = 'green';
                $suggestColor2 = '🟢';
            }
        }
        
        // ถ้าแพ้ติดต่อกัน >= 2 ให้เปลี่ยนกลยุทธ์
        if ($lossCon >= 2) {
            if ($thisColor === 'red') {
                $suggestColor = 'red';
                $suggestColor2 = '🔴';
            } else {
                $suggestColor = 'green';
                $suggestColor2 = '🟢';
            }
        }
        
        return array($suggestColor, $suggestColor2);
    }
    
    /**
     * แสดงรายงานแบบ HTML
     */
    public function printHTMLReport($showSubTrade = 0) {
        $concludeLab = array_fill(0, 16, 0);
        
        echo '<style>
            td { padding:10px; border:1px solid lightgray } 
            th { padding:10px; border:1px solid lightgray; background:#004080; color:white }
            .win { background-color: #d4edda; }
            .loss { background-color: #f8d7da; }
        </style>';
        
        echo "<h3>ข้อสังเกต: ATR ที่เกิดบนแท่ง Green แท่งต่อไปมักจะเป็น Green</h3>";
        
        for ($i = 0; $i < count($this->AllArray); $i++) {
            if (count($this->AllArray[$i]) >= $showSubTrade) {
                
                echo '<h3>เทรดครั้งที่ :: ' . ($i + 1);
                echo ' จำนวน Sub Trade :: ' . count($this->AllArray[$i]) . '</h3>';
                
                $sIndex = count($this->AllArray[$i]) - 1;
                $concludeLab[$sIndex]++;
                
                echo '<table><tr>';
                echo '<th>Sub TradeNo</th>';
                echo '<th>Time Trade</th>';
                echo '<th>Color</th>';
                echo '<th>Target Color</th>';
                echo '<th>Next Color</th>';
                echo '<th>Short Direction</th>';
                echo '<th>Medium Direction</th>';
                echo '<th>Win Status</th>';
                echo '<th>Loss Con</th>';
                echo '<th>Fixed Target</th>';
                echo '<th>ChangeTargetTo</th>';
                echo '<th>Result Fixed Target</th>';
                echo '</tr>';
                
                foreach ($this->AllArray[$i] as $i2 => $trade) {
                    $rowClass = ($trade->WinStatus === 'Win') ? 'win' : 'loss';
                    echo '<tr class="' . $rowClass . '">';
                    
                    echo '<td>' . ($i2 + 1) . '</td>';
                    echo '<td>' . ($trade->timeDisplay ?? '') . '</td>';
                    
                    // Color icons
                    $thisColor = ($trade->color === 'red') ? '🔴' : '🟢';
                    $targetColorShow = ($trade->targetColor === 'red') ? '🔴' : '🟢';
                    $nextColor = isset($trade->nextColor) && $trade->nextColor === 'red' ? '🔴' : '🟢';
                    
                    echo '<td>' . $thisColor . '</td>';
                    echo '<td>' . $targetColorShow . '</td>';
                    echo '<td>' . $nextColor . '</td>';
                    
                    // EMA Direction icons
                    $upIcon = 'https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcQB_txg8ioM2UthRx3oOJmO-zar4tCqmYncdQ&s';
                    $downIcon = 'https://cdn-icons-png.flaticon.com/128/5548/5548112.png';
                    
                    $shortImg = ($trade->emaShortDirection === 'Up') ? $upIcon : $downIcon;
                    $mediumImg = ($trade->emaMediumDirection === 'Up') ? $upIcon : $downIcon;
                    
                    echo '<td><img src="' . $shortImg . '" style="width:35px"></td>';
                    echo '<td><img src="' . $mediumImg . '" style="width:35px"></td>';
                    
                    $winStatusColor = ($trade->WinStatus === 'Win') ? '✅' : '❌';
                    echo '<td>' . $winStatusColor . '</td>';
                    echo '<td>' . $trade->LossCon . '</td>';
                    echo '<td>' . $trade->ShouldFix . '</td>';
                    
                    if ($trade->ShouldFix === 'y') {
                        $changeTargetIcon = ($trade->ChangeTargetTo === 'green') ? '🟢' : '🔴';
                        echo '<td>' . $changeTargetIcon . '</td>';
                        
                        $fixResult = ($trade->ChangeTargetTo === $trade->nextColor) ? '✅ Win' : '❌ Loss';
                        echo '<td>' . $fixResult . '</td>';
                    } else {
                        echo '<td></td><td></td>';
                    }
                    
                    echo '</tr>';
                }
                
                echo '</table>';
            }
        }
        
        // Summary table
        $this->printSummaryTable($concludeLab);
    }
    
    /**
     * แสดงตารางสรุป
     */
    private function printSummaryTable($concludeLab) {
        echo '<h3>สรุปผลการเทรด</h3>';
        echo '<table><tr>';
        
        for ($i = 0; $i < count($concludeLab) - 2; $i++) {
            echo '<td>' . ($i + 1) . '</td>';
        }
        echo '</tr><tr>';
        
        $totalTrade = 0;
        foreach ($concludeLab as $count) {
            if ($count > 0) {
                echo '<td>' . $count . '</td>';
                $totalTrade += $count;
            } else {
                echo '<td></td>';
            }
        }
        
        echo '</tr></table>';
        echo "<p><strong>Total Trades: {$totalTrade}</strong></p>";
    }
    
    /**
     * ส่งออกเป็น JSON
     */
    public function exportJSON($prettyPrint = true) {
        $flags = JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES;
        if ($prettyPrint) {
            $flags |= JSON_PRETTY_PRINT;
        }
        
        return json_encode($this->AllArray, $flags);
    }
    
    /**
     * ดึงสถิติ
     */
    public function getStatistics() {
        return [
            'total_trades' => $this->numFound,
            'grand_balance' => $this->grandBalance,
            'trade_distribution' => $this->numTradeArray,
            'all_trades' => $this->AllArray
        ];
    }
    
    /**
     * รีเซ็ตข้อมูล
     */
    public function reset() {
        $this->numFound = 0;
        $this->grandBalance = 0;
        $this->numTradeArray = array_fill(0, 14, 0);
        $this->AllArray = array();
        $this->tradeNo = 1;
    }
} // end class

// ==================== ตัวอย่างการใช้งาน ====================

// เปิดการแสดง error
ini_set('display_errors', 1);
ini_set('display_startup_errors', 1);
error_reporting(E_ALL);
ob_start();

try {
    // สร้าง instance
    $analyzer = new ATRCandleAnalyzer();
    
    // โหลดข้อมูล
    $candleData = $analyzer->loadDataFromFile('data.json');
    
    // วิเคราะห์
    $results = $analyzer->analyze($candleData);
    
    // แสดงรายงาน HTML
    $analyzer->printHTMLReport(0);

	/*
	$str_json=json_encode($results, JSON_UNESCAPED_UNICODE  | JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES);
	echo '<pre>' . $str_json . '</pre>' ;
	*/
	
	
    // แสดงสถิติ
    $stats = $analyzer->getStatistics();
    echo "<h3>สถิติรวม</h3>";
    echo "<p>จำนวนสัญญาณทั้งหมด: {$stats['total_trades']}</p>";
    echo "<p>กำไร/ขาดทุนรวม: \${$stats['grand_balance']}</p>";
    
    // Export JSON (ถ้าต้องการ)
    // $json = $analyzer->exportJSON();
    // file_put_contents('analysis_result.json', $json);
    
} catch (Exception $e) {
    echo "<div style='color:red; padding:20px; border:1px solid red;'>";
    echo "<h3>Error:</h3>";
    echo "<p>" . $e->getMessage() . "</p>";
    echo "</div>";
}

?>
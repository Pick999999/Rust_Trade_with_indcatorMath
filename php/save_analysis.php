<?php
/**
 * API สำหรับรับข้อมูลจาก AJAX POST เพื่อบันทึกข้อมูลการวิเคราะห์ (Analysis Data)
 * ลงในตาราง full_analysis_result
 */

header("Access-Control-Allow-Origin: *");
header("Content-Type: application/json; charset=UTF-8");
header("Access-Control-Allow-Methods: POST, OPTIONS");
header("Access-Control-Allow-Headers: Content-Type, Access-Control-Allow-Headers, Authorization, X-Requested-With");

// จัดการกับ Preflight Request (CORS) 
if ($_SERVER['REQUEST_METHOD'] === 'OPTIONS') {
    http_response_code(200);
    exit;
}

// ==========================================
// 1. ตั้งค่าการเชื่อมต่อฐานข้อมูล (Database Config)
// ==========================================
include_once 'config.db.php';
$charset = 'utf8mb4';

$dsn = "mysql:host=$host;dbname=$db;charset=$charset";
$options = [
    PDO::ATTR_ERRMODE            => PDO::ERRMODE_EXCEPTION,
    PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC,
    PDO::ATTR_EMULATE_PREPARES   => false,
];

try {
    $pdo = new PDO($dsn, $user, $pass, $options);
} catch (\PDOException $e) {
    http_response_code(500);
    echo json_encode(["status" => "error", "message" => "Database connection failed: " . $e->getMessage()]);
    exit;
}

// ==========================================
// 2. รับข้อมูลจาก AJAX POST
// ==========================================
// รองรับทั้งข้อมูลที่ส่งมาเป็น application/json
$inputJSON = file_get_contents('php://input');
$data = json_decode($inputJSON, true);
$jsonError = json_last_error_msg();

// ถ้ารับแบบ raw json ไม่ได้ ลองรับจาก Form Data แบบ x-www-form-urlencoded
if ($data === null && isset($_POST['data'])) {
    $inputJSON = $_POST['data'];
    $data = json_decode($inputJSON, true);
    $jsonError = json_last_error_msg();
}

// ตรวจสอบว่ามีข้อมูลหรือไม่ (เปลี่ยนจาก !$data เป็น $data === null เผื่อกรณีข้อมูลเป็น array เปล่า)
if ($data === null) {
    http_response_code(400);
    $debugInfo = [
        "status" => "error", 
        "message" => "No valid data received or Invalid JSON format",
        "json_error" => $jsonError,
        "received_length" => strlen($inputJSON),
        "received_sample" => substr($inputJSON, 0, 100) // แสดงแค่ 100 ตัวอักษรแรกเพื่อดูว่าเป็นข้อมูลอะไร
    ];
    echo json_encode($debugInfo);
    exit;
}

// ถ้าข้อมูลที่ส่งมาเป็น object เดียว ให้ครอบเป็น array เพื่อให้วนลูป (รองรับการส่งทีละหลายแถว)
if (!is_array(reset($data))) {
    $data = [$data];
}

// ==========================================
// 3. กำหนดชื่อคอลัมน์ทั้งหมดตามโครงสร้าง Table
// ==========================================
$columns = [
    'row_index', 'asset_code', 'candletime', 'candletime_display', 'open', 'high', 'low', 'close', 
    'color', 'next_color', 'pip_size', 'ema_short_value', 'ema_short_direction', 'ema_short_turn_type', 
    'ema_medium_value', 'ema_medium_direction', 'ema_long_value', 'ema_long_direction', 'ema_above', 
    'ema_long_above', 'macd_12', 'macd_23', 'previous_ema_short_value', 'previous_ema_medium_value', 
    'previous_ema_long_value', 'previous_macd_12', 'previous_macd_23', 'ema_convergence_type', 
    'ema_long_convergence_type', 'choppy_indicator', 'adx_value', 'rsi_value', 'bb_values', 'bb_position', 
    'atr_value', 'is_abnormal_candle', 'is_abnormal_atr', 'u_wick', 'u_wick_percent', 'body', 'body_percent', 
    'l_wick', 'l_wick_percent', 'ema_cut_position', 'ema_cut_long_type', 'candles_since_ema_cut', 
    'ema_short_pos', 'ema_medium_pos', 'ema_long_pos', 'bb_bandwidth', 'is_bb_squeeze', 'up_con_medium_ema', 
    'down_con_medium_ema', 'up_con_long_ema', 'down_con_long_ema', 'is_mark', 'status_code', 'status_desc', 
    'status_desc_0', 'hint_status', 'suggest_color', 'win_status', 'win_con', 'loss_con', 'smc', 
    'tick_volatility', 'range_detector'
];

// ==========================================
// 4. บันทึกข้อมูลลงฐานข้อมูล (Insert Data)
// ==========================================
$insertedCount = 0;

try {
    $pdo->beginTransaction();

    // ลบข้อมูลเดิมของ asset และวันที่นั้นๆ ออกก่อนเพื่อป้องกันข้อมูลซ้ำซ้อน
    if (!empty($data)) {
        $firstRow = $data[0];
        $assetCode = isset($firstRow['assetCode']) ? $firstRow['assetCode'] : null;
        $candletime = isset($firstRow['candletime']) ? $firstRow['candletime'] : null;
        
        if ($assetCode && $candletime) {
            $candleDate = date('Y-m-d', $candletime);
            $delSql = "DELETE FROM full_analysis_result WHERE asset_code = ? AND candleDate = ?";
            $delStmt = $pdo->prepare($delSql);
            $delStmt->execute([$assetCode, $candleDate]);
        }
    }

    // สร้าง SQL query ด้วย Placeholders (?)
    $placeholders = array_fill(0, count($columns), '?');
    $sql = "INSERT INTO full_analysis_result (" . implode(", ", $columns) . ") VALUES (" . implode(", ", $placeholders) . ")";
    $stmt = $pdo->prepare($sql);

    foreach ($data as $row) {
        $values = [];
        foreach ($columns as $col) {
            // แมปชื่อคอลัมน์ในฐานข้อมูลให้ตรงกับ Key ใน JSON Payload
            $jsonKey = $col;
            if ($col === 'row_index') {
                $jsonKey = 'index';
            } elseif ($col === 'asset_code') {
                $jsonKey = 'assetCode';
            } elseif ($col === 'atr_value') {
                $jsonKey = 'atrValue';
            }

            $val = isset($row[$jsonKey]) ? $row[$jsonKey] : null;
            
            // แปลงข้อมูล Array/Object ให้เป็น JSON String สำหรับคอลัมน์ประเภท JSON
            if (in_array($col, ['bb_values', 'smc', 'tick_volatility', 'range_detector']) && (is_array($val) || is_object($val))) {
                $val = json_encode($val, JSON_UNESCAPED_UNICODE);
            }
            
            // แปลง Boolean ให้เป็น 1 / 0 (ถึงแม้ PDO มักจะจัดการให้ แต่แปลงไว้ชัวร์กว่า)
            if (in_array($col, ['is_abnormal_candle', 'is_abnormal_atr', 'is_bb_squeeze']) && is_bool($val)) {
                $val = $val ? 1 : 0;
            }
            
            $values[] = $val;
        }
        $stmt->execute($values);
        $insertedCount++;
    }

    // อัปเดตข้อมูล candleDate จาก candletime หลังจาก insert ข้อมูลครบ
    $updateSql = "UPDATE full_analysis_result 
                  SET candleDate = FROM_UNIXTIME(candletime, '%Y-%m-%d') 
                  WHERE candleDate = '0000-00-00' OR candleDate IS NULL";
    $pdo->exec($updateSql);

    $pdo->commit();
    echo json_encode(["status" => "success", "message" => "บันทึกข้อมูลสำเร็จ จำนวน $insertedCount รายการ พร้อมอัปเดตวันที่"]);

} catch (\Exception $e) {
    $pdo->rollBack();
    http_response_code(500);
    echo json_encode(["status" => "error", "message" => "Database insert failed: " . $e->getMessage()]);
}
?>

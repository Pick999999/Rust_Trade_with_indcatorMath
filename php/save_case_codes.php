<?php
// save_case_codes.php — Endpoint for updating case_codes.json across project directories
header('Content-Type: application/json; charset=utf-8');
header('Access-Control-Allow-Origin: *');
header('Access-Control-Allow-Methods: POST, OPTIONS');
header('Access-Control-Allow-Headers: Content-Type');

if ($_SERVER['REQUEST_METHOD'] === 'OPTIONS') {
    http_response_code(200);
    exit;
}

if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
    http_response_code(405);
    echo json_encode(['success' => false, 'error' => 'Method Not Allowed. Use POST.']);
    exit;
}

$inputRaw = file_get_contents('php://input');
$data = json_decode($inputRaw, true);

if (!$data) {
    http_response_code(400);
    echo json_encode(['success' => false, 'error' => 'Invalid JSON input payload.']);
    exit;
}

$casesList = [];
if (isset($data['cases']) && is_array($data['cases'])) {
    $casesList = $data['cases'];
} elseif (is_array($data)) {
    $casesList = $data;
}

if (empty($casesList)) {
    http_response_code(400);
    echo json_encode(['success' => false, 'error' => 'No cases data provided.']);
    exit;
}

// Normalize and validate cases
$normalizedCases = [];
foreach ($casesList as $item) {
    if (!isset($item['codeNo']) || !isset($item['caseCode'])) {
        continue;
    }
    $codeNo = intval($item['codeNo']);
    $caseCode = trim($item['caseCode']);
    $caseDesc = isset($item['caseDesc']) ? trim($item['caseDesc']) : '';
    $group = isset($item['group']) ? trim($item['group']) : 'Sideways';
    $trend = isset($item['trend']) ? $item['trend'] : null;
    $category = isset($item['category']) ? trim($item['category']) : ($codeNo <= 20 ? ($group === 'System' ? 'SYSTEM' : 'CORE') : 'EXTENDED');
    $isSelectedToAction = (isset($item['isSelectedToAction']) && strtolower(trim($item['isSelectedToAction'])) === 'y') ? 'y' : 'n';
    $description = isset($item['description']) ? trim($item['description']) : '';

    $normalizedCases[] = [
        'codeNo' => $codeNo,
        'caseCode' => $caseCode,
        'caseDesc' => $caseDesc,
        'group' => $group,
        'trend' => $trend,
        'category' => $category,
        'isSelectedToAction' => $isSelectedToAction,
        'description' => $description
    ];
}

// Sort by codeNo ascending
usort($normalizedCases, function ($a, $b) {
    return $a['codeNo'] <=> $b['codeNo'];
});

$finalPayload = [
    'total' => count($normalizedCases),
    'version' => '5.0',
    'updated_at' => date('Y-m-d H:i:s P'),
    'cases' => $normalizedCases
];

$jsonFormatted = json_encode($finalPayload, JSON_PRETTY_PRINT | JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES);

// Paths to save across the project
$targetPaths = [
    __DIR__ . '/../case_codes.json',
    __DIR__ . '/../../case_codes.json',
    __DIR__ . '/../public/case_codes.json',
    __DIR__ . '/../../../dynamicChart/case_codes.json',
    __DIR__ . '/../../../dynamicChart/indicator/case_codes.json',
    __DIR__ . '/../../../dynamicChart/php/case_codes.json',
];

$savedPaths = [];
$errors = [];

foreach ($targetPaths as $path) {
    $realDir = dirname($path);
    if (is_dir($realDir)) {
        $ok = file_put_contents($path, $jsonFormatted);
        if ($ok !== false) {
            $savedPaths[] = realpath($path) ?: $path;
        } else {
            $errors[] = "Failed to write to $path";
        }
    }
}

if (count($savedPaths) > 0) {
    echo json_encode([
        'success' => true,
        'message' => 'บันทึก Case Codes สำเร็จ (' . count($savedPaths) . ' ตำแหน่ง)',
        'saved_count' => count($normalizedCases),
        'saved_paths' => $savedPaths,
        'errors' => $errors
    ]);
} else {
    http_response_code(500);
    echo json_encode([
        'success' => false,
        'error' => 'ไม่สามารถบันทึกไฟล์ได้ในทุกตำแหน่ง',
        'details' => $errors
    ]);
}

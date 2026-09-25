<?php
ob_start();
/**
 * VPS Trade Schedule Manager (Command Center)
 * ติดตั้งที่: lovetoshopmall.com
 * ฟังก์ชัน: 
 *   - ดึงสถานะ OnTradeStatus และเวลา TradeControl ({startTradeTime, stopTradeTime, useSchedule}) จากแต่ละ VPS
 *   - ฟอร์มตั้งเวลาเทรด startdatetime, stopdatetime, switchbox useSchedule
 *   - สั่ง Restart Service (systemd) จากหน้าเว็บกลางได้ทันที
 *   - รองรับ 2 โหมดการเชื่อมต่อ:
 *       1. PHP Proxy (Server-to-Server) -> สำหรับ Domain / Port 80 / 443 เช่น Google Cloud
 *       2. Browser Direct (Client-to-VPS) -> สำหรับ Port พิเศษ เช่น Oracle Port 3000 (ยิงตรงจาก Browser ข้ามไฟร์วอลล์โฮสต์)
 *   - มี Auto-Fallback: ถ้า PHP Proxy เชื่อมต่อไม่ได้ จะสลับไปลองยิงตรงจาก Browser ให้อัตโนมัติ!
 */

// รายการ VPS Master
$vpsList = [
    [
        "id" => "1",
        "vpsCode" => "1",
        "vpsName" => "AWS",
        "publicIP" => "pkderiv.online",
        "privateIP" => null,
        "portno" => null,
        "url" => "pkderiv.online",
        "remark" => null,
        "DerivAccountID" => "3"
    ],
    [
        "id" => "2",
        "vpsCode" => "2",
        "vpsName" => "Google Cloud",
        "publicIP" => "gpkderiv.shop",
        "privateIP" => null,
        "portno" => null,
        "url" => "gpkderiv.shop",
        "remark" => null,
        "DerivAccountID" => "4"
    ],
    [
        "id" => "3",
        "vpsCode" => "3",
        "vpsName" => "Oracle-Free-Tier-3",
        "publicIP" => "161.118.203.228",
        "privateIP" => null,
        "portno" => null,
        "url" => null,
        "remark" => "Test link",
        "DerivAccountID" => "5"
    ],
    [
        "id" => "4",
        "vpsCode" => "4",
        "vpsName" => "Oracle-Free-Tier-4",
        "publicIP" => "161.118.217.177",
        "privateIP" => null,
        "portno" => null,
        "url" => null,
        "remark" => null,
        "DerivAccountID" => "1"
    ]
];

// Helper แปลง URL พื้นฐาน
function resolveVpsBaseUrl($vps) {
    if (!empty($vps['url'])) {
        $u = trim($vps['url']);
        if (strpos($u, 'http://') === 0 || strpos($u, 'https://') === 0) {
            return rtrim($u, '/');
        }
        return 'https://' . rtrim($u, '/');
    }
    $ip = trim($vps['publicIP']);
    $port = !empty($vps['portno']) ? ':' . trim($vps['portno']) : '';
    return 'http://' . $ip . $port;
}

// Helper แปลง URL ไปยังหน้า short term trading UI สำหรับเปิดแท็บใหม่
function resolveVpsWebUrl($vps) {
    $base = resolveVpsBaseUrl($vps);
    return rtrim($base, '/') . '/index_short_term.html';
}

// Helper ค้นหา Node ใน multi_node_summary ให้ตรงกับ VPS แต่ละตัว
function findNodeForVps($nodes, $vps) {
    if (!is_array($nodes) || empty($nodes)) return null;

    $vpsId = strval($vps['id'] ?? '');
    $vpsName = strtolower(trim($vps['vpsName'] ?? ''));
    $publicIp = strtolower(trim($vps['publicIP'] ?? ''));
    $url = strtolower(trim($vps['url'] ?? ''));

    // แผนที่จับคู่ชื่อและไอดีของแต่ละโฮสต์
    $aliases = [
        '1' => ['aws', 'pkderiv.online', 'node-1'],
        '2' => ['gcp', 'google cloud', 'google', 'gpkderiv.shop', 'node-2'],
        '3' => ['oracle3', 'oracle-3', 'oracle 3', 'oracle-free-tier-3', '161.118.203.228', 'node-3'],
        '4' => ['oracle4', 'oracle-4', 'oracle 4', 'oracle-free-tier-4', '161.118.217.177', 'node-4']
    ];

    $vpsAliases = $aliases[$vpsId] ?? [];
    $vpsAliases[] = $vpsName;
    if ($publicIp) $vpsAliases[] = $publicIp;
    if ($url) $vpsAliases[] = $url;

    foreach ($nodes as $node) {
        $nodeName = strtolower(trim($node['node_name'] ?? ''));
        $nodeClean = preg_replace('/[^a-z0-9]/', '', $nodeName);

        foreach ($vpsAliases as $alias) {
            $aliasClean = preg_replace('/[^a-z0-9]/', '', $alias);
            if ($nodeName === $alias || $nodeClean === $aliasClean) {
                return $node;
            }
            if (!empty($aliasClean) && (strpos($nodeClean, $aliasClean) !== false || strpos($aliasClean, $nodeClean) !== false)) {
                return $node;
            }
        }
    }

    return null;
}

// -------------------------------------------------------------
// API PROXY HANDLER (เมื่อเรียกผ่าน AJAX ในโหมด PHP Proxy)
// -------------------------------------------------------------
if (isset($_GET['action'])) {
    while (ob_get_level()) {
        ob_end_clean();
    }
    header('Content-Type: application/json; charset=utf-8');
    $action = $_GET['action'];

    // 1. ดึงข้อมูลสถานะ, Trade Control, Metrics (Profit, Balance, MaxLoss) ของ VPS รายตัว
    if ($action === 'fetch_vps_data') {
        $vpsId = $_GET['id'] ?? '';
        $targetVps = null;
        foreach ($vpsList as $v) {
            if ($v['id'] === $vpsId) {
                $targetVps = $v;
                break;
            }
        }

        if (!$targetVps) {
            echo json_encode(['success' => false, 'message' => 'VPS not found']);
            exit;
        }

        $baseUrl = resolveVpsBaseUrl($targetVps);

        // ดึง 4 endpoints พร้อมกันแบบ Parallel (tradeControl, status, setup, multi_node_summary)
        $endpoints = [
            'tc'      => $baseUrl . '/api/tradeControl',
            'status'  => $baseUrl . '/api/status',
            'setup'   => $baseUrl . '/api/setup',
            'summary' => $baseUrl . '/api/multi_node_summary'
        ];

        $mh = curl_multi_init();
        $handles = [];
        foreach ($endpoints as $key => $url) {
            $ch = curl_init();
            curl_setopt($ch, CURLOPT_URL, $url);
            curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
            curl_setopt($ch, CURLOPT_TIMEOUT, 4);
            curl_setopt($ch, CURLOPT_CONNECTTIMEOUT, 2);
            curl_setopt($ch, CURLOPT_SSL_VERIFYPEER, false);
            curl_setopt($ch, CURLOPT_SSL_VERIFYHOST, false);
            curl_multi_add_handle($mh, $ch);
            $handles[$key] = $ch;
        }

        $running = null;
        do {
            $mrc = curl_multi_exec($mh, $running);
            if ($running) {
                curl_multi_select($mh, 0.05);
            }
        } while ($running > 0 && $mrc == CURLM_OK);

        $results = [];
        $curlErrors = [];
        foreach ($handles as $key => $ch) {
            $content = curl_multi_getcontent($ch);
            $httpCode = curl_getinfo($ch, CURLINFO_HTTP_CODE);
            $err = curl_error($ch);
            if ($err) $curlErrors[$key] = $err;
            $results[$key] = [
                'code' => $httpCode,
                'data' => json_decode($content, true),
                'raw'  => $content
            ];
            curl_multi_remove_handle($mh, $ch);
            curl_close($ch);
        }
        curl_multi_close($mh);

        $tcData      = $results['tc']['data'];
        $statusData  = $results['status']['data'];
        $setupData   = $results['setup']['data'];
        $summaryData = $results['summary']['data'];

        $tcHttpCode     = $results['tc']['code'];
        $statusHttpCode = $results['status']['code'];

        $isOnline = (($tcHttpCode >= 200 && $tcHttpCode < 300) || ($statusHttpCode >= 200 && $statusHttpCode < 300));
        $isTrading = false;
        $tradeStatusText = 'ไม่ได้เทรด';

        if ($statusData && isset($statusData['is_trading']) && $statusData['is_trading']) {
            $isTrading = true;
            $tradeStatusText = !empty($statusData['active_assets'])
                ? 'กำลังเทรด (' . implode(', ', $statusData['active_assets']) . ')'
                : 'กำลังเทรด';
        } elseif ($statusData && isset($statusData['bot_count']) && $statusData['bot_count'] > 0 && !empty($statusData['active_assets'])) {
            $isTrading = true;
            $tradeStatusText = 'กำลังเทรด (' . implode(', ', $statusData['active_assets']) . ')';
        }

        if ($tcData && isset($tcData['data']['TradeStatus']) && $tcData['data']['TradeStatus'] === 'กำลังเทรด') {
            $isTrading = true;
            if ($tradeStatusText === 'ไม่ได้เทรด') {
                $tradeStatusText = 'กำลังเทรด';
            }
        }

        // 1. Balance
        $balance = 0.0;
        if ($statusData && isset($statusData['balance'])) {
            $balance = floatval($statusData['balance']);
        }

        // 2. Profit, Balance, MaxLoss Con & Asset
        $profit = 0.0;
        $maxLossCon = null;
        $maxLossAsset = '';

        $matchedNode = findNodeForVps($summaryData['data']['nodes'] ?? [], $targetVps);

        if ($matchedNode) {
            if (isset($matchedNode['balance']) && floatval($matchedNode['balance']) > 0) {
                $balance = floatval($matchedNode['balance']);
            }
            if (isset($matchedNode['today_profit_usd'])) {
                $profit = floatval($matchedNode['today_profit_usd']);
            } elseif (isset($matchedNode['round_profit_usd'])) {
                $profit = floatval($matchedNode['round_profit_usd']);
            }
            if (isset($matchedNode['max_loss_streak'])) {
                $maxLossCon = intval($matchedNode['max_loss_streak']);
            }
            if (!empty($matchedNode['max_loss_streak_asset'])) {
                $maxLossAsset = trim($matchedNode['max_loss_streak_asset']);
            }
            if (isset($matchedNode['is_trading']) && $matchedNode['is_trading']) {
                $isTrading = true;
                if ($tradeStatusText === 'ไม่ได้เทรด') $tradeStatusText = 'กำลังเทรด';
            }
        }

        // Fallback maxLossCon จาก setup.json หากไม่มีใน summary
        if ($maxLossCon === null && $setupData && isset($setupData['trade']['maxLossCon'])) {
            $maxLossCon = intval($setupData['trade']['maxLossCon']);
        }

        // ดึงค่า suggestStrategy จาก setup.json
        $suggestStrategy = null;
        if ($setupData) {
            if (!empty($setupData['trade']['suggestStrategy'])) {
                $suggestStrategy = $setupData['trade']['suggestStrategy'];
            } elseif (!empty($setupData['suggestStrategy'])) {
                $suggestStrategy = $setupData['suggestStrategy'];
            }
        }

        // Localhost fallback: ถ้า remote ไม่พบค่า หรือกำลังทดสอบ ให้ลองดึงจาก localhost:3000/api/setup
        if (!$suggestStrategy && ($targetVps['id'] === '1' || strpos($baseUrl, 'localhost') !== false || strpos($baseUrl, '127.0.0.1') !== false)) {
            $ctx = stream_context_create(['http' => ['timeout' => 1]]);
            $localSetup = @file_get_contents('http://localhost:3000/api/setup', false, $ctx);
            if ($localSetup) {
                $localJson = json_decode($localSetup, true);
                $suggestStrategy = $localJson['trade']['suggestStrategy'] ?? $localJson['suggestStrategy'] ?? null;
            }
        }

        echo json_encode([
            'success'         => $isOnline,
            'baseUrl'         => $baseUrl,
            'webUrl'          => resolveVpsWebUrl($targetVps),
            'isOnline'        => $isOnline,
            'isTrading'       => $isTrading,
            'tradeStatusText' => $tradeStatusText,
            'tradeControl'    => $tcData['data'] ?? null,
            'status'          => $statusData ?? null,
            'setup'           => $setupData ?? null,
            'suggestStrategy' => $suggestStrategy,
            'balance'         => $balance,
            'profit'          => $profit,
            'maxLossCon'      => $maxLossCon,
            'maxLossAsset'    => $maxLossAsset,
            'rawError'        => !empty($curlErrors) ? implode(', ', $curlErrors) : ($isOnline ? null : "HTTP Code: tc=$tcHttpCode, status=$statusHttpCode")
        ]);
        exit;
    }

    // 2. บันทึกข้อมูล Trade Control ไปยัง VPS รายตัว
    if ($action === 'save_trade_control') {
        $input = json_decode(file_get_contents('php://input'), true);
        $vpsId = $input['id'] ?? '';
        $startTradeTime = $input['startTradeTime'] ?? null;
        $stopTradeTime = $input['stopTradeTime'] ?? null;
        $useSchedule = isset($input['useSchedule']) ? (bool)$input['useSchedule'] : false;

        $targetVps = null;
        foreach ($vpsList as $v) {
            if ($v['id'] === $vpsId) {
                $targetVps = $v;
                break;
            }
        }

        if (!$targetVps) {
            echo json_encode(['success' => false, 'message' => 'VPS not found']);
            exit;
        }

        $baseUrl = resolveVpsBaseUrl($targetVps);
        $postUrl = $baseUrl . '/api/tradeControl';

        $payload = json_encode([
            'startTradeTime' => $startTradeTime,
            'stopTradeTime' => $stopTradeTime,
            'useSchedule' => $useSchedule
        ]);

        $ch = curl_init();
        curl_setopt($ch, CURLOPT_URL, $postUrl);
        curl_setopt($ch, CURLOPT_POST, true);
        curl_setopt($ch, CURLOPT_POSTFIELDS, $payload);
        curl_setopt($ch, CURLOPT_HTTPHEADER, ['Content-Type: application/json']);
        curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
        curl_setopt($ch, CURLOPT_TIMEOUT, 6);
        curl_setopt($ch, CURLOPT_SSL_VERIFYPEER, false);
        curl_setopt($ch, CURLOPT_SSL_VERIFYHOST, false);
        $res = curl_exec($ch);
        $httpCode = curl_getinfo($ch, CURLINFO_HTTP_CODE);
        $curlErr = curl_error($ch);
        curl_close($ch);

        $jsonRes = json_decode($res, true);
        $isOk = ($httpCode >= 200 && $httpCode < 300);

        echo json_encode([
            'success' => $isOk,
            'httpCode' => $httpCode,
            'response' => $jsonRes,
            'error' => $curlErr
        ]);
        exit;
    }

    // 3. สั่ง Restart Service (systemd) ไปยัง VPS รายตัว
    if ($action === 'restart_service') {
        $input = json_decode(file_get_contents('php://input'), true);
        $vpsId = $input['id'] ?? '';
        $targetVps = null;
        foreach ($vpsList as $v) {
            if ($v['id'] === $vpsId) {
                $targetVps = $v;
                break;
            }
        }

        if (!$targetVps) {
            echo json_encode(['success' => false, 'message' => 'VPS not found']);
            exit;
        }

        $baseUrl = resolveVpsBaseUrl($targetVps);
        $postUrl = $baseUrl . '/api/system/restart';

        $ch = curl_init();
        curl_setopt($ch, CURLOPT_URL, $postUrl);
        curl_setopt($ch, CURLOPT_POST, true);
        curl_setopt($ch, CURLOPT_POSTFIELDS, '{}');
        curl_setopt($ch, CURLOPT_HTTPHEADER, ['Content-Type: application/json']);
        curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
        curl_setopt($ch, CURLOPT_TIMEOUT, 4);
        curl_setopt($ch, CURLOPT_SSL_VERIFYPEER, false);
        curl_setopt($ch, CURLOPT_SSL_VERIFYHOST, false);
        $res = curl_exec($ch);
        $httpCode = curl_getinfo($ch, CURLINFO_HTTP_CODE);
        $curlErr = curl_error($ch);
        curl_close($ch);

        $jsonRes = json_decode($res, true);
        $isOk = ($httpCode >= 200 && $httpCode < 300);

        echo json_encode([
            'success' => $isOk,
            'httpCode' => $httpCode,
            'response' => $jsonRes,
            'error' => $curlErr
        ]);
        exit;
    }

    // 4. สั่ง Stop Trade ไปยัง Rust บน VPS รายตัว
    if ($action === 'stop_trade') {
        $input = json_decode(file_get_contents('php://input'), true);
        $vpsId = $input['id'] ?? '';
        $targetVps = null;
        foreach ($vpsList as $v) {
            if ($v['id'] === $vpsId) {
                $targetVps = $v;
                break;
            }
        }

        if (!$targetVps) {
            echo json_encode(['success' => false, 'message' => 'VPS not found']);
            exit;
        }

        $baseUrl = resolveVpsBaseUrl($targetVps);
        $postUrl = $baseUrl . '/api/stop';

        $ch = curl_init();
        curl_setopt($ch, CURLOPT_URL, $postUrl);
        curl_setopt($ch, CURLOPT_POST, true);
        curl_setopt($ch, CURLOPT_POSTFIELDS, '{}');
        curl_setopt($ch, CURLOPT_HTTPHEADER, ['Content-Type: application/json']);
        curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
        curl_setopt($ch, CURLOPT_TIMEOUT, 6);
        curl_setopt($ch, CURLOPT_SSL_VERIFYPEER, false);
        curl_setopt($ch, CURLOPT_SSL_VERIFYHOST, false);
        $res = curl_exec($ch);
        $httpCode = curl_getinfo($ch, CURLINFO_HTTP_CODE);
        $curlErr = curl_error($ch);
        curl_close($ch);

        $jsonRes = json_decode($res, true);
        $isOk = ($httpCode >= 200 && $httpCode < 300);

        echo json_encode([
            'success' => $isOk,
            'httpCode' => $httpCode,
            'response' => $jsonRes,
            'error' => $curlErr
        ]);
        exit;
    }

    // 5. ดึงอัตราแลกเปลี่ยน USD/THB ปัจจุบัน
    if ($action === 'get_usd_thb_rate') {
        $rate = 33.36;
        $ctx = stream_context_create(['http' => ['timeout' => 3]]);
        $jsonStr = @file_get_contents('https://api.exchangerate-api.com/v4/latest/USD', false, $ctx);
        if ($jsonStr) {
            $data = json_decode($jsonStr, true);
            if (!empty($data['rates']['THB'])) {
                $rate = floatval($data['rates']['THB']);
            }
        }
        echo json_encode(['success' => true, 'rate' => $rate]);
        exit;
    }
}
?>
<!DOCTYPE html>
<html lang="th">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>VPS Trade Schedule Center - lovetoshopmall.com</title>
    <link href="https://fonts.googleapis.com/css2?family=Kanit:wght@300;400;500;600;700&family=Share+Tech+Mono&display=swap" rel="stylesheet">
    <style>
        :root {
            --bg: #0b0f19;
            --card-bg: #121929;
            --card-border: #1e293b;
            --text-main: #f8fafc;
            --text-muted: #94a3b8;
            --primary: #3b82f6;
            --primary-hover: #2563eb;
            --accent: #00d2b4;
            --success: #10b981;
            --danger: #ef4444;
            --warning: #f59e0b;
            --shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.4), 0 8px 10px -6px rgba(0, 0, 0, 0.3);
            --radius: 12px;
        }

        * {
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }

        body {
            background-color: var(--bg);
            color: var(--text-main);
            font-family: 'Kanit', sans-serif;
            min-height: 100vh;
            padding: 24px;
        }

        .container {
            max-width: 1280px;
            margin: 0 auto;
        }

        /* Header */
        .top-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            flex-wrap: wrap;
            gap: 16px;
            padding-bottom: 20px;
            margin-bottom: 24px;
            border-bottom: 1px solid var(--card-border);
        }

        .brand-title {
            display: flex;
            align-items: center;
            gap: 12px;
        }

        .brand-icon {
            width: 42px;
            height: 42px;
            background: linear-gradient(135deg, var(--primary), var(--accent));
            border-radius: 10px;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 22px;
        }

        .brand-title h1 {
            font-size: 22px;
            font-weight: 700;
            letter-spacing: 0.5px;
        }

        .brand-title p {
            font-size: 13px;
            color: var(--text-muted);
        }

        .global-actions {
            display: flex;
            gap: 10px;
            align-items: center;
            flex-wrap: wrap;
        }

        .btn {
            display: inline-flex;
            align-items: center;
            gap: 8px;
            padding: 9px 16px;
            border-radius: 8px;
            font-size: 14px;
            font-weight: 500;
            font-family: 'Kanit', sans-serif;
            cursor: pointer;
            border: none;
            transition: all 0.2s;
        }

        .btn-primary {
            background: var(--primary);
            color: #fff;
        }

        .btn-primary:hover {
            background: var(--primary-hover);
            transform: translateY(-1px);
        }

        .btn-outline {
            background: transparent;
            color: var(--text-main);
            border: 1px solid var(--card-border);
        }

        .btn-outline:hover {
            background: rgba(255, 255, 255, 0.05);
            border-color: var(--text-muted);
        }

        .btn-success {
            background: var(--success);
            color: #fff;
        }

        .btn-success:hover {
            background: #059669;
        }

        .btn-warning {
            background: rgba(245, 158, 11, 0.15);
            color: var(--warning);
            border: 1px solid rgba(245, 158, 11, 0.3);
        }

        .btn-warning:hover {
            background: rgba(245, 158, 11, 0.25);
            border-color: var(--warning);
        }

        .btn-danger {
            background: rgba(239, 68, 68, 0.15);
            color: var(--danger);
            border: 1px solid rgba(239, 68, 68, 0.3);
        }

        .btn-danger:hover {
            background: rgba(239, 68, 68, 0.25);
            border-color: var(--danger);
        }

        /* Grid */
        .vps-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(580px, 1fr));
            gap: 24px;
        }

        @media (max-width: 768px) {
            .vps-grid {
                grid-template-columns: 1fr;
            }
        }

        /* Card */
        .vps-card {
            background: var(--card-bg);
            border: 1px solid var(--card-border);
            border-radius: var(--radius);
            box-shadow: var(--shadow);
            padding: 22px;
            display: flex;
            flex-direction: column;
            gap: 18px;
            position: relative;
            transition: border-color 0.2s, transform 0.2s;
        }

        .vps-card:hover {
            border-color: rgba(59, 130, 246, 0.4);
        }

        .card-header {
            display: flex;
            justify-content: space-between;
            align-items: flex-start;
            gap: 12px;
            border-bottom: 1px solid rgba(255, 255, 255, 0.06);
            padding-bottom: 14px;
        }

        .card-title-group h2 {
            font-size: 18px;
            font-weight: 600;
            display: flex;
            align-items: center;
            gap: 8px;
        }

        .vps-meta {
            font-size: 12px;
            color: var(--text-muted);
            font-family: 'Share Tech Mono', monospace;
            margin-top: 4px;
            display: flex;
            gap: 10px;
            flex-wrap: wrap;
            align-items: center;
        }

        .badge {
            display: inline-flex;
            align-items: center;
            gap: 6px;
            padding: 4px 10px;
            border-radius: 20px;
            font-size: 12px;
            font-weight: 500;
        }

        .badge-trading {
            background: rgba(16, 185, 129, 0.15);
            color: var(--success);
            border: 1px solid rgba(16, 185, 129, 0.4);
        }

        .badge-idle {
            background: rgba(148, 163, 184, 0.12);
            color: var(--text-muted);
            border: 1px solid rgba(148, 163, 184, 0.25);
        }

        .badge-offline {
            background: rgba(239, 68, 68, 0.15);
            color: var(--danger);
            border: 1px solid rgba(239, 68, 68, 0.4);
        }

        .badge-restarting {
            background: rgba(245, 158, 11, 0.15);
            color: var(--warning);
            border: 1px solid rgba(245, 158, 11, 0.4);
        }

        .badge-mode {
            font-size: 11px;
            padding: 2px 8px;
            border-radius: 4px;
            background: rgba(59, 130, 246, 0.15);
            color: #60a5fa;
            border: 1px solid rgba(59, 130, 246, 0.3);
            cursor: pointer;
        }

        .badge-mode.direct {
            background: rgba(0, 210, 180, 0.15);
            color: var(--accent);
            border-color: rgba(0, 210, 180, 0.4);
        }

        .pulse-dot {
            width: 8px;
            height: 8px;
            border-radius: 50%;
            background: currentColor;
            box-shadow: 0 0 8px currentColor;
            animation: pulse 1.8s infinite;
        }

        @keyframes pulse {
            0% { opacity: 1; transform: scale(1); }
            50% { opacity: 0.4; transform: scale(1.2); }
            100% { opacity: 1; transform: scale(1); }
        }

        /* Form Inputs */
        .form-row {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 14px;
        }

        .form-group {
            display: flex;
            flex-direction: column;
            gap: 6px;
        }

        .form-label {
            font-size: 13px;
            color: var(--text-muted);
            font-weight: 400;
        }

        .form-control {
            background: #080c14;
            border: 1px solid var(--card-border);
            color: var(--text-main);
            padding: 9px 12px;
            border-radius: 8px;
            font-size: 14px;
            font-family: 'Kanit', sans-serif;
            outline: none;
            transition: border-color 0.2s;
        }

        .form-control:focus {
            border-color: var(--primary);
        }

        /* Toggle Switch */
        .toggle-group {
            display: flex;
            align-items: center;
            justify-content: space-between;
            background: #080c14;
            border: 1px solid var(--card-border);
            padding: 10px 14px;
            border-radius: 8px;
        }

        .toggle-text {
            font-size: 14px;
            font-weight: 500;
            display: flex;
            flex-direction: column;
        }

        .toggle-subtext {
            font-size: 11px;
            color: var(--text-muted);
        }

        .switch {
            position: relative;
            display: inline-block;
            width: 44px;
            height: 24px;
        }

        .switch input {
            opacity: 0;
            width: 0;
            height: 0;
        }

        .slider {
            position: absolute;
            cursor: pointer;
            top: 0; left: 0; right: 0; bottom: 0;
            background-color: #334155;
            transition: .3s;
            border-radius: 24px;
        }

        .slider:before {
            position: absolute;
            content: "";
            height: 18px;
            width: 18px;
            left: 3px;
            bottom: 3px;
            background-color: white;
            transition: .3s;
            border-radius: 50%;
        }

        input:checked + .slider {
            background-color: var(--accent);
        }

        input:checked + .slider:before {
            transform: translateX(20px);
        }

        /* Card Actions */
        .card-actions {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-top: 4px;
            padding-top: 14px;
            border-top: 1px solid rgba(255, 255, 255, 0.06);
            flex-wrap: wrap;
            gap: 10px;
        }

        .status-msg {
            font-size: 12px;
            color: var(--text-muted);
        }

        .quick-actions {
            display: flex;
            gap: 6px;
            flex-wrap: wrap;
        }

        .btn-sm {
            padding: 5px 12px;
            font-size: 12px;
            border-radius: 6px;
        }

        /* =================================================== */
        /* VPS Live Thumbnails Overview Panel (Top Section)   */
        /* =================================================== */
        .vps-overview-panel {
            background: linear-gradient(135deg, rgba(18, 25, 41, 0.95) 0%, rgba(11, 15, 25, 0.98) 100%);
            border: 1px solid rgba(59, 130, 246, 0.28);
            border-radius: var(--radius);
            padding: 20px 24px;
            margin-bottom: 24px;
            box-shadow: 0 14px 35px -10px rgba(0, 0, 0, 0.65);
            backdrop-filter: blur(14px);
        }

        .vps-overview-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            flex-wrap: wrap;
            gap: 16px;
            padding-bottom: 16px;
            border-bottom: 1px solid rgba(255, 255, 255, 0.08);
        }

        .overview-title-group {
            display: flex;
            align-items: center;
            gap: 12px;
        }

        .overview-icon {
            font-size: 26px;
            width: 44px;
            height: 44px;
            display: flex;
            align-items: center;
            justify-content: center;
            background: rgba(59, 130, 246, 0.15);
            border: 1px solid rgba(59, 130, 246, 0.35);
            border-radius: 10px;
        }

        .overview-title-group h3 {
            font-size: 18px;
            font-weight: 600;
            color: #fff;
            display: flex;
            align-items: center;
            gap: 8px;
        }

        .overview-title-group p {
            font-size: 12px;
            color: var(--text-muted);
            margin-top: 3px;
        }

        /* Polling Controls (Switch + Textbox) */
        .polling-control-box {
            display: flex;
            align-items: center;
            gap: 12px;
            background: rgba(8, 12, 20, 0.85);
            border: 1px solid var(--card-border);
            padding: 8px 14px;
            border-radius: 10px;
            flex-wrap: wrap;
        }

        .polling-switch-group {
            display: flex;
            align-items: center;
            gap: 8px;
        }

        .switch-sm {
            width: 38px;
            height: 20px;
        }

        .switch-sm .slider:before {
            height: 14px;
            width: 14px;
            left: 3px;
            bottom: 3px;
        }

        .switch-sm input:checked + .slider:before {
            transform: translateX(18px);
        }

        .polling-label {
            font-size: 13px;
            font-weight: 500;
            color: var(--text-main);
        }

        .polling-interval-group {
            display: flex;
            align-items: center;
            gap: 6px;
            font-size: 12px;
            color: var(--text-muted);
            border-left: 1px solid rgba(255, 255, 255, 0.12);
            padding-left: 12px;
        }

        .poll-input {
            width: 54px;
            background: #0b0f19;
            border: 1px solid #334155;
            color: var(--accent);
            padding: 4px 6px;
            border-radius: 6px;
            font-family: 'Share Tech Mono', monospace;
            font-weight: 700;
            text-align: center;
            font-size: 13px;
            outline: none;
            transition: border-color 0.2s, box-shadow 0.2s;
        }

        .poll-input:focus {
            border-color: var(--primary);
            box-shadow: 0 0 8px rgba(59, 130, 246, 0.45);
        }

        .poll-status-badge {
            display: inline-flex;
            align-items: center;
            gap: 6px;
            font-size: 11px;
            font-family: 'Share Tech Mono', monospace;
            padding: 4px 10px;
            border-radius: 6px;
            background: rgba(16, 185, 129, 0.15);
            color: #10b981;
            border: 1px solid rgba(16, 185, 129, 0.35);
        }

        .poll-status-badge.paused {
            background: rgba(239, 68, 68, 0.15);
            color: #ef4444;
            border-color: rgba(239, 68, 68, 0.35);
        }

        .btn-xs {
            padding: 4px 10px;
            font-size: 11.5px;
            border-radius: 6px;
        }

        /* Thumbnails Grid */
        .vps-thumbnails-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
            gap: 16px;
            margin-top: 16px;
        }

        /* Thumbnail Card */
        .vps-thumb-card {
            background: rgba(18, 25, 41, 0.95);
            border: 1px solid rgba(255, 255, 255, 0.08);
            border-radius: 12px;
            padding: 16px 18px;
            text-decoration: none;
            color: var(--text-main);
            display: flex;
            flex-direction: column;
            gap: 12px;
            transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
            position: relative;
            overflow: hidden;
            box-shadow: 0 4px 14px rgba(0, 0, 0, 0.3);
            cursor: pointer;
        }

        .vps-thumb-card:hover {
            transform: translateY(-4px);
            border-color: rgba(59, 130, 246, 0.6);
            box-shadow: 0 12px 28px -4px rgba(0, 0, 0, 0.6), 0 0 18px rgba(59, 130, 246, 0.25);
        }

        /* Border indicator depending on trading state */
        .vps-thumb-card.is-trading {
            border-color: rgba(16, 185, 129, 0.65);
            background: linear-gradient(180deg, rgba(16, 185, 129, 0.08) 0%, rgba(18, 25, 41, 0.96) 100%);
            box-shadow: 0 4px 18px rgba(16, 185, 129, 0.18);
        }

        .vps-thumb-card.is-trading:hover {
            box-shadow: 0 12px 28px -4px rgba(16, 185, 129, 0.35), 0 0 22px rgba(16, 185, 129, 0.3);
            border-color: var(--success);
        }

        .vps-thumb-card.is-stopped {
            border-color: rgba(239, 68, 68, 0.35);
        }

        .thumb-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: 8px;
            flex-wrap: nowrap;
        }

        .thumb-title-wrap {
            display: flex;
            align-items: center;
            gap: 6px;
            min-width: 0;
            flex: 1;
        }

        .thumb-name {
            font-size: 14px;
            font-weight: 600;
            color: #fff;
            letter-spacing: 0.3px;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }

        .thumb-link-icon {
            font-size: 13px;
            color: var(--primary);
            opacity: 0.7;
            transition: transform 0.2s, opacity 0.2s;
            flex-shrink: 0;
        }

        .vps-thumb-card:hover .thumb-link-icon {
            opacity: 1;
            transform: translate(2px, -2px);
        }

        /* 1. Status Badges: Green / Red */
        .thumb-status-badge {
            display: inline-flex;
            align-items: center;
            gap: 5px;
            padding: 3px 9px;
            border-radius: 14px;
            font-size: 11.5px;
            font-weight: 600;
            white-space: nowrap;
            flex-shrink: 0;
        }

        .badge-trading-green {
            background: rgba(16, 185, 129, 0.2);
            color: #10b981;
            border: 1px solid rgba(16, 185, 129, 0.5);
            box-shadow: 0 0 10px rgba(16, 185, 129, 0.25);
        }

        .badge-idle-red {
            background: rgba(239, 68, 68, 0.15);
            color: #ef4444;
            border: 1px solid rgba(239, 68, 68, 0.45);
        }

        .thumb-dot {
            width: 7px;
            height: 7px;
            border-radius: 50%;
            background: currentColor;
            box-shadow: 0 0 6px currentColor;
        }

        .badge-trading-green .thumb-dot {
            animation: pulse 1.6s infinite;
        }

        /* 2. Metrics 3-column Grid */
        .thumb-metrics-grid {
            display: grid;
            grid-template-columns: 1fr 1fr 1fr;
            gap: 8px;
            background: rgba(8, 12, 20, 0.65);
            border: 1px solid rgba(255, 255, 255, 0.06);
            border-radius: 8px;
            padding: 9px 10px;
        }

        .thumb-metric-col {
            display: flex;
            flex-direction: column;
            gap: 2px;
        }

        .thumb-metric-label {
            font-size: 10px;
            color: var(--text-muted);
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }

        .thumb-metric-val {
            font-family: 'Share Tech Mono', monospace;
            font-size: 13.5px;
            font-weight: 700;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }

        .val-profit-pos { color: #10b981; }
        .val-profit-neg { color: #ef4444; }
        .val-balance { color: #f8fafc; }
        .val-maxloss { color: #f59e0b; }

        /* 3 & 4. Schedule and Remaining Time Box */
        .thumb-schedule-box {
            background: rgba(8, 12, 20, 0.45);
            border: 1px solid rgba(255, 255, 255, 0.05);
            border-radius: 8px;
            padding: 8px 10px;
            display: flex;
            flex-direction: column;
            gap: 5px;
            font-size: 12px;
        }

        .thumb-schedule-row {
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: 6px;
        }

        .schedule-row-label {
            color: var(--text-muted);
            display: flex;
            align-items: center;
            gap: 4px;
        }

        .schedule-time-val {
            font-family: 'Share Tech Mono', monospace;
            font-weight: 600;
            color: #93c5fd;
        }

        .schedule-rem-val {
            font-family: 'Share Tech Mono', monospace;
            font-weight: 700;
            color: var(--text-muted);
        }

        .schedule-rem-val.active {
            color: #34d399;
        }

        .schedule-rem-val.warning {
            color: #fbbf24;
        }

        .schedule-rem-val.expired {
            color: #f87171;
        }

        /* Thumbnail Footer */
        .thumb-footer {
            display: flex;
            align-items: center;
            justify-content: space-between;
            border-top: 1px dashed rgba(255, 255, 255, 0.08);
            padding-top: 8px;
            margin-top: 2px;
            font-size: 11px;
            color: var(--text-muted);
        }

        .thumb-url-text {
            font-family: 'Share Tech Mono', monospace;
            color: #64748b;
            font-size: 11px;
        }

        .thumb-open-btn {
            color: var(--primary);
            font-weight: 500;
            display: flex;
            align-items: center;
            gap: 4px;
            transition: color 0.2s, transform 0.2s;
        }

        .vps-thumb-card:hover .thumb-open-btn {
            color: var(--accent);
            transform: translateX(3px);
        }

        /* Strategy Display (Yellow) & View Trade List Button */
        .thumb-strategy-box {
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 6px;
            margin: 8px 0 6px 0;
            padding: 4px 8px;
            background: rgba(0, 0, 0, 0.25);
            border-radius: 6px;
            border: 1px solid rgba(255, 255, 255, 0.05);
        }

        .thumb-strategy-label {
            font-size: 11px;
            color: var(--text-muted);
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }

        .thumb-strategy-val {
            font-family: 'Share Tech Mono', monospace;
            font-size: 12.5px;
            font-weight: 700;
            color: yellow;
            letter-spacing: 0.5px;
            text-shadow: 0 0 6px rgba(255, 255, 0, 0.4);
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }

        .thumb-view-trade-btn {
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 6px;
            width: 100%;
            padding: 8px 12px;
            margin-bottom: 6px;
            background: rgba(30, 41, 59, 0.7);
            border: 1px solid rgba(59, 130, 246, 0.35);
            border-radius: 8px;
            color: #f8fafc;
            font-size: 12.5px;
            font-weight: 600;
            cursor: pointer;
            box-shadow: 0 2px 4px rgba(0, 0, 0, 0.25);
            transition: all 0.2s ease;
            text-decoration: none;
        }

        .thumb-view-trade-btn:hover {
            background: rgba(59, 130, 246, 0.25);
            border-color: rgba(59, 130, 246, 0.7);
            color: #fff;
            transform: translateY(-1px);
            box-shadow: 0 4px 10px rgba(59, 130, 246, 0.3);
        }

        /* ================================================================= */
        /* Global Aggregate Summary Banner (ยอดรวมทุก VPS บนสุด)           */
        /* ================================================================= */
        .global-summary-bar {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
            gap: 14px;
            margin-bottom: 22px;
        }

        .summary-stat-card {
            background: rgba(18, 25, 41, 0.75);
            border: 1px solid rgba(255, 255, 255, 0.08);
            backdrop-filter: blur(16px);
            border-radius: 12px;
            padding: 14px 18px;
            display: flex;
            align-items: center;
            gap: 14px;
            box-shadow: 0 4px 16px rgba(0, 0, 0, 0.25);
            transition: transform 0.2s ease, border-color 0.2s ease, box-shadow 0.2s ease;
        }

        .summary-stat-card:hover {
            transform: translateY(-2px);
            border-color: rgba(59, 130, 246, 0.45);
            box-shadow: 0 6px 20px rgba(0, 0, 0, 0.35);
        }

        .stat-icon-wrap {
            width: 44px;
            height: 44px;
            border-radius: 10px;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 22px;
            flex-shrink: 0;
        }

        .stat-info {
            display: flex;
            flex-direction: column;
            gap: 2px;
            min-width: 0;
        }

        .stat-label {
            font-size: 11px;
            font-weight: 500;
            color: var(--text-muted);
            text-transform: uppercase;
            letter-spacing: 0.5px;
            white-space: nowrap;
        }

        .stat-value {
            font-family: 'Share Tech Mono', monospace;
            font-size: 18px;
            font-weight: 700;
            color: var(--text-main);
            letter-spacing: 0.5px;
        }

        .stat-value.val-profit {
            color: #10b981;
        }

        .stat-value.val-balance {
            color: #60a5fa;
        }
    </style>
</head>
<body>

<div class="container">
    <!-- Top Header -->
    <div class="top-header">
        <div class="brand-title">
            <div class="brand-icon">⚡</div>
            <div>
                <h1>VPS Trade Schedule Center</h1>
                <p>ศูนย์ควบคุมและกำหนดเวลาเทรด Multi-Cloud (lovetoshopmall.com)</p>
            </div>
        </div>
        <div class="global-actions">
            <button class="btn btn-outline" id="btnRefreshAll" onclick="fetchAllVpsData()">
                🔄 โหลดข้อมูลใหม่ทุกเครื่อง
            </button>
            <button class="btn btn-primary" onclick="syncAllCurrentTimes()">
                ⏱️ ตั้งเวลาปัจจุบัน +60 นาทีทุกเครื่อง
            </button>
            <button class="btn btn-success" onclick="saveAllVps()">
                💾 บันทึกทุกเครื่องพร้อมกัน
            </button>
            <button class="btn btn-danger" onclick="stopAllTrade()">
                🛑 หยุดเทรดทุกเครื่อง
            </button>
        </div>
    </div>

    <!-- ================================================================= -->
    <!-- Global Aggregate Summary Banner (ยอดรวมทุก VPS บนสุด)           -->
    <!-- ================================================================= -->
    <div class="global-summary-bar" id="globalSummaryBar">
        <div class="summary-stat-card">
            <div class="stat-icon-wrap" style="background: rgba(16, 185, 129, 0.12); color: #10b981;">💰</div>
            <div class="stat-info">
                <div style="display: flex; justify-content: space-between; align-items: center; gap: 8px;">
                    <span class="stat-label">ยอดกำไรรวมทุก VPS (Profit)</span>
                    <span id="globalUsdThbRateBadge" style="font-size: 10px; color: #94a3b8; font-family: 'Share Tech Mono', monospace; background: rgba(255,255,255,0.06); padding: 1px 6px; border-radius: 4px;" title="อัตราแลกเปลี่ยน USD/THB ล่าสุด">1$ = 33.36฿</span>
                </div>
                <div style="display: flex; align-items: baseline; gap: 8px; flex-wrap: wrap;">
                    <span class="stat-value val-profit" id="globalTotalProfit">$0.00</span>
                    <span class="stat-value-thb" id="globalTotalProfitThb" style="font-family: 'Share Tech Mono', monospace; font-size: 14.5px; font-weight: 700; color: #34d399;">(฿0.00)</span>
                </div>
            </div>
        </div>

        <div class="summary-stat-card">
            <div class="stat-icon-wrap" style="background: rgba(59, 130, 246, 0.12); color: #3b82f6;">🏦</div>
            <div class="stat-info">
                <span class="stat-label">ยอดเงินรวมทุก VPS (Balance)</span>
                <span class="stat-value val-balance" id="globalTotalBalance">$0.00</span>
            </div>
        </div>

        <div class="summary-stat-card">
            <div class="stat-icon-wrap" style="background: rgba(0, 210, 180, 0.12); color: #00d2b4;">🤖</div>
            <div class="stat-info">
                <span class="stat-label">กำลังเทรด (Active Bots)</span>
                <span class="stat-value" id="globalActiveTrading">0 / 4 เครื่อง</span>
            </div>
        </div>

        <div class="summary-stat-card">
            <div class="stat-icon-wrap" style="background: rgba(168, 85, 247, 0.12); color: #a855f7;">🌐</div>
            <div class="stat-info">
                <span class="stat-label">สถานะออนไลน์ (Online VPS)</span>
                <span class="stat-value" id="globalOnlineCount">0 / 4 ออนไลน์</span>
            </div>
        </div>

        <div class="summary-stat-card">
            <div class="stat-icon-wrap" style="background: rgba(239, 68, 68, 0.12); color: #ef4444;">⚠️</div>
            <div class="stat-info">
                <span class="stat-label">MaxLoss สูงสุด (Highest Streak)</span>
                <span class="stat-value" id="globalHighestLoss" style="color: #f87171;">0</span>
            </div>
        </div>
    </div>

    <!-- ================================================================= -->
    <!-- VPS Thumbnail Live Overview (ตำแหน่งบนสุดตาม Requirements)        -->
    <!-- ================================================================= -->
    <div class="vps-overview-panel">
        <div class="vps-overview-header">
            <div class="overview-title-group">
                <div class="overview-icon">📊</div>
                <div>
                    <h3>VPS Live Status Overview (Thumbnails)</h3>
                    <p>แสดงสถานะและข้อมูลการเทรดสดของแต่ละ VPS (คลิกที่ thumbnail เพื่อเปิดหน้าเทรดในแท็บใหม่)</p>
                </div>
            </div>

            <!-- Requirement #6: Switch Box + Textbox ให้ตั้งเวลา Request ข้อมูล -->
            <div class="polling-control-box">
                <div class="polling-switch-group">
                    <label class="switch switch-sm" title="เปิด/ปิดการดึงข้อมูลอัตโนมัติ">
                        <input type="checkbox" id="autoPollToggle" checked>
                        <span class="slider"></span>
                    </label>
                    <span class="polling-label">ดึงข้อมูลอัตโนมัติ</span>
                </div>
                <div class="polling-interval-group">
                    <span>ทุก</span>
                    <input type="number" id="pollIntervalInput" min="2" max="300" value="62" class="poll-input" title="ระบุจำนวนวินาที (2-300 วินาที)">
                    <span>วินาที</span>
                </div>
                <span class="poll-status-badge" id="pollStatusBadge">
                    <span class="thumb-dot"></span>
                    <span id="pollCountdownText">ดึงข้อมูลในอีก 62s</span>
                </span>
                <button class="btn btn-outline btn-xs" onclick="fetchAllVpsData(true)" title="ดึงข้อมูลเดี๋ยวนี้">
                    ⚡ ดึงข้อมูลทันที
                </button>
            </div>
        </div>

        <!-- Thumbnails Grid Container -->
        <div class="vps-thumbnails-grid" id="vpsThumbnailsGrid">
            <?php foreach ($vpsList as $vps): 
                $vpsId = htmlspecialchars($vps['id']);
                $vpsName = htmlspecialchars($vps['vpsName']);
                $targetWebUrl = resolveVpsWebUrl($vps);
                $hostDisplay = parse_url($targetWebUrl, PHP_URL_HOST) ?? $targetWebUrl;
                if (!empty($vps['portno'])) {
                    $hostDisplay .= ':' . $vps['portno'];
                }
            ?>
            <!-- Requirement #5: เมื่อคลิกที่ thumbnail ให้เปิดแท็บใหม่ไปยัง URL เช่น https://gpkderiv.shop/index_short_term.html -->
            <a href="<?= htmlspecialchars($targetWebUrl) ?>" target="_blank" rel="noopener noreferrer" 
               class="vps-thumb-card is-stopped" id="thumb-card-<?= $vpsId ?>"
               data-id="<?= $vpsId ?>" data-name="<?= $vpsName ?>" data-url="<?= htmlspecialchars($targetWebUrl) ?>"
               title="คลิกเพื่อเปิด <?= htmlspecialchars($targetWebUrl) ?> ในแท็บใหม่">
                
                <!-- Header: Name & Status Badge -->
                <div class="thumb-header">
                    <div class="thumb-title-wrap">
                        <span class="thumb-name"><?= $vpsName ?></span>
                        <span class="thumb-link-icon" title="เปิดหน้าเว็บในแท็บใหม่">↗</span>
                    </div>
                    <!-- Requirement #1: สถานะการเทรด เป็น เขียว หรือ แดง -->
                    <span class="thumb-status-badge badge-idle-red" id="thumb-badge-<?= $vpsId ?>">
                        <span class="thumb-dot"></span>
                        <span class="thumb-status-text" id="thumb-status-text-<?= $vpsId ?>">🔴 ไม่ได้เทรด</span>
                    </span>
                </div>

                <!-- Requirement #2: ข้อมูล Profit, Balance, MaxLoss Con ณ ปัจจุบัน -->
                <div class="thumb-metrics-grid">
                    <div class="thumb-metric-col">
                        <span class="thumb-metric-label">Profit</span>
                        <span class="thumb-metric-val" id="thumb-profit-<?= $vpsId ?>">$0.00</span>
                    </div>
                    <div class="thumb-metric-col">
                        <span class="thumb-metric-label">Balance</span>
                        <span class="thumb-metric-val val-balance" id="thumb-balance-<?= $vpsId ?>">$0.00</span>
                    </div>
                    <div class="thumb-metric-col">
                        <span class="thumb-metric-label">MaxLoss Con</span>
                        <span class="thumb-metric-val val-maxloss" id="thumb-maxloss-<?= $vpsId ?>">-</span>
                    </div>
                </div>

                <!-- Requirement #3 & #4: เวลาเริ่ม-สิ้นสุด (hh:mm ถึง hh:mm) และ นาทีที่เหลือ -->
                <div class="thumb-schedule-box">
                    <div class="thumb-schedule-row">
                        <span class="schedule-row-label">⏱️ เวลาเทรด:</span>
                        <!-- Requirement #3: ถ้ากำลังเทรด ให้แสดง hh:mm ถึง hh:mm -->
                        <span class="schedule-time-val" id="thumb-timerange-<?= $vpsId ?>">-</span>
                    </div>
                    <div class="thumb-schedule-row">
                        <span class="schedule-row-label">⏳ นาทีที่เหลือ:</span>
                        <!-- Requirement #4: แสดงจำนวนนาทีที่เหลือในการเทรด -->
                        <span class="schedule-rem-val" id="thumb-remaining-<?= $vpsId ?>">-</span>
                    </div>
                </div>

                <!-- Strategy Display (font color = yellow) เหนือปุ่ม View Trade List -->
                <div class="thumb-strategy-box">
                    <span class="thumb-strategy-label">Strategy:</span>
                    <span class="thumb-strategy-val" id="thumb-strategy-<?= $vpsId ?>">-</span>
                </div>

                <!-- ปุ่ม View Trade List -->
                <button type="button" class="thumb-view-trade-btn" onclick="event.preventDefault(); event.stopPropagation(); window.open('<?= htmlspecialchars($targetWebUrl) ?>', '_blank')">
                    <span>📋 View Trade List</span>
                </button>

                <!-- Footer: URL Preview and Open Hint -->
                <div class="thumb-footer">
                    <span class="thumb-url-text"><?= htmlspecialchars($hostDisplay) ?></span>
                    <span class="thumb-open-btn">เปิดหน้าเทรด ➔</span>
                </div>
            </a>
            <?php endforeach; ?>
        </div>
    </div>

    <!-- VPS Grid Cards -->
    <div class="vps-grid">
        <?php foreach ($vpsList as $vps): 
            $vpsId = $vps['id'];
            $vpsName = htmlspecialchars($vps['vpsName']);
            $targetUrl = resolveVpsBaseUrl($vps);
            $derivAcc = htmlspecialchars($vps['DerivAccountID']);
            $defaultMode = !empty($vps['portno']) ? 'direct' : 'proxy';
        ?>
        <div class="vps-card" id="card-<?= $vpsId ?>" data-id="<?= $vpsId ?>" data-url="<?= htmlspecialchars($targetUrl) ?>" data-name="<?= $vpsName ?>" data-mode="<?= $defaultMode ?>">
            <!-- Header -->
            <div class="card-header">
                <div class="card-title-group">
                    <h2>
                        <span>🖥️ <?= $vpsName ?></span>
                    </h2>
                    <div class="vps-meta">
                        <span>🔗 <?= htmlspecialchars($targetUrl) ?></span>
                        <span>👤 Acc: <?= $derivAcc ?></span>
                        <span id="mode-badge-<?= $vpsId ?>" class="badge-mode <?= $defaultMode === 'direct' ? 'direct' : '' ?>" onclick="toggleConnectionMode('<?= $vpsId ?>')" title="คลิกเพื่อสลับโหมดเชื่อมต่อ">
                            <?= $defaultMode === 'direct' ? '🌐 Browser Direct' : '⚡ PHP Proxy' ?>
                        </span>
                    </div>
                </div>
                <!-- OnTradeStatus Badge -->
                <div id="badge-<?= $vpsId ?>" class="badge badge-idle">
                    <span class="pulse-dot"></span>
                    <span class="status-label">กำลังตรวจสอบ...</span>
                </div>
            </div>

            <!-- Form -->
            <div class="form-group">
                <label class="form-label">ชื่อ VPS (vpsName)</label>
                <input type="text" class="form-control" value="<?= $vpsName ?>" readonly style="background: rgba(255,255,255,0.03); color: var(--text-muted);">
            </div>

            <div class="form-row">
                <div class="form-group">
                    <div style="display:flex; justify-content:space-between; align-items:center;">
                        <label class="form-label">เวลาเริ่มต้น (startdatetime)</label>
                        <button type="button" class="btn btn-outline btn-sm" onclick="setVpsNow('<?= $vpsId ?>')">ปัจจุบัน</button>
                    </div>
                    <input type="datetime-local" class="form-control" id="start-<?= $vpsId ?>">
                </div>

                <div class="form-group">
                    <div style="display:flex; justify-content:space-between; align-items:center;">
                        <label class="form-label">เวลาสิ้นสุด (stopdatetime)</label>
                        <button type="button" class="btn btn-outline btn-sm" onclick="addMinutesVps('<?= $vpsId ?>', 60)">+60 น.</button>
                    </div>
                    <input type="datetime-local" class="form-control" id="stop-<?= $vpsId ?>">
                </div>
            </div>

            <!-- Toggle Switch useSchedule -->
            <div class="toggle-group">
                <div class="toggle-text">
                    <span>ใช้ตารางเวลา (useSchedule)</span>
                    <span class="toggle-subtext">เปิดเพื่อให้บอทออกเทรดอัตโนมัติเมื่อถึง startdatetime</span>
                </div>
                <label class="switch">
                    <input type="checkbox" id="schedule-<?= $vpsId ?>" checked>
                    <span class="slider"></span>
                </label>
            </div>

            <!-- Card Actions -->
            <div class="card-actions">
                <span class="status-msg" id="msg-<?= $vpsId ?>">รอโหลดข้อมูล...</span>
                <div class="quick-actions">
                    <button class="btn btn-outline btn-sm" onclick="fetchSingleVpsData('<?= $vpsId ?>')">
                        🔄 รีเฟรช
                    </button>
                    <button class="btn btn-danger btn-sm" id="btnStop-<?= $vpsId ?>" onclick="stopVpsTrade('<?= $vpsId ?>')">
                        🛑 หยุดเทรด
                    </button>
                    <button class="btn btn-warning btn-sm" id="btnRestart-<?= $vpsId ?>" onclick="restartVpsService('<?= $vpsId ?>')">
                        ⚡ Restart Service
                    </button>
                    <button class="btn btn-success btn-sm" id="btnSave-<?= $vpsId ?>" onclick="saveSingleVps('<?= $vpsId ?>')">
                        💾 บันทึกเวลา
                    </button>
                </div>
            </div>
        </div>
        <?php endforeach; ?>
    </div>
</div>

<script>
// แปลง Datetime Local ให้เป็น Format YYYY-MM-DD HH:MM:SS สำหรับ Rust
function formatToRustDateTime(localIso) {
    if (!localIso) return null;
    return localIso.replace("T", " ") + ":00";
}

// แปลงจาก Rust DateTime (YYYY-MM-DD HH:MM:SS) ให้เป็น format datetime-local (YYYY-MM-DDTHH:MM)
function parseRustToInput(dtStr) {
    if (!dtStr) return "";
    let clean = dtStr.replace(" ", "T");
    if (clean.length >= 16) {
        return clean.substring(0, 16);
    }
    return clean;
}

// ฟังก์ชันสร้างเวลา ISO สำหรับใส่ datetime-local input
function getLocalISODate(d) {
    const tzoffset = d.getTimezoneOffset() * 60000;
    return (new Date(d.getTime() - tzoffset)).toISOString().slice(0, 16);
}

// สลับโหมดการเชื่อมต่อระหว่าง PHP Proxy และ Browser Direct
function toggleConnectionMode(vpsId) {
    const card = document.getElementById(`card-${vpsId}`);
    const badge = document.getElementById(`mode-badge-${vpsId}`);
    const currentMode = card.getAttribute('data-mode') || 'proxy';
    const newMode = currentMode === 'proxy' ? 'direct' : 'proxy';

    card.setAttribute('data-mode', newMode);
    if (newMode === 'direct') {
        badge.className = 'badge-mode direct';
        badge.textContent = '🌐 Browser Direct';
    } else {
        badge.className = 'badge-mode';
        badge.textContent = '⚡ PHP Proxy';
    }
    fetchSingleVpsData(vpsId);
}

// Data store สำหรับเก็บค่า Metrics ของแต่ละ VPS
const vpsMetricsStore = {};

// ป้องกันการวนลูป Auto-Fallback ไม่สิ้นสุด (Proxy ↔ Direct)
const vpsFallbackAttempts = {};

// Helper แปลงเวลาให้เป็น HH:mm
function extractHHmm(dtStr) {
    if (!dtStr) return null;
    const parts = dtStr.trim().split(/[\sT]/);
    if (parts.length >= 2) {
        const timeParts = parts[1].split(':');
        if (timeParts.length >= 2) {
            return `${timeParts[0].padStart(2, '0')}:${timeParts[1].padStart(2, '0')}`;
        }
    }
    return null;
}

// Helper ค้นหา Node ใน summary สำหรับ Browser Direct Mode
function findNodeForVpsInSummary(nodes, vpsId, vpsName) {
    if (!Array.isArray(nodes) || nodes.length === 0) return null;
    const aliasesMap = {
        '1': ['aws', 'pkderiv.online', 'node-1'],
        '2': ['gcp', 'google cloud', 'google', 'gpkderiv.shop', 'node-2'],
        '3': ['oracle3', 'oracle-3', 'oracle 3', 'oracle-free-tier-3', '161.118.203.228', 'node-3'],
        '4': ['oracle4', 'oracle-4', 'oracle 4', 'oracle-free-tier-4', '161.118.217.177', 'node-4']
    };
    const aliases = (aliasesMap[String(vpsId)] || []).concat([String(vpsName || '').toLowerCase()]);

    for (const node of nodes) {
        const nodeName = String(node.node_name || '').toLowerCase().trim();
        const nodeClean = nodeName.replace(/[^a-z0-9]/g, '');
        for (const alias of aliases) {
            const aliasClean = alias.replace(/[^a-z0-9]/g, '');
            if (nodeName === alias || nodeClean === aliasClean) return node;
            if (aliasClean && (nodeClean.includes(aliasClean) || aliasClean.includes(nodeClean))) return node;
        }
    }
    return nodes.length === 1 ? nodes[0] : null;
}

// ฟังก์ชันอัปเดตข้อมูลบน Thumbnail Card ของ VPS แต่ละตัว
function updateVpsThumbnail(vpsId, data) {
    const thumbCard = document.getElementById(`thumb-card-${vpsId}`);
    if (!thumbCard) return;

    const badgeEl = document.getElementById(`thumb-badge-${vpsId}`);
    const badgeTextEl = document.getElementById(`thumb-status-text-${vpsId}`);
    const profitEl = document.getElementById(`thumb-profit-${vpsId}`);
    const balanceEl = document.getElementById(`thumb-balance-${vpsId}`);
    const maxLossEl = document.getElementById(`thumb-maxloss-${vpsId}`);
    const timeRangeEl = document.getElementById(`thumb-timerange-${vpsId}`);

    vpsMetricsStore[vpsId] = {
        isOnline: data.isOnline !== false,
        isTrading: Boolean(data.isTrading),
        startTradeTime: data.startTradeTime || null,
        stopTradeTime: data.stopTradeTime || null,
        profit: typeof data.profit !== 'undefined' ? data.profit : 0,
        balance: typeof data.balance !== 'undefined' ? data.balance : 0,
        maxLossCon: data.maxLossCon !== undefined ? data.maxLossCon : null,
        maxLossAsset: data.maxLossAsset || ''
    };

    // 1. สถานะการเทรด เป็น เขียว หรือ แดง
    if (data.isTrading) {
        thumbCard.className = "vps-thumb-card is-trading";
        badgeEl.className = "thumb-status-badge badge-trading-green";
        badgeTextEl.textContent = "🟢 กำลังเทรด";
    } else {
        thumbCard.className = "vps-thumb-card is-stopped";
        badgeEl.className = "thumb-status-badge badge-idle-red";
        badgeTextEl.textContent = (data.isOnline === false) ? "🔴 ออฟไลน์" : "🔴 ไม่ได้เทรด";
    }

    // 2. ข้อมูล Profit, Balance, MaxLoss Con ณ ปัจจุบัน
    const profit = parseFloat(data.profit) || 0;
    if (profit > 0) {
        profitEl.textContent = `+$${profit.toFixed(2)}`;
        profitEl.style.color = "#10b981"; // สีเขียวเมื่อกำไร > 0
        profitEl.className = "thumb-metric-val val-profit-pos";
    } else if (profit < 0) {
        profitEl.textContent = `-$${Math.abs(profit).toFixed(2)}`;
        profitEl.style.color = "#ef4444"; // สีแดงเมื่อกำไร < 0
        profitEl.className = "thumb-metric-val val-profit-neg";
    } else {
        profitEl.textContent = "$0.00";
        profitEl.style.color = "var(--text-muted)"; // สีเทาเมื่อเท่ากับ 0
        profitEl.className = "thumb-metric-val";
    }

    const balance = parseFloat(data.balance) || 0;
    balanceEl.textContent = `$${balance.toLocaleString(undefined, {minimumFractionDigits: 2, maximumFractionDigits: 2})}`;

    const maxLoss = data.maxLossCon;
    if (maxLoss !== null && maxLoss !== undefined) {
        if (data.maxLossAsset) {
            maxLossEl.innerHTML = `<span style="color:#ef4444; font-weight:700;">${maxLoss}</span> <span style="font-size:10px; color:#f87171; font-weight:600;">On ${data.maxLossAsset}</span>`;
        } else {
            maxLossEl.textContent = maxLoss;
        }
    } else {
        maxLossEl.textContent = '-';
    }

    // 3. ถ้ากำลัง เทรด ให้แสดง เวลาเริ่มเทรด - เวลาสิ้นสุดการเทรด ในรูปแบบ hh:mm ถึง hh:mm
    const startHHmm = extractHHmm(data.startTradeTime);
    const stopHHmm = extractHHmm(data.stopTradeTime);

    if (data.isTrading && startHHmm && stopHHmm) {
        timeRangeEl.textContent = `${startHHmm} ถึง ${stopHHmm}`;
        timeRangeEl.style.color = "#34d399";
    } else if (startHHmm && stopHHmm) {
        timeRangeEl.textContent = `${startHHmm} ถึง ${stopHHmm}`;
        timeRangeEl.style.color = "var(--text-muted)";
    } else {
        timeRangeEl.textContent = "-";
        timeRangeEl.style.color = "var(--text-muted)";
    }

    // 4. แสดง จำนวนนาทีที่เหลือ ในการเทรด
    updateSingleRemainingTime(vpsId);

    // 5. แสดง suggestStrategy เหนือปุ่ม View Trade List ด้วย font color = yellow
    const strategyEl = document.getElementById(`thumb-strategy-${vpsId}`);
    if (strategyEl) {
        if (data.suggestStrategy) {
            strategyEl.textContent = data.suggestStrategy;
            strategyEl.style.color = "yellow";
        } else if (data.isOnline === false) {
            strategyEl.textContent = "-";
            strategyEl.style.color = "var(--text-muted)";
        } else {
            strategyEl.textContent = "-";
            strategyEl.style.color = "yellow";
        }
    }

    // 6. อัปเดตยอดรวมทุก VPS ที่อยู่บนสุด (Global Aggregate Summary)
    updateGlobalSummary();
}

// ฟังก์ชันคำนวณและอัปเดตยอดรวมทุก VPS ที่อยู่บนสุด
function updateGlobalSummary() {
    let totalProfit = 0;
    let totalBalance = 0;
    let tradingCount = 0;
    let onlineCount = 0;
    let highestLossStreak = 0;
    let highestLossAsset = '';

    const vpsKeys = Object.keys(vpsMetricsStore);
    const totalCount = vpsKeys.length || 4;

    for (const vpsId of vpsKeys) {
        const m = vpsMetricsStore[vpsId];
        if (!m) continue;
        if (m.isOnline) onlineCount++;
        if (m.isTrading) tradingCount++;
        totalProfit += parseFloat(m.profit) || 0;
        totalBalance += parseFloat(m.balance) || 0;
        if (m.maxLossCon !== null && m.maxLossCon !== undefined && m.maxLossCon > highestLossStreak) {
            highestLossStreak = m.maxLossCon;
            highestLossAsset = m.maxLossAsset || '';
        }
    }

    const elProfit = document.getElementById('globalTotalProfit');
    const elProfitThb = document.getElementById('globalTotalProfitThb');
    const elBalance = document.getElementById('globalTotalBalance');
    const elTrading = document.getElementById('globalActiveTrading');
    const elOnline = document.getElementById('globalOnlineCount');
    const elMaxLoss = document.getElementById('globalHighestLoss');

    const totalProfitThb = totalProfit * currentUsdThbRate;

    if (elProfit) {
        if (totalProfit > 0) {
            elProfit.textContent = `+$${totalProfit.toFixed(2)}`;
            elProfit.style.color = '#10b981';
        } else if (totalProfit < 0) {
            elProfit.textContent = `-$${Math.abs(totalProfit).toFixed(2)}`;
            elProfit.style.color = '#ef4444';
        } else {
            elProfit.textContent = '$0.00';
            elProfit.style.color = 'var(--text-muted)';
        }
    }

    if (elProfitThb) {
        if (totalProfit > 0) {
            elProfitThb.textContent = `(+฿${totalProfitThb.toLocaleString(undefined, {minimumFractionDigits: 2, maximumFractionDigits: 2})})`;
            elProfitThb.style.color = '#34d399';
        } else if (totalProfit < 0) {
            elProfitThb.textContent = `(-฿${Math.abs(totalProfitThb).toLocaleString(undefined, {minimumFractionDigits: 2, maximumFractionDigits: 2})})`;
            elProfitThb.style.color = '#f87171';
        } else {
            elProfitThb.textContent = '(฿0.00)';
            elProfitThb.style.color = 'var(--text-muted)';
        }
    }

    if (elBalance) {
        elBalance.textContent = `$${totalBalance.toLocaleString(undefined, {minimumFractionDigits: 2, maximumFractionDigits: 2})}`;
    }

    if (elTrading) {
        elTrading.innerHTML = `<span style="color:${tradingCount > 0 ? '#10b981' : 'var(--text-muted)'}; font-weight:700;">${tradingCount}</span> / ${totalCount} เครื่อง`;
    }

    if (elOnline) {
        elOnline.innerHTML = `<span style="color:${onlineCount === totalCount ? '#10b981' : '#fbbf24'}; font-weight:700;">${onlineCount}</span> / ${totalCount} ออนไลน์`;
    }

    if (elMaxLoss) {
        if (highestLossStreak > 0) {
            elMaxLoss.innerHTML = `<span style="color:#ef4444; font-weight:700;">${highestLossStreak}</span> ${highestLossAsset ? `<span style="font-size:11px; color:#f87171;">(${highestLossAsset})</span>` : ''}`;
        } else {
            elMaxLoss.textContent = '0';
        }
    }
}

// คำนวณและแสดงจำนวนนาทีที่เหลือในการเทรด
function updateSingleRemainingTime(vpsId) {
    const m = vpsMetricsStore[vpsId];
    const remainingEl = document.getElementById(`thumb-remaining-${vpsId}`);
    if (!m || !remainingEl) return;

    if (m.isTrading && m.stopTradeTime) {
        const stopDate = new Date(m.stopTradeTime.replace(" ", "T"));
        const now = new Date();
        if (!isNaN(stopDate.getTime())) {
            const diffMs = stopDate.getTime() - now.getTime();
            const diffMins = Math.floor(diffMs / 60000);
            const diffSecs = Math.floor((diffMs % 60000) / 1000);

            if (diffMins > 0) {
                if (diffMins >= 60) {
                    const h = Math.floor(diffMins / 60);
                    const remM = diffMins % 60;
                    remainingEl.textContent = `${h} ชม. ${remM} นาที (${diffMins} นาที)`;
                } else {
                    remainingEl.textContent = `${diffMins} นาที (${diffSecs}s)`;
                }
                remainingEl.className = "schedule-rem-val active";
            } else if (diffMs > 0) {
                remainingEl.textContent = `< 1 นาที (${diffSecs}s)`;
                remainingEl.className = "schedule-rem-val warning";
            } else {
                remainingEl.textContent = "หมดเวลาเทรดแล้ว (0 นาที)";
                remainingEl.className = "schedule-rem-val expired";
            }
            return;
        }
    }
    remainingEl.textContent = "-";
    remainingEl.className = "schedule-rem-val";
}

function updateAllRemainingTimes() {
    for (const vpsId in vpsMetricsStore) {
        updateSingleRemainingTime(vpsId);
    }
}
setInterval(updateAllRemainingTimes, 1000);

// 6. ระบบ Auto Polling พร้อม Switch Box + Textbox (Default: 62 วินาที)
let pollIntervalSeconds = 62;
let pollCountdown = 62;
let pollTimerInterval = null;

function setupAutoPolling() {
    const toggle = document.getElementById('autoPollToggle');
    const input = document.getElementById('pollIntervalInput');
    const badge = document.getElementById('pollStatusBadge');
    const countdownText = document.getElementById('pollCountdownText');

    if (!toggle || !input) return;

    const savedActive = localStorage.getItem('vps_auto_poll_active');
    if (savedActive !== null) {
        toggle.checked = (savedActive === 'true');
    }
    const savedSec = localStorage.getItem('vps_auto_poll_interval');
    if (savedSec) {
        input.value = Math.max(2, parseInt(savedSec, 10) || 62);
    } else {
        input.value = 62;
    }

    pollIntervalSeconds = parseInt(input.value, 10) || 62;
    pollCountdown = pollIntervalSeconds;

    toggle.addEventListener('change', () => {
        localStorage.setItem('vps_auto_poll_active', toggle.checked);
        pollCountdown = pollIntervalSeconds;
        updateBadgeUI();
    });

    input.addEventListener('change', () => {
        let val = parseInt(input.value, 10);
        if (isNaN(val) || val < 2) val = 2;
        if (val > 300) val = 300;
        input.value = val;
        pollIntervalSeconds = val;
        pollCountdown = val;
        localStorage.setItem('vps_auto_poll_interval', val);
        updateBadgeUI();
    });

    function updateBadgeUI() {
        if (toggle.checked) {
            badge.className = "poll-status-badge";
            countdownText.textContent = `ดึงข้อมูลในอีก ${pollCountdown}s`;
        } else {
            badge.className = "poll-status-badge paused";
            countdownText.textContent = "⏸️ ปิดอยู่";
        }
    }

    if (pollTimerInterval) clearInterval(pollTimerInterval);
    pollTimerInterval = setInterval(() => {
        if (!toggle.checked) {
            updateBadgeUI();
            return;
        }

        pollCountdown--;
        if (pollCountdown <= 0) {
            pollCountdown = pollIntervalSeconds;
            fetchAllVpsData(true);
        }
        updateBadgeUI();
    }, 1000);

    updateBadgeUI();
}

// -------------------------------------------------------------
// 1. ดึงข้อมูลจาก VPS แต่ละตัว
// -------------------------------------------------------------
async function fetchSingleVpsData(vpsId, isAutoPoll = false) {
    const card = document.getElementById(`card-${vpsId}`);
    const mode = card.getAttribute('data-mode') || 'proxy';
    const baseUrl = card.getAttribute('data-url');
    const msgEl = document.getElementById(`msg-${vpsId}`);
    const badgeEl = document.getElementById(`badge-${vpsId}`);

    // รีเซ็ต fallback counter เมื่อเป็นการดึงข้อมูลใหม่ (ไม่ใช่จาก toggleConnectionMode)
    if (!vpsFallbackAttempts[vpsId]) vpsFallbackAttempts[vpsId] = 0;

    if (!isAutoPoll) {
        msgEl.textContent = `⏳ กำลังดึงข้อมูล (${mode === 'direct' ? 'Direct' : 'Proxy'})...`;
        msgEl.style.color = "var(--text-muted)";
    }

    if (mode === 'direct') {
        // --- โหมด Browser Direct: ยิงตรงจากเครื่อง Client ไปยัง VPS ---
        try {
            const [tcSettled, statusSettled, setupSettled, summarySettled] = await Promise.allSettled([
                fetch(`${baseUrl}/api/tradeControl`, { cache: 'no-store' }).then(r => r.json()),
                fetch(`${baseUrl}/api/status`, { cache: 'no-store' }).then(r => r.json()),
                fetch(`${baseUrl}/api/setup`, { cache: 'no-store' }).then(r => r.json()),
                fetch(`${baseUrl}/api/multi_node_summary`, { cache: 'no-store' }).then(r => r.json())
            ]);

            const tcRes = tcSettled.status === 'fulfilled' ? tcSettled.value : null;
            const statusRes = statusSettled.status === 'fulfilled' ? statusSettled.value : null;
            const setupRes = setupSettled.status === 'fulfilled' ? setupSettled.value : null;
            const summaryRes = summarySettled.status === 'fulfilled' ? summarySettled.value : null;

            const isOnline = (tcRes && tcRes.status === 'success') || (statusRes && statusRes.status !== 'error');

            if (isOnline) {
                const tc = tcRes ? (tcRes.data || {}) : {};
                let isTrading = false;
                let tradeStatusText = 'ไม่ได้เทรด';

                if (statusRes && statusRes.is_trading) {
                    isTrading = true;
                    tradeStatusText = statusRes.active_assets && statusRes.active_assets.length > 0
                        ? 'กำลังเทรด (' + statusRes.active_assets.join(', ') + ')'
                        : 'กำลังเทรด';
                } else if (statusRes && statusRes.bot_count > 0 && statusRes.active_assets && statusRes.active_assets.length > 0) {
                    isTrading = true;
                    tradeStatusText = 'กำลังเทรด (' + statusRes.active_assets.join(', ') + ')';
                } else if (tc.TradeStatus === 'กำลังเทรด') {
                    isTrading = true;
                    tradeStatusText = 'กำลังเทรด';
                }

                // Balance
                let balance = 0.0;
                if (statusRes && typeof statusRes.balance !== 'undefined') {
                    balance = parseFloat(statusRes.balance) || 0;
                }

                // Profit & MaxLoss
                let profit = 0.0;
                let maxLossCon = (setupRes && setupRes.trade && typeof setupRes.trade.maxLossCon !== 'undefined')
                    ? setupRes.trade.maxLossCon
                    : null;
                let maxLossAsset = '';

                // Extract suggestStrategy
                let suggestStrategy = (setupRes && setupRes.trade && setupRes.trade.suggestStrategy)
                    ? setupRes.trade.suggestStrategy
                    : (setupRes && setupRes.suggestStrategy ? setupRes.suggestStrategy : null);

                // Fallback to localhost:3000 if needed
                if (!suggestStrategy && (String(vpsId) === '1' || baseUrl.includes('localhost') || baseUrl.includes('127.0.0.1'))) {
                    try {
                        const localRes = await fetch('http://localhost:3000/api/setup', { cache: 'no-store' }).then(r => r.json());
                        suggestStrategy = localRes?.trade?.suggestStrategy || localRes?.suggestStrategy || null;
                    } catch (e) {}
                }

                if (summaryRes && summaryRes.data && Array.isArray(summaryRes.data.nodes)) {
                    const node = findNodeForVpsInSummary(summaryRes.data.nodes, vpsId, card.getAttribute('data-name'));
                    if (node) {
                        if (balance === 0.0 && typeof node.balance !== 'undefined') balance = parseFloat(node.balance);
                        if (typeof node.today_profit_usd !== 'undefined') profit = parseFloat(node.today_profit_usd);
                        else if (typeof node.round_profit_usd !== 'undefined') profit = parseFloat(node.round_profit_usd);
                        if (typeof node.max_loss_streak !== 'undefined') maxLossCon = parseInt(node.max_loss_streak, 10);
                        if (node.max_loss_streak_asset) maxLossAsset = String(node.max_loss_streak_asset).trim();
                        if (node.is_trading) {
                            isTrading = true;
                            if (tradeStatusText === 'ไม่ได้เทรด') tradeStatusText = 'กำลังเทรด';
                        }
                    }
                }

                if (isTrading) {
                    badgeEl.className = "badge badge-trading";
                    badgeEl.innerHTML = `<span class="pulse-dot"></span><span class="status-label">🟢 ${tradeStatusText}</span>`;
                } else {
                    badgeEl.className = "badge badge-idle";
                    badgeEl.innerHTML = `<span class="pulse-dot"></span><span class="status-label">⚪ ${tradeStatusText}</span>`;
                }

                if (tc.startTradeTime) {
                    document.getElementById(`start-${vpsId}`).value = parseRustToInput(tc.startTradeTime);
                }
                if (tc.stopTradeTime) {
                    document.getElementById(`stop-${vpsId}`).value = parseRustToInput(tc.stopTradeTime);
                }
                if (typeof tc.useSchedule !== 'undefined') {
                    document.getElementById(`schedule-${vpsId}`).checked = Boolean(tc.useSchedule);
                }

                msgEl.textContent = `✅ อัปเดตล่าสุด: ${new Date().toLocaleTimeString()} (Direct)`;
                msgEl.style.color = "var(--success)";

                // Update Thumbnail
                updateVpsThumbnail(vpsId, {
                    isOnline: true,
                    isTrading: isTrading,
                    tradeStatusText: tradeStatusText,
                    balance: balance,
                    profit: profit,
                    maxLossCon: maxLossCon,
                    maxLossAsset: maxLossAsset,
                    startTradeTime: tc.startTradeTime,
                    stopTradeTime: tc.stopTradeTime,
                    suggestStrategy: suggestStrategy
                });
                return;
            } else {
                throw new Error("tradeControl response invalid");
            }
        } catch (err) {
            // ถ้ายังไม่เคย fallback → ลองสลับไป Proxy
            if (vpsFallbackAttempts[vpsId] < 1) {
                vpsFallbackAttempts[vpsId]++;
                console.warn(`[VPS ${vpsId}] Direct failed, fallback to Proxy (attempt ${vpsFallbackAttempts[vpsId]})`);
                toggleConnectionMode(vpsId);
                return;
            }
            // ถ้าลองมาแล้วทั้ง 2 โหมด → หยุด แสดง Offline
            vpsFallbackAttempts[vpsId] = 0;
            badgeEl.className = "badge badge-offline";
            badgeEl.innerHTML = `<span class="pulse-dot"></span><span class="status-label">Offline / ติดต่อไม่ได้</span>`;
            msgEl.textContent = `❌ ติดต่อไม่ได้ทั้ง 2 โหมด: ${err.message}`;
            msgEl.style.color = "var(--danger)";

            updateVpsThumbnail(vpsId, {
                isOnline: false,
                isTrading: false,
                tradeStatusText: 'ติดต่อไม่ได้',
                balance: 0,
                profit: 0,
                maxLossCon: null,
                maxLossAsset: '',
                startTradeTime: null,
                stopTradeTime: null
            });
            return;
        }
    }

    // --- โหมด PHP Proxy: ยิงผ่านหลังบ้าน lovetoshopmall ---
    try {
        const res = await fetch(`?action=fetch_vps_data&id=${vpsId}`);
        const text = await res.text();
        const jsonStart = text.indexOf('{');
        const jsonEnd = text.lastIndexOf('}');
        if (jsonStart === -1 || jsonEnd === -1) throw new Error("Invalid JSON: " + text.slice(0, 80));
        const data = JSON.parse(text.substring(jsonStart, jsonEnd + 1));

        if (!data.success && !data.isOnline) {
            // ถ้ายังไม่เคย fallback → ลองสลับไป Direct
            if (vpsFallbackAttempts[vpsId] < 1) {
                vpsFallbackAttempts[vpsId]++;
                console.warn(`[VPS ${vpsId}] PHP Proxy reports offline, fallback to Direct (attempt ${vpsFallbackAttempts[vpsId]})`);
                toggleConnectionMode(vpsId);
                return;
            }
            // ถ้าลองมาแล้วทั้ง 2 โหมด → หยุด แสดง Offline
            vpsFallbackAttempts[vpsId] = 0;
            badgeEl.className = "badge badge-offline";
            badgeEl.innerHTML = `<span class="pulse-dot"></span><span class="status-label">🔴 Offline</span>`;
            msgEl.textContent = `❌ VPS ออฟไลน์ (ทั้ง Proxy และ Direct): ${data.rawError || 'ไม่สามารถเชื่อมต่อได้'}`;
            msgEl.style.color = "var(--danger)";
            updateVpsThumbnail(vpsId, {
                isOnline: false, isTrading: false, tradeStatusText: 'ออฟไลน์',
                balance: 0, profit: 0, maxLossCon: null, maxLossAsset: '',
                startTradeTime: null, stopTradeTime: null
            });
            return;
        }

        // แสดง OnTradeStatus
        if (data.isTrading) {
            badgeEl.className = "badge badge-trading";
            badgeEl.innerHTML = `<span class="pulse-dot"></span><span class="status-label">🟢 ${data.tradeStatusText}</span>`;
        } else {
            badgeEl.className = "badge badge-idle";
            badgeEl.innerHTML = `<span class="pulse-dot"></span><span class="status-label">⚪ ${data.tradeStatusText}</span>`;
        }

        // นำค่า TradeControl มาใส่ Form
        if (data.tradeControl) {
            const tc = data.tradeControl;
            if (tc.startTradeTime) {
                document.getElementById(`start-${vpsId}`).value = parseRustToInput(tc.startTradeTime);
            }
            if (tc.stopTradeTime) {
                document.getElementById(`stop-${vpsId}`).value = parseRustToInput(tc.stopTradeTime);
            }
            if (typeof tc.useSchedule !== 'undefined') {
                document.getElementById(`schedule-${vpsId}`).checked = Boolean(tc.useSchedule);
            }
            msgEl.textContent = `✅ อัปเดตล่าสุด: ${new Date().toLocaleTimeString()} (Proxy)`;
            msgEl.style.color = "var(--success)";
        } else {
            msgEl.textContent = `⚠️ ติดต่อได้ แต่ไม่พบข้อมูล tradeControl`;
            msgEl.style.color = "var(--warning)";
        }

        // Update Thumbnail
        updateVpsThumbnail(vpsId, {
            isOnline: data.isOnline,
            isTrading: data.isTrading,
            tradeStatusText: data.tradeStatusText,
            balance: data.balance,
            profit: data.profit,
            maxLossCon: data.maxLossCon,
            maxLossAsset: data.maxLossAsset || '',
            startTradeTime: data.tradeControl ? data.tradeControl.startTradeTime : null,
            stopTradeTime: data.tradeControl ? data.tradeControl.stopTradeTime : null,
            suggestStrategy: data.suggestStrategy || (data.setup && data.setup.trade ? data.setup.trade.suggestStrategy : null) || (data.setup ? data.setup.suggestStrategy : null)
        });

    } catch (e) {
        // ถ้ายังไม่เคย fallback → ลองสลับไป Direct
        if (vpsFallbackAttempts[vpsId] < 1) {
            vpsFallbackAttempts[vpsId]++;
            console.warn(`[VPS ${vpsId}] Proxy fetch error, fallback to Direct (attempt ${vpsFallbackAttempts[vpsId]})`);
            toggleConnectionMode(vpsId);
        } else {
            // หยุดวนลูป แสดง Offline
            vpsFallbackAttempts[vpsId] = 0;
            badgeEl.className = "badge badge-offline";
            badgeEl.innerHTML = `<span class="pulse-dot"></span><span class="status-label">🔴 Offline</span>`;
            msgEl.textContent = `❌ ติดต่อไม่ได้ทั้ง 2 โหมด`;
            msgEl.style.color = "var(--danger)";
            updateVpsThumbnail(vpsId, {
                isOnline: false, isTrading: false, tradeStatusText: 'ออฟไลน์',
                balance: 0, profit: 0, maxLossCon: null, maxLossAsset: '',
                startTradeTime: null, stopTradeTime: null
            });
        }
    }
}

// ดึงข้อมูลครบทุกเครื่อง
async function fetchAllVpsData(isAutoPoll = false) {
    // รีเซ็ต fallback counter ทุกรอบ เพื่อให้ VPS ที่เคยล่มลองใหม่ได้
    const cards = document.querySelectorAll('.vps-card');
    for (const card of cards) {
        vpsFallbackAttempts[card.getAttribute('data-id')] = 0;
    }
    for (const card of cards) {
        const vpsId = card.getAttribute('data-id');
        fetchSingleVpsData(vpsId, isAutoPoll);
    }
}

// -------------------------------------------------------------
// 2. บันทึกข้อมูลไปยัง VPS แต่ละตัว
// -------------------------------------------------------------
async function saveSingleVps(vpsId) {
    const card = document.getElementById(`card-${vpsId}`);
    const mode = card.getAttribute('data-mode') || 'proxy';
    const baseUrl = card.getAttribute('data-url');
    const btn = document.getElementById(`btnSave-${vpsId}`);
    const msgEl = document.getElementById(`msg-${vpsId}`);
    const originalText = btn.textContent;
    btn.textContent = "⏳ กำลังบันทึก...";
    btn.disabled = true;

    const startVal = document.getElementById(`start-${vpsId}`).value;
    const stopVal = document.getElementById(`stop-${vpsId}`).value;
    const useSchedule = document.getElementById(`schedule-${vpsId}`).checked;

    const payload = {
        startTradeTime: formatToRustDateTime(startVal),
        stopTradeTime: formatToRustDateTime(stopVal),
        useSchedule: useSchedule
    };

    if (mode === 'direct') {
        try {
            const res = await fetch(`${baseUrl}/api/tradeControl`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });
            const data = await res.json();
            if (res.ok && data.status === 'success') {
                btn.textContent = "✅ บันทึกแล้ว!";
                msgEl.textContent = `บันทึกเรียบร้อย (${new Date().toLocaleTimeString()})`;
                msgEl.style.color = "var(--success)";
            } else {
                throw new Error(data.message || 'บันทึกล้มเหลว');
            }
        } catch (e) {
            btn.textContent = "❌ ล้มเหลว";
            msgEl.textContent = `บันทึกไม่สำเร็จ: ${e.message}`;
            msgEl.style.color = "var(--danger)";
        }
    } else {
        try {
            const res = await fetch('?action=save_trade_control', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ id: vpsId, ...payload })
            });

            const data = await res.json();
            if (data.success) {
                btn.textContent = "✅ บันทึกแล้ว!";
                msgEl.textContent = `บันทึกเรียบร้อย (${new Date().toLocaleTimeString()})`;
                msgEl.style.color = "var(--success)";
            } else {
                btn.textContent = "❌ ล้มเหลว";
                msgEl.textContent = `บันทึกล้มเหลว: ${data.error || 'Server error'}`;
                msgEl.style.color = "var(--danger)";
            }
        } catch (e) {
            btn.textContent = "❌ เกิดข้อผิดพลาด";
            msgEl.textContent = `ข้อผิดพลาด: ${e.message}`;
            msgEl.style.color = "var(--danger)";
        }
    }

    setTimeout(() => {
        btn.textContent = originalText;
        btn.disabled = false;
    }, 2500);
}

// บันทึกทุกเครื่องพร้อมกัน
async function saveAllVps() {
    const cards = document.querySelectorAll('.vps-card');
    for (const card of cards) {
        const vpsId = card.getAttribute('data-id');
        saveSingleVps(vpsId);
    }
}

// -------------------------------------------------------------
// 3. สั่ง Restart Service บน VPS
// -------------------------------------------------------------
async function restartVpsService(vpsId) {
    const card = document.getElementById(`card-${vpsId}`);
    const vpsName = card.getAttribute('data-name') || vpsId;
    const mode = card.getAttribute('data-mode') || 'proxy';
    const baseUrl = card.getAttribute('data-url');
    const btn = document.getElementById(`btnRestart-${vpsId}`);
    const msgEl = document.getElementById(`msg-${vpsId}`);
    const badgeEl = document.getElementById(`badge-${vpsId}`);

    const ok = confirm(`⚠️ ยืนยันการสั่ง Restart Service ของ "${vpsName}" ใช่หรือไม่?\n\n- โปรแกรมจะปิดตัวลงและระบบ systemd จะปลุกบอทขึ้นมาใหม่ใน 5 วินาที\n- บอทที่กำลังเทรดอยู่จะถูกรีเซ็ตใหม่`);
    if (!ok) return;

    btn.disabled = true;
    const originalText = btn.textContent;
    btn.textContent = "⏳ สั่งการ...";

    try {
        if (mode === 'direct') {
            await fetch(`${baseUrl}/api/system/restart`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: '{}'
            });
        } else {
            await fetch('?action=restart_service', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ id: vpsId })
            });
        }
    } catch (e) {
        // บางครั้ง service ปิดตัวทันทีทำให้ connection drop ซึ่งถือเป็นปกติ
        console.log(`[Restart ${vpsId}] Signal sent (socket closed expected):`, e);
    }

    // อัปเดตสถานะนับถอยหลัง 6 วินาที
    badgeEl.className = "badge badge-restarting";
    badgeEl.innerHTML = `<span class="pulse-dot" style="background:var(--warning);"></span><span class="status-label">🔄 กำลังรีสตาร์ท...</span>`;

    let countdown = 6;
    const timer = setInterval(() => {
        msgEl.textContent = `🔄 กำลังเริ่มเซอร์วิสใหม่... รออีก ${countdown} วินาที`;
        msgEl.style.color = "var(--warning)";
        countdown--;
        if (countdown < 0) {
            clearInterval(timer);
            msgEl.textContent = "⏳ กำลังเชื่อมต่อใหม่...";
            btn.textContent = originalText;
            btn.disabled = false;
            // ดึงข้อมูลใหม่อีกครั้ง
            fetchSingleVpsData(vpsId);
        }
    }, 1000);
}

// -------------------------------------------------------------
// 4. สั่ง Stop Trade บน VPS
// -------------------------------------------------------------
async function stopVpsTrade(vpsId) {
    const card = document.getElementById(`card-${vpsId}`);
    const vpsName = card.getAttribute('data-name') || vpsId;
    const mode = card.getAttribute('data-mode') || 'proxy';
    const baseUrl = card.getAttribute('data-url');
    const btn = document.getElementById(`btnStop-${vpsId}`);
    const msgEl = document.getElementById(`msg-${vpsId}`);

    const ok = confirm(`🛑 ยืนยันการสั่ง "หยุดเทรด" ของ "${vpsName}" ใช่หรือไม่?\n\n- บอทจะหยุดทำงานทันที\n- บันทึกสถานะ "ปิดเทรดอยู่" และส่งแจ้งเตือน Telegram`);
    if (!ok) return;

    btn.disabled = true;
    const originalText = btn.textContent;
    btn.textContent = "⏳ สั่งหยุด...";

    try {
        let resData;
        if (mode === 'direct') {
            const resp = await fetch(`${baseUrl}/api/stop`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: '{}'
            });
            resData = await resp.json();
        } else {
            const resp = await fetch('?action=stop_trade', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ id: vpsId })
            });
            resData = await resp.json();
        }

        msgEl.textContent = "🛑 สั่งหยุดเทรดเรียบร้อยแล้ว!";
        msgEl.style.color = "var(--danger)";
        setTimeout(() => {
            fetchSingleVpsData(vpsId);
        }, 1200);
    } catch (e) {
        console.error(`[StopTrade ${vpsId}] Error:`, e);
        msgEl.textContent = `❌ สั่งหยุดเทรดไม่สำเร็จ: ${e.message}`;
        msgEl.style.color = "var(--danger)";
    } finally {
        setTimeout(() => {
            btn.textContent = originalText;
            btn.disabled = false;
        }, 2000);
    }
}

// สั่งหยุดเทรดทุกเครื่องพร้อมกัน
async function stopAllTrade() {
    const ok = confirm("🚨 คำเตือน: คุณต้องการสั่ง 'หยุดเทรด' ทุกเครื่องพร้อมกันใช่หรือไม่?");
    if (!ok) return;

    const cards = document.querySelectorAll('.vps-card');
    for (const card of cards) {
        const vpsId = card.getAttribute('data-id');
        const mode = card.getAttribute('data-mode') || 'proxy';
        const baseUrl = card.getAttribute('data-url');
        const msgEl = document.getElementById(`msg-${vpsId}`);

        try {
            if (mode === 'direct') {
                fetch(`${baseUrl}/api/stop`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: '{}'
                }).then(() => fetchSingleVpsData(vpsId));
            } else {
                fetch('?action=stop_trade', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ id: vpsId })
                }).then(() => fetchSingleVpsData(vpsId));
            }
            msgEl.textContent = "🛑 ส่งคำสั่งหยุดเทรดแล้ว...";
            msgEl.style.color = "var(--danger)";
        } catch (e) {
            console.error(e);
        }
    }
}

// -------------------------------------------------------------
// Quick Time Helpers
// -------------------------------------------------------------
function setVpsNow(vpsId) {
    const now = new Date();
    document.getElementById(`start-${vpsId}`).value = getLocalISODate(now);
    addMinutesVps(vpsId, 60);
}

function addMinutesVps(vpsId, mins) {
    const startVal = document.getElementById(`start-${vpsId}`).value;
    if (!startVal) return;
    const start = new Date(startVal);
    const stop = new Date(start.getTime() + (mins * 60000));
    document.getElementById(`stop-${vpsId}`).value = getLocalISODate(stop);
}

function syncAllCurrentTimes() {
    const now = new Date();
    const nowIso = getLocalISODate(now);
    const stopIso = getLocalISODate(new Date(now.getTime() + (60 * 60000)));

    const cards = document.querySelectorAll('.vps-card');
    cards.forEach(card => {
        const vpsId = card.getAttribute('data-id');
        document.getElementById(`start-${vpsId}`).value = nowIso;
        document.getElementById(`stop-${vpsId}`).value = stopIso;
        document.getElementById(`schedule-${vpsId}`).checked = true;
    });
}

// อัตราแลกเปลี่ยน USD/THB ปัจจุบัน (Default: 33.36)
let currentUsdThbRate = 33.36;

async function fetchUsdThbRate() {
    try {
        const res = await fetch('https://api.exchangerate-api.com/v4/latest/USD', { cache: 'no-store' });
        if (res.ok) {
            const data = await res.json();
            if (data && data.rates && data.rates.THB) {
                currentUsdThbRate = parseFloat(data.rates.THB);
                console.log(`[Rate] Fetched live USD/THB rate: ${currentUsdThbRate}`);
                updateRateUI();
                updateGlobalSummary();
                return;
            }
        }
    } catch (e) {
        console.warn('[Rate] Direct fetch failed, trying proxy...', e);
    }

    try {
        const res = await fetch('?action=get_usd_thb_rate');
        if (res.ok) {
            const data = await res.json();
            if (data && data.rate) {
                currentUsdThbRate = parseFloat(data.rate);
                console.log(`[Rate] Proxy fetched USD/THB rate: ${currentUsdThbRate}`);
                updateRateUI();
                updateGlobalSummary();
                return;
            }
        }
    } catch (e) {}

    updateRateUI();
}

function updateRateUI() {
    const rateBadge = document.getElementById('globalUsdThbRateBadge');
    if (rateBadge) {
        rateBadge.textContent = `1$ = ${currentUsdThbRate.toFixed(2)}฿`;
        rateBadge.title = `อัตราแลกเปลี่ยน USD/THB ปัจจุบัน: 1 USD = ${currentUsdThbRate.toFixed(4)} THB`;
    }
}

// เมื่อเปิดเว็บขึ้นมา ให้ดึงข้อมูลทันทีและเริ่มระบบ Auto Polling
window.addEventListener('DOMContentLoaded', () => {
    updateGlobalSummary();
    fetchUsdThbRate();
    fetchAllVpsData();
    setupAutoPolling();
});
</script>

</body>
</html>

# Access-감시.ps1
# 백그라운드에서 실행되어 에이전트 프로세스를 감시합니다.
# 모든 에이전트(ZCode, Hermes 게이트웨이, OMP)가 종료되면
# Access 포스트잇과 API 서버도 함께 종료합니다.
#
# 시작: powershell -WindowStyle Hidden -File Access-감시.ps1

$agents = @("ZCode", "hermes_cli", "omp")
$pollInterval = 10  # 초

# Access-포스트잇은 단독으로도 실행될 수 있으므로,
# 감시 대상 에이전트가 하나라도 있으면 Access를 유지합니다.

# 감시 시작 전: API가 뜰 때까지 최대 30초 대기 (시작 경합 방지)
$api = $null
for ($i = 0; $i -lt 15; $i++) {
    $api = Get-Process "Access-API" -ErrorAction SilentlyContinue
    if ($api) { break }
    Start-Sleep -Seconds 2
}
if (-not $api) {
    # API가 없으면 감시할 의미 없음
    exit 0
}

$deadStreak = 0
while ($true) {
    Start-Sleep -Seconds $pollInterval

    # API 서버가 죽었으면 감시도 종료
    $api = Get-Process "Access-API" -ErrorAction SilentlyContinue
    if (-not $api) { exit 0 }

    # 에이전트 프로세스 확인
    $anyAgentAlive = $false
    foreach ($name in $agents) {
        if (Get-Process $name -ErrorAction SilentlyContinue) {
            $anyAgentAlive = $true
            break
        }
    }

    # Hermes 게이트웨이는 python.exe / pythonw.exe로 실행되므로 둘 다 확인
    # (이 PC 실측: 게이트웨이는 python.exe -m hermes_cli.main --profile <n> gateway run)
    # 조회 자체가 실패하면 "살아있음"으로 간주(보수적 유지 — 오판 종료 방지)
    if (-not $anyAgentAlive) {
        $hermesGw = $null
        try {
            $hermesGw = Get-CimInstance Win32_Process -Filter "Name='python.exe' or Name='pythonw.exe'" -ErrorAction Stop |
                Where-Object { $_.CommandLine -like "*hermes_cli*gateway*" }
        } catch { $hermesGw = $null }
        if ($hermesGw) { $anyAgentAlive = $true }
    }

    if (-not $anyAgentAlive) {
        # 3연속(30초) 죽어 있어야 정리 — 일시적 조회 실패·스파이크에 전체 종료하지 않도록
        $deadStreak++
        Add-Content -Path (Join-Path $PSScriptRoot "watchdog.log") -Value "$(Get-Date -Format o) all-dead streak=$deadStreak"
        if ($deadStreak -ge 3) {
            Stop-Process -Name "Access-포스트잇" -Force -ErrorAction SilentlyContinue
            Stop-Process -Name "Access-API" -Force -ErrorAction SilentlyContinue
            exit 0
        }
    } else {
        $deadStreak = 0
    }
}

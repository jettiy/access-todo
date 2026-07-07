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

# 감시 시작 전: Access-API가 실행 중이 아니면 종료 (보호)
$api = Get-Process "Access-API" -ErrorAction SilentlyContinue
if (-not $api) {
    # API가 없으면 감시할 의미 없음
    exit 0
}

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

    # Hermes 게이트웨이는 pythonw.exe로 실행되므로 별도 확인
    $hermesGw = Get-CimInstance Win32_Process -Filter "Name='pythonw.exe'" -ErrorAction SilentlyContinue |
        Where-Object { $_.CommandLine -like "*hermes_cli*gateway*" }
    if ($hermesGw) { $anyAgentAlive = $true }

    if (-not $anyAgentAlive) {
        # 모든 에이전트가 종료됨 → Access 정리
        Stop-Process -Name "Access-포스트잇" -Force -ErrorAction SilentlyContinue
        Stop-Process -Name "Access-API" -Force -ErrorAction SilentlyContinue
        exit 0
    }
}

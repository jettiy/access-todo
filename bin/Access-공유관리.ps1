# Access-공유관리.ps1
# 여러 에이전트가 Access를 공유합니다.
# 시작: Access-공유관리.ps1 start <agent>
# 종료: Access-공유관리.ps1 stop <agent>
# 마지막 에이전트가 종료되면 Access도 함께 종료됩니다.

param(
    [Parameter(Mandatory=$true)][string]$Action,
    [string]$Agent = "unknown"
)

$lockDir = Join-Path $env:TEMP "access-locks"
if (-not (Test-Path $lockDir)) { New-Item -ItemType Directory -Path $lockDir -Force | Out-Null }

if ($Action -eq "start") {
    # 이 에이전트의 락 파일 생성
    $lockFile = Join-Path $lockDir "$Agent.lock"
    (Get-Date).ToString("o") | Out-File -FilePath $lockFile -Encoding utf8
}
elseif ($Action -eq "stop") {
    # 자신의 락 파일 삭제
    $lockFile = Join-Path $lockDir "$Agent.lock"
    Remove-Item $lockFile -ErrorAction SilentlyContinue

    # 남은 락이 없으면 Access 종료
    $remaining = Get-ChildItem "$lockDir\*.lock" -ErrorAction SilentlyContinue
    if (-not $remaining) {
        Stop-Process -Name "Access-API" -Force -ErrorAction SilentlyContinue
        Stop-Process -Name "Access-포스트잇" -Force -ErrorAction SilentlyContinue
        Remove-Item $lockDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

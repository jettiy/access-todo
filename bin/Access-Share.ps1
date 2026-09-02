param(
    [Parameter(Mandatory=$true)][string]$Action,
    [string]$Agent = "unknown"
)
$lockDir = Join-Path $env:TEMP "access-locks"
if (-not (Test-Path $lockDir)) { New-Item -ItemType Directory -Path $lockDir -Force | Out-Null }
if ($Action -eq "start") {
    $lockFile = Join-Path $lockDir "$Agent.lock"
    (Get-Date).ToString("o") | Out-File -FilePath $lockFile -Encoding utf8
}
elseif ($Action -eq "stop") {
    $lockFile = Join-Path $lockDir "$Agent.lock"
    Remove-Item $lockFile -ErrorAction SilentlyContinue
    $remaining = Get-ChildItem "$lockDir\*.lock" -ErrorAction SilentlyContinue
    if (-not $remaining) {
        Stop-Process -Name "Access-API" -Force -ErrorAction SilentlyContinue
        Stop-Process -Name "Access-Postit" -Force -ErrorAction SilentlyContinue
        Remove-Item $lockDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

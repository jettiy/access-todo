# Access-Watch.ps1 - monitors agent processes, shuts down Access when all agents exit
$agents = @("ZCode", "hermes_cli", "omp")
$pollInterval = 10

$api = Get-Process "Access-API" -ErrorAction SilentlyContinue
if (-not $api) { exit 0 }

while ($true) {
    Start-Sleep -Seconds $pollInterval
    $api = Get-Process "Access-API" -ErrorAction SilentlyContinue
    if (-not $api) { exit 0 }

    $anyAgentAlive = $false
    foreach ($name in $agents) {
        if (Get-Process $name -ErrorAction SilentlyContinue) { $anyAgentAlive = $true; break }
    }
    $hermesGw = Get-CimInstance Win32_Process -Filter "Name='pythonw.exe'" -ErrorAction SilentlyContinue |
        Where-Object { $_.CommandLine -like "*hermes_cli*gateway*" }
    if ($hermesGw) { $anyAgentAlive = $true }

    if (-not $anyAgentAlive) {
        Stop-Process -Name "Access-Postit" -Force -ErrorAction SilentlyContinue
        Stop-Process -Name "Access-API" -Force -ErrorAction SilentlyContinue
        exit 0
    }
}

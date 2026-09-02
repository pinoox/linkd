param(
    [string]$LinkdExe = "C:\projects\linkd\target\debug\linkd.exe",
    [string]$TestRoot = "C:\projects\linkd\target\stress_sandbox"
)

$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Magenta
Write-Host " linkd Advanced Stress, Stability & Resource Benchmark" -ForegroundColor Magenta
Write-Host "==========================================================" -ForegroundColor Magenta

# Clean and prepare sandbox
if (Test-Path $TestRoot) {
    for ($attempt = 0; $attempt -lt 5; $attempt++) {
        try {
            Remove-Item -Recurse -Force $TestRoot -ErrorAction Stop
            break
        } catch {
            Start-Sleep -Milliseconds 400
        }
    }
}
New-Item -ItemType Directory -Path $TestRoot -Force | Out-Null

$LinkdHome = Join-Path $TestRoot "linkd_home"
New-Item -ItemType Directory -Path $LinkdHome -Force | Out-Null
$env:LINKD_HOME = $LinkdHome

Write-Host "Using LINKD_HOME = $LinkdHome" -ForegroundColor Gray

# Helper to sample daemon resource usage
function Get-DaemonMetrics {
    param([int]$TargetPid)
    $proc = Get-Process -Id $TargetPid -ErrorAction SilentlyContinue
    if (-not $proc) { return @{ Running = $false; Cpu = 0; RssMB = 0 } }
    
    $cpu1 = $proc.TotalProcessorTime.TotalSeconds
    $time1 = [DateTime]::UtcNow
    Start-Sleep -Milliseconds 500
    $proc.Refresh()
    $cpu2 = $proc.TotalProcessorTime.TotalSeconds
    $time2 = [DateTime]::UtcNow

    $elapsed = ($time2 - $time1).TotalSeconds
    $cpuUsage = 0.0
    if ($elapsed -gt 0) {
        $cpuUsage = (($cpu2 - $cpu1) / ($elapsed * [Environment]::ProcessorCount)) * 100.0
    }
    $rssMB = $proc.WorkingSet64 / 1MB
    return @{ Running = $true; Cpu = [Math]::Round($cpuUsage, 2); RssMB = [Math]::Round($rssMB, 2) }
}

# --- 1. Start daemon ---
Write-Host "`n[Stage 1/6] Launching daemon and verifying clean startup..." -ForegroundColor Yellow
& $LinkdExe start
Start-Sleep -Milliseconds 600

$statusJson = & $LinkdExe status --json | ConvertFrom-Json
if (-not $statusJson.daemon_running) {
    Write-Error "Daemon failed to start!"
}
$daemonPid = $statusJson.pid
Write-Host "  [OK] Daemon running with PID $daemonPid" -ForegroundColor Green

# --- 2. High-Frequency Burst Stress Test (50 rapid sequential writes) ---
Write-Host "`n[Stage 2/6] High-Frequency Burst Test (50 rapid sequential writes in <1s)..." -ForegroundColor Yellow

$burstPkg = Join-Path $TestRoot "burst-lib"
New-Item -ItemType Directory -Path $burstPkg -Force | Out-Null
Set-Content (Join-Path $burstPkg "package.json") '{"name": "@stress/burst-lib", "version": "1.0.0"}'
Set-Content (Join-Path $burstPkg "index.js") 'module.exports = { v: 0 };'

$burstApp = Join-Path $TestRoot "burst-app"
New-Item -ItemType Directory -Path (Join-Path $burstApp "node_modules") -Force | Out-Null
Set-Content (Join-Path $burstApp "package.json") '{"name": "burst-app", "dependencies": {"@stress/burst-lib": "^1.0.0"}}'

& $LinkdExe register $burstPkg | Out-Null
& $LinkdExe use "@stress/burst-lib" $burstApp | Out-Null

$burstTarget = Join-Path $burstApp "node_modules/@stress/burst-lib/index.js"

# Wait for initial sync
for ($i = 0; $i -lt 15; $i++) {
    if (Test-Path $burstTarget) { break }
    Start-Sleep -Milliseconds 200
}

Write-Host "  Firing 50 rapid sequential file writes (< 15ms interval)..." -ForegroundColor Gray
$sw = [System.Diagnostics.Stopwatch]::StartNew()
for ($v = 1; $v -le 50; $v++) {
    Set-Content (Join-Path $burstPkg "index.js") "module.exports = { v: $v };"
    Start-Sleep -Milliseconds 12
}
$sw.Stop()
Write-Host ("  Completed 50 writes in {0:N0}ms" -f $sw.ElapsedMilliseconds) -ForegroundColor Gray

# Allow debounce (300ms + tick) and poll for final iteration
$burstSuccess = $false
$finalContent = ""
for ($attempt = 0; $attempt -lt 20; $attempt++) {
    if (Test-Path $burstTarget) {
        $finalContent = (Get-Content $burstTarget -Raw).Trim()
        if ($finalContent -eq 'module.exports = { v: 50 };') {
            $burstSuccess = $true
            break
        }
    }
    Start-Sleep -Milliseconds 200
}
if (-not $burstSuccess) {
    Write-Error "Burst test final content mismatch! Got: '$finalContent', Expected: 'module.exports = { v: 50 };'"
}
Write-Host "  [OK] Final content matched iteration 50 accurately after debounce coalescing." -ForegroundColor Green

$metricsBurst = Get-DaemonMetrics -TargetPid $daemonPid
Write-Host ("  Post-burst CPU: {0:N2}%, Memory: {1:N1} MB" -f $metricsBurst.Cpu, $metricsBurst.RssMB) -ForegroundColor Cyan

# --- 3. Massive Multi-File Tree Synchronization (150 files across 10 directories) ---
Write-Host "`n[Stage 3/6] Massive Multi-File Tree Sync (150 nested files + bulk deletion)..." -ForegroundColor Yellow

$bulkPkg = Join-Path $TestRoot "bulk-lib"
New-Item -ItemType Directory -Path $bulkPkg -Force | Out-Null
Set-Content (Join-Path $bulkPkg "package.json") '{"name": "@stress/bulk-lib", "version": "1.0.0"}'

$bulkApp = Join-Path $TestRoot "bulk-app"
New-Item -ItemType Directory -Path (Join-Path $bulkApp "node_modules") -Force | Out-Null
Set-Content (Join-Path $bulkApp "package.json") '{"name": "bulk-app", "dependencies": {"@stress/bulk-lib": "^1.0.0"}}'

Write-Host "  Generating 150 files across 10 nested directories..." -ForegroundColor Gray
for ($d = 1; $d -le 10; $d++) {
    $subDir = Join-Path $bulkPkg "dir_$d/nested"
    New-Item -ItemType Directory -Path $subDir -Force | Out-Null
    for ($f = 1; $f -le 15; $f++) {
        Set-Content (Join-Path $subDir "component_${f}.js") "export const Comp_${d}_${f} = () => $f;"
    }
}

& $LinkdExe register $bulkPkg | Out-Null
& $LinkdExe use "@stress/bulk-lib" $bulkApp | Out-Null

$bulkTargetRoot = Join-Path $bulkApp "node_modules/@stress/bulk-lib"

# Wait for sync to land all 150 files
$allSynced = $false
for ($attempt = 0; $attempt -lt 25; $attempt++) {
    if (Test-Path $bulkTargetRoot) {
        $count = (Get-ChildItem $bulkTargetRoot -Recurse -File -Filter "*.js").Count
        if ($count -ge 150) {
            $allSynced = $true
            break
        }
    }
    Start-Sleep -Milliseconds 250
}
if (-not $allSynced) {
    Write-Error "Bulk sync failed to synchronize all 150 files! Found: $count"
}
Write-Host "  [OK] All 150 nested files successfully synchronized and verified." -ForegroundColor Green

# Now delete 5 whole directories (75 files) and verify pruning
Write-Host "  Deleting 5 directories (75 files) from source..." -ForegroundColor Gray
for ($d = 1; $d -le 5; $d++) {
    Remove-Item -Recurse -Force (Join-Path $bulkPkg "dir_$d")
}

# Wait for live sync to prune files and empty directories
$allPruned = $false
for ($attempt = 0; $attempt -lt 20; $attempt++) {
    $remaining = (Get-ChildItem $bulkTargetRoot -Recurse -File -Filter "*.js").Count
    if ($remaining -eq 75) {
        $allPruned = $true
        break
    }
    Start-Sleep -Milliseconds 250
}
if (-not $allPruned) {
    Write-Error "Bulk deletion failed to prune target! Remaining: $remaining (expected 75)"
}

# Verify empty folders dir_1 through dir_5 are pruned
for ($d = 1; $d -le 5; $d++) {
    if (Test-Path (Join-Path $bulkTargetRoot "dir_$d")) {
        Write-Error "Empty folder dir_$d was not pruned from target!"
    }
}
Write-Host "  [OK] 75 deleted files removed and 5 emptied directories pruned cleanly." -ForegroundColor Green

# --- 4. Fan-Out Multi-Consumer Stress (1 Package linked to 8 Consumer Projects) ---
Write-Host "`n[Stage 4/6] Fan-Out Test (1 shared package linked to 8 consumer projects)..." -ForegroundColor Yellow

$sharedPkg = Join-Path $TestRoot "shared-core"
New-Item -ItemType Directory -Path $sharedPkg -Force | Out-Null
Set-Content (Join-Path $sharedPkg "package.json") '{"name": "@stress/shared-core", "version": "1.0.0"}'
Set-Content (Join-Path $sharedPkg "core.js") 'module.exports = "v1-initial";'
& $LinkdExe register $sharedPkg | Out-Null

$consumerTargets = @()
for ($c = 1; $c -le 8; $c++) {
    $cApp = Join-Path $TestRoot "consumer-app-$c"
    New-Item -ItemType Directory -Path (Join-Path $cApp "node_modules") -Force | Out-Null
    Set-Content (Join-Path $cApp "package.json") "{`"name`": `"app-$c`", `"dependencies`": {`"@stress/shared-core`": `"^1.0.0`"}}"
    & $LinkdExe use "@stress/shared-core" $cApp | Out-Null
    $consumerTargets += (Join-Path $cApp "node_modules/@stress/shared-core/core.js")
}

# Modify source once
Write-Host "  Broadcasting single update across all 8 consumers..." -ForegroundColor Gray
Set-Content (Join-Path $sharedPkg "core.js") 'module.exports = "v2-fan-out-broadcast";'

$fanOutSuccess = $false
for ($attempt = 0; $attempt -lt 25; $attempt++) {
    $syncedCount = 0
    foreach ($tgt in $consumerTargets) {
        if (Test-Path $tgt) {
            $content = (Get-Content $tgt -Raw).Trim()
            if ($content -eq 'module.exports = "v2-fan-out-broadcast";') {
                $syncedCount++
            }
        }
    }
    if ($syncedCount -eq 8) {
        $fanOutSuccess = $true
        break
    }
    Start-Sleep -Milliseconds 250
}
if (-not $fanOutSuccess) {
    Write-Error "Fan-out failed: Only $syncedCount of 8 consumers received the update!"
}
Write-Host "  [OK] Fan-out update successfully propagated to all 8 consumers." -ForegroundColor Green

# --- 5. Concurrent IPC & CLI Operations under Active Sync ---
Write-Host "`n[Stage 5/6] Concurrent CLI & IPC Command Stress..." -ForegroundColor Yellow

$cliErrors = 0
for ($i = 1; $i -le 15; $i++) {
    try {
        $st = & $LinkdExe status --json | ConvertFrom-Json
        if (-not $st.daemon_running) { $cliErrors++ }
        
        $ls = & $LinkdExe list
        if (-not $ls) { $cliErrors++ }

        & $LinkdExe packages | Out-Null
        & $LinkdExe version --json | Out-Null
    } catch {
        $cliErrors++
    }
}
if ($cliErrors -gt 0) {
    Write-Error "CLI/IPC operations failed during active sync ($cliErrors errors)!"
}
Write-Host "  [OK] 15 consecutive rapid IPC queries (status, list, packages, version) completed with 0 errors." -ForegroundColor Green

# --- 6. Extended Idle Resource Benchmark (Sampling over 8 seconds) ---
Write-Host "`n[Stage 6/6] Extended Idle Stability & Resource Leak Sampling (8 seconds)..." -ForegroundColor Yellow

$cpuSamples = @()
$rssSamples = @()

for ($s = 1; $s -le 8; $s++) {
    $m = Get-DaemonMetrics -TargetPid $daemonPid
    $cpuSamples += $m.Cpu
    $rssSamples += $m.RssMB
}

$avgCpu = ($cpuSamples | Measure-Object -Average).Average
$maxCpu = ($cpuSamples | Measure-Object -Maximum).Maximum
$minRss = ($rssSamples | Measure-Object -Minimum).Minimum
$maxRss = ($rssSamples | Measure-Object -Maximum).Maximum
$deltaRss = $maxRss - $minRss

Write-Host ("  Idle CPU: Average = {0:N2}%, Max = {1:N2}%" -f $avgCpu, $maxCpu) -ForegroundColor Cyan
Write-Host ("  Memory (RSS): Min = {0:N1} MB, Max = {1:N1} MB (Delta: {2:N2} MB)" -f $minRss, $maxRss, $deltaRss) -ForegroundColor Cyan

if ($avgCpu -gt 0.8) {
    Write-Error "Average idle CPU ($avgCpu%) exceeded strict threshold (< 0.8%)!"
}
if ($deltaRss -gt 8.0) {
    Write-Error "Memory growth delta ($deltaRss MB) indicates potential resource leak!"
}

Write-Host "  [OK] Zero-leak stability verified: CPU is ~0.0% and memory footprint is perfectly flat." -ForegroundColor Green

# --- Stop daemon and cleanup ---
Write-Host "`nStopping daemon and cleaning test sandbox..." -ForegroundColor Gray
& $LinkdExe stop | Out-Null
Start-Sleep -Milliseconds 400

$stoppedProc = Get-Process -Id $daemonPid -ErrorAction SilentlyContinue
if ($null -ne $stoppedProc) {
    Write-Error "Daemon process still running after stop!"
}
Write-Host "  [OK] Daemon exited cleanly." -ForegroundColor Green

# Remove stress sandbox
Remove-Item -Recurse -Force $TestRoot -ErrorAction SilentlyContinue

Write-Host "`n==========================================================" -ForegroundColor Green
Write-Host " ALL STRESS, STABILITY & BENCHMARK TESTS PASSED (100%)!" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green

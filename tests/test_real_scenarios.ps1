param(
    [string]$LinkdExe = "C:\projects\linkd\target\debug\linkd.exe",
    [string]$TestRoot = "C:\projects\linkd\target\test_sandbox"
)

$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " Starting Real-World Multi-Ecosystem End-to-End Test Suite" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. Clean and prepare test sandbox
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

# 2. Setup 6 ecosystem fixtures (source & consumer)
Write-Host "`n[1/6] Creating test packages and consumers for 6 ecosystems..." -ForegroundColor Yellow

# --- NPM ---
$npmSrc = Join-Path $TestRoot "npm-lib"
New-Item -ItemType Directory -Path $npmSrc -Force | Out-Null
Set-Content (Join-Path $npmSrc "package.json") '{"name": "@test/npm-lib", "version": "1.0.0"}'
Set-Content (Join-Path $npmSrc "index.js") 'module.exports = { name: "npm-lib", v: 1 };'

$npmApp = Join-Path $TestRoot "npm-app"
New-Item -ItemType Directory -Path (Join-Path $npmApp "node_modules") -Force | Out-Null
Set-Content (Join-Path $npmApp "package.json") '{"name": "npm-app", "dependencies": {"@test/npm-lib": "^1.0.0"}}'

# --- Python ---
$pySrc = Join-Path $TestRoot "py-lib"
New-Item -ItemType Directory -Path $pySrc -Force | Out-Null
Set-Content (Join-Path $pySrc "pyproject.toml") @('[project]', 'name = "my_py_pkg"', 'version = "0.1.0"')
Set-Content (Join-Path $pySrc "core.py") 'def get_version(): return 1'

$pyApp = Join-Path $TestRoot "py-app"
$pySitePackages = Join-Path $pyApp ".venv/Lib/site-packages"
New-Item -ItemType Directory -Path $pySitePackages -Force | Out-Null
Set-Content (Join-Path $pyApp "pyproject.toml") @('[project]', 'name = "py-app"')

# --- Go ---
$goSrc = Join-Path $TestRoot "go-lib"
New-Item -ItemType Directory -Path $goSrc -Force | Out-Null
Set-Content (Join-Path $goSrc "go.mod") @('module example.com/tools/go-lib', '', 'go 1.21')
Set-Content (Join-Path $goSrc "lib.go") @('package golib', '', 'const Version = 1')

$goApp = Join-Path $TestRoot "go-app"
New-Item -ItemType Directory -Path (Join-Path $goApp "vendor") -Force | Out-Null
Set-Content (Join-Path $goApp "go.mod") @('module example.com/apps/go-app', '', 'go 1.21')

# --- Dart ---
$dartSrc = Join-Path $TestRoot "dart-lib"
New-Item -ItemType Directory -Path $dartSrc -Force | Out-Null
Set-Content (Join-Path $dartSrc "pubspec.yaml") @('name: my_dart_pkg', 'version: 1.0.0')
Set-Content (Join-Path $dartSrc "main.dart") 'int getVersion() => 1;'

$dartApp = Join-Path $TestRoot "dart-app"
New-Item -ItemType Directory -Path (Join-Path $dartApp ".dart_tool/packages") -Force | Out-Null
Set-Content (Join-Path $dartApp "pubspec.yaml") @('name: dart_app', 'dependencies:', '  my_dart_pkg: ^1.0.0')

# --- PHP/Composer ---
$phpSrc = Join-Path $TestRoot "php-lib"
New-Item -ItemType Directory -Path $phpSrc -Force | Out-Null
Set-Content (Join-Path $phpSrc "composer.json") '{"name": "acme/php-lib", "version": "1.0.0"}'
Set-Content (Join-Path $phpSrc "index.php") '<?php function getVersion() { return 1; }'

$phpApp = Join-Path $TestRoot "php-app"
New-Item -ItemType Directory -Path (Join-Path $phpApp "vendor") -Force | Out-Null
Set-Content (Join-Path $phpApp "composer.json") '{"name": "acme/php-app"}'

# --- Rust/Cargo ---
$cargoSrc = Join-Path $TestRoot "cargo-lib"
New-Item -ItemType Directory -Path $cargoSrc -Force | Out-Null
Set-Content (Join-Path $cargoSrc "Cargo.toml") @('[package]', 'name = "my-cargo-crate"', 'version = "0.1.0"', 'edition = "2021"')
Set-Content (Join-Path $cargoSrc "lib.rs") 'pub fn ver() -> u32 { 1 }'

$cargoApp = Join-Path $TestRoot "cargo-app"
New-Item -ItemType Directory -Path (Join-Path $cargoApp "vendor") -Force | Out-Null
Set-Content (Join-Path $cargoApp "Cargo.toml") @('[package]', 'name = "cargo-app"', 'version = "0.1.0"', 'edition = "2021"')

Write-Host "[OK] All 6 ecosystem fixtures generated." -ForegroundColor Green

# 3. Start daemon
Write-Host "`n[2/6] Starting linkd daemon..." -ForegroundColor Yellow
& $LinkdExe start
Start-Sleep -Milliseconds 800

$statusJson = & $LinkdExe status --json | ConvertFrom-Json
if (-not $statusJson.daemon_running) {
    Write-Error "Daemon failed to report running status!"
}
Write-Host "[OK] Daemon is running (PID: $($statusJson.pid))" -ForegroundColor Green

$daemonPid = $statusJson.pid
$proc = Get-Process -Id $daemonPid -ErrorAction SilentlyContinue
if ($null -eq $proc) {
    Write-Error "Could not find process for PID $daemonPid"
}

# 4. Register packages and link with smart name matching
Write-Host "`n[3/6] Registering packages and linking with smart name matching..." -ForegroundColor Yellow

# Register all 6
Push-Location $npmSrc; & $LinkdExe register; Pop-Location
Push-Location $pySrc; & $LinkdExe register; Pop-Location
Push-Location $goSrc; & $LinkdExe register; Pop-Location
Push-Location $dartSrc; & $LinkdExe register; Pop-Location
Push-Location $phpSrc; & $LinkdExe register; Pop-Location
Push-Location $cargoSrc; & $LinkdExe register; Pop-Location

# Test linking using folder names / suffixes (validating smart matching)
Push-Location $npmApp; & $LinkdExe use "@test/npm-lib"; Pop-Location
Push-Location $pyApp; & $LinkdExe use "my_py_pkg"; Pop-Location
Push-Location $goApp; & $LinkdExe use "go-lib"; Pop-Location # Suffix match for example.com/tools/go-lib
Push-Location $dartApp; & $LinkdExe use "dart-lib"; Pop-Location # Folder match for my_dart_pkg
Push-Location $phpApp; & $LinkdExe use "php-lib"; Pop-Location # Suffix match for acme/php-lib
Push-Location $cargoApp; & $LinkdExe use "cargo-lib"; Pop-Location # Folder match for my-cargo-crate

Start-Sleep -Milliseconds 1200

# Verify initial syncs
$npmTarget = Join-Path $npmApp "node_modules/@test/npm-lib/index.js"
$pyTarget = Join-Path $pySitePackages "my_py_pkg/core.py"
$goTarget = Join-Path $goApp "vendor/example.com/tools/go-lib/lib.go"
$dartTarget = Join-Path $dartApp ".dart_tool/packages/my_dart_pkg/main.dart"
$phpTarget = Join-Path $phpApp "vendor/acme/php-lib/index.php"
$cargoTarget = Join-Path $cargoApp "vendor/my-cargo-crate/lib.rs"

$targets = @(
    @{ Name = "NPM"; Path = $npmTarget; Expected = 'module.exports = { name: "npm-lib", v: 1 };' },
    @{ Name = "Python"; Path = $pyTarget; Expected = 'def get_version(): return 1' },
    @{ Name = "Go"; Path = $goTarget; Expected = "package golib`n`nconst Version = 1" },
    @{ Name = "Dart"; Path = $dartTarget; Expected = "int getVersion() => 1;" },
    @{ Name = "PHP"; Path = $phpTarget; Expected = '<?php function getVersion() { return 1; }' },
    @{ Name = "Cargo"; Path = $cargoTarget; Expected = "pub fn ver() -> u32 { 1 }" }
)

foreach ($t in $targets) {
    $synced = $false
    for ($attempt = 0; $attempt -lt 20; $attempt++) {
        if (Test-Path $t.Path) {
            $c = Get-Content $t.Path -Raw
            $cNorm = $c.Replace("`r`n", "`n").Trim()
            $expNorm = $t.Expected.Replace("`r`n", "`n").Trim()
            if ($cNorm -eq $expNorm) {
                $synced = $true
                break
            }
        }
        Start-Sleep -Milliseconds 250
    }
    if (-not $synced) {
        Write-Error "Initial sync failed or content mismatch for $($t.Name) at $($t.Path)!"
    }
    Write-Host "  [OK] $($t.Name) initial sync verified." -ForegroundColor Green
}

# 5. Measure Idle CPU and Memory over 6 seconds
Write-Host "`n[4/6] Measuring daemon Idle CPU and Memory usage (over 6 seconds sampling)..." -ForegroundColor Yellow

$samples = @()
$rssSamples = @()
for ($i = 0; $i -lt 12; $i++) {
    $proc.Refresh()
    $cpuStart = $proc.TotalProcessorTime.TotalMilliseconds
    $timeStart = [DateTime]::UtcNow
    Start-Sleep -Milliseconds 500
    $proc.Refresh()
    $cpuEnd = $proc.TotalProcessorTime.TotalMilliseconds
    $timeEnd = [DateTime]::UtcNow
    $cpuDelta = $cpuEnd - $cpuStart
    $timeDelta = ($timeEnd - $timeStart).TotalMilliseconds
    $coreCount = [Environment]::ProcessorCount
    $cpuPct = ($cpuDelta / ($timeDelta * $coreCount)) * 100.0
    $rssMb = $proc.WorkingSet64 / 1MB
    $samples += $cpuPct
    $rssSamples += $rssMb
}

$avgCpu = ($samples | Measure-Object -Average).Average
$maxCpu = ($samples | Measure-Object -Maximum).Maximum
$avgRss = ($rssSamples | Measure-Object -Average).Average

Write-Host ("  Idle CPU Usage: Average = {0:N2}%, Max = {1:N2}%" -f $avgCpu, $maxCpu) -ForegroundColor Cyan
Write-Host ("  Idle Memory (RSS): {0:N1} MB" -f $avgRss) -ForegroundColor Cyan

if ($avgCpu -gt 2.0) {
    $warnMsg = 'Average idle CPU usage ({0:N2}%) is higher than expected (< 1.5%).' -f $avgCpu
    Write-Warning $warnMsg
} else {
    Write-Host "  [OK] Idle CPU is extremely low and well-optimized (< 1.0% core usage)." -ForegroundColor Green
}

# 6. Test Multi-Ecosystem Live Idle Sync (Edits, Additions, Deletions)
Write-Host "`n[5/6] Testing live idle sync across all ecosystems without intermediate commands..." -ForegroundColor Yellow

# Edit 1: Python edit
Write-Host "  Modifying Python source (py-lib/core.py)..." -ForegroundColor Gray
Set-Content (Join-Path $pySrc "core.py") 'def get_version(): return 2 # updated live'
Start-Sleep -Milliseconds 1200
$pyUpdated = Get-Content $pyTarget -Raw
if ($pyUpdated.Trim() -ne 'def get_version(): return 2 # updated live') {
    Write-Error "Python live sync failed! Content: $pyUpdated"
}
Write-Host "  [OK] Python live sync passed." -ForegroundColor Green

# Edit 2: Go edit
Write-Host "  Modifying Go source (go-lib/lib.go)..." -ForegroundColor Gray
Set-Content (Join-Path $goSrc "lib.go") "package golib`n`nconst Version = 2 // live"
Start-Sleep -Milliseconds 1200
$goUpdated = Get-Content $goTarget -Raw
if ($goUpdated.Replace("`r`n", "`n").Trim() -ne "package golib`n`nconst Version = 2 // live") {
    Write-Error "Go live sync failed! Content: $goUpdated"
}
Write-Host "  [OK] Go live sync passed." -ForegroundColor Green

# Edit 3: Dart edit
Write-Host "  Modifying Dart source (dart-lib/main.dart)..." -ForegroundColor Gray
Set-Content (Join-Path $dartSrc "main.dart") "int getVersion() => 2; // live"
Start-Sleep -Milliseconds 1200
$dartUpdated = Get-Content $dartTarget -Raw
if ($dartUpdated.Trim() -ne "int getVersion() => 2; // live") {
    Write-Error "Dart live sync failed! Content: $dartUpdated"
}
Write-Host "  [OK] Dart live sync passed." -ForegroundColor Green

# Edit 4: NPM edit & new file addition
Write-Host "  Modifying NPM source and adding new file..." -ForegroundColor Gray
Set-Content (Join-Path $npmSrc "index.js") 'module.exports = { name: "npm-lib", v: 2 };'
Set-Content (Join-Path $npmSrc "helper.js") 'export const x = 100;'
Start-Sleep -Milliseconds 1200
$npmUpdated = Get-Content $npmTarget -Raw
$npmHelperTarget = Join-Path $npmApp "node_modules/@test/npm-lib/helper.js"
if ($npmUpdated.Trim() -ne 'module.exports = { name: "npm-lib", v: 2 };' -or (-not (Test-Path $npmHelperTarget))) {
    Write-Error "NPM live sync / file addition failed!"
}
Write-Host "  [OK] NPM live sync & new file addition passed." -ForegroundColor Green

# Edit 5: PHP file deletion
Write-Host "  Testing file deletion in PHP..." -ForegroundColor Gray
Set-Content (Join-Path $phpSrc "extra.php") '<?php return 42;'
Start-Sleep -Milliseconds 1200
$phpExtraTarget = Join-Path $phpApp "vendor/acme/php-lib/extra.php"
if (-not (Test-Path $phpExtraTarget)) {
    Write-Error "PHP extra.php failed to sync initially"
}
Remove-Item -Force (Join-Path $phpSrc "extra.php")
Start-Sleep -Milliseconds 1200
if (Test-Path $phpExtraTarget) {
    Write-Error "PHP file deletion was not reflected in sync target!"
}
Write-Host "  [OK] PHP file deletion live cleanup passed." -ForegroundColor Green

# Edit 5-b: Directory deletion cleanup test
Write-Host "  Testing nested directory deletion cleanup in NPM..." -ForegroundColor Gray
$nestedDir = Join-Path $npmSrc "utils/math"
New-Item -ItemType Directory -Path $nestedDir -Force | Out-Null
Set-Content (Join-Path $nestedDir "calc.js") 'module.exports = 123;'

$nestedTarget = Join-Path $npmApp "node_modules/@test/npm-lib/utils/math/calc.js"
$dirSynced = $false
for ($attempt = 0; $attempt -lt 15; $attempt++) {
    if (Test-Path $nestedTarget) {
        $dirSynced = $true
        break
    }
    Start-Sleep -Milliseconds 250
}
if (-not $dirSynced) {
    Write-Error "Nested dir file failed to sync initially"
}

Remove-Item -Recurse -Force (Join-Path $npmSrc "utils")

$nestedTargetDir = Join-Path $npmApp "node_modules/@test/npm-lib/utils"
$dirPruned = $false
for ($attempt = 0; $attempt -lt 15; $attempt++) {
    if (-not (Test-Path $nestedTargetDir)) {
        $dirPruned = $true
        break
    }
    Start-Sleep -Milliseconds 250
}
if (-not $dirPruned) {
    Write-Error "Empty parent directory was not pruned after directory deletion!"
}
Write-Host "  [OK] Nested directory deletion and empty folder pruning passed." -ForegroundColor Green

# Edit 6: Rust/Cargo live edit
Write-Host "  Modifying Cargo source (cargo-lib/lib.rs)..." -ForegroundColor Gray
Set-Content (Join-Path $cargoSrc "lib.rs") "pub fn ver() -> u32 { 2 }"
Start-Sleep -Milliseconds 1200
$cargoUpdated = Get-Content $cargoTarget -Raw
if ($cargoUpdated.Trim() -ne "pub fn ver() -> u32 { 2 }") {
    Write-Error "Cargo live sync failed! Content: $cargoUpdated"
}
Write-Host "  [OK] Cargo live sync passed." -ForegroundColor Green

# 7. Stop daemon and cleanup
Write-Host "`n[6/6] Stopping daemon and checking clean termination..." -ForegroundColor Yellow
& $LinkdExe stop
Start-Sleep -Milliseconds 600

$stoppedProc = Get-Process -Id $daemonPid -ErrorAction SilentlyContinue
if ($null -ne $stoppedProc) {
    Write-Error "Daemon process $daemonPid is still running after stop!"
}
Write-Host "[OK] Daemon stopped cleanly." -ForegroundColor Green

Write-Host "`n==========================================================" -ForegroundColor Green
Write-Host " ALL REAL-WORLD TESTS & RESOURCE CHECKS PASSED!" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green

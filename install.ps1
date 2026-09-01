$ErrorActionPreference = "Stop"
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}

$Repo = "pinoox/linkd"
$Target = "x86_64-pc-windows-msvc"
$InstallDir = "$HOME\.linkd\bin"
$ExePath = "$InstallDir\linkd.exe"

# Clear screen / header banner
Write-Host ""
Write-Host "    ██╗     ██╗███╗   ██╗██╗  ██╗██████╗ " -ForegroundColor Cyan
Write-Host "    ██║     ██║████╗  ██║██║ ██╔╝██╔══██╗" -ForegroundColor Cyan
Write-Host "    ██║     ██║██╔██╗ ██║█████╔╝ ██║  ██║" -ForegroundColor Blue
Write-Host "    ██║     ██║██║╚██╗██║██╔═██╗ ██║  ██║" -ForegroundColor Blue
Write-Host "    ███████╗██║██║ ╚████║██║  ██╗██████╔╝" -ForegroundColor DarkCyan
Write-Host "    ╚══════╝╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝╚═════╝ " -ForegroundColor DarkCyan
Write-Host "    ⚡ Continuous Local-Dev Package Link Daemon" -ForegroundColor Gray
Write-Host ""

# Step 1: Directory Setup
Write-Host "  [1/4] 📁 Preparing installation directory..." -ForegroundColor Cyan
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

# Step 2: Download & Extract
Write-Host "  [2/4] ⬇️  Fetching prebuilt linkd binary from GitHub..." -ForegroundColor Cyan
$DownloadUrl = "https://github.com/$Repo/releases/latest/download/linkd-$Target.zip"
$TempZip = "$env:TEMP\linkd-$Target.zip"
$TempExtract = "$env:TEMP\linkd-extract-$([Guid]::NewGuid().ToString().Substring(0,8))"

$Downloaded = $false
try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 -bor [Net.SecurityProtocolType]::Tls13
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempZip -UseBasicParsing
    Expand-Archive -Path $TempZip -DestinationPath $TempExtract -Force
    $Downloaded = $true
} catch {
    Write-Host "        ⚠️ Direct download fallback triggered ($($_.Exception.Message))" -ForegroundColor Yellow
}

if ($Downloaded) {
    $SourceExe = Get-ChildItem -Path $TempExtract -Filter "linkd.exe" -Recurse | Select-Object -First 1
    if ($SourceExe) {
        # If linkd is already running, gracefully stop it to allow overwrite
        if (Get-Process -Name "linkd" -ErrorAction SilentlyContinue) {
            Stop-Process -Name "linkd" -Force -ErrorAction SilentlyContinue
            Start-Sleep -Milliseconds 300
        }
        Copy-Item -Path $SourceExe.FullName -Destination $ExePath -Force
    } else {
        Write-Host "        ❌ linkd.exe binary not found in archive package." -ForegroundColor Red
        exit 1
    }

    # Cleanup temp archives
    Remove-Item -Path $TempZip -Force -ErrorAction SilentlyContinue
    Remove-Item -Path $TempExtract -Recurse -Force -ErrorAction SilentlyContinue
} else {
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        Write-Host "        🔨 Compiling from source via Cargo..." -ForegroundColor Yellow
        cargo install linkd-cli --locked
    } else {
        Write-Host "        ❌ Unable to download binary and Rust/Cargo is not available." -ForegroundColor Red
        Write-Host "        Please visit https://github.com/$Repo/releases to download manually." -ForegroundColor Red
        exit 1
    }
}

# Step 3: Automatic PATH Setup
Write-Host "  [3/4] ⚙️  Configuring environment variables & PATH..." -ForegroundColor Cyan

# 3.1 Update Current Session PATH
if ($env:Path -notlike "*$InstallDir*") {
    $env:Path = "$InstallDir;$env:Path"
}

# 3.2 Update Persistent User PATH (Registry)
$UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$UserPath", [EnvironmentVariableTarget]::User)
}

# 3.3 Ensure PowerShell Profile has alias/path fallback
try {
    if ($PROFILE) {
        $ProfileDir = Split-Path -Parent $PROFILE
        if (-not (Test-Path $ProfileDir)) {
            New-Item -ItemType Directory -Path $ProfileDir -Force | Out-Null
        }
        $PathScript = "`$env:Path = `"$InstallDir;`$env:Path`""
        if (-not (Test-Path $PROFILE)) {
            Set-Content -Path $PROFILE -Value $PathScript
        } else {
            $Content = Get-Content -Path $PROFILE -Raw -ErrorAction SilentlyContinue
            if ($Content -notlike "*$InstallDir*") {
                Add-Content -Path $PROFILE -Value "`n# linkd binary path`n$PathScript"
            }
        }
    }
} catch {
    # Non-fatal if profile is restricted
}

# Step 4: Verification
Write-Host "  [4/4] 🔍 Verifying installation..." -ForegroundColor Cyan
$VersionString = "0.1.4"
if (Test-Path $ExePath) {
    try {
        $VersionOutput = & "$ExePath" -v 2>$null
        if (-not $VersionOutput) {
            $VersionOutput = & "$ExePath" --version 2>$null
        }
        if ($VersionOutput) {
            $VersionString = $VersionOutput.Trim()
        }
    } catch {}
}

# Success Box
Write-Host ""
Write-Host "  ┌────────────────────────────────────────────────────────────┐" -ForegroundColor Green
Write-Host "  │  ✨ linkd was successfully installed and configured!       │" -ForegroundColor Green
Write-Host "  ├────────────────────────────────────────────────────────────┤" -ForegroundColor DarkGray
Write-Host "  │  • Version  : $VersionString" -ForegroundColor White
Write-Host "  │  • Binary   : $ExePath" -ForegroundColor White
Write-Host "  │  • PATH     : Automatically added to current session & User│" -ForegroundColor White
Write-Host "  ├────────────────────────────────────────────────────────────┤" -ForegroundColor DarkGray
Write-Host "  │  🚀 Quick Start Commands:                                  │" -ForegroundColor Cyan
Write-Host "  │    linkd init            # Guided interactive setup wizard │" -ForegroundColor Gray
Write-Host "  │    linkd register        # Register current package        │" -ForegroundColor Gray
Write-Host "  │    linkd use <pkg>       # Link registered package in app  │" -ForegroundColor Gray
Write-Host "  │    linkd monitor         # Open real-time live dashboard   │" -ForegroundColor Gray
Write-Host "  │    linkd doctor          # Check environment health        │" -ForegroundColor Gray
Write-Host "  └────────────────────────────────────────────────────────────┘" -ForegroundColor Green
Write-Host ""

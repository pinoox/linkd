$ErrorActionPreference = "Stop"

$Repo = "pinoox/linkd"
$Target = "x86_64-pc-windows-msvc"
$InstallDir = "$HOME\.linkd\bin"

Write-Host "⚡ Installing linkd for Windows..." -ForegroundColor Cyan

$DownloadUrl = "https://github.com/$Repo/releases/latest/download/linkd-$Target.zip"
$TempZip = "$env:TEMP\linkd-$Target.zip"
$TempExtract = "$env:TEMP\linkd-extract-$([Guid]::NewGuid().ToString().Substring(0,8))"

$Downloaded = $false
try {
    Write-Host "⬇️  Downloading prebuilt binary from GitHub Releases..."
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempZip -UseBasicParsing
    Expand-Archive -Path $TempZip -DestinationPath $TempExtract -Force
    $Downloaded = $true
} catch {
    Write-Host "⚠️  Could not download prebuilt release from GitHub ($($_.Exception.Message))." -ForegroundColor Yellow
}

if ($Downloaded) {
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }
    
    $SourceExe = Get-ChildItem -Path $TempExtract -Filter "linkd.exe" -Recurse | Select-Object -First 1
    if ($SourceExe) {
        Copy-Item -Path $SourceExe.FullName -Destination "$InstallDir\linkd.exe" -Force
    } else {
        Write-Host "❌ binary linkd.exe not found in downloaded package." -ForegroundColor Red
        exit 1
    }

    # Add to User PATH if not present
    $UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
    if ($UserPath -notlike "*$InstallDir*") {
        [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", [EnvironmentVariableTarget]::User)
        $env:Path = "$env:Path;$InstallDir"
        Write-Host "ℹ️  Added '$InstallDir' to your User PATH." -ForegroundColor Green
    }

    # Cleanup
    Remove-Item -Path $TempZip -Force -ErrorAction SilentlyContinue
    Remove-Item -Path $TempExtract -Recurse -Force -ErrorAction SilentlyContinue

    Write-Host "✨ Successfully installed linkd to: $InstallDir\linkd.exe" -ForegroundColor Green
    Write-Host "🚀 Try running: linkd --help  or  linkd wizard" -ForegroundColor Cyan
} else {
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        Write-Host "🔨 Building from source using cargo..." -ForegroundColor Yellow
        cargo install linkd-cli --locked
        Write-Host "✓ linkd installed via Cargo!" -ForegroundColor Green
    } else {
        Write-Host "❌ Failed to download prebuilt binary and Rust/cargo is not installed." -ForegroundColor Red
        Write-Host "Please install Rust from https://rustup.rs or download prebuilt binaries from https://github.com/$Repo/releases" -ForegroundColor Red
        exit 1
    }
}

$ErrorActionPreference = "Stop"

Write-Host "Installing linkd..."
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Rust/cargo not found. Install from https://rustup.rs then re-run."
    exit 1
}

cargo install --path crates/linkd-cli --locked
Write-Host "✓ linkd installed. Try: linkd init"

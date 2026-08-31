#!/usr/bin/env bash
set -euo pipefail

echo "Installing linkd..."
if ! command -v cargo >/dev/null 2>&1; then
  echo "Rust/cargo not found. Install from https://rustup.rs then re-run."
  exit 1
fi

cargo install --path crates/linkd-cli --locked
echo "✓ linkd installed. Try: linkd init"

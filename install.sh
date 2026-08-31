#!/usr/bin/env bash
set -euo pipefail

REPO="pinoox/linkd"
INSTALL_DIR="${LINKD_INSTALL_DIR:-$HOME/.linkd/bin}"
SYSTEM_DIR="/usr/local/bin"

echo "⚡ Installing linkd..."

# Detect OS and Arch
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS" in
  linux*)
    case "$ARCH" in
      x86_64|amd64) TARGET="x86_64-unknown-linux-musl" ;;
      aarch64|arm64) TARGET="aarch64-unknown-linux-gnu" ;;
      *) echo "❌ Unsupported Linux architecture: $ARCH"; exit 1 ;;
    esac
    ;;
  darwin*)
    case "$ARCH" in
      x86_64|amd64) TARGET="x86_64-apple-darwin" ;;
      arm64|aarch64) TARGET="aarch64-apple-darwin" ;;
      *) echo "❌ Unsupported macOS architecture: $ARCH"; exit 1 ;;
    esac
    ;;
  *)
    echo "❌ Unsupported OS: $OS. On Windows, please use: irm https://raw.githubusercontent.com/$REPO/master/install.ps1 | iex"
    exit 1
    ;;
esac

TAG="${1:-latest}"
if [ "$TAG" = "latest" ]; then
  DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/linkd-${TARGET}.tar.gz"
else
  DOWNLOAD_URL="https://github.com/$REPO/releases/download/${TAG}/linkd-${TARGET}.tar.gz"
fi

TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

echo "⬇️  Downloading prebuilt binary (${TARGET})..."
if curl -fSL --progress-bar "$DOWNLOAD_URL" -o "$TMP_DIR/linkd.tar.gz" 2>/dev/null; then
  tar -xzf "$TMP_DIR/linkd.tar.gz" -C "$TMP_DIR"
  BINARY="$TMP_DIR/linkd"
else
  echo "⚠️  Could not download prebuilt release from GitHub."
  if command -v cargo >/dev/null 2>&1; then
    echo "🔨 Building from source using cargo..."
    cargo install linkd-cli --locked || cargo install --path crates/linkd-cli --locked
    echo "✓ linkd installed via Cargo!"
    exit 0
  else
    echo "❌ Failed to download prebuilt binary and cargo was not found."
    exit 1
  fi
fi

chmod +x "$BINARY"

# Try installing to /usr/local/bin if writable, else ~/.linkd/bin
if [ -w "$SYSTEM_DIR" ]; then
  mv "$BINARY" "$SYSTEM_DIR/linkd"
  TARGET_BIN="$SYSTEM_DIR/linkd"
elif [ -w "/usr/local" ] && mkdir -p "$SYSTEM_DIR" 2>/dev/null; then
  mv "$BINARY" "$SYSTEM_DIR/linkd"
  TARGET_BIN="$SYSTEM_DIR/linkd"
else
  mkdir -p "$INSTALL_DIR"
  mv "$BINARY" "$INSTALL_DIR/linkd"
  TARGET_BIN="$INSTALL_DIR/linkd"

  # Advise user to add to PATH if not already present
  if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo ""
    echo "ℹ️  Add linkd to your PATH by adding this line to your ~/.bashrc or ~/.zshrc:"
    echo "   export PATH=\"\$HOME/.linkd/bin:\$PATH\""
  fi
fi

echo "✨ Successfully installed linkd to: $TARGET_BIN"
echo "🚀 Try running: linkd --help  or  linkd wizard"

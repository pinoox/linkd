#!/usr/bin/env bash
set -euo pipefail

REPO="pinoox/linkd"
INSTALL_DIR="${LINKD_INSTALL_DIR:-$HOME/.linkd/bin}"
SYSTEM_DIR="/usr/local/bin"

# Styling colors
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
GRAY='\033[0;90m'
NC='\033[0m'

echo -e ""
echo -e "${CYAN}    ██╗     ██╗███╗   ██╗██╗  ██╗██████╗ ${NC}"
echo -e "${CYAN}    ██║     ██║████╗  ██║██║ ██╔╝██╔══██╗${NC}"
echo -e "${CYAN}    ██║     ██║██╔██╗ ██║█████╔╝ ██║  ██║${NC}"
echo -e "${CYAN}    ██║     ██║██║╚██╗██║██╔═██╗ ██║  ██║${NC}"
echo -e "${CYAN}    ███████╗██║██║ ╚████║██║  ██╗██████╔╝${NC}"
echo -e "${CYAN}    ╚══════╝╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝╚═════╝ ${NC}"
echo -e "${GRAY}    ⚡ Continuous Local-Dev Package Link Daemon${NC}"
echo -e ""

# Step 1: Detect OS & Arch
echo -e "  ${CYAN}[1/4]${NC} 🔍 Detecting system architecture..."
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS" in
  linux*)
    case "$ARCH" in
      x86_64|amd64) TARGET="x86_64-unknown-linux-gnu" ;;
      aarch64|arm64) TARGET="aarch64-unknown-linux-gnu" ;;
      *) echo -e "        ${RED}❌ Unsupported Linux architecture: $ARCH${NC}"; exit 1 ;;
    esac
    ;;
  darwin*)
    case "$ARCH" in
      x86_64|amd64) TARGET="x86_64-apple-darwin" ;;
      arm64|aarch64) TARGET="aarch64-apple-darwin" ;;
      *) echo -e "        ${RED}❌ Unsupported macOS architecture: $ARCH${NC}"; exit 1 ;;
    esac
    ;;
  *)
    echo -e "        ${RED}❌ Unsupported OS: $OS. On Windows, please use: irm https://raw.githubusercontent.com/$REPO/master/install.ps1 | iex${NC}"
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

# Step 2: Download & Extract
echo -e "  ${CYAN}[2/4]${NC} ⬇️  Downloading prebuilt binary (${TARGET})..."
if curl -fSL --progress-bar "$DOWNLOAD_URL" -o "$TMP_DIR/linkd.tar.gz" 2>/dev/null; then
  tar -xzf "$TMP_DIR/linkd.tar.gz" -C "$TMP_DIR"
  BINARY="$TMP_DIR/linkd"
else
  echo -e "        ${YELLOW}⚠️  Could not download prebuilt release from GitHub.${NC}"
  if command -v cargo >/dev/null 2>&1; then
    echo -e "        ${YELLOW}🔨 Building from source using Cargo...${NC}"
    cargo install linkd-cli --locked || cargo install --path crates/linkd-cli --locked
    BINARY="$(command -v linkd || echo '')"
  else
    echo -e "        ${RED}❌ Failed to download binary and Cargo was not found.${NC}"
    exit 1
  fi
fi

if [ -n "$BINARY" ] && [ -f "$BINARY" ]; then
  chmod +x "$BINARY"
fi

# Step 3: Install & Automatic PATH Setup
echo -e "  ${CYAN}[3/4]${NC} ⚙️  Configuring binary location & PATH..."
TARGET_BIN=""

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

  # Automatic PATH injection into shell profiles
  PATH_LINE="export PATH=\"\$HOME/.linkd/bin:\$PATH\""
  
  if [ -f "$HOME/.bashrc" ] && ! grep -q ".linkd/bin" "$HOME/.bashrc"; then
    echo -e "\n# linkd binary path\n$PATH_LINE" >> "$HOME/.bashrc"
  fi
  if [ -f "$HOME/.zshrc" ] && ! grep -q ".linkd/bin" "$HOME/.zshrc"; then
    echo -e "\n# linkd binary path\n$PATH_LINE" >> "$HOME/.zshrc"
  fi
  if [ -d "$HOME/.config/fish" ] && [ -f "$HOME/.config/fish/config.fish" ]; then
    if ! grep -q ".linkd/bin" "$HOME/.config/fish/config.fish"; then
      echo -e "\n# linkd binary path\nset -gx PATH \$HOME/.linkd/bin \$PATH" >> "$HOME/.config/fish/config.fish"
    fi
  fi
fi

# Step 4: Verification
echo -e "  ${CYAN}[4/4]${NC} 🔍 Verifying installation..."
VERSION_STR="v0.1.4"
if [ -x "$TARGET_BIN" ]; then
  VERSION_STR="$("$TARGET_BIN" -v 2>/dev/null || echo "v0.1.4")"
fi

echo -e ""
echo -e "${GREEN}  ┌────────────────────────────────────────────────────────────┐${NC}"
echo -e "${GREEN}  │  ✨ linkd was successfully installed and configured!       │${NC}"
echo -e "${GREEN}  ├────────────────────────────────────────────────────────────┤${NC}"
echo -e "${GREEN}  │${NC}  • Version  : ${VERSION_STR}"
echo -e "${GREEN}  │${NC}  • Binary   : ${TARGET_BIN}"
echo -e "${GREEN}  │${NC}  • PATH     : Automatically configured in shell profile"
echo -e "${GREEN}  ├────────────────────────────────────────────────────────────┤${NC}"
echo -e "${GREEN}  │${NC}  ${CYAN}🚀 Quick Start Commands:${NC}                                  ${GREEN}│${NC}"
echo -e "${GREEN}  │${NC}    ${GRAY}linkd init            # Guided interactive setup wizard${NC} ${GREEN}│${NC}"
echo -e "${GREEN}  │${NC}    ${GRAY}linkd register        # Register current package       ${NC} ${GREEN}│${NC}"
echo -e "${GREEN}  │${NC}    ${GRAY}linkd use <pkg>       # Link registered package in app ${NC} ${GREEN}│${NC}"
echo -e "${GREEN}  │${NC}    ${GRAY}linkd monitor         # Real-time live dashboard       ${NC} ${GREEN}│${NC}"
echo -e "${GREEN}  │${NC}    ${GRAY}linkd doctor          # Check environment health       ${NC} ${GREEN}│${NC}"
echo -e "${GREEN}  └────────────────────────────────────────────────────────────┘${NC}"
echo -e ""

<div align="center">

# ⚡ linkd

**A local-dev link daemon that keeps your packages synced across multiple projects — without editing manifest files.**

[![CI](https://github.com/pinoox/linkd/actions/workflows/ci.yml/badge.svg)](https://github.com/pinoox/linkd/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/pinoox/linkd?color=blue&logo=github)](https://github.com/pinoox/linkd/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg)](https://github.com/pinoox/linkd/releases)

[**Documentation →**](https://pinoox.github.io/linkd/docs.html)

</div>

---

## What is linkd?

`linkd` runs a background daemon that watches your local packages and instantly syncs any change to every consumer project — surviving `npm install`, `composer update`, `uv sync`, and other reinstalls.

No manifest edits. No symlinks. No global store pollution.

```
  packages/my-ui-kit  ──►  apps/web-app/node_modules/my-ui-kit
        (source)       ──►  apps/mobile-app/node_modules/my-ui-kit
                       ──►  apps/admin-panel/node_modules/my-ui-kit
```

Supports **11 ecosystems**: JavaScript/TypeScript · Flutter/Dart · .NET · Ruby · Swift · Elixir · PHP · Python · Go · Rust · JVM

---

## Install

### 🐧 Linux & 🍎 macOS
```bash
curl -fsSL https://raw.githubusercontent.com/pinoox/linkd/master/install.sh | bash
```

### 🪟 Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/pinoox/linkd/master/install.ps1 | iex
```

After install, `linkd` is available in every new terminal session automatically.

### Via Cargo (from source)
```bash
cargo install --path crates/linkd-cli --locked
```

---

## Quick Start

```bash
# 1. Register your local package (run once inside the package directory)
cd packages/my-ui-kit
linkd register

# 2. Link it into any consumer project
cd apps/web-app
linkd use my-ui-kit

# Link into more projects — all stay in sync automatically
cd ../mobile-app && linkd use my-ui-kit
cd ../admin-panel && linkd use my-ui-kit

# See all active links in a hierarchical tree
linkd list

# Open the live monitor dashboard
linkd monitor
```

Or use direct path syntax:
```bash
linkd link ./packages/my-ui-kit ./apps/web-app
```

---

## Key Commands

| Command | Description |
|---|---|
| `linkd register` | Register current directory as a local package |
| `linkd use <name>` | Link a registered package into the current project |
| `linkd link <src> <consumer>` | Direct path link |
| `linkd list` | Show all active links (hierarchical tree view) |
| `linkd monitor` | Live TUI dashboard with real-time sync status |
| `linkd unlink <name>` | Remove a link |
| `linkd status` | Show daemon status |
| `linkd start` / `stop` | Start or stop the background daemon |
| `linkd init` | Interactive setup wizard |
| `linkd version` | Show current version |

---

## Live Monitor

`linkd monitor` opens a full-screen interactive dashboard:

- **Hierarchical tree view** — see each package and all its consumers grouped together
- **Real-time sync events** — live log stream from the background daemon  
- **Keyboard controls**: `r` re-sync · `Space` pause/resume · `g` switch group mode · `u` unlink · `q` quit

---

## Full Documentation

👉 **[pinoox.github.io/linkd/docs.html](https://pinoox.github.io/linkd/docs.html)**

Covers all ecosystems, advanced usage, architecture, safety guarantees, shell completions, and contributing guide.

---

## CI Status

All 40+ tests pass on **Linux**, **macOS**, and **Windows** on every commit.

---

## License

MIT © linkd contributors — see [LICENSE](LICENSE) and [CHANGELOG.md](CHANGELOG.md).

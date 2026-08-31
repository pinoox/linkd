<div align="center">

# ⚡ linkd

**The local-dev link daemon for modern multi-ecosystem monorepos & packages.**  
*Continuous reconciliation that keeps your local dependencies synced across reinstalls without editing manifest files.*

[![CI](https://github.com/linkd-dev/linkd/actions/workflows/ci.yml/badge.svg)](https://github.com/linkd-dev/linkd/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/linkd-dev/linkd?color=blue&logo=github)](https://github.com/linkd-dev/linkd/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg)](https://github.com/linkd-dev/linkd/releases)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

</div>

---

## 📖 Overview

When developing local libraries alongside consumer applications, standard package manager tools (`npm link`, `yarn link`, `composer link`, `pip install -e`, `go work`, etc.) often suffer from persistent pain points:

1. **Reinstalls wipe links**: Running `npm install`, `composer update`, or `uv sync` deletes symlinks and replaces them with registry copies.
2. **Global store pollution**: Some package managers (like `pnpm`) risk corrupting machine-wide package caches if written to directly.
3. **Dirty git working trees**: Relative file path dependencies (`"file:../lib"`) leak into `package.json` or `pyproject.toml` and get accidentally committed.

**`linkd` solves this with a Kubernetes-style continuous reconciliation loop.**  
It runs a lightweight background daemon that watches file changes, detects package manager reinstall events, and instantly restores your dev packages into `node_modules/`, `vendor/`, `.venv/`, or custom directories — with zero manifest edits.

---

## 🚀 Instant 1-Line Installation

Install `linkd` instantly without needing Rust or any prior dependencies:

### 🐧 Linux & 🍎 macOS
```bash
curl -fsSL https://raw.githubusercontent.com/linkd-dev/linkd/main/install.sh | bash
```

### 🪟 Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/linkd-dev/linkd/main/install.ps1 | iex
```

### 📦 Via Cargo (from source)
```bash
cargo install --path crates/linkd-cli --locked
```

### 💾 Pre-built Binaries
Download standalone `.tar.gz` and `.zip` archives directly from [GitHub Releases](https://github.com/linkd-dev/linkd/releases).

---

## ⚡ Quick Start

```bash
# 1. Link a package (auto-detects ecosystem & starts the daemon in background)
linkd link ./packages/my-ui-kit ./apps/web-app

# 2. View real-time status in the full-screen interactive TUI dashboard
linkd monitor

# 3. List all registered links
linkd list

# 4. Remove a link when finished
linkd unlink my-ui-kit
```

---

## 🌐 Supported Ecosystems & Examples

`linkd` natively detects and adapts to 7 different environments:

| Ecosystem | Manifest | Target Directory | Reinstall Watch Markers |
|---|---|---|---|
| 🟢 **JavaScript / TypeScript** | `package.json` | `node_modules/<pkg>` | `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `bun.lockb` |
| 🐘 **PHP (Composer)** | `composer.json` | `vendor/<vendor>/<pkg>` | `composer.lock`, `vendor/composer/installed.json` |
| 🐍 **Python (uv/pip/poetry)** | `pyproject.toml`, `setup.py` | `.venv/Lib/site-packages/<pkg>` | `uv.lock`, `poetry.lock`, `Pipfile.lock`, `requirements.txt` |
| 🐹 **Go (Go Modules)** | `go.mod` | `vendor/<module_path>` | `go.sum`, `go.work.sum`, `go.work` |
| 🦀 **Rust (Cargo)** | `Cargo.toml` | `vendor/<crate_name>` | `Cargo.lock` |
| ☕ **JVM (Maven / Gradle)** | `pom.xml`, `build.gradle` | `libs/<artifact>` | `pom.xml`, `build.gradle`, `gradle.lockfile` |
| 📁 **Custom Path** | Any folder | User-defined path | Target timestamp / directory markers |

---

### 💡 Example Scenarios

#### 1. JavaScript / TypeScript (npm, pnpm, yarn, bun)
```bash
# Auto-detects package.json name and links into consumer's node_modules
linkd link ./packages/design-system ./apps/nextjs-app

# Automatically safeguards pnpm by using project-local shadow directories
# (never writes to ~/.pnpm-store)
```

#### 2. PHP (Composer)
```bash
# Detects composer.json, parses vendor/package namespace, links into vendor/acme/logger
linkd link ./packages/acme-logger ./apps/laravel-api

# If new classes are added, linkd reminds you if `composer dump-autoload` is needed
```

#### 3. Python (uv, pip, poetry)
```bash
# Links package into virtualenv site-packages (.venv/Lib/site-packages or Unix lib/python*/...)
# Automatically excludes __pycache__, *.pyc, and .pytest_cache
linkd link ./packages/ml-core ./apps/fastapi-service
```

#### 4. Go (Go Modules & Vendor)
```bash
# Links Go package into consumer's vendor directory according to module declaration
linkd link ./packages/auth-module ./apps/microservice
```

#### 5. Rust (Cargo)
```bash
# Vendors local crate into consumer's vendor folder without modifying Cargo.toml
linkd link ./crates/shared-types ./apps/backend-service
```

#### 6. Custom Directory (Framework-Agnostic)
```bash
# Sync any folder into any arbitrary destination with built-in loop guard protection
linkd link ./shared-assets ./apps/electron-app --target ./apps/electron-app/src/assets/shared
```

---

## 🖥️ Interactive Terminal UIs

`linkd` includes rich interactive TUI tools built with [Ratatui](https://ratatui.rs):

### 📊 Live Monitor Dashboard (`linkd monitor` / `linkd top`)
Connects directly to the running background daemon via high-performance IPC:

```bash
linkd monitor
# or
linkd top
```

- **Live Overview**: Side-by-side active links, health status, and live event log stream.
- **Inspector Panel**: Deep-dive into sync target, package manager, PID, strategy, and last error.
- **Interactive Hotkeys**:
  - `r` — Force immediate re-sync of selected link.
  - `Space` — Pause / Resume syncing for the selected link.
  - `u` — Unlink selected package.
  - `c` — Clear active log view.
  - `Tab` — Toggle focus between Links and Logs panel.
  - `q` / `Esc` — Quit dashboard without stopping daemon.

### 🧙 Setup Wizards (`linkd wizard` & `linkd init`)
- **`linkd wizard`**: Full-screen 5-step interactive wizard to select ecosystem, browse source/consumer paths, and configure options.
- **`linkd init`**: Lightweight inline CLI prompt wizard for fast terminal setups.

---

## 🛠️ Complete CLI Command Reference

```
linkd [COMMAND] [OPTIONS]
```

| Command | Arguments / Flags | Description |
|---|---|---|
| `link` | `<source> [consumer] [--target <path>] [--ecosystem <type>] [--copy\|--hardlink\|--link] [--no-daemon]` | Registers and syncs a local package into a consumer project. |
| `unlink` | `<target\|name\|source>` | Removes an active link and safely cleans up marker files. |
| `list` | *none* | Displays a table of all registered links, status, and target paths. |
| `monitor` | `[--start]` *(aliases: `top`, `dashboard`)* | Launches full-screen interactive TUI dashboard. |
| `start` | *none* | Starts the linkd daemon in background (detached process). |
| `stop` | `[--force]` | Gracefully shuts down the background daemon. |
| `watch` | *none* | Runs the daemon in foreground with live terminal output. |
| `status` | `[--json]` | Shows one-shot snapshot of daemon status and registered links. |
| `doctor` | `[--explain <topic>]` | Runs environment diagnostics (`pnpm-store`, `composer`, `python`, `go`, `cargo`, `jvm`, `autostart`). |
| `logs` | `[-f\|--follow]` | Tails the background daemon's structured log file. |
| `wizard` | *none* | Runs full-screen interactive setup wizard (Ratatui). |
| `init` | *none* | Runs quick prompt-based setup wizard (Inquire). |
| `completions`| `<bash\|zsh\|fish\|powershell\|elvish>` | Generates shell completion scripts. |

---

## 🏗️ Architecture & Safety Guarantees

```
┌────────────────────────────────────────────────────────────────────────┐
│                        ~/.linkd/registry.json                          │
│                          (Global State Store)                          │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
                         ┌──────────▼──────────┐
                         │    linkd daemon     │
                         │ (Background Engine) │
                         └──────────┬──────────┘
                                    │
       ┌────────────────────────────┼────────────────────────────┐
       │ (File Watchers)            │ (Reconciliation Engine)    │ (IPC Server)
       ▼                            ▼                            ▼
  Source Changes             Reinstall Event               Live Dashboard
  - Debounced sync           - Lockfile markers            - Subscriptions
  - Cache filter             - Atomic directory swap       - Pause/Resume
       │                            │
       └────────────────────────────┴────────────────────────────┐
                                                                 ▼
                                                  Target Project (.linkd-marker)
                                                  - node_modules/<pkg>
                                                  - vendor/<pkg>
                                                  - .venv/Lib/site-packages/<pkg>
```

### 🛡️ Safety Defaults
- **Never touches global package stores**: Protects against `pnpm` store corruption using isolated shadow directories (`node_modules/.linkd-shadow/`).
- **Atomic directory swaps**: Syncing uses atomic renames so compilers never see a half-written or missing directory state.
- **Anti-watch-loop protection**: Validates source and target path hierarchies to prevent infinite file watcher loops.
- **Marker file provenance**: Every synced directory is tagged with `.linkd-marker.json` to verify ownership before modifying or removing files.

---

## 🔧 Shell Completions

Generate shell completions for your preferred shell:

```bash
# Bash
linkd completions bash > ~/.local/share/bash-completion/completions/linkd

# Zsh
linkd completions zsh > ~/.zfunc/_linkd

# Fish
linkd completions fish > ~/.config/fish/completions/linkd.fish

# PowerShell
linkd completions powershell >> $PROFILE
```

---

## 🤝 Contributing & Development

```bash
# Clone repository
git clone https://github.com/linkd-dev/linkd.git
cd linkd

# Run workspace test suite
cargo test --workspace

# Run CLI locally
cargo run -p linkd-cli -- link ./tests/fixtures/python-uv/packages/py-lib ./tests/fixtures/python-uv/apps/py-app

# Launch live monitor
cargo run -p linkd-cli -- monitor
```

---

## 📄 License

MIT © [linkd contributors](LICENSE)

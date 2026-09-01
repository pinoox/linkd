# Changelog

All notable changes to the **linkd** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.2] - 2026-09-01

### 🚀 Public Release (v0.1.2)

`linkd` is a continuous local-dev package link daemon engineered to eliminate fragile symlinks and broken package registries across modern multi-ecosystem monorepos.

---

### ✨ Added

#### 1. Multi-Ecosystem Adapter Architecture
- **JavaScript & TypeScript (`linkd-adapters-npm`)**:
  - Auto-detects `package.json`, scoped packages (e.g. `@acme/ui`), and resolves targets to `node_modules/<name>`.
  - Full support for `npm`, `pnpm`, `yarn`, and `bun`.
  - Monitors `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, and `bun.lockb` for reinstall events.
- **PHP Composer (`linkd-adapters-composer`)**:
  - Auto-detects `composer.json` namespaces and syncs directly into consumer `vendor/<vendor>/<package>`.
  - Autoload marker detection with actionable `composer dump-autoload` suggestions.
  - Monitors `composer.lock` and `installed.json`.
- **Python (`linkd-adapters-python`)**:
  - Supports PEP 621 `pyproject.toml`, `setup.py`, and `setup.cfg`.
  - Automatically resolves virtual environments (`.venv/Lib/site-packages` on Windows, `.venv/lib/python*/site-packages` on Unix).
  - Automatically excludes `__pycache__`, `*.pyc`, `.pytest_cache`, `.mypy_cache`, and `*.egg-info`.
  - Monitors `uv.lock`, `poetry.lock`, `Pipfile.lock`, and `requirements.txt`.
- **Go Modules (`linkd-adapters-go`)**:
  - Parses module paths from `go.mod` and resolves sync destination to `vendor/<module_path>`.
  - Monitors `go.sum`, `go.work.sum`, `go.work`, and `vendor/modules.txt`.
- **Rust Cargo (`linkd-adapters-cargo`)**:
  - Parses `[package.name]` from `Cargo.toml` and syncs into consumer `vendor/<crate_name>`.
  - Automatically filters `target/`, `.cargo/`, and `.git/`.
  - Monitors `Cargo.lock` and `.cargo/config.toml`.
- **Java & Kotlin JVM (`linkd-adapters-jvm`)**:
  - Extracts Group ID and Artifact ID from `pom.xml`, `build.gradle`, or `build.gradle.kts`.
  - Routes sync targets to consumer `libs/<artifact>`.
  - Filters `build/`, `target/`, `.gradle/`, and `*.class` files.
- **Custom Directories (`linkd-adapters-custom`)**:
  - Framework-agnostic linking for arbitrary asset directories and internal libraries via `--target`.

#### 2. Global Package Store & Multi-Consumer Workflows
- **Global Package Registration (`linkd register` / `linkd pin` / `linkd add`)**:
  - Register any local library once globally into `~/.linkd/packages.json`.
- **Global Package Consumer Attachment (`linkd use <package>` / `linkd on <package>`)**:
  - Connect a registered package into any consumer application with a single command without remembering directory paths.
- **Package Management (`linkd packages` & `linkd unregister`)**:
  - List and unregister globally pinned libraries.
- **Multi-Consumer Live Reconciliation**:
  - Connect 1 library to multiple distinct consumer applications simultaneously. Editing 1 file in the library reconciles all attached consumers in parallel.
- **Windows Path Normalization**:
  - Completely strips verbatim `\\?\` and `\\?\UNC\` prefixes across all commands, interactive prompts, and status tables.

#### 3. Continuous Reconciliation Engine (`linkd-sync` & `linkd-watcher`)
- **State Machine & Loop**: Reconciles desired state in `~/.linkd/registry.json` against filesystem reality.
- **Atomic Swapping**: Synchronizes changes to a temporary staging root and performs atomic directory swaps via filesystem renames to prevent build tool race conditions.
- **Reflink Copy Acceleration**: Leverages `reflink-copy` for instant copy-on-write file cloning on APFS, Btrfs, XFS, and ReFS.
- **Marker Provenance**: Seals all synced directories with `.linkd-marker.json` to guarantee safe cleanup without deleting untracked files.

#### 3. Enterprise Safety Guarantees
- **pnpm Global Store Safety Gate**: Detects and intercepts hardlinks pointing to `~/.pnpm-store`, redirecting writes into project-local shadow directories (`node_modules/.linkd-shadow/`).
- **Anti-Watch-Loop Validation**: Hierarchical ancestor validation prevents infinite watcher loops when linking nested parent/child paths.

#### 4. Interactive Terminal UIs (TUI) & Tools
- **Live Monitor Dashboard (`linkd monitor` / `linkd top` / `linkd dashboard`)**:
  - Full-screen Ratatui dashboard connected to the running daemon via IPC.
  - Live link statuses, file sync metrics, event logs, and an inspector panel.
  - Real-time keyboard shortcuts:
    - <kbd>r</kbd>: Force immediate link reconcile.
    - <kbd>Space</kbd>: Pause / Resume sync.
    - <kbd>u</kbd>: Unlink package.
    - <kbd>c</kbd>: Clear active log view.
    - <kbd>Tab</kbd>: Switch panel focus.
    - <kbd>q</kbd> / <kbd>Esc</kbd>: Exit monitor without stopping the daemon.
- **Setup Wizards**:
  - `linkd wizard`: Full-screen 5-step guided Ratatui wizard with arrow navigation and path validation.
  - `linkd init`: Sequential terminal prompt questionnaire built with Inquire.
- **Diagnostics (`linkd doctor`)**:
  - Validates inotify watches, socket permissions, package manager binaries, and global store safety.
  - In-depth help topics with `linkd doctor --explain <topic>`.

#### 5. Cross-Platform Background Daemon (`linkd-daemon` & `linkd-ipc`)
- High-performance non-blocking IPC over Unix Domain Sockets (`~/.linkd/linkd.sock`) on Linux/macOS and Named Pipes (`\\.\pipe\linkd-ipc`) on Windows.
- Cross-platform process management with PID files and stale process cleanup.
- Autostart capability that launches the background daemon transparently upon running `linkd link`.
- Systemd User Unit (Linux) and Launchd Agent plist (macOS) service configurations.

#### 6. Installation & Distribution
- Zero-prerequisite 1-line installation scripts:
  - Linux & macOS: `curl -fsSL https://raw.githubusercontent.com/pinoox/linkd/master/install.sh | bash`
  - Windows: `irm https://raw.githubusercontent.com/pinoox/linkd/master/install.ps1 | iex`
- GitHub Actions CI/CD matrix producing standalone release binaries:
  - `linkd-x86_64-unknown-linux-gnu.tar.gz`
  - `linkd-x86_64-unknown-linux-musl.tar.gz` (fully static binary)
  - `linkd-aarch64-unknown-linux-gnu.tar.gz` (ARM64 Linux)
  - `linkd-x86_64-apple-darwin.tar.gz` (Intel macOS)
  - `linkd-aarch64-apple-darwin.tar.gz` (Apple Silicon M-series)
  - `linkd-x86_64-pc-windows-msvc.zip` (Windows 64-bit)
- Official GitHub Pages website and interactive documentation portal hosted at [https://pinoox.github.io/linkd/](https://pinoox.github.io/linkd/).

---

[0.1.2]: https://github.com/pinoox/linkd/releases/tag/v0.1.2

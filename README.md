# linkd

Local-dev link daemon for npm/pnpm/yarn/bun and Composer. Keeps your in-development packages synced into `node_modules` or `vendor` **without** editing manifest files — survives reinstalls via a reconciliation loop.

## Quick start

```bash
# Install
cargo install --path crates/linkd-cli
# or: curl -fsSL https://linkd.dev/install.sh | sh

# Link a local package (auto-starts background daemon)
linkd link ./packages/my-lib ../my-app

# PHP / Composer
linkd link ./packages/php-lib ../php-app

# Custom path (no package manager)
linkd link ./shared ./apps/web --target ./apps/web/lib/shared

# Daemon control
linkd start    # background
linkd stop     # graceful shutdown
linkd watch    # foreground + live UI
```

## Why linkd?

Package managers copy registry versions on every install. Traditional symlinks break when installs rerun. **linkd** runs a background controller that restores your dev copy after installs — like a Kubernetes reconciler for local paths.

### Safety defaults

- **reflink/copy** by default (not hardlink)
- **Never writes to pnpm global store** — project-local shadow copies
- **Atomic directory swap** — no window where the package path is missing
- **PM completion markers** instead of fragile process detection
- **Nested path guard** — prevents watch loops on custom paths

## Commands

| Command | Description |
|---|---|
| `linkd link <source> [consumer]` | Register + sync (npm/composer auto-detect) |
| `linkd link ... --target <path>` | Custom path sync |
| `linkd unlink <name\|source>` | Remove a link |
| `linkd list` | List active links |
| `linkd start` | Background daemon |
| `linkd stop [--force]` | Stop daemon |
| `linkd watch` | Foreground daemon + live UI |
| `linkd status [--json]` | Daemon + links snapshot |
| `linkd doctor [--explain topic]` | Environment checks |
| `linkd logs [-f]` | View daemon logs |
| `linkd init` | Quick setup wizard (inquire) |
| `linkd wizard` | Full-screen setup wizard (ratatui) |
| `linkd completions <shell>` | Shell completions |

## Architecture

```
~/.linkd/registry.json   ← desired state (global)
linkd daemon             ← watchers + reconciler
node_modules/<pkg>       ← npm sync target + .linkd-marker.json
vendor/<v>/<p>           ← composer sync target + .linkd-marker.json
```

See [docs/adr/](docs/adr/) for design decisions.

## Development

```bash
cargo test --workspace
cargo run -p linkd-cli -- link ./tests/fixtures/my-lib ./tests/fixtures/consumer-smoke
cargo run -p linkd-cli -- start
```

## License

MIT

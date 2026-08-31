# linkd

Local-dev link daemon for npm/pnpm. Keeps your in-development packages synced into `node_modules` **without** editing `package.json` — survives `npm install` / `pnpm install` via a reconciliation loop.

## Quick start

```bash
# Install
cargo install --path crates/linkd-cli
# or: curl -fsSL https://linkd.dev/install.sh | sh

# Link a local package into a consumer project
linkd link ./packages/my-lib ../my-app

# Run daemon with live terminal UI
linkd watch
```

## Why linkd?

Package managers copy registry versions into `node_modules` on every install. Traditional `npm link` / symlinks break when installs rerun. **linkd** runs a background controller that restores your dev copy after installs — like a Kubernetes reconciler for local paths.

### Safety defaults (v2)

- **reflink/copy** by default (not hardlink)
- **Never writes to pnpm global store** — uses project-local shadow copies
- **Atomic directory swap** — no window where the package path is missing
- **PM completion markers** (`.modules.yaml`, `.package-lock.json`) instead of fragile process detection

## Commands

| Command | Description |
|---|---|
| `linkd link <source> [consumer]` | Register + sync a local package |
| `linkd unlink <name\|source>` | Remove a link |
| `linkd list` | List active links |
| `linkd watch` | Foreground daemon + live UI |
| `linkd status [--json]` | One-shot status |
| `linkd doctor [--explain pnpm-store]` | Environment checks |
| `linkd logs [-f]` | View daemon logs |
| `linkd init` | Interactive setup wizard |
| `linkd completions <shell>` | Shell completions |

## Architecture

```
~/.linkd/registry.json   ← desired state (global, not in your repo)
linkd daemon             ← watchers + reconciler
node_modules/<pkg>       ← actual state + .linkd-marker.json
```

See [docs/adr/](docs/adr/) for design decisions.

## Development

```bash
cargo test --workspace
cargo run -p linkd-cli -- link ./fixtures/my-lib ./fixtures/my-app
cargo run -p linkd-cli -- watch
```

## License

MIT

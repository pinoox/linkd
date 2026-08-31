# ADR-006: Background daemon lifecycle

## Status

Accepted

## Context

`linkd watch` requires a foreground terminal. Users need background operation with start/stop and auto-start on `linkd link`.

## Decision

- `linkd start` spawns detached daemon; PID stored in `~/.linkd/daemon.pid` as JSON
- PID validation checks process name contains `linkd` (stale PID cleanup)
- `linkd stop` sends IPC Shutdown; `--force` kills process if needed
- Windows: `CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS`
- Auto-start on link when `~/.linkd/config.json` has `auto_start_daemon: true` (default)

## Consequences

- Users run `linkd start` once (or rely on auto-start) instead of keeping `watch` open
- `linkd watch` remains for dev/debug with live TUI

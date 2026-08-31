# ADR-002: Atomic Directory Swap for Sync

## Status

Accepted

## Context

Delete-then-recreate of `node_modules/<pkg>` creates a window where the package path is missing. Bundlers and dev servers reading mid-sync can crash or serve stale modules.

## Decision

Sync always builds into a temp directory under `~/.linkd/tmp/<linkId>-<timestamp>`, writes marker, then atomically swaps into place.

### Unix

Single `rename(tmp, target)` when target absent; otherwise three-step swap.

### Windows

When `rename(tmp, target)` fails with target exists:

1. `rename(target → target.old-<ts>)`
2. `rename(tmp → target)`
3. Async delete `target.old-*`

**Invariant:** the canonical target path is never absent — either old or new content is always reachable at the path.

## Consequences

- Temp disk usage spikes during sync (cleaned after swap).
- Slightly more complex Windows path handling.

## Test Requirements

- `atomic_swap_under_load`: concurrent readers never see empty directory.
- `reinstall_simulation`: marker restored after simulated PM overwrite.

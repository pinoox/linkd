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

When `rename(tmp, target)` fails because the target directory already exists, use **in-place content replacement**: keep the target path present, clear its contents, copy from tmp, then remove tmp. This preserves the ADR invariant that the canonical package path is never absent (critical for bundlers on NTFS).

For fresh installs (target absent), a single `rename(tmp, target)` is used.

## Consequences

- Temp disk usage spikes during sync (cleaned after swap).
- Slightly more complex Windows path handling.

## Test Requirements

- `atomic_swap_under_load`: concurrent readers never see empty directory.
- `reinstall_simulation`: marker restored after simulated PM overwrite.

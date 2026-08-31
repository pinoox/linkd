# ADR-001: Reflink/Copy as Default Sync Strategy

## Status

Accepted

## Context

Local dev linking tools often default to hardlinks for speed. Hardlinks share inodes: writes through `node_modules` silently mutate the source tree. Bundlers and editors may also open files in `node_modules` for hot reload, amplifying the risk.

Filesystem support varies:

| FS | Reflink (CoW) | Hardlink | Notes |
|---|---|---|---|
| APFS (macOS) | Yes (`clonefile`) | Yes | Reflink preferred |
| Btrfs/XFS (Linux) | Yes (`FICLONE`) | Yes | Reflink preferred |
| NTFS (Windows) | No (ReFS only) | Yes | Copy fallback |
| Cross-device | No reflink | No hardlink | Copy only |

## Decision

- Default strategy: **reflink when same-volume and supported**, else **copy**.
- **Hardlink** only via explicit `--hardlink` with CLI warning.
- **Symlink** only via explicit `--link`.

## Consequences

- Slightly slower sync on Windows (copy) but safe by default.
- Source tree never mutated through accidental writes in `node_modules`.
- `linkd doctor` reports detected strategy per link.

## Test Requirements

- Integration: Windows CI falls back to copy without error.
- Unit: `SyncEngine` selects copy when reflink returns unsupported.

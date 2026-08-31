# ADR-003: pnpm Global Store Write Gate

## Status

Accepted

## Context

pnpm hoists packages into a global content-addressable store. `node_modules/<pkg>` is often a symlink into `.pnpm/...` which may ultimately resolve into the global store (e.g. `~/.local/share/pnpm/store`).

Writing dev versions directly into the global store would affect **every project** on the machine using that package version hash.

## Decision

1. Before any write, resolve target path (follow symlinks).
2. Compare against cached `pnpm store path` output and known store roots.
3. If resolved path is inside global store → **never write there**.
4. Instead:
   - Create project-local shadow at `node_modules/.linkd-shadow/<pkg>`
   - Sync dev content into shadow
   - Repoint project symlink to shadow
   - Set `isolationMode: "shadow"` in registry

`SyncEngine::write()` enforces an allowlist: paths must be under `consumerRoot/node_modules/` and must not be under global store roots.

Violations return a controlled error (not silent continue).

## Consequences

- Extra disk for shadow copies when pnpm uses global store symlinks.
- Shadow symlinks may need re-reconciliation after `pnpm install`.

## Test Requirements

- **`pnpm_store_leak_test`** (mandatory CI): assert no bytes written under global store path during sync.
- `write_guard` unit tests reject global store paths.

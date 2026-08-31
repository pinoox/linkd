# ADR-007: Path validation watch-loop guard

## Status

Accepted

## Context

Custom path links where source and target are nested can cause watcher → reconcile → write → watcher infinite loops.

## Decision

`validate_link_paths` rejects identical or nested source/target paths (canonicalized best-effort) at link registration time in CLI, wizard, and adapter `resolve_link`.

## Consequences

- Custom path users must pick non-overlapping directories
- Integration test covers nested rejection

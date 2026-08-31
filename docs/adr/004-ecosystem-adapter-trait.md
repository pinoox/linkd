# ADR-004: Ecosystem adapter trait

## Status

Accepted

## Context

linkd started as npm-only. Expanding to Composer and custom paths requires per-ecosystem logic for target resolution, file listing, and install completion markers without duplicating the sync engine.

## Decision

Introduce `linkd-adapters` with an `EcosystemAdapter` trait and ecosystem-specific implementations (`npm`, `composer`, `custom`). The reconciler dispatches through adapters; `linkd-sync` remains unchanged.

## Consequences

- Adding a new ecosystem is a new adapter crate + enum variant
- Shared validation (`validate_link_paths`) lives in the adapter layer
- Registry v2 stores `ecosystem`, `link_mode`, and optional `custom_target`

# ADR-005: Composer vendor layout and autoload markers

## Status

Accepted

## Context

Composer installs packages under `vendor/<vendor>/<package>`. Reinstalls overwrite linked packages. Optimized autoload (`composer dump-autoload -o`) caches classmaps that won't reflect new PHP files until regenerated.

## Decision

- Resolve sync target to `vendor/<vendor>/<package>` from `composer.json` name
- Watch completion markers: `vendor/composer/installed.json`, `composer.lock`, `autoload_classmap.php`, `autoload_static.php`
- Post-sync hint when PHP files exist in source: suggest `composer dump-autoload`
- File listing via walkdir (no composer binary required for sync)

## Consequences

- New classes may require manual `composer dump-autoload` in consumer
- Doctor can explain autoload behavior

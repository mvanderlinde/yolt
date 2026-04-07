# yolt specifications

This directory is the **behavioral contract** for yolt. Implementation and tests should trace to these documents. When behavior changes, update the relevant spec in the same change.

## Reading order

1. [product.md](product.md) — Goals, non-goals, platform guarantees, glossary.
2. [cli.md](cli.md) — Commands, flags, exit codes, examples.
3. [config.md](config.md) — Configuration sources, defaults, validation.
4. [watcher.md](watcher.md) — Filesystem watching, debouncing, event mapping.
5. [backup-store.md](backup-store.md) — On-disk layout, backup run IDs, copy semantics.
6. [retention.md](retention.md) — Time-based and disk-cap pruning.
7. [ignore-rules.md](ignore-rules.md) — Pattern syntax, default list, overrides.
8. [distribution.md](distribution.md) — Homebrew, releases, supported macOS versions.
9. [testing.md](testing.md) — Automated tests and release checklist.

## Document index

| Document | Primary audience |
|----------|------------------|
| product.md | Everyone |
| cli.md | Users, integrators |
| config.md | Users, operators |
| watcher.md | Implementers |
| backup-store.md | Implementers |
| retention.md | Implementers |
| ignore-rules.md | Users, implementers |
| distribution.md | Maintainers |
| testing.md | Contributors, CI |

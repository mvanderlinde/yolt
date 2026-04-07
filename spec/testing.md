# Testing specification

## Unit tests

- **Ignore matcher:** paths match / don’t match expected patterns.
- **Backup run ID:** monotonic / unique / sortable.
- **Backup dedup:** zero-byte sources are skipped; identical content (same SHA-256 as newest backup) is not copied twice (`backup::tests`).
- **Retention:** with temp dirs, verify ordering of deletes (oldest first) and time cutoff.
- **Config parsing:** TOML + env + CLI merge.

## Integration tests

- **Tempdir:** create files, run watcher briefly, assert backup appears (may require short sleep; use `notify` debounce timing).
- **Restore:** synthetic backup runs under a temp backup root; assert `runs_containing_rel_path` order and `restore(..., dry_run)` pick the expected run id (`tests/restore_test.rs`).

## Manual checklist (releases)

- `yolt --version`
- `yolt watch /tmp/test-watch --print-config`
- Modify file under watch; confirm new run under backup root.

## Traceability

Tests should reference spec sections in comments where helpful (e.g. `// spec/retention.md: disk cap`).

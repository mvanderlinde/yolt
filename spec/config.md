# Configuration specification

## Sources (precedence)

Higher items override lower where applicable for **scalar** values (last wins):

1. Default values (built into the binary).
2. TOML file (`--config`), if provided.
3. Environment variables (`YOLT_*`).
4. Command-line flags.

**Ignore patterns:** Defaults (unless `--no-default-ignores`) **union** with `.yoltignore` **union** with repeated `--ignore` flags.

## Required inputs

- **Watch root** must be set via `--watch` or `YOLT_WATCH` or `watch` in config file.

## Fields

| Field | Type | Default | Notes |
|-------|------|---------|-------|
| `watch` | string | — | Directory to watch; must exist as a directory. |
| `backup_root` | string | `{temp_dir}/yolt` | Root for all backup runs. Default uses `std::env::temp_dir()` (per-user on macOS, not world-writable `/tmp`). |
| `retention` | duration | `30m` | Parsed as humantime (e.g. `30m`, `1h`). |
| `max_disk` | u64 or string | `0` | `0` disables. Accepts plain bytes or suffix `K`, `M`, `G`, `T`. |
| `debounce_ms` | u64 | `300` | Milliseconds. |
| `session_idle_ms` | u64 | `10000` | If the next backup batch starts within this many ms of the previous batch finishing, **reuse** the same `backup_run_id` folder (session). Reduces run-folder sprawl during bursts. |
| `prune_interval_secs` | u64 | `60` | Seconds between retention sweeps. |
| `snapshot_initial` | bool | `true` | Initial full snapshot at startup. |
| `no_default_ignores` | bool | `false` | Maps to `--no-default-ignores`. |
| `ignore` | array of string | `[]` | Extra patterns. |

## TOML example

```toml
watch = "/Users/me/project"
backup_root = "/path/to/backup-root"
retention = "30m"
max_disk = "2G"
debounce_ms = 300
session_idle_ms = 10000
snapshot_initial = true
ignore = ["custom-cache/", "*.local"]
```

## Validation

- `watch` must be an existing directory.
- `backup_root` is created if it does not exist (parent must exist or be creatable).
- `retention` must be > 0.
- `max_disk` if non-zero must be >= minimum practical size (implementation-defined; e.g. 1 MB) or warn.

## Error messages

Errors on stderr, one primary line, prefixed with `error:` where applicable.

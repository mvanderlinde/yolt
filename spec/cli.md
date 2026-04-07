# CLI specification

## Name

`yolt`

## Subcommands

### `watch`

Run the watcher in the **foreground** until interrupted (`SIGINT` / `SIGTERM`). This is the primary mode.

`watch` accepts an optional positional `DIR` argument. If omitted, it defaults to `.` (current directory).

Only one `yolt watch` process may watch a given canonical watch root at a time. If another process
already holds that watch lock, `watch` exits with an error.

### `restore`

Restore a **file or directory** from the backup store into the watch tree.

- **Positional `PATH`:** relative to the watch root (e.g. `src/main.rs`), or an **absolute** path that lies under the watch root.
- **`--back N`:** skip the `N` newest backups that contain this path (default `0` = restore from the newest matching run).
- **`--dry-run`:** print which run would be used and the destination path; do not copy.

Defaults project root to current directory (`.`). Use `--dir PATH` to target another project root. Also supports `--backup-root` and `--config` (see below).

### `help`

Shows help (also via `-h` / `--help` on the root or subcommand).

## `watch` flags

| Flag | Env var | Description |
|------|---------|-------------|
| `--backup-root PATH` | `YOLT_BACKUP_ROOT` | Directory for backups. Default: `{temp_dir}/yolt` (see `std::env::temp_dir`). |
| `--retention DURATION` | `YOLT_RETENTION` | Max age of backup runs before eligible for time-based prune. Default: `30m`. Human-readable (e.g. `30m`, `1h`, `90s`). |
| `--max-disk BYTES` | `YOLT_MAX_DISK` | Max total bytes under backup root; oldest runs removed until under cap. `0` = disabled. No default limit. |
| `--config PATH` | `YOLT_CONFIG` | Optional TOML config file. |
| `--ignore PATTERN` | — | Repeatable. Extra ignore patterns (gitignore-style). |
| `--no-default-ignores` | — | If set, do not apply built-in default ignore patterns. |
| `--debounce MS` | `YOLT_DEBOUNCE` | Debounce window for coalescing rapid events per path. Default: `300` ms. |
| `--session-idle-ms MS` | `YOLT_SESSION_IDLE_MS` | If a new batch of changes starts within this many ms of the previous batch end, **reuse** the same backup run folder. Default: `10000` ms (10 s). Helps avoid hundreds of run folders per LLM burst. |
| `--snapshot-initial` | `YOLT_SNAPSHOT_INITIAL` | If set (default: true), walk watch root at startup and copy non-ignored files into the first backup run. Use `--no-snapshot-initial` to disable. |
| `--prune-interval SECS` | `YOLT_PRUNE_INTERVAL` | How often to run retention sweep. Default: `60`. |
| `--print-config` | — | Print effective configuration and exit `0`. |

Boolean env vars: `1`, `true`, `yes` (case-insensitive) = true; `0`, `false`, `no` = false.

## Config file

Optional TOML (see [config.md](config.md)). Path from `--config` or default search: `./.yolt.toml` then `~/.config/yolt/config.toml` (first found wins if implemented; v1 may only use `--config` explicitly — implementation uses explicit `--config` + env + flags for simplicity).

**v1 implementation:** `--config` loads TOML; if absent, env + flags only.

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success (`--print-config`, or clean shutdown after `watch`). |
| `1` | User error (invalid args, invalid paths). |
| `2` | Runtime failure (watcher error, I/O error that aborts the run). |

## Stdout / stderr

- **Normal operational logs:** stderr (human-readable).
- **Stdout:** reserved for future machine-readable output; v1 may print nothing to stdout during `watch`.

## Version

`yolt --version` prints crate version.

## Example invocations

```sh
yolt watch ~/Projects/myapp --retention 45m --max-disk 2G
YOLT_WATCH=~/Projects/myapp yolt watch --backup-root /tmp/yolt
yolt watch --config ./.yolt.toml --print-config
yolt watch
yolt restore src/main.rs --dir ~/Projects/myapp
yolt restore src/main.rs --dir ~/Projects/myapp --back 1
```

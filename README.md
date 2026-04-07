# yolt

**Undo destructive LLM actions** — **Yolt** (*You Only Live Twice*) is a macOS CLI that watches a directory tree (via FSEvents) and **automatically backs up files before they change**, so you can revert quickly when AI does things it should not. A safer way to yolo. It is not a replacement for Git or Time Machine. Full behavior, guarantees, and limitations are defined in [`spec/`](spec/README.md).

Backups **skip empty (zero-byte) files** and **skip copying** when file content (SHA-256) is unchanged from the **latest stored backup** of that path, so you do not accumulate duplicate or blank snapshots for the same bytes.

## Limitations (honest)

- macOS does **not** offer a supported “just before write” hook for arbitrary paths. This tool reacts to filesystem events **after** they are reported; that is usually fast enough for interactive work.
- If a file is **deleted** before a backup copy exists, the bytes cannot be read back at delete time. Recovery uses the **last backup from an earlier change** plus the **initial snapshot** when the watcher starts (for files not ignored).

## Requirements

- macOS
- Rust 1.83+ (to build from source)

## Build from source

```sh
cargo install --path .
# or
cargo build --release
# binary: target/release/yolt
```

## Homebrew (tap)

The tap lives in a **separate** repo ([`mvanderlinde/homebrew-yolt`](https://github.com/mvanderlinde/homebrew-yolt)). Maintainer copy lives in this repo under [`packaging/homebrew-tap/`](packaging/homebrew-tap/README.md).

After the tap is published:

```sh
brew tap mvanderlinde/yolt
brew install yolt
```

Until a stable `url`/`sha256` is in the tap formula, use `brew install --HEAD yolt`. Local smoke test from a clone of this repo:

```sh
brew install --build-from-source ./packaging/homebrew-tap/Formula/yolt.rb
```

## Usage

```sh
yolt watch ~/Projects/myapp --retention 30m --max-disk 2G
yolt watch --retention 30m --max-disk 2G
```

### Restore

Restore a file or directory from backups (defaults to the **newest** run that still contains that path):

```sh
yolt restore src/app/page.tsx --dir ~/Projects/myapp
yolt restore src/app/page.tsx --dir ~/Projects/myapp --back 2
yolt restore path/to/dir --dir ~/Projects/myapp --dry-run
```

- `--back N` skips the `N` newest matching backups (0 = latest).
- Uses the same watch-root selection behavior as `watch` and the same `--backup-root` / `--config` options.

### Fewer backup folders (sessions)

By default, new backup **runs** are grouped into **sessions**: while changes keep arriving within **`session_idle_ms`** (default **10 seconds**) of the previous batch, files go into the **same** run folder. After a quiet gap longer than that, the next change starts a **new** run. This avoids hundreds of nearly empty timestamp folders from a single LLM burst.

Tune with `--session-idle-ms` or `YOLT_SESSION_IDLE_MS` / TOML `session_idle_ms`.

Environment variables (see [`spec/cli.md`](spec/cli.md) and [`spec/config.md`](spec/config.md)):

- `YOLT_WATCH` — watch root
- `YOLT_BACKUP_ROOT` — backup root (default `{temp_dir}/yolt`, i.e. per-user temp, not shared `/tmp`)
- `YOLT_RETENTION` — e.g. `30m`, `1h`
- `YOLT_MAX_DISK` — total cap, e.g. `500M`, `2G` (`0` = no cap)
- `YOLT_DEBOUNCE`, `YOLT_PRUNE_INTERVAL`, `YOLT_SNAPSHOT_INITIAL`, `YOLT_SESSION_IDLE_MS`

Optional TOML config:

```toml
watch = "/path/to/project"
backup_root = "/path/to/backup-root"
retention = "30m"
max_disk = "2G"
ignore = ["*.local"]
```

```sh
yolt watch . --config ./.yolt.toml
```

### Ignores

Built-in defaults skip common dependency and cache paths (including `.git/`). Project patterns go in `.yoltignore` at the watch root (gitignore-style). Use `--no-default-ignores` to replace defaults entirely.

### Backup layout

Backups are stored as:

`{backup_root}/{project_id}/{run_id}/…` mirroring paths under the watch root.

`project_id` is deterministic for a given canonical watch root path, so different projects do not
mix backups under the default shared backup root.

```sh
cp "$TMPDIR/yolt/20260406120000123_0000/path/to/file" ./path/to/file
```

## Permissions

For most projects under your home directory, no extra macOS privacy settings are required. Watching certain protected system locations may require **Full Disk Access** for the terminal or the binary.

## License

MIT OR Apache-2.0 (see `LICENSE-MIT` and `LICENSE-APACHE`).

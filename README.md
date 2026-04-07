# yolt

Yolt (*You Only Live Twice*) is a small macOS CLI that watches a project directory for filesystem events and copies files into a backup tree **just before** they change, so you can roll back quickly when an editor or LLM does something you regret. It's a safer way to run LLMs in yolo mode.

Backup retention limits *(30m default)* and optional disk space limits are used to control disk usage.

Details, flags, and behavior are spelled out in `[spec/](spec/README.md)`.

## Requirements

- macOS
- Rust 1.83+ if you build from source

## Install

**Homebrew:**

```sh
brew tap mvanderlinde/yolt
brew install yolt
```

To track `main` instead of a release tarball:

```sh
brew install --HEAD yolt
```

**Manually Build From Source:**

```sh
cargo install --path .
# or
cargo build --release   # binary: target/release/yolt
```

## Usage

Start watching (directory is optional, defaults to the current directory):

```sh
yolt watch
yolt watch ~/Projects/myapp --retention 30m --max-disk 2G
yolt watch --retention 30m --max-disk 2G
```

Restore a file or directory from backups (defaults to the newest run that still has that path):

```sh
yolt restore src/app/page.tsx
yolt restore src/app/page.tsx --dir ~/Projects/myapp
yolt restore src/app/page.tsx --dir ~/Projects/myapp --back 2
yolt restore path/to/dir --dir ~/Projects/myapp --dry-run
```

`--back N` means “skip the N newest matching backups” (`0` = latest). Restore uses the same watch-root rules as `watch`, including `--backup-root` and `--config`.

### Runs and sessions

By default, changes that arrive close together share one **run** folder. While new edits keep landing within **10 seconds** of the previous batch (`session_idle_ms`), backups go into the same run; after a longer gap, the next change starts a new run. That keeps one aggressive edit burst from creating hundreds of nearly empty timestamp directories.

Override with `--session-idle-ms` or `YOLT_SESSION_IDLE_MS` / TOML `session_idle_ms`.

### Config and environment

Optional TOML:

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

Common environment variables: `YOLT_WATCH`, `YOLT_BACKUP_ROOT`, `YOLT_RETENTION`, `YOLT_MAX_DISK`, plus debounce, prune interval, initial snapshot, and session idle — see `[spec/cli.md](spec/cli.md)` and `[spec/config.md](spec/config.md)`.

### Ignores

Built-in patterns skip typical dependency and cache trees (including `.git/`). Add project-specific rules in `.yoltignore` at the watch root (gitignore-style). Use `--no-default-ignores` only if you want to replace the defaults entirely.

### Where backups go

Layout:

`{backup_root}/{project_id}/{run_id}/…`

Paths under the watch root are mirrored under each `run_id`. `project_id` is derived from the canonical watch path so different projects stay separated under one `backup_root`. Default `backup_root` is your user temp directory plus `yolt` (not a world-writable `/tmp`).

Prefer `yolt restore` over copying by hand. If you do browse the tree, a file might look like:

`$TMPDIR/yolt/<project-id>/<run-id>/path/to/file`

## Permissions

Projects under your home folder usually work without extra steps. Watching some system-protected locations can require **Full Disk Access** for Terminal or the `yolt` binary.

## License

MIT OR Apache-2.0 — see `LICENSE-MIT` and `LICENSE-APACHE`.
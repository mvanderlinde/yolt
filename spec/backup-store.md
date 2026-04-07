# Backup store specification

## Layout

```
{backup_root}/
  {project_id}/
    {backup_run_id}/
      {relative_path_from_watch_root}
```

Example:

- Watch root: `/Users/me/proj`
- File: `/Users/me/proj/src/main.rs`
- Backup root: e.g. `$TMPDIR/yolt` (default from `std::env::temp_dir()`)
- Backup run id: `20260406143022123`
- Project id: deterministic id from canonical watch root path
- Stored file: `{backup_root}/{project_id}/20260406143022123/src/main.rs`

Parent directories under the backup run are created as needed (`mkdir -p` semantics).

## Backup run ID

- **Format:** UTC timestamp `YYYYMMDDhhmmssmmm` (17 digits, millisecond resolution).
- **Sortability:** Lexicographic sort of run IDs matches chronological order.
- **Uniqueness:** If two runs occur in the same millisecond, the implementation appends a short suffix (e.g. `-2`, `-3`) or increments milliseconds — implementation must avoid collisions.

**Implementation note:** Use `chrono` UTC with millisecond; on collision, append nanos slice or sequence counter.

## Initial snapshot

- On startup (if enabled), create **one** backup run and copy all non-ignored **files** under watch root into that run (respecting symlink policy).

## Per-event backups (sessions)

- Each **debounced flush** copies the changed files into a `backup_run_id` directory.
- **Session coalescing:** If another flush starts within **`session_idle_ms`** (default 10s) of the previous flush finishing, the implementation **reuses** the same `backup_run_id` so rapid LLM bursts do not create hundreds of nearly empty run folders. After a longer idle gap, the next flush starts a **new** run id.

## Content deduplication

- **Empty files:** Files of **length 0** are not backed up (no “blank” zero-byte snapshots).
- **Unchanged content:** Before copying, the implementation computes **SHA-256** of the source file. If it matches the **newest existing backup** of that relative path (in-memory cache and/or on-disk scan of run folders, newest first), the copy is **skipped** so identical content is not written again.

## Copy semantics

- **Regular files:** Copy bytes and preserve **Unix permissions** (`mode`) when possible (`std::fs::copy` or `copyfile`).
- **Symlinks:** **Do not follow** by default for snapshot walk — **skip** symlinks to avoid escaping watch root, OR copy symlink as symlink if within tree (implementation: **skip** non-regular files for v1 except symlinks that point inside watch — follow spec: **backup symlink as file by copying target** if target is under watch root; else skip). **v1 simplification:** Skip symlinks (log debug). Only regular files.

## Atomicity

- Write to temp name then rename into place under backup run to avoid partial files visible (optional; if not, document best-effort).

## Acceptance criteria

- Restoring a file is copying from a path under `{backup_root}/{project_id}/{id}/...` back to watch tree.
- Backup run directories contain only mirrored relative paths, never absolute watch root prefix repeated incorrectly.

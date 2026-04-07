# Watcher specification

## Backend

On macOS, use the `notify` crate with the **recommended** backend (FSEvents).

## Registration

- **Recursive** watch on the **watch root**.
- Events are filtered through **ignore rules** before any backup action.

## Event handling

| notify event kind | Action |
|-------------------|--------|
| `Create` | If path is a **file** and not ignored, queue backup for that path. |
| `Modify` | If path is a **file** and not ignored, queue backup (debounced). |
| `Remove` | No copy possible; may log at debug level. Relies on prior backups. |
| `Rename` | Treat as remove + create on new path; attempt backup of new path if file. |
| Other / `Any` | Inspect path; if non-directory file and not ignored, queue backup. |

**Directories:** Do not copy directory nodes as files; only **regular files** are backed up.

## Debouncing

- Per **canonical path** key, coalesce events within `debounce_ms` (default 300ms).
- On debounce fire, enqueue one **copy job** for that path.

## Failure modes

- If the watcher returns an error, log and exit with code `2`.
- Spurious duplicate events are acceptable; idempotent copy to the **current backup run** is acceptable (last write wins within same run).

## Acceptance criteria

- Modifying a tracked file produces at least one backup copy within debounce + processing time under normal load.
- Ignored paths never enqueue a copy.

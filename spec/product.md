# Product specification

## Problem statement

Interactive tools (including LLM-assisted editors) can change or delete project files unexpectedly. Users need **recent, restorable copies** of files under a chosen directory so they can recover from mistakes without relying solely on undo or version control.

## Goals

- Watch a user-configured directory tree on **macOS** and create **backup copies** of files when they change.
- Store backups under a dedicated backup root with **mirrored relative paths** for straightforward manual restore (`cp`).
- **Ignore** dependency caches, build artifacts, and other paths that waste space or do not need point-in-time recovery.
- **Prune** old backups by **maximum age** and/or **maximum total disk usage**.
- Ship as a **Homebrew-installable CLI** with clear documentation of limitations.

## Non-goals (v1)

- **Linux** or **Windows** first-class support.
- **Synchronous “before write” interception** via kernel or Endpoint Security (not available to a normal Homebrew binary).
- **Continuous replication** of entire trees on a fixed interval (only event-driven + initial snapshot).
- **GUI** or **IDE plugins**.

## Platform guarantees (macOS)

- The tool uses **FSEvents** (via the `notify` crate) to observe changes. Notifications are **best-effort** and may arrive **after** a write has started or completed.
- On **delete**, the file may **no longer be readable** when the tool handles the event. Recovery relies on **the last successful backup** from earlier **modify** events, plus **initial snapshot** at startup for files not yet modified.

### Acceptance criteria

- The README and this spec state honestly that **pre-delete byte capture** is not guaranteed.
- A file that was **modified** at least once after watch start (and not ignored) has a **recoverable copy** in a backup run directory unless pruned by retention.
- A file **never modified** after watch start but present at startup is captured by **initial snapshot** (if not ignored).

## Personas

- **Developer** — Runs the CLI against a project root while coding or using AI tools; wants low friction and predictable disk use.
- **Maintainer** — Ships releases via Homebrew; documents behavior and tests against specs.

## Glossary

| Term | Meaning |
|------|---------|
| **Watch root** | Absolute path of the directory tree to protect. |
| **Backup root** | Directory containing one subdirectory per **backup run**. |
| **Backup run** | One batch of copies sharing a single **backup run ID** (e.g. one event or one initial snapshot pass). |
| **Backup run ID** | Sortable identifier used as the name of a subdirectory under the backup root. |

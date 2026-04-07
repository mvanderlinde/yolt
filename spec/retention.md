# Retention specification

## Policies

Two independent policies; **both** apply when configured:

1. **Time:** Delete entire **backup run** directories whose **mtime** (or parsed ID time) is older than **now - retention**.
2. **Disk cap:** If total size of `backup_root` exceeds `max_disk`, delete **oldest** backup runs (by run id sort / directory mtime) until total size ≤ `max_disk` or no runs remain.

## Ordering

1. Run sweep on a timer (`prune_interval_secs`).
2. First, remove runs older than retention (time policy).
3. Then, if `max_disk > 0` and usage still exceeds cap, delete oldest runs until under cap.

## Measurement

- **Total usage:** Sum of file sizes under `backup_root` (implementation may cache per run).
- **Oldest:** Lexicographically smallest `backup_run_id` that parses as valid, or oldest mtime.

## Edge cases

- Do not delete the **currently open** backup run if mid-write — implementation uses “runs fully completed” only (each run is closed before starting next; sweeper only deletes **complete** past runs). For simplicity, sweeper deletes any run directory not equal to **current run id** being written — **race:** if writing to run A, don’t delete A. Track `current_run_id` and skip.

## Acceptance criteria

- With `max_disk` set small, oldest runs disappear first.
- With short retention, no run older than retention remains (modulo sweep interval).

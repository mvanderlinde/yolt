//! Prune old backup runs by age and/or total disk usage. See spec/retention.md.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Total size in bytes of all regular files under `root`.
pub fn dir_tree_size(root: &Path) -> io::Result<u64> {
    let mut total = 0u64;
    if !root.exists() {
        return Ok(0);
    }
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            total += entry.metadata()?.len();
        }
    }
    Ok(total)
}

/// Apply time-based and disk-cap pruning. Skips `current_run_id` if set.
pub fn prune(
    backup_root: &Path,
    retention: Duration,
    max_disk: u64,
    current_run_id: Option<&str>,
) -> io::Result<()> {
    if !backup_root.exists() {
        return Ok(());
    }

    let cutoff = SystemTime::now() - retention;

    // Time policy
    for e in fs::read_dir(backup_root)? {
        let e = e?;
        if !e.file_type()?.is_dir() {
            continue;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        if current_run_id.map(|c| c == name.as_str()).unwrap_or(false) {
            continue;
        }
        let mtime = e.metadata()?.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        if mtime < cutoff {
            fs::remove_dir_all(e.path())?;
        }
    }

    if max_disk == 0 {
        return Ok(());
    }

    let mut total = dir_tree_size(backup_root)?;
    if total <= max_disk {
        return Ok(());
    }

    let mut runs: Vec<(String, PathBuf)> = Vec::new();
    for e in fs::read_dir(backup_root)? {
        let e = e?;
        if !e.file_type()?.is_dir() {
            continue;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        if current_run_id.map(|c| c == name.as_str()).unwrap_or(false) {
            continue;
        }
        runs.push((name, e.path()));
    }

    // Oldest first: lexicographic run id (sortable format)
    runs.sort_by(|a, b| a.0.cmp(&b.0));

    for (_, path) in runs {
        if total <= max_disk {
            break;
        }
        let freed = dir_tree_size(&path)?;
        if freed > 0 {
            fs::remove_dir_all(&path)?;
            total = total.saturating_sub(freed);
        }
    }

    Ok(())
}

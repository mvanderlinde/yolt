//! Prune old backup runs by age and/or total disk usage. See spec/retention.md.
//!
//! The **initial snapshot** run (see `.yolt_base_snapshot`) is never removed by pruning.
//! Before any other run is deleted, its files are merged into that base run so the oldest
//! retained bytes for each path stay available for restore and for `copy_file_into_run`
//! deduplication.

use std::collections::HashSet;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Written at `{backup_root}/.yolt_base_snapshot` — contains the run id of the initial snapshot.
pub const MARKER_BASE_SNAPSHOT: &str = ".yolt_base_snapshot";

/// Tracks relative paths whose content in the base run was updated from a **deleted** run
/// (so newer deleted runs must not overwrite those copies).
const MARKER_MERGED_PATHS: &str = ".yolt_merged_paths";

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

/// Record the base snapshot run id and reset merge tracking (call when a new initial snapshot is taken).
pub fn set_base_snapshot_run(backup_root: &Path, run_id: &str) -> io::Result<()> {
    fs::create_dir_all(backup_root)?;
    fs::write(backup_root.join(MARKER_BASE_SNAPSHOT), run_id.as_bytes())?;
    clear_merged_paths_file(backup_root)
}

/// Remove base/merge marker files (e.g. when `snapshot_initial` is disabled).
pub fn remove_base_snapshot_markers(backup_root: &Path) -> io::Result<()> {
    let _ = fs::remove_file(backup_root.join(MARKER_BASE_SNAPSHOT));
    let _ = fs::remove_file(backup_root.join(MARKER_MERGED_PATHS));
    Ok(())
}

fn read_base_run_id(backup_root: &Path) -> io::Result<Option<String>> {
    let p = backup_root.join(MARKER_BASE_SNAPSHOT);
    if !p.is_file() {
        return Ok(None);
    }
    let s = fs::read_to_string(p)?;
    let t = s.trim();
    if t.is_empty() {
        return Ok(None);
    }
    Ok(Some(t.to_owned()))
}

fn merged_paths_path(backup_root: &Path) -> PathBuf {
    backup_root.join(MARKER_MERGED_PATHS)
}

fn clear_merged_paths_file(backup_root: &Path) -> io::Result<()> {
    let p = merged_paths_path(backup_root);
    if p.exists() {
        fs::remove_file(p)?;
    }
    Ok(())
}

fn read_merged_paths(backup_root: &Path) -> io::Result<HashSet<PathBuf>> {
    let p = merged_paths_path(backup_root);
    if !p.is_file() {
        return Ok(HashSet::new());
    }
    let f = fs::File::open(p)?;
    let mut out = HashSet::new();
    for line in BufReader::new(f).lines() {
        let line = line?;
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        out.insert(PathBuf::from(t));
    }
    Ok(out)
}

fn write_merged_paths(backup_root: &Path, merged: &HashSet<PathBuf>) -> io::Result<()> {
    let p = merged_paths_path(backup_root);
    if merged.is_empty() {
        if p.exists() {
            fs::remove_file(p)?;
        }
        return Ok(());
    }
    let mut tmp = p.clone();
    tmp.set_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        for rel in merged {
            writeln!(f, "{}", rel.display())?;
        }
    }
    fs::rename(tmp, p)?;
    Ok(())
}

fn is_backup_run_dir(name: &str) -> bool {
    !name.starts_with('.')
}

/// List `(run_id, path, mtime)` for backup run directories under `backup_root`.
fn list_run_dirs(backup_root: &Path) -> io::Result<Vec<(String, PathBuf, SystemTime)>> {
    let mut out = Vec::new();
    if !backup_root.exists() {
        return Ok(out);
    }
    for e in fs::read_dir(backup_root)? {
        let e = e?;
        if !e.file_type()?.is_dir() {
            continue;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        if !is_backup_run_dir(&name) {
            continue;
        }
        let mtime = e.metadata()?.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        out.push((name, e.path(), mtime));
    }
    Ok(out)
}

/// Copy files from `run_path` into `backup_root/base_run_id/`, skipping paths that already
/// received an older merged copy (see `merged`).
fn merge_run_into_base(
    backup_root: &Path,
    base_run_id: &str,
    run_path: &Path,
    merged: &mut HashSet<PathBuf>,
) -> io::Result<()> {
    let base_root = backup_root.join(base_run_id);
    for entry in walkdir::WalkDir::new(run_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let rel = path.strip_prefix(run_path).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "run file not under run root",
            )
        })?;
        if merged.contains(rel) {
            continue;
        }
        let meta = fs::metadata(path)?;
        if meta.len() == 0 {
            continue;
        }
        let dest = base_root.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(path, &dest)?;
        merged.insert(rel.to_path_buf());
    }
    Ok(())
}

/// Apply time-based and disk-cap pruning. Skips `current_run_id` if set.
/// Never deletes the base snapshot run (see [`MARKER_BASE_SNAPSHOT`]); merges losing runs into it first.
pub fn prune(
    backup_root: &Path,
    retention: Duration,
    max_disk: u64,
    current_run_id: Option<&str>,
) -> io::Result<()> {
    if !backup_root.exists() {
        return Ok(());
    }

    let base_run_id = read_base_run_id(backup_root)?;
    let mut merged = read_merged_paths(backup_root)?;

    let cutoff = SystemTime::now() - retention;

    // Time policy: delete all expired runs (oldest first), merging each into base first.
    let mut expired: Vec<(String, PathBuf)> = Vec::new();
    for (name, path, mtime) in list_run_dirs(backup_root)? {
        if current_run_id.map(|c| c == name.as_str()).unwrap_or(false) {
            continue;
        }
        if base_run_id.as_deref() == Some(name.as_str()) {
            continue;
        }
        if mtime < cutoff {
            expired.push((name, path));
        }
    }
    expired.sort_by(|a, b| a.0.cmp(&b.0));

    for (_name, path) in expired {
        if let Some(ref base) = base_run_id {
            merge_run_into_base(backup_root, base, &path, &mut merged)?;
            write_merged_paths(backup_root, &merged)?;
        }
        fs::remove_dir_all(path)?;
    }

    if max_disk == 0 {
        return Ok(());
    }

    let mut total = dir_tree_size(backup_root)?;
    if total <= max_disk {
        return Ok(());
    }

    // Disk cap: repeatedly remove the oldest non-base run, merging into base first.
    loop {
        if total <= max_disk {
            break;
        }

        let mut runs: Vec<(String, PathBuf)> = Vec::new();
        for (name, path, _) in list_run_dirs(backup_root)? {
            if current_run_id.map(|c| c == name.as_str()).unwrap_or(false) {
                continue;
            }
            if base_run_id.as_deref() == Some(name.as_str()) {
                continue;
            }
            runs.push((name, path));
        }

        if runs.is_empty() {
            break;
        }

        runs.sort_by(|a, b| a.0.cmp(&b.0));
        let Some((_, path)) = runs.into_iter().next() else {
            break;
        };

        if let Some(ref base) = base_run_id {
            merge_run_into_base(backup_root, base, &path, &mut merged)?;
            write_merged_paths(backup_root, &merged)?;
        }
        fs::remove_dir_all(&path)?;
        total = dir_tree_size(backup_root)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use filetime::{set_file_mtime, FileTime};
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn prune_time_removes_expired_run_merged_into_base() {
        let root = tempdir().unwrap();
        let b = root.path().join("backups");
        fs::create_dir_all(&b).unwrap();

        let base = b.join("20260101000000000_0000");
        let old = b.join("20200101000000000_0000");
        let new = b.join("20260101000000001_0000");
        fs::create_dir_all(&base).unwrap();
        fs::create_dir_all(&old).unwrap();
        fs::create_dir_all(&new).unwrap();
        fs::write(base.join("f.txt"), b"base").unwrap();
        fs::write(old.join("f.txt"), b"x").unwrap();
        fs::write(new.join("g.txt"), b"yy").unwrap();

        set_base_snapshot_run(&b, "20260101000000000_0000").unwrap();

        set_file_mtime(&old, FileTime::from_unix_time(0, 0)).unwrap();

        prune(&b, Duration::from_secs(3600), 0, None).unwrap();

        assert!(!old.exists());
        assert!(new.exists());
        assert!(base.exists());
        assert_eq!(fs::read_to_string(base.join("f.txt")).unwrap(), "x");
    }

    #[test]
    fn only_base_run_kept_when_under_disk_cap() {
        let root = tempdir().unwrap();
        let b = root.path().join("backups");
        fs::create_dir_all(&b).unwrap();

        let base = b.join("20260101000000000_0000");
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("f.txt"), b"base").unwrap();

        set_base_snapshot_run(&b, "20260101000000000_0000").unwrap();

        let other = b.join("20260101000000001_0000");
        fs::create_dir_all(&other).unwrap();
        fs::write(other.join("g.txt"), b"yy").unwrap();

        // Force-delete until tiny cap; base must survive and absorb `other` before removal.
        prune(&b, Duration::from_secs(365 * 24 * 3600), 1, None).unwrap();

        assert!(base.exists());
        assert!(!other.exists());
        assert_eq!(fs::read_to_string(base.join("g.txt")).unwrap(), "yy");
    }
}

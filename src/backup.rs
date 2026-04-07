//! Backup run IDs and file copies. See spec/backup-store.md.

use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use chrono::{Datelike, Timelike, Utc};
use sha2::{Digest, Sha256};

static RUN_COUNTER: AtomicU64 = AtomicU64::new(0);

/// UTC sortable id: `YYYYMMDDhhmmssmmm` plus disambiguator if needed.
pub fn new_backup_run_id() -> String {
    let t = Utc::now();
    let n = RUN_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        "{:04}{:02}{:02}{:02}{:02}{:02}{:03}_{:04}",
        t.year(),
        t.month(),
        t.day(),
        t.hour(),
        t.minute(),
        t.second(),
        t.nanosecond() / 1_000_000,
        n
    )
}

/// SHA-256 hash of file contents.
pub fn hash_file(path: &Path) -> io::Result<[u8; 32]> {
    let mut f = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().into())
}

/// Hash of the newest existing backup file at `rel` (runs scanned newest-first).
pub fn hash_of_newest_backup(backup_root: &Path, rel: &Path) -> io::Result<Option<[u8; 32]>> {
    for run_id in crate::restore::list_run_ids_newest_first(backup_root)? {
        let p = backup_root.join(&run_id).join(rel);
        if p.is_file() {
            if p.metadata()?.len() == 0 {
                continue;
            }
            return Ok(Some(hash_file(&p)?));
        }
    }
    Ok(None)
}

/// Copy `source` into `backup_root/run_id/…` unless the file is empty, or its hash matches the
/// latest backed-up version (in-memory cache or on-disk newest backup).
pub fn copy_file_into_run(
    watch_root: &Path,
    backup_root: &Path,
    run_id: &str,
    source: &Path,
    hash_cache: &Mutex<HashMap<PathBuf, [u8; 32]>>,
) -> io::Result<u64> {
    let rel = source
        .strip_prefix(watch_root)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "source not under watch root"))?
        .to_path_buf();

    let len = fs::metadata(source)?.len();
    if len == 0 {
        return Ok(0);
    }

    let h = hash_file(source)?;

    let last_hash = {
        let g = hash_cache.lock().unwrap();
        if let Some(&x) = g.get(&rel) {
            Some(x)
        } else {
            drop(g);
            let from_disk = hash_of_newest_backup(backup_root, &rel)?;
            let mut g = hash_cache.lock().unwrap();
            if let Some(hd) = from_disk {
                g.insert(rel.clone(), hd);
            }
            from_disk
        }
    };

    if last_hash == Some(h) {
        return Ok(0);
    }

    let dest = backup_root.join(run_id).join(&rel);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let n = fs::copy(source, &dest)?;
    hash_cache.lock().unwrap().insert(rel, h);
    Ok(n)
}

/// Walk with directory pruning for ignored dirs.
pub fn initial_snapshot_pruned(
    watch_root: &Path,
    backup_root: &Path,
    run_id: &str,
    ignore: &crate::ignore_rules::IgnoreRules,
    hash_cache: &Mutex<HashMap<PathBuf, [u8; 32]>>,
) -> io::Result<u64> {
    let mut total: u64 = 0;
    let walker = walkdir::WalkDir::new(watch_root).into_iter();
    for entry in walker.filter_entry(|e| {
        let p = e.path();
        if p == watch_root {
            return true;
        }
        let is_dir = e.file_type().is_dir();
        !ignore.is_ignored(p, is_dir)
    }) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if ignore.is_ignored(path, false) {
            continue;
        }
        total += copy_file_into_run(watch_root, backup_root, run_id, path, hash_cache)?;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::Mutex;
    use tempfile::tempdir;

    #[test]
    fn skips_zero_byte_and_duplicate_hash() {
        let watch = tempdir().unwrap();
        let backup = tempdir().unwrap();
        let run = "testrun1";
        let cache = Mutex::new(HashMap::new());

        let f = watch.path().join("a.txt");
        let mut file = fs::File::create(&f).unwrap();
        file.write_all(b"hello").unwrap();
        drop(file);

        let n1 = copy_file_into_run(watch.path(), backup.path(), run, &f, &cache).unwrap();
        assert!(n1 > 0);

        let n2 = copy_file_into_run(watch.path(), backup.path(), run, &f, &cache).unwrap();
        assert_eq!(n2, 0, "same content should not copy again");

        fs::write(&f, b"").unwrap();
        let n3 = copy_file_into_run(watch.path(), backup.path(), run, &f, &cache).unwrap();
        assert_eq!(n3, 0, "empty file should not be backed up");
    }
}

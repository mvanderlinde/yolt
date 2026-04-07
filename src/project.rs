//! Project namespace and single-watcher lock helpers.

use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use fs2::FileExt;
use sha2::{Digest, Sha256};

/// Derive a deterministic project id from the canonical watch root path.
pub fn project_id_for_watch(watch_root: &Path) -> io::Result<String> {
    let canonical = fs::canonicalize(watch_root)?;
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_os_str().as_bytes());
    let digest = hasher.finalize();

    // 24 hex chars (96 bits) is short and collision-resistant enough for namespace keys.
    let mut id = String::with_capacity(24);
    for b in digest.iter().take(12) {
        let _ = write!(&mut id, "{b:02x}");
    }
    Ok(id)
}

/// Return `{backup_root}/{project_id}` for this watch root.
pub fn namespaced_backup_root(backup_root: &Path, watch_root: &Path) -> io::Result<(String, PathBuf)> {
    let project_id = project_id_for_watch(watch_root)?;
    let project_backup_root = backup_root.join(&project_id);
    Ok((project_id, project_backup_root))
}

/// Held lock file guard for a watched project.
pub struct WatchLock {
    _file: File,
    path: PathBuf,
}

impl WatchLock {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Acquire a non-blocking exclusive project lock. Fails if another watcher already holds it.
pub fn acquire_watch_lock(project_backup_root: &Path) -> io::Result<WatchLock> {
    fs::create_dir_all(project_backup_root)?;
    let lock_path = project_backup_root.join(".watch.lock");
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&lock_path)?;

    file.try_lock_exclusive().map_err(|e| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("lock busy at {}: {e}", lock_path.display()),
        )
    })?;

    Ok(WatchLock {
        _file: file,
        path: lock_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn project_id_is_stable_for_same_path() {
        let d = tempdir().unwrap();
        let a = project_id_for_watch(d.path()).unwrap();
        let b = project_id_for_watch(d.path()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn lock_is_exclusive() {
        let d = tempdir().unwrap();
        let (_id, root) = namespaced_backup_root(d.path(), d.path()).unwrap();
        let _first = acquire_watch_lock(&root).unwrap();
        let second = acquire_watch_lock(&root);
        assert!(second.is_err(), "second lock should fail while first is held");
    }
}

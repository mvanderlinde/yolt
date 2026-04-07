//! List backup runs and restore paths from a chosen run. See spec/cli.md.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RestoreError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("path is not under watch root: {0}")]
    NotUnderWatch(PathBuf),
    #[error("no backup runs contain {0}")]
    NoBackups(PathBuf),
    #[error("only {found} backup version(s) contain this path; --back {back} is out of range")]
    NotEnoughVersions { found: usize, back: usize },
    #[error("source missing in chosen run: {0}")]
    SourceMissing(PathBuf),
}

/// Run directory names under `backup_root`, sorted **newest first** (lexicographic desc works for our ids).
pub fn list_run_ids_newest_first(backup_root: &Path) -> io::Result<Vec<String>> {
    let mut names = Vec::new();
    if !backup_root.exists() {
        return Ok(names);
    }
    for e in fs::read_dir(backup_root)? {
        let e = e?;
        if !e.file_type()?.is_dir() {
            continue;
        }
        names.push(e.file_name().to_string_lossy().into_owned());
    }
    names.sort_by(|a, b| b.cmp(a));
    Ok(names)
}

/// Runs (newest first) where `backup_root/{run_id}/rel` exists (file or directory).
pub fn runs_containing_rel_path(backup_root: &Path, rel: &Path) -> io::Result<Vec<String>> {
    let mut out = Vec::new();
    for run_id in list_run_ids_newest_first(backup_root)? {
        let p = backup_root.join(&run_id).join(rel);
        if p.exists() {
            out.push(run_id);
        }
    }
    Ok(out)
}

/// Resolve `user_path` to a path relative to `watch_root`.
/// Relative paths are resolved lexically under the canonical watch root (blocks `..` escape).
/// Absolute paths must lie under the watch root after canonicalization.
pub fn normalize_relative_to_watch(
    watch_root: &Path,
    user_path: &Path,
) -> Result<PathBuf, RestoreError> {
    let watch = fs::canonicalize(watch_root).map_err(RestoreError::Io)?;
    if user_path.is_absolute() {
        let abs = fs::canonicalize(user_path).map_err(RestoreError::Io)?;
        return abs
            .strip_prefix(&watch)
            .map(|p| p.to_path_buf())
            .map_err(|_| RestoreError::NotUnderWatch(abs));
    }
    let mut resolved = watch.clone();
    for component in user_path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {
                return Err(RestoreError::NotUnderWatch(user_path.to_path_buf()));
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if !resolved.pop() {
                    return Err(RestoreError::NotUnderWatch(user_path.to_path_buf()));
                }
            }
            Component::Normal(c) => resolved.push(c),
        }
        if !resolved.starts_with(&watch) {
            return Err(RestoreError::NotUnderWatch(user_path.to_path_buf()));
        }
    }
    resolved
        .strip_prefix(&watch)
        .map(|p| p.to_path_buf())
        .map_err(|_| RestoreError::NotUnderWatch(user_path.to_path_buf()))
}

/// Copy file or directory tree from `backup_root/{run_id}/rel` into `watch_root/rel`.
pub fn copy_tree_into_watch(
    watch_root: &Path,
    backup_root: &Path,
    run_id: &str,
    rel: &Path,
) -> io::Result<()> {
    let src = backup_root.join(run_id).join(rel);
    if !src.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{}", src.display()),
        ));
    }
    let dst = watch_root.join(rel);
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    let meta = fs::metadata(&src)?;
    if meta.is_file() {
        fs::copy(&src, &dst)?;
        return Ok(());
    }
    if meta.is_dir() {
        copy_dir_recursive(&src, &dst)?;
        return Ok(());
    }
    Err(io::Error::other(
        "unsupported file type (expected file or directory)",
    ))
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)? {
        let e = e?;
        let p = e.path();
        let name = e.file_name();
        let out = dst.join(&name);
        let t = e.file_type()?;
        if t.is_dir() {
            copy_dir_recursive(&p, &out)?;
        } else if t.is_file() {
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&p, &out)?;
        }
    }
    Ok(())
}

/// Restore `rel` from the `versions_back`-th newest run that contains it (`0` = newest).
pub fn restore(
    watch_root: &Path,
    backup_root: &Path,
    rel: &Path,
    versions_back: usize,
    dry_run: bool,
) -> Result<(String, PathBuf), RestoreError> {
    let runs = runs_containing_rel_path(backup_root, rel)?;
    if runs.is_empty() {
        return Err(RestoreError::NoBackups(rel.to_path_buf()));
    }
    if versions_back >= runs.len() {
        return Err(RestoreError::NotEnoughVersions {
            found: runs.len(),
            back: versions_back,
        });
    }
    let run_id = runs[versions_back].clone();
    let src = backup_root.join(&run_id).join(rel);
    if !src.exists() {
        return Err(RestoreError::SourceMissing(src));
    }
    let dst = watch_root.join(rel);
    if dry_run {
        eprintln!(
            "dry-run: would restore from run {} -> {}",
            run_id,
            dst.display()
        );
        return Ok((run_id, dst));
    }
    copy_tree_into_watch(watch_root, backup_root, &run_id, rel)?;
    Ok((run_id, dst))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn normalize_rejects_parent_dir_escape() {
        let watch = tempdir().unwrap();
        fs::create_dir_all(watch.path().join("sub")).unwrap();
        let err = normalize_relative_to_watch(watch.path(), Path::new("../outside")).unwrap_err();
        assert!(matches!(err, RestoreError::NotUnderWatch(_)));
    }

    #[test]
    fn normalize_accepts_safe_relative() {
        let watch = tempdir().unwrap();
        fs::create_dir_all(watch.path().join("a")).unwrap();
        let rel = normalize_relative_to_watch(watch.path(), Path::new("a/b.txt")).unwrap();
        assert_eq!(rel, PathBuf::from("a/b.txt"));
    }
}

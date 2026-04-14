//! spec/retention.md — time-based pruning (no base snapshot marker).

use std::fs;
use std::time::Duration;

use filetime::{set_file_mtime, FileTime};
use tempfile::tempdir;

#[test]
fn prune_time_deletes_expired_without_base_marker() {
    let root = tempdir().unwrap();
    let b = root.path().join("backups");
    fs::create_dir_all(&b).unwrap();

    let old = b.join("20200101000000000_0000");
    let new = b.join("20260101000000000_0000");
    fs::create_dir_all(&old).unwrap();
    fs::create_dir_all(&new).unwrap();
    fs::write(old.join("f.txt"), b"x").unwrap();
    fs::write(new.join("g.txt"), b"yy").unwrap();

    set_file_mtime(&old, FileTime::from_unix_time(0, 0)).unwrap();

    yolt::retention::prune(&b, Duration::from_secs(3600), 0, None).unwrap();

    assert!(!old.exists());
    assert!(new.exists());
}

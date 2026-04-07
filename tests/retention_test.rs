//! spec/retention.md — disk cap deletes oldest runs first.

use std::fs;
use std::time::Duration;

use tempfile::tempdir;

#[test]
fn prune_disk_cap_removes_oldest_first() {
    let root = tempdir().unwrap();
    let b = root.path().join("backups");
    fs::create_dir_all(&b).unwrap();

    let old = b.join("20200101000000000_0000");
    let new = b.join("20260101000000000_0000");
    fs::create_dir_all(&old).unwrap();
    fs::create_dir_all(&new).unwrap();
    fs::write(old.join("f.txt"), b"x").unwrap();
    fs::write(new.join("g.txt"), b"yy").unwrap();

    yolt::retention::prune(&b, Duration::from_secs(365 * 24 * 3600), 2, None).unwrap();

    assert!(!old.exists(), "oldest run should be removed first");
    assert!(new.exists());
}

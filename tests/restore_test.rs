//! spec/cli.md — restore picks Nth newest run that contains the path.

use std::fs;
use std::path::Path;

use tempfile::tempdir;

#[test]
fn restore_picks_nth_newest_run() {
    let backup = tempdir().unwrap();
    let watch = tempdir().unwrap();

    let old = "20200101000000000_0001";
    let mid = "20200102000000000_0002";
    let new = "20200103000000000_0003";

    fs::create_dir_all(backup.path().join(old).join("src")).unwrap();
    fs::write(backup.path().join(old).join("src/a.txt"), b"v1").unwrap();
    fs::create_dir_all(backup.path().join(mid).join("src")).unwrap();
    fs::write(backup.path().join(mid).join("src/a.txt"), b"v2").unwrap();
    fs::create_dir_all(backup.path().join(new).join("src")).unwrap();
    fs::write(backup.path().join(new).join("src/a.txt"), b"v3").unwrap();

    let rel = Path::new("src/a.txt");
    let runs = yolt::restore::runs_containing_rel_path(backup.path(), rel).unwrap();
    assert_eq!(runs.len(), 3);
    assert_eq!(runs[0], new);
    assert_eq!(runs[1], mid);
    assert_eq!(runs[2], old);

    let (run0, _) = yolt::restore::restore(watch.path(), backup.path(), rel, 0, true).unwrap();
    assert_eq!(run0, new);

    let (run1, _) = yolt::restore::restore(watch.path(), backup.path(), rel, 1, true).unwrap();
    assert_eq!(run1, mid);
}

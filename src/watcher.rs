//! FSEvents watcher with debouncing. See spec/watcher.md.

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::event::EventKind;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use thiserror::Error;

use crate::ignore_rules::IgnoreRules;

#[derive(Debug, Error)]
pub enum WatcherError {
    #[error("notify error: {0}")]
    Notify(#[from] notify::Error),
    #[error("batch callback: {0}")]
    Io(#[from] io::Error),
    #[error("watcher channel closed")]
    Disconnected,
}

/// Debounced filesystem watch. On each flush, invokes `on_batch` with file paths under `watch_root`
/// that are not ignored.
pub fn run<F>(
    watch_root: PathBuf,
    debounce: Duration,
    ignore: Arc<IgnoreRules>,
    mut on_batch: F,
) -> Result<(), WatcherError>
where
    F: FnMut(Vec<PathBuf>) -> io::Result<()>,
{
    let (tx, rx) = mpsc::channel();
    let root = watch_root.clone();

    let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| {
        if let Ok(ev) = res {
            let _ = tx.send(ev);
        }
    })?;

    watcher.watch(&watch_root, RecursiveMode::Recursive)?;

    let tick = Duration::from_millis(50);
    let mut pending: HashMap<PathBuf, Instant> = HashMap::new();

    loop {
        match rx.recv_timeout(tick) {
            Ok(event) => {
                if !event.paths.iter().any(|p| p.starts_with(&root)) {
                    // fall through to flush
                } else {
                    let interesting = matches!(
                        event.kind,
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Other
                    );
                    if interesting {
                        for path in event.paths {
                            if !path.starts_with(&root) {
                                continue;
                            }
                            pending.insert(path, Instant::now());
                        }
                    }
                }
            }
            Err(RecvTimeoutError::Disconnected) => return Err(WatcherError::Disconnected),
            Err(RecvTimeoutError::Timeout) => {}
        }

        let now = Instant::now();
        let mut ready: Vec<PathBuf> = Vec::new();
        pending.retain(|path, t| {
            if now.duration_since(*t) >= debounce {
                ready.push(path.clone());
                false
            } else {
                true
            }
        });

        if ready.is_empty() {
            continue;
        }

        let mut files: Vec<PathBuf> = Vec::new();
        for path in ready {
            if !path.exists() {
                continue;
            }
            if !path.is_file() {
                continue;
            }
            if ignore.is_ignored(&path, false) {
                continue;
            }
            files.push(path);
        }

        if !files.is_empty() {
            on_batch(files)?;
        }
    }
}

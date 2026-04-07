//! Ignore rules: built-in defaults + `.yoltignore` + extra patterns.
//! See spec/ignore-rules.md.

use std::path::{Path, PathBuf};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use thiserror::Error;

/// Built-in default patterns (gitignore syntax).
pub const DEFAULT_IGNORES: &str = r#"
node_modules/
.next/
.nuxt/
dist/
build/
.turbo/
.parcel-cache/
__pycache__/
*.pyc
*.pyo
.venv/
venv/
.tox/
.mypy_cache/
target/
vendor/bundle/
vendor/
.DS_Store
Thumbs.db
.git/
"#;

#[derive(Debug, Error)]
pub enum IgnoreError {
    #[error("gitignore: {0}")]
    Gitignore(#[from] ignore::Error),
}

pub struct IgnoreRules {
    watch_root: PathBuf,
    gitignore: Gitignore,
}

impl IgnoreRules {
    pub fn new(
        watch_root: &Path,
        use_defaults: bool,
        project_file: Option<&Path>,
        extra: &[String],
    ) -> Result<Self, IgnoreError> {
        let mut builder = GitignoreBuilder::new(watch_root);
        if use_defaults {
            for line in DEFAULT_IGNORES.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                builder.add_line(None, line)?;
            }
        }
        if let Some(p) = project_file {
            if p.exists() {
                if let Some(e) = builder.add(p) {
                    return Err(IgnoreError::Gitignore(e));
                }
            }
        }
        for pat in extra {
            let line = pat.trim();
            if !line.is_empty() {
                builder.add_line(None, line)?;
            }
        }
        let gitignore = builder.build()?;
        Ok(Self {
            watch_root: watch_root.to_path_buf(),
            gitignore,
        })
    }

    /// `path` must be under `watch_root` (or absolute path under watch).
    pub fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        let rel = match path.strip_prefix(&self.watch_root) {
            Ok(r) => r,
            Err(_) => return false,
        };
        self.gitignore
            .matched_path_or_any_parents(rel, is_dir)
            .is_ignore()
    }
}

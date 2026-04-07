//! Configuration merge: defaults < TOML < env < CLI.
//! See spec/config.md.

use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Config {
    pub watch: PathBuf,
    pub backup_root: PathBuf,
    pub retention: Duration,
    /// 0 = disabled
    pub max_disk: u64,
    pub debounce_ms: u64,
    pub prune_interval_secs: u64,
    pub snapshot_initial: bool,
    pub no_default_ignores: bool,
    pub extra_ignores: Vec<String>,
    /// After this much idle time since the last backup batch, the next batch starts a new run folder.
    pub session_idle_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            watch: PathBuf::new(),
            // Per-user temp (not world-writable `/tmp/...` shared across users).
            backup_root: std::env::temp_dir().join("yolt"),
            retention: Duration::from_secs(30 * 60),
            max_disk: 0,
            debounce_ms: 300,
            prune_interval_secs: 60,
            snapshot_initial: true,
            no_default_ignores: false,
            extra_ignores: Vec::new(),
            session_idle_ms: 10_000,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TomlConfig {
    pub watch: Option<PathBuf>,
    pub backup_root: Option<PathBuf>,
    pub retention: Option<String>,
    pub max_disk: Option<toml::Value>,
    pub debounce_ms: Option<u64>,
    pub prune_interval_secs: Option<u64>,
    pub snapshot_initial: Option<bool>,
    pub no_default_ignores: Option<bool>,
    pub ignore: Option<Vec<String>>,
    pub session_idle_ms: Option<u64>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("invalid retention duration: {0}")]
    Retention(#[from] humantime::DurationError),
    #[error("invalid max_disk: {0}")]
    MaxDisk(String),
    #[error("retention must be greater than zero")]
    ZeroRetention,
    #[error("watch path is required (set watch in config, YOLT_WATCH, pass DIR to `watch`, or use restore --dir)")]
    MissingWatch,
    #[error("watch path is not a directory: {0}")]
    NotADirectory(PathBuf),
}

pub fn parse_byte_size(s: &str) -> Result<u64, String> {
    let input = s.trim();
    if input.is_empty() {
        return Err("empty string".into());
    }
    let upper = input.to_uppercase();
    let last = upper.chars().last().unwrap();
    let mult = match last {
        'K' => 1024u64,
        'M' => 1024u64.pow(2),
        'G' => 1024u64.pow(3),
        'T' => 1024u64.pow(4),
        _ => {
            return input
                .parse::<u64>()
                .map_err(|e| format!("parse integer: {e}"));
        }
    };
    let num_part = &input[..input.len() - 1];
    let n: f64 = num_part
        .trim()
        .parse()
        .map_err(|e: std::num::ParseFloatError| e.to_string())?;
    let bytes = (n * mult as f64) as u64;
    Ok(bytes)
}

fn parse_max_disk_value(v: &toml::Value) -> Result<u64, String> {
    match v {
        toml::Value::Integer(i) => {
            if *i < 0 {
                return Err("max_disk must be non-negative".into());
            }
            Ok(*i as u64)
        }
        toml::Value::String(s) => parse_byte_size(s),
        _ => Err("max_disk must be integer or string".into()),
    }
}

impl TomlConfig {
    pub fn apply_to(&self, cfg: &mut Config) -> Result<(), ConfigError> {
        if let Some(ref p) = self.watch {
            cfg.watch = p.clone();
        }
        if let Some(ref p) = self.backup_root {
            cfg.backup_root = p.clone();
        }
        if let Some(ref s) = self.retention {
            let d = humantime::parse_duration(s).map_err(ConfigError::Retention)?;
            cfg.retention = d;
        }
        if let Some(ref v) = self.max_disk {
            cfg.max_disk = parse_max_disk_value(v).map_err(ConfigError::MaxDisk)?;
        }
        if let Some(v) = self.debounce_ms {
            cfg.debounce_ms = v;
        }
        if let Some(v) = self.prune_interval_secs {
            cfg.prune_interval_secs = v;
        }
        if let Some(v) = self.snapshot_initial {
            cfg.snapshot_initial = v;
        }
        if let Some(v) = self.no_default_ignores {
            cfg.no_default_ignores = v;
        }
        if let Some(ref list) = self.ignore {
            cfg.extra_ignores.extend(list.iter().cloned());
        }
        if let Some(v) = self.session_idle_ms {
            cfg.session_idle_ms = v;
        }
        Ok(())
    }
}

pub fn apply_env(cfg: &mut Config) {
    if let Ok(v) = std::env::var("YOLT_WATCH") {
        if !v.is_empty() {
            cfg.watch = PathBuf::from(v);
        }
    }
    if let Ok(v) = std::env::var("YOLT_BACKUP_ROOT") {
        if !v.is_empty() {
            cfg.backup_root = PathBuf::from(v);
        }
    }
    if let Ok(v) = std::env::var("YOLT_RETENTION") {
        if let Ok(d) = humantime::parse_duration(&v) {
            cfg.retention = d;
        }
    }
    if let Ok(v) = std::env::var("YOLT_MAX_DISK") {
        if let Ok(n) = parse_byte_size(&v) {
            cfg.max_disk = n;
        }
    }
    if let Ok(v) = std::env::var("YOLT_DEBOUNCE") {
        if let Ok(ms) = v.parse::<u64>() {
            cfg.debounce_ms = ms;
        }
    }
    if let Ok(v) = std::env::var("YOLT_PRUNE_INTERVAL") {
        if let Ok(s) = v.parse::<u64>() {
            cfg.prune_interval_secs = s;
        }
    }
    if let Ok(v) = std::env::var("YOLT_SNAPSHOT_INITIAL") {
        if let Some(b) = parse_bool_env(&v) {
            cfg.snapshot_initial = b;
        }
    }
    if let Ok(v) = std::env::var("YOLT_SESSION_IDLE_MS") {
        if let Ok(ms) = v.parse::<u64>() {
            cfg.session_idle_ms = ms;
        }
    }
}

fn parse_bool_env(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" => Some(true),
        "0" | "false" | "no" => Some(false),
        _ => None,
    }
}

pub fn load_toml_file(path: &std::path::Path) -> Result<TomlConfig, ConfigError> {
    let s = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&s)?)
}

pub fn validate(cfg: &Config) -> Result<(), ConfigError> {
    if cfg.watch.as_os_str().is_empty() {
        return Err(ConfigError::MissingWatch);
    }
    if !cfg.watch.is_dir() {
        return Err(ConfigError::NotADirectory(cfg.watch.clone()));
    }
    if cfg.retention.is_zero() {
        return Err(ConfigError::ZeroRetention);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_byte_size_units() {
        assert_eq!(parse_byte_size("1024").unwrap(), 1024);
        assert_eq!(parse_byte_size("2K").unwrap(), 2048);
        assert_eq!(parse_byte_size("1M").unwrap(), 1024 * 1024);
        assert_eq!(parse_byte_size("1G").unwrap(), 1024u64.pow(3));
    }
}

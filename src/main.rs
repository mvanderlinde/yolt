//! yolt — macOS filesystem backup watcher. See spec/.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use yolt::backup::{self, initial_snapshot_pruned};
use yolt::config::{self, Config, ConfigError, TomlConfig};
use yolt::ignore_rules::IgnoreRules;
use yolt::project;
use yolt::restore;
use yolt::retention;
use yolt::watcher;

#[derive(Parser)]
#[command(
    name = "yolt",
    version,
    about = "Undo destructive LLM actions — backs up files before they change; quick revert when AI misbehaves (macOS)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Watch the filesystem and write backups (foreground).
    Watch(WatchArgs),
    /// Restore a file or directory from backups (default: newest; use --back to go further back).
    Restore(RestoreArgs),
}

#[derive(Parser)]
struct WatchArgs {
    /// Directory to watch (defaults to current directory).
    #[arg(value_name = "dir", default_value = ".")]
    watch: PathBuf,

    #[arg(long)]
    backup_root: Option<PathBuf>,

    #[arg(long)]
    retention: Option<String>,

    #[arg(long)]
    max_disk: Option<String>,

    #[arg(long)]
    config: Option<PathBuf>,

    #[arg(long)]
    debounce_ms: Option<u64>,

    #[arg(long)]
    prune_interval_secs: Option<u64>,

    #[arg(long)]
    session_idle_ms: Option<u64>,

    #[arg(long, default_value_t = false)]
    no_snapshot_initial: bool,

    #[arg(long, default_value_t = false)]
    no_default_ignores: bool,

    #[arg(long = "ignore")]
    ignore: Vec<String>,

    #[arg(long, default_value_t = false)]
    print_config: bool,
}

#[derive(Parser)]
struct RestoreArgs {
    /// File or directory path (relative to the watch root, or absolute under it).
    path: PathBuf,

    #[arg(long)]
    dir: Option<PathBuf>,

    #[arg(long)]
    backup_root: Option<PathBuf>,

    #[arg(long)]
    config: Option<PathBuf>,

    /// How many versions to skip from the newest (0 = latest backup that contains this path).
    #[arg(long, default_value_t = 0)]
    back: usize,

    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

struct CoalesceState {
    run_id: Option<String>,
    last_batch_end: Option<Instant>,
}

fn merge_watch_config(args: &WatchArgs) -> Result<Config, ConfigError> {
    let mut cfg = Config::default();

    if let Some(ref path) = args.config {
        let file: TomlConfig = config::load_toml_file(path)?;
        file.apply_to(&mut cfg)?;
    }

    config::apply_env(&mut cfg);

    cfg.watch = args.watch.clone();
    if let Some(ref b) = args.backup_root {
        cfg.backup_root = b.clone();
    }
    if let Some(ref r) = args.retention {
        cfg.retention = humantime::parse_duration(r).map_err(ConfigError::Retention)?;
    }
    if let Some(ref m) = args.max_disk {
        cfg.max_disk = if m == "0" {
            0
        } else {
            config::parse_byte_size(m).map_err(ConfigError::MaxDisk)?
        };
    }
    if let Some(d) = args.debounce_ms {
        cfg.debounce_ms = d;
    }
    if let Some(p) = args.prune_interval_secs {
        cfg.prune_interval_secs = p;
    }
    if let Some(s) = args.session_idle_ms {
        cfg.session_idle_ms = s;
    }
    if args.no_snapshot_initial {
        cfg.snapshot_initial = false;
    }
    if args.no_default_ignores {
        cfg.no_default_ignores = true;
    }
    cfg.extra_ignores.extend(args.ignore.iter().cloned());

    Ok(cfg)
}

fn merge_restore_config(args: &RestoreArgs) -> Result<Config, ConfigError> {
    let mut cfg = Config::default();
    cfg.watch = PathBuf::from(".");

    if let Some(ref path) = args.config {
        let file: TomlConfig = config::load_toml_file(path)?;
        file.apply_to(&mut cfg)?;
    }

    config::apply_env(&mut cfg);

    if let Some(ref w) = args.dir {
        cfg.watch = w.clone();
    }
    if let Some(ref b) = args.backup_root {
        cfg.backup_root = b.clone();
    }

    Ok(cfg)
}

fn print_effective_config(cfg: &Config) {
    eprintln!("watch: {}", cfg.watch.display());
    eprintln!("backup_root: {}", cfg.backup_root.display());
    eprintln!("retention: {:?}", cfg.retention);
    eprintln!("max_disk: {} bytes", cfg.max_disk);
    eprintln!("debounce_ms: {}", cfg.debounce_ms);
    eprintln!("prune_interval_secs: {}", cfg.prune_interval_secs);
    eprintln!("session_idle_ms: {}", cfg.session_idle_ms);
    eprintln!("snapshot_initial: {}", cfg.snapshot_initial);
    eprintln!("no_default_ignores: {}", cfg.no_default_ignores);
    eprintln!("extra_ignores: {:?}", cfg.extra_ignores);
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Watch(args) => {
            let cfg = merge_watch_config(&args)?;
            config::validate(&cfg)?;
            let (project_id, project_backup_root) =
                project::namespaced_backup_root(&cfg.backup_root, &cfg.watch)?;

            if args.print_config {
                print_effective_config(&cfg);
                eprintln!("project_id: {}", project_id);
                eprintln!("project_backup_root: {}", project_backup_root.display());
                return Ok(());
            }

            let watch_lock = project::acquire_watch_lock(&project_backup_root).map_err(|e| {
                anyhow::anyhow!(
                    "watch root is already active: {} ({})",
                    cfg.watch.display(),
                    e
                )
            })?;
            std::fs::create_dir_all(&project_backup_root)?;

            let hash_cache: Arc<Mutex<HashMap<PathBuf, [u8; 32]>>> =
                Arc::new(Mutex::new(HashMap::new()));

            let project_ignore = cfg.watch.join(".yoltignore");
            let ignore = Arc::new(
                IgnoreRules::new(
                    &cfg.watch,
                    !cfg.no_default_ignores,
                    Some(project_ignore.as_path()),
                    &cfg.extra_ignores,
                )
                .map_err(|e| anyhow::anyhow!("{e}"))?,
            );

            let current_run: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
            let coalesce: Arc<Mutex<CoalesceState>> = Arc::new(Mutex::new(CoalesceState {
                run_id: None,
                last_batch_end: None,
            }));

            let backup_root = project_backup_root.clone();
            let retention_dur = cfg.retention;
            let max_disk = cfg.max_disk;
            let prune_every = cfg.prune_interval_secs;
            let cur_for_prune = Arc::clone(&current_run);

            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_secs(prune_every.max(1)));
                let cur = cur_for_prune.lock().ok().and_then(|g| g.clone());
                if let Err(e) =
                    retention::prune(&backup_root, retention_dur, max_disk, cur.as_deref())
                {
                    eprintln!("error: prune failed: {e}");
                }
            });

            let watch_path = cfg.watch.clone();
            let backup_path = project_backup_root.clone();

            if cfg.snapshot_initial {
                let run_id = backup::new_backup_run_id();
                eprintln!("info: initial snapshot run {run_id}");
                {
                    let mut g = current_run.lock().unwrap();
                    *g = Some(run_id.clone());
                }
                let n = initial_snapshot_pruned(
                    &watch_path,
                    &backup_path,
                    &run_id,
                    &ignore,
                    hash_cache.as_ref(),
                )?;
                {
                    let mut g = current_run.lock().unwrap();
                    *g = None;
                }
                eprintln!("info: initial snapshot copied {n} bytes");
            }

            let debounce = Duration::from_millis(cfg.debounce_ms.max(1));
            let session_idle = Duration::from_millis(cfg.session_idle_ms.max(1));

            eprintln!(
                "info: watching {} -> {} (session_idle={}ms)",
                watch_path.display(),
                backup_path.display(),
                cfg.session_idle_ms
            );

            let watch_root = watch_path.clone();
            let wr = cfg.watch.clone();
            let br = project_backup_root.clone();
            let coalesce_w = Arc::clone(&coalesce);
            let hash_cache_w = Arc::clone(&hash_cache);

            // Keep guard alive for the full run.
            let _watch_lock = watch_lock;
            watcher::run(watch_root, debounce, ignore, move |files| {
                let now = Instant::now();
                let run_id = {
                    let mut st = coalesce_w.lock().unwrap();
                    let gap_ok = st
                        .last_batch_end
                        .map(|t| now.duration_since(t) < session_idle)
                        .unwrap_or(false);
                    if gap_ok {
                        match &st.run_id {
                            Some(r) => r.clone(),
                            None => {
                                let r = backup::new_backup_run_id();
                                st.run_id = Some(r.clone());
                                r
                            }
                        }
                    } else {
                        let r = backup::new_backup_run_id();
                        st.run_id = Some(r.clone());
                        r
                    }
                };

                {
                    let mut g = current_run.lock().unwrap();
                    *g = Some(run_id.clone());
                }
                for path in &files {
                    if let Err(e) =
                        backup::copy_file_into_run(&wr, &br, &run_id, path, hash_cache_w.as_ref())
                    {
                        eprintln!("error: copy {}: {e}", path.display());
                    }
                }
                {
                    let mut g = current_run.lock().unwrap();
                    *g = None;
                }
                {
                    let mut st = coalesce_w.lock().unwrap();
                    st.last_batch_end = Some(Instant::now());
                }
                Ok(())
            })
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        Commands::Restore(args) => {
            let cfg = merge_restore_config(&args)?;
            config::validate(&cfg)?;
            let (_project_id, project_backup_root) =
                project::namespaced_backup_root(&cfg.backup_root, &cfg.watch)?;
            let rel = restore::normalize_relative_to_watch(&cfg.watch, &args.path)?;
            let (run_id, dst) = restore::restore(
                &cfg.watch,
                &project_backup_root,
                &rel,
                args.back,
                args.dry_run,
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;
            if !args.dry_run {
                eprintln!("info: restored from run {} -> {}", run_id, dst.display());
            }
        }
    }

    Ok(())
}

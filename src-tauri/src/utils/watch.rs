//! Watch folders: run an action again when its inputs change (the app's Watch toggle and `--watch`).
//!
//! Events are debounced (`notify-debouncer-full`), filtered (include / exclude globs, the action's own
//! output folders, tool and temp files), then each changed file must *settle* (same size and mtime
//! twice, `settle_ms` apart; at most 10 s) before the run. Changes during a run mark the watch dirty:
//! it runs once more afterwards, never N times.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use globset::{Glob, GlobSet, GlobSetBuilder};
use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, RecommendedCache};
use serde::{Deserialize, Serialize};

/// Event name; keep in sync with `EVENTS.watch` in `src/lib/ipc.ts`.
pub const WATCH_EVENT: &str = "watch://status";

/// Folders never watched (VCS, package managers, engine caches).
const IGNORED_DIRS: &[&str] = &[".git", "node_modules", "Library", "Temp", "Intermediate", "Saved", ".godot"];
/// Temp and backup files of editors and browsers.
const IGNORED_FILES: &[&str] = &["~*", "*.tmp", "*.part", "*.crdownload", ".~lock.*", "*.bak"];
const SETTLE_LIMIT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WatchConfig {
    /// Folders and files to watch.
    pub paths: Vec<PathBuf>,
    pub recursive: bool,
    /// Globs on the file name or the path; empty = everything.
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    /// Output folders of the action itself (a run must not trigger the next one).
    pub ignore: Vec<PathBuf>,
    pub debounce_ms: u64,
    pub settle_ms: u64,
}

impl Default for WatchConfig {
    fn default() -> Self {
        WatchConfig { paths: Vec::new(), recursive: true, include: Vec::new(), exclude: Vec::new(), ignore: Vec::new(), debounce_ms: 500, settle_ms: 300 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WatchState {
    Idle,
    Running,
    Error,
}

/// Payload of `watch://status`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchStatus {
    pub watch_id: u64,
    pub state: WatchState,
    pub last_run_at: Option<u64>,
    pub last_duration_ms: Option<u64>,
    pub last_summary: Option<String>,
    pub last_error: Option<String>,
    pub last_issues: Option<usize>,
    pub changed_paths: Vec<String>,
}

impl WatchStatus {
    fn new(watch_id: u64) -> Self {
        WatchStatus { watch_id, state: WatchState::Idle, last_run_at: None, last_duration_ms: None, last_summary: None, last_error: None, last_issues: None, changed_paths: Vec::new() }
    }
}

/// What one run of the action reports back.
#[derive(Debug, Clone, Default)]
pub struct RunSummary {
    pub summary: String,
    /// Issues at warn or error level.
    pub issues: usize,
    /// The run itself failed (bad input); the watch keeps going.
    pub error: Option<String>,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn globs(patterns: &[String]) -> Result<GlobSet, String> {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        b.add(Glob::new(p).map_err(|e| format!("Bad pattern {p}: {e}"))?);
    }
    b.build().map_err(|e| e.to_string())
}

/// Which changed paths a config cares about.
pub struct PathFilter {
    include: Option<GlobSet>,
    exclude: GlobSet,
    temp: GlobSet,
    ignore: Vec<PathBuf>,
    roots: Vec<PathBuf>,
}

impl PathFilter {
    pub fn new(cfg: &WatchConfig) -> Result<Self, String> {
        Ok(PathFilter {
            include: (!cfg.include.is_empty()).then(|| globs(&cfg.include)).transpose()?,
            exclude: globs(&cfg.exclude)?,
            temp: globs(&IGNORED_FILES.iter().map(|s| s.to_string()).collect::<Vec<_>>())?,
            ignore: cfg.ignore.iter().map(|p| super::paths::normalize(p)).collect(),
            roots: cfg.paths.iter().map(|p| super::paths::normalize(p)).collect(),
        })
    }

    pub fn accepts(&self, p: &Path) -> bool {
        let p = super::paths::normalize(p);
        if self.ignore.iter().any(|i| p.starts_with(i)) {
            return false;
        }
        // Folder names and globs apply below the watched folder, not to where it lives (`C:\…\Temp\art`).
        let rel = self.roots.iter().filter_map(|r| p.strip_prefix(r).ok()).min_by_key(|r| r.components().count()).unwrap_or(&p);
        if rel.components().any(|c| IGNORED_DIRS.iter().any(|d| c.as_os_str() == *d)) {
            return false;
        }
        let name = p.file_name().unwrap_or_default();
        if self.temp.is_match(name) || self.exclude.is_match(name) || self.exclude.is_match(rel) {
            return false;
        }
        self.include.as_ref().is_none_or(|inc| inc.is_match(name) || inc.is_match(rel))
    }
}

fn stamp(p: &Path) -> Option<(u64, SystemTime)> {
    let m = std::fs::metadata(p).ok()?;
    m.is_file().then(|| (m.len(), m.modified().unwrap_or(UNIX_EPOCH)))
}

/// Wait until every existing file keeps its size and mtime over `settle`. Returns the files that never
/// settled within 10 s (they are left out of the run).
pub fn wait_settled(paths: &[PathBuf], settle: Duration, stop: &AtomicBool) -> Vec<PathBuf> {
    let started = Instant::now();
    let mut pending: Vec<(PathBuf, Option<(u64, SystemTime)>)> = paths.iter().filter(|p| p.is_file()).map(|p| (p.clone(), stamp(p))).collect();
    while !pending.is_empty() && started.elapsed() < SETTLE_LIMIT && !stop.load(Ordering::Relaxed) {
        std::thread::sleep(settle);
        pending.retain_mut(|(p, before)| {
            let now = stamp(p);
            let moving = now != *before;
            *before = now;
            moving
        });
    }
    pending.into_iter().map(|(p, _)| p).collect()
}

/// A running watch; dropping it stops the watcher and its worker thread.
pub struct WatchHandle {
    pub id: u64,
    status: Arc<Mutex<WatchStatus>>,
    stop: Arc<AtomicBool>,
    _debouncer: Debouncer<notify::RecommendedWatcher, RecommendedCache>,
    worker: Option<JoinHandle<()>>,
}

impl WatchHandle {
    pub fn status(&self) -> WatchStatus {
        self.status.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl Drop for WatchHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

pub struct Watcher;

impl Watcher {
    /// Start watching. `initial_run`: run the action once right away (`--watch` and the Watch toggle do).
    /// `on_status` gets every state change; `on_run` gets the changed paths (empty for the initial run).
    pub fn start(
        id: u64,
        config: WatchConfig,
        initial_run: bool,
        on_status: impl Fn(&WatchStatus) + Send + 'static,
        on_run: impl Fn(&[PathBuf]) -> RunSummary + Send + 'static,
    ) -> Result<WatchHandle, String> {
        if config.paths.is_empty() {
            return Err("Nothing to watch".into());
        }
        let filter = PathFilter::new(&config)?;
        let (tx, rx) = channel::<DebounceEventResult>();
        let mut debouncer = new_debouncer(Duration::from_millis(config.debounce_ms.max(1)), None, tx).map_err(|e| e.to_string())?;
        for p in &config.paths {
            let mode = if config.recursive && p.is_dir() { RecursiveMode::Recursive } else { RecursiveMode::NonRecursive };
            debouncer.watch(p, mode).map_err(|e| format!("Cannot watch {}: {e}", p.display()))?;
        }
        let status = Arc::new(Mutex::new(WatchStatus::new(id)));
        let stop = Arc::new(AtomicBool::new(false));
        let worker = {
            let (status, stop) = (status.clone(), stop.clone());
            let settle = Duration::from_millis(config.settle_ms);
            std::thread::spawn(move || worker_loop(rx, filter, settle, initial_run, &status, &stop, on_status, on_run))
        };
        Ok(WatchHandle { id, status, stop, _debouncer: debouncer, worker: Some(worker) })
    }
}

/// Changed paths in a debounced batch that pass the filter.
fn relevant(batch: DebounceEventResult, filter: &PathFilter, out: &mut BTreeSet<PathBuf>) {
    if let Ok(events) = batch {
        for ev in events {
            if ev.kind.is_access() {
                continue;
            }
            out.extend(ev.paths.iter().filter(|p| filter.accepts(p)).cloned());
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn worker_loop(
    rx: Receiver<DebounceEventResult>,
    filter: PathFilter,
    settle: Duration,
    initial_run: bool,
    status: &Mutex<WatchStatus>,
    stop: &AtomicBool,
    on_status: impl Fn(&WatchStatus),
    on_run: impl Fn(&[PathBuf]) -> RunSummary,
) {
    let publish = |f: &dyn Fn(&mut WatchStatus)| {
        let snapshot = {
            let mut s = status.lock().unwrap_or_else(|e| e.into_inner());
            f(&mut s);
            s.clone()
        };
        on_status(&snapshot);
    };
    let run = |changed: Vec<PathBuf>| {
        publish(&|s| {
            s.state = WatchState::Running;
            s.changed_paths = changed.iter().map(|p| p.to_string_lossy().into_owned()).collect();
        });
        let started = Instant::now();
        let r = on_run(&changed);
        publish(&|s| {
            s.state = if r.error.is_some() { WatchState::Error } else { WatchState::Idle };
            s.last_run_at = Some(now_ms());
            s.last_duration_ms = Some(started.elapsed().as_millis() as u64);
            s.last_summary = Some(r.summary.clone());
            s.last_error = r.error.clone();
            s.last_issues = Some(r.issues);
        });
    };

    if initial_run {
        run(Vec::new());
    } else {
        publish(&|_| {});
    }
    let mut changed = BTreeSet::new();
    while !stop.load(Ordering::Relaxed) {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(batch) => relevant(batch, &filter, &mut changed),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        // Everything that arrived meanwhile (including during the last run) goes into one run.
        while let Ok(batch) = rx.try_recv() {
            relevant(batch, &filter, &mut changed);
        }
        if changed.is_empty() {
            continue;
        }
        let paths: Vec<PathBuf> = std::mem::take(&mut changed).into_iter().collect();
        let unsettled = wait_settled(&paths, settle, stop);
        if stop.load(Ordering::Relaxed) {
            break;
        }
        let paths: Vec<PathBuf> = paths.into_iter().filter(|p| !unsettled.contains(p)).collect();
        if !unsettled.is_empty() {
            publish(&|s| s.last_error = Some(format!("Skipped {} file(s) still being written: {}", unsettled.len(), unsettled[0].display())));
        }
        if !paths.is_empty() {
            run(paths);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn filter_skips_outputs_temp_and_engine_folders() {
        let root = std::env::temp_dir().join("watch-filter");
        let cfg = WatchConfig { paths: vec![root.clone()], include: vec!["*.png".into()], exclude: vec!["*_old.png".into()], ignore: vec![root.join("build")], ..Default::default() };
        let f = PathFilter::new(&cfg).unwrap();
        assert!(f.accepts(&root.join("art/hero.png")));
        assert!(!f.accepts(&root.join("art/hero.txt")), "include");
        assert!(!f.accepts(&root.join("art/hero_old.png")), "exclude");
        assert!(!f.accepts(&root.join("build/hero.png")), "own output");
        assert!(!f.accepts(&root.join("Library/x.png")), "engine cache");
        assert!(!f.accepts(&root.join("art/~hero.png")), "temp file");
    }

    #[test]
    fn reruns_when_a_file_changes() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = WatchConfig { paths: vec![dir.path().to_path_buf()], debounce_ms: 100, settle_ms: 50, ..Default::default() };
        let (tx, rx) = mpsc::channel();
        let runs = Arc::new(Mutex::new(Vec::<Vec<PathBuf>>::new()));
        let seen = runs.clone();
        let handle = Watcher::start(
            7,
            cfg,
            true,
            move |s| {
                let _ = tx.send(s.clone());
            },
            move |changed| {
                seen.lock().unwrap().push(changed.to_vec());
                RunSummary { summary: format!("{} changed", changed.len()), issues: 0, error: None }
            },
        )
        .unwrap();

        let wait_idle_after_run = |rx: &mpsc::Receiver<WatchStatus>| loop {
            let s = rx.recv_timeout(Duration::from_secs(5)).expect("status within 5 s");
            if s.state == WatchState::Idle && s.last_run_at.is_some() {
                return s;
            }
        };
        let first = wait_idle_after_run(&rx);
        assert_eq!((first.watch_id, first.last_summary.as_deref()), (7, Some("0 changed")));

        std::fs::write(dir.path().join("a.png"), b"x").unwrap();
        let second = wait_idle_after_run(&rx);
        assert!(second.changed_paths.iter().any(|p| p.ends_with("a.png")), "{second:?}");
        assert_eq!(handle.status().state, WatchState::Idle);
        drop(handle);
        assert!(runs.lock().unwrap().len() >= 2);
    }
}

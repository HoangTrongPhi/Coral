use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use super::CoralState;
use crate::core::pipeline::{run_pack, summary, PackOptions};
use crate::core::project::Project;
use crate::core::sources::SourceCache;
use crate::core::{count_at_least, RunCtx, Severity};
use crate::utils::logbus::{self, Level};
use crate::utils::watch::{RunSummary, WatchConfig, WatchHandle, WatchStatus, Watcher, WATCH_EVENT};

/// What a watch re-runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum WatchAction {
    /// Pack & export the project again when a source or the project file changes (spec §7).
    PackProject {
        project: Project,
        /// When saved: edits to the file are picked up too.
        #[serde(default)]
        project_path: Option<String>,
    },
}

impl WatchAction {
    /// Every source path and the project file; export folders ignored (a run must not trigger the next).
    pub fn default_config(&self) -> WatchConfig {
        match self {
            WatchAction::PackProject { project, project_path } => {
                let mut paths: Vec<PathBuf> = project.sources.iter().map(|s| PathBuf::from(s.path())).collect();
                paths.extend(project_path.iter().map(PathBuf::from));
                paths.sort();
                paths.dedup();
                let ignore: Vec<PathBuf> = project.exports.iter().map(|e| PathBuf::from(&e.out)).collect();
                WatchConfig { paths, recursive: true, ignore, exclude: vec!["*.psb~".into(), "*.aseprite~".into()], ..Default::default() }
            }
        }
    }

    pub fn label(&self) -> String {
        match self {
            WatchAction::PackProject { project, .. } => format!("pack {}", project.name),
        }
    }
}

/// The state a pack watch keeps between runs (the project may be reloaded from its file).
pub struct PackWatch {
    project: Mutex<Project>,
    project_path: Option<PathBuf>,
    cache: Arc<SourceCache>,
}

impl PackWatch {
    pub fn new(project: Project, project_path: Option<PathBuf>, cache: Arc<SourceCache>) -> Self {
        PackWatch { project: Mutex::new(project), project_path, cache }
    }

    /// One run: reload the project if its file changed, pack, export. Returns the summary and the
    /// level-counted issues.
    pub fn run(&self, changed: &[PathBuf], cancel: &AtomicBool) -> (RunSummary, Option<crate::core::pipeline::PackReport>) {
        if let Some(pp) = &self.project_path {
            if changed.iter().any(|c| crate::utils::paths::normalize(c) == crate::utils::paths::normalize(pp)) {
                match Project::load(pp) {
                    Ok(p) => *self.project.lock().unwrap_or_else(|e| e.into_inner()) = p,
                    Err(e) => return (RunSummary { summary: String::new(), issues: 0, error: Some(format!("Project file: {e}")) }, None),
                }
            }
        }
        let project = self.project.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let (report, _) = run_pack(&project, &PackOptions::default(), &self.cache, &RunCtx { progress: &|_, _, _| {}, cancel });
        let error = report.issues.iter().find(|i| i.severity == Severity::Error).map(|i| i.message.clone());
        (RunSummary { summary: summary(&report), issues: count_at_least(&report.issues, Severity::Warn), error }, Some(report))
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchRequest {
    pub action: WatchAction,
    #[serde(default)]
    pub config: Option<WatchConfig>,
}

/// Tauri state: running watches. They stop when the app closes and never restart on their own.
#[derive(Default)]
pub struct WatchRegistry {
    next: Mutex<u64>,
    watches: Mutex<HashMap<u64, WatchHandle>>,
}

/// Start watching; runs the action once at once, then after every relevant change. Returns the watch id.
#[tauri::command]
pub fn watch_start<R: Runtime>(app: AppHandle<R>, registry: State<'_, WatchRegistry>, request: WatchRequest) -> Result<u64, String> {
    let id = {
        let mut n = registry.next.lock().map_err(|e| e.to_string())?;
        *n += 1;
        *n
    };
    let config = request.config.unwrap_or_else(|| request.action.default_config());
    let label = request.action.label();
    let WatchAction::PackProject { project, project_path } = request.action;
    let cache = app.state::<CoralState>().cache.clone();
    let job = PackWatch::new(project, project_path.map(PathBuf::from), cache);
    let (status_app, run_app) = (app.clone(), app.clone());
    let never = AtomicBool::new(false);
    let handle = Watcher::start(
        id,
        config,
        true,
        move |s: &WatchStatus| {
            let _ = status_app.emit(WATCH_EVENT, s);
        },
        move |changed| {
            let (r, _) = job.run(changed, &never);
            match &r.error {
                Some(e) => logbus::emit(&run_app, Level::Error, "watch", e.clone()),
                None => logbus::emit(&run_app, if r.issues > 0 { Level::Warn } else { Level::Info }, "watch", format!("Packed ({} change(s)): {}", changed.len(), r.summary)),
            }
            r
        },
    )?;
    registry.watches.lock().map_err(|e| e.to_string())?.insert(id, handle);
    logbus::emit(&app, Level::Info, "watch", format!("Watching: {label}"));
    Ok(id)
}

/// False when no watch had this id.
#[tauri::command]
pub fn watch_stop<R: Runtime>(app: AppHandle<R>, registry: State<'_, WatchRegistry>, watch_id: u64) -> Result<bool, String> {
    let removed = registry.watches.lock().map_err(|e| e.to_string())?.remove(&watch_id);
    if removed.is_some() {
        logbus::emit(&app, Level::Info, "watch", format!("Stopped watch {watch_id}"));
    }
    Ok(removed.is_some())
}

#[tauri::command]
pub fn watch_list(registry: State<'_, WatchRegistry>) -> Result<Vec<WatchStatus>, String> {
    let w = registry.watches.lock().map_err(|e| e.to_string())?;
    let mut v: Vec<WatchStatus> = w.values().map(WatchHandle::status).collect();
    v.sort_by_key(|s| s.watch_id);
    Ok(v)
}

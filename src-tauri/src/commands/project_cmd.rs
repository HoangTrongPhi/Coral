//! Projects and packing: load / save, sources → sprites, preview, pack & export, thumbnails.

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::ipc::Response;
use tauri::{AppHandle, Runtime, State};

use super::{run_job, CoralState};
use crate::core::pipeline::{self, PackOptions, PackReport, SpriteInfo};
use crate::core::project::{Animation, Project, Source};
use crate::core::sources::{self, SourceCache};
use crate::core::{image_io, quiet_ctx, Issue};
use crate::utils::jobs::JobRegistry;
use crate::utils::logbus::{self, Level};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub sprites: Vec<SpriteInfo>,
    pub animations: Vec<Animation>,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedProject {
    /// Paths resolved (absolute).
    pub project: Project,
    pub path: String,
    #[serde(flatten)]
    pub scan: ScanResult,
}

fn scan(project: &Project, cache: &SourceCache) -> ScanResult {
    let (sprites, animations, issues) = quiet_ctx(|ctx| pipeline::scan_sprites(project, cache, ctx));
    ScanResult { sprites, animations, issues }
}

/// Open a `*.coral.json`: the project (absolute paths) with its sprites and source problems.
#[tauri::command]
pub async fn load_project<R: Runtime>(app: AppHandle<R>, state: State<'_, CoralState>, path: String) -> Result<LoadedProject, String> {
    let cache = state.cache.clone();
    let p = PathBuf::from(&path);
    let r = tauri::async_runtime::spawn_blocking(move || {
        let project = Project::load(&p).map_err(|e| e.to_string())?;
        let scan = scan(&project, &cache);
        Ok::<_, String>(LoadedProject { project, path: p.to_string_lossy().into_owned(), scan })
    })
    .await
    .map_err(|e| e.to_string())?;
    match &r {
        Ok(l) => logbus::emit(&app, Level::Info, "project", format!("Opened {path}: {} sprite(s), {} animation(s)", l.scan.sprites.len(), l.scan.animations.len())),
        Err(e) => logbus::emit(&app, Level::Error, "project", e.clone()),
    }
    r
}

/// Save with paths relative to the file (`*.coral.json` added when missing).
#[tauri::command]
pub fn save_project<R: Runtime>(app: AppHandle<R>, path: String, project: Project) -> Result<String, String> {
    let mut p = PathBuf::from(&path);
    if !path.ends_with(crate::core::project::EXTENSION) {
        let stem = p.file_name().unwrap_or_default().to_string_lossy().trim_end_matches(".json").to_string();
        p = p.with_file_name(format!("{stem}{}", crate::core::project::EXTENSION));
    }
    project.save(&p).map_err(|e| e.to_string())?;
    logbus::emit(&app, Level::Info, "project", format!("Saved {}", p.display()));
    Ok(p.to_string_lossy().into_owned())
}

/// Sources for dropped paths (folders, .aseprite, .psd, .gif) and the loose images among them.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DropSources {
    pub sources: Vec<Source>,
    /// PNG / TGA files dropped one by one: their folder becomes a source.
    pub ignored: Vec<String>,
}

#[tauri::command]
pub fn sources_for_paths(paths: Vec<String>) -> DropSources {
    let mut sources: Vec<Source> = Vec::new();
    let mut ignored = Vec::new();
    for p in &paths {
        let path = Path::new(p);
        match Source::for_path(path) {
            Some(s) => sources.push(s),
            None if image_io::is_image(path) => {
                // A loose image: use its folder (non-recursive, that file's type).
                if let Some(dir) = path.parent() {
                    let ext = crate::utils::paths::ext_lower(path);
                    let s = Source::Folder { path: dir.to_string_lossy().into_owned(), recursive: false, include: vec![format!("*.{ext}")] };
                    if !sources.contains(&s) {
                        sources.push(s);
                    }
                }
            }
            None => ignored.push(p.clone()),
        }
    }
    DropSources { sources, ignored }
}

/// Sprites of an (unsaved) project, without packing.
#[tauri::command]
pub async fn scan_sources<R: Runtime>(app: AppHandle<R>, jobs: State<'_, JobRegistry>, state: State<'_, CoralState>, project: Project) -> Result<ScanResult, String> {
    let cache = state.cache.clone();
    run_job(&app, &jobs, "scan", move |ctx| {
        let (sprites, animations, issues) = pipeline::scan_sprites(&project, &cache, ctx);
        ScanResult { sprites, animations, issues }
    })
    .await
}

fn remember(state: &CoralState, packed: &pipeline::Packed) {
    let mut prev = state.previews.lock().unwrap_or_else(|e| e.into_inner());
    prev.retain(|k, _| !k.starts_with("atlas:") && !k.starts_with("normal:"));
    for (i, img) in packed.images.iter().enumerate() {
        // Previews are PNG whatever the output format (the webview cannot show TGA).
        prev.insert(format!("atlas:{i}"), image_io::encode_png(img));
    }
    for (i, img) in packed.normals.iter().enumerate() {
        prev.insert(format!("normal:{i}"), image_io::encode_png(img));
    }
    drop(prev);
    *state.last_pack.lock().unwrap_or_else(|e| e.into_inner()) = Some(packed.clone());
}

/// Pack in memory (nothing written): the report; page images via `preview_png("atlas:<i>")`.
#[tauri::command]
pub async fn pack_preview<R: Runtime>(app: AppHandle<R>, jobs: State<'_, JobRegistry>, state: State<'_, CoralState>, project: Project) -> Result<PackReport, String> {
    let cache = state.cache.clone();
    // A dry run: export problems (no Godot project, rotation) show up before anything is written.
    let (report, packed) = run_job(&app, &jobs, "pack", move |ctx| pipeline::run_pack(&project, &PackOptions { dry_run: true, targets: Vec::new() }, &cache, ctx)).await?;
    remember(&state, &packed);
    Ok(report)
}

/// Pack and write every export (`dryRun`: only the list of files that would be written).
#[tauri::command]
pub async fn pack_and_export<R: Runtime>(app: AppHandle<R>, jobs: State<'_, JobRegistry>, state: State<'_, CoralState>, project: Project, options: Option<PackOptions>) -> Result<PackReport, String> {
    let cache = state.cache.clone();
    let options = options.unwrap_or_default();
    let name = project.name.clone();
    let (report, packed) = run_job(&app, &jobs, "pack", move |ctx| pipeline::run_pack(&project, &options, &cache, ctx)).await?;
    remember(&state, &packed);
    let level = if report.ok { Level::Info } else if report.cancelled { Level::Warn } else { Level::Error };
    logbus::emit(&app, level, "pack", format!("{name}: {} in {} ms", pipeline::summary(&report), report.elapsed_ms));
    for i in report.issues.iter().filter(|i| i.severity == crate::core::Severity::Error).take(20) {
        logbus::emit(&app, Level::Error, "pack", i.message.clone());
    }
    Ok(report)
}

/// PNG of one sprite as the source gives it (untrimmed), at most `maxSize` px.
#[tauri::command]
pub async fn sprite_thumbnail(state: State<'_, CoralState>, project: Project, name: String, max_size: u32) -> Result<Response, String> {
    let cache = state.cache.clone();
    tauri::async_runtime::spawn_blocking(move || {
        for src in &project.sources {
            let loaded = quiet_ctx(|ctx| sources::load_source(src, &cache, ctx));
            if let Some(s) = loaded.sprites.iter().find(|s| s.name == name) {
                return Ok(Response::new(image_io::encode_png(&image_io::thumbnail(&s.image, max_size))));
            }
        }
        Err(format!("No sprite named {name}"))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// A cached preview image (`atlas:<page>`, `normal:<page>`, `flipbook`, `normal`).
#[tauri::command]
pub fn preview_png(state: State<'_, CoralState>, key: String) -> Result<Response, String> {
    state.previews.lock().map_err(|e| e.to_string())?.get(&key).cloned().map(Response::new).ok_or_else(|| format!("No preview \"{key}\" yet"))
}

/// Any PNG / TGA / GIF on disk as PNG bytes (the webview cannot show TGA itself).
#[tauri::command]
pub async fn image_png(path: String) -> Result<Response, String> {
    tauri::async_runtime::spawn_blocking(move || image_io::load(Path::new(&path)).map(|i| Response::new(image_io::encode_png(&i)))).await.map_err(|e| e.to_string())?
}

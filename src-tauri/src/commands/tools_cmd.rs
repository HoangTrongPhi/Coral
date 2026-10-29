//! Slice, Flipbook, Pixel and Normal 2D modules.

use std::path::Path;

use tauri::ipc::Response;
use tauri::{AppHandle, Runtime, State};

use super::{run_job, CoralState};
use crate::core::flipbook::{self, FlipbookOptions, FlipbookReport};
use crate::core::normal2d::{self, NormalOptions, NormalSettings};
use crate::core::palette::{self, BleedOptions, PaletteReport, PaletteSort, RemapOptions, SwapOptions, UpscaleOptions};
use crate::core::slice::{self, ApplySlice, SliceOptions, SliceResult};
use crate::core::{image_io, WriteReport};
use crate::utils::jobs::JobRegistry;
use crate::utils::logbus::{self, Level};

fn log_write<R: Runtime>(app: &AppHandle<R>, source: &str, what: &str, r: &WriteReport) {
    let verb = if r.dry_run { "would write" } else { "wrote" };
    let level = if r.ok { Level::Info } else { Level::Error };
    logbus::emit(app, level, source, format!("{what}: {verb} {} file(s) in {} ms{}", r.files.len(), r.elapsed_ms, r.notes.first().map(|n| format!(" — {n}")).unwrap_or_default()));
}

#[tauri::command]
pub async fn slice_sheet(path: String, options: SliceOptions) -> Result<SliceResult, String> {
    tauri::async_runtime::spawn_blocking(move || slice::slice_file(Path::new(&path), &options)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn apply_slice<R: Runtime>(app: AppHandle<R>, jobs: State<'_, JobRegistry>, request: ApplySlice) -> Result<WriteReport, String> {
    let r = run_job(&app, &jobs, "slice", move |ctx| slice::apply(&request, ctx)).await?;
    log_write(&app, "slice", "Slice", &r);
    Ok(r)
}

/// Lay out the flipbook (preview in `preview_png("flipbook")`), writing unless `dryRun`.
#[tauri::command]
pub async fn build_flipbook<R: Runtime>(app: AppHandle<R>, jobs: State<'_, JobRegistry>, state: State<'_, CoralState>, options: FlipbookOptions) -> Result<FlipbookReport, String> {
    let (r, sheet) = run_job(&app, &jobs, "flipbook", move |ctx| flipbook::build(&options, ctx)).await?;
    if let Some(s) = sheet {
        state.set_preview("flipbook", image_io::encode_png(&image_io::thumbnail(&s, 2048)));
    }
    if !r.write.dry_run {
        log_write(&app, "flipbook", &format!("Flipbook {}×{}", r.cols, r.rows), &r.write);
    }
    Ok(r)
}

#[tauri::command]
pub async fn palette_extract(paths: Vec<String>, sort: Option<PaletteSort>) -> Result<PaletteReport, String> {
    tauri::async_runtime::spawn_blocking(move || palette::extract(&paths, sort.unwrap_or_default())).await.map_err(|e| e.to_string())
}

/// Write a palette file; the type follows the extension (`.hex`, `.gpl`, `.pal`).
#[tauri::command]
pub fn palette_save<R: Runtime>(app: AppHandle<R>, path: String, colors: Vec<String>) -> Result<(), String> {
    let cols: Vec<[u8; 4]> = colors.iter().map(|c| palette::parse_hex(c).ok_or(format!("bad colour {c}"))).collect::<Result<_, _>>()?;
    let p = Path::new(&path);
    let name = p.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    let text = palette::palette_text(&cols, &crate::utils::paths::ext_lower(p), &name)?;
    crate::utils::paths::write_atomic(p, text.as_bytes()).map_err(|e| e.to_string())?;
    logbus::emit(&app, Level::Info, "pixel", format!("Saved palette {path} ({} colours)", cols.len()));
    Ok(())
}

/// Colours of a palette file, as `#rrggbb`.
#[tauri::command]
pub fn palette_load(path: String) -> Result<Vec<String>, String> {
    palette::read_palette(Path::new(&path)).map(|c| c.into_iter().map(palette::to_hex).collect())
}

#[tauri::command]
pub async fn palette_remap<R: Runtime>(app: AppHandle<R>, jobs: State<'_, JobRegistry>, options: RemapOptions) -> Result<WriteReport, String> {
    let r = run_job(&app, &jobs, "pixel", move |ctx| palette::remap(&options, ctx)).await?;
    log_write(&app, "pixel", "Remap", &r);
    Ok(r)
}

#[tauri::command]
pub async fn palette_swap<R: Runtime>(app: AppHandle<R>, jobs: State<'_, JobRegistry>, options: SwapOptions) -> Result<WriteReport, String> {
    let r = run_job(&app, &jobs, "pixel", move |ctx| palette::swap(&options, ctx)).await?;
    log_write(&app, "pixel", "Palette swap", &r);
    Ok(r)
}

#[tauri::command]
pub async fn upscale_nearest<R: Runtime>(app: AppHandle<R>, jobs: State<'_, JobRegistry>, options: UpscaleOptions) -> Result<WriteReport, String> {
    let r = run_job(&app, &jobs, "pixel", move |ctx| palette::upscale(&options, ctx)).await?;
    log_write(&app, "pixel", "Upscale", &r);
    Ok(r)
}

#[tauri::command]
pub async fn alpha_bleed<R: Runtime>(app: AppHandle<R>, jobs: State<'_, JobRegistry>, options: BleedOptions) -> Result<WriteReport, String> {
    let r = run_job(&app, &jobs, "pixel", move |ctx| palette::bleed_files(&options, ctx)).await?;
    log_write(&app, "pixel", "Alpha bleed", &r);
    Ok(r)
}

#[tauri::command]
pub async fn normal_map<R: Runtime>(app: AppHandle<R>, jobs: State<'_, JobRegistry>, state: State<'_, CoralState>, options: NormalOptions) -> Result<WriteReport, String> {
    let (r, preview) = run_job(&app, &jobs, "normal", move |ctx| normal2d::run(&options, ctx)).await?;
    if let Some(p) = preview {
        state.set_preview("normal", image_io::encode_png(&p));
    }
    log_write(&app, "normal", "Normal maps", &r);
    Ok(r)
}

/// Live preview of one image's normal map (nothing written).
#[tauri::command]
pub async fn normal_preview(path: String, settings: NormalSettings) -> Result<Response, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let img = image_io::load(Path::new(&path))?;
        let img = image_io::thumbnail(&img, 1024);
        Ok(Response::new(image_io::encode_png(&normal2d::normal_map(&img, &settings))))
    })
    .await
    .map_err(|e| e.to_string())?
}

//! Flipbooks for VFX: a numbered frame sequence laid out on an even N×M grid (no trim, no padding),
//! with a small JSON `{ cols, rows, frames, fps, cell }`. Niagara SubUV: Sub Image Size = cols × rows;
//! Unity Texture Sheet Animation: Tiles X = cols, Y = rows.

use std::path::{Path, PathBuf};
use std::time::Instant;

use image::{imageops, RgbaImage};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::image_io::{self, ImageKind};
use super::{collect_images, Issue, RunCtx, WriteReport, Writer};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FlipbookOptions {
    /// Folder of frames (or a list of files).
    pub paths: Vec<String>,
    /// 1-based first and last frame of the naturally sorted list (0 = from the start / to the end).
    pub from: usize,
    pub to: usize,
    pub step: usize,
    /// 0 = suggested (closest to square).
    pub cols: u32,
    pub rows: u32,
    /// Cell size in px; 0 = the frames' own size, shrunk to fit `maxSize`.
    pub cell: u32,
    pub max_size: u32,
    pub fps: f32,
    pub format: ImageKind,
    /// Output image file; the JSON goes next to it.
    pub out: String,
    pub json: bool,
    pub dry_run: bool,
}

impl Default for FlipbookOptions {
    fn default() -> Self {
        FlipbookOptions { paths: Vec::new(), from: 0, to: 0, step: 1, cols: 0, rows: 0, cell: 0, max_size: 4096, fps: 30.0, format: ImageKind::Png, out: String::new(), json: true, dry_run: false }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlipbookReport {
    pub frames: usize,
    pub cols: u32,
    pub rows: u32,
    /// Cell width / height.
    pub cell: [u32; 2],
    pub width: u32,
    pub height: u32,
    pub sources: Vec<String>,
    pub write: WriteReport,
}

/// Grid closest to square that holds `n` cells (fewest empty cells on ties).
pub fn suggest_grid(n: usize) -> (u32, u32) {
    if n == 0 {
        return (0, 0);
    }
    let cols = (n as f64).sqrt().ceil() as u32;
    let rows = (n as u32).div_ceil(cols);
    (cols, rows)
}

/// Frames chosen from the sorted list.
pub fn select<T: Clone>(all: &[T], from: usize, to: usize, step: usize) -> Vec<T> {
    let start = from.saturating_sub(1).min(all.len());
    let end = if to == 0 { all.len() } else { to.min(all.len()) };
    if start >= end {
        return Vec::new();
    }
    all[start..end].iter().step_by(step.max(1)).cloned().collect()
}

pub fn build(o: &FlipbookOptions, ctx: &RunCtx) -> (FlipbookReport, Option<RgbaImage>) {
    let started = Instant::now();
    let (files, issues) = collect_images(&o.paths, false);
    let files: Vec<PathBuf> = select(&files, o.from, o.to, o.step);
    let mut w = Writer::new(o.dry_run);
    w.issues = issues;
    let empty = |w: Writer, files: &[PathBuf]| FlipbookReport { frames: 0, cols: 0, rows: 0, cell: [0, 0], width: 0, height: 0, sources: files.iter().map(|f| f.to_string_lossy().into_owned()).collect(), write: WriteReport::from_writer(w, started, ctx.cancelled(), vec![]) };
    if files.is_empty() {
        w.issues.push(Issue::error("no-frames", "No frames to lay out (check the folder and the frame range)"));
        return (empty(w, &files), None);
    }
    let total = files.len() as u64;
    let loaded: Vec<Result<RgbaImage, String>> = files.par_iter().map(|f| image_io::load(f)).collect();
    let mut frames = Vec::new();
    for (f, r) in files.iter().zip(loaded) {
        match r {
            Ok(i) => frames.push(i),
            Err(e) => w.issues.push(Issue::error("unreadable-image", format!("{}: {e}", f.display())).at(f)),
        }
    }
    if frames.is_empty() || super::has_errors(&w.issues) {
        return (empty(w, &files), None);
    }
    let (fw, fh) = frames[0].dimensions();
    if frames.iter().any(|f| f.dimensions() != (fw, fh)) {
        w.issues.push(Issue::warn("mixed-sizes", format!("Frames differ in size; all are resized to the first one's {fw}×{fh}")));
    }
    let (sc, sr) = suggest_grid(frames.len());
    let (mut cols, mut rows) = (if o.cols > 0 { o.cols } else { sc }, if o.rows > 0 { o.rows } else { sr });
    if o.cols > 0 && o.rows == 0 {
        rows = (frames.len() as u32).div_ceil(cols);
    }
    if o.rows > 0 && o.cols == 0 {
        cols = (frames.len() as u32).div_ceil(rows);
    }
    if (cols * rows) < frames.len() as u32 {
        w.issues.push(Issue::warn("grid-too-small", format!("{cols}×{rows} holds {} frames; the last {} are left out", cols * rows, frames.len() as u32 - cols * rows)));
        frames.truncate((cols * rows) as usize);
    }
    // Cell: requested size (square, keeping aspect inside) or the frame size, scaled down to fit maxSize.
    let (mut cw, mut ch) = if o.cell > 0 {
        let s = o.cell as f32 / fw.max(fh) as f32;
        (((fw as f32 * s).round() as u32).max(1), ((fh as f32 * s).round() as u32).max(1))
    } else {
        (fw, fh)
    };
    let max = o.max_size.max(1);
    if cw * cols > max || ch * rows > max {
        let s = (max as f32 / (cw * cols) as f32).min(max as f32 / (ch * rows) as f32);
        cw = ((cw as f32 * s).floor() as u32).max(1);
        ch = ((ch as f32 * s).floor() as u32).max(1);
        w.issues.push(Issue::info("cells-scaled", format!("Cells scaled to {cw}×{ch} to stay within {max} px")));
    }
    let mut sheet = RgbaImage::new(cw * cols, ch * rows);
    for (i, f) in frames.iter().enumerate() {
        if ctx.cancelled() {
            break;
        }
        (ctx.progress)(i as u64 + 1, total, "Laying out");
        let cell = if f.dimensions() == (cw, ch) { f.clone() } else { imageops::resize(f, cw, ch, imageops::FilterType::Lanczos3) };
        let (c, r) = (i as u32 % cols, i as u32 / cols);
        imageops::replace(&mut sheet, &cell, (c * cw) as i64, (r * ch) as i64);
    }
    let out = PathBuf::from(&o.out);
    if o.out.is_empty() {
        w.issues.push(Issue::error("no-output", "Choose where to save the flipbook"));
    } else {
        let out = if out.extension().is_none() { out.with_extension(o.format.ext()) } else { out };
        match image_io::encode(&sheet, o.format) {
            Ok(b) => {
                w.write(&out, "image", &b, None);
            }
            Err(e) => w.issues.push(Issue::error("encode-failed", e)),
        }
        if o.json {
            let j = json!({ "image": out.file_name().map(|f| f.to_string_lossy().into_owned()), "cols": cols, "rows": rows, "frames": frames.len(), "fps": o.fps, "cell": { "w": cw, "h": ch } });
            w.write(&out.with_extension("json"), "json", (serde_json::to_string_pretty(&j).unwrap_or_default() + "\n").as_bytes(), None);
        }
    }
    let report = FlipbookReport {
        frames: frames.len(),
        cols,
        rows,
        cell: [cw, ch],
        width: sheet.width(),
        height: sheet.height(),
        sources: files.iter().map(|f| f.to_string_lossy().into_owned()).collect(),
        write: WriteReport::from_writer(w, started, ctx.cancelled(), vec![format!("Niagara SubUV: Sub Image Size = {cols} × {rows}"), format!("Unity Texture Sheet Animation: Tiles X = {cols}, Y = {rows}")]),
    };
    (report, Some(sheet))
}

pub fn default_out(folder: &Path) -> PathBuf {
    let name = super::sources::file_stem(folder);
    folder.parent().unwrap_or(folder).join(format!("T_{name}_SubUV.png"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::quiet_ctx;
    use image::Rgba;

    #[test]
    fn sixty_four_frames_make_an_8x8_grid_in_order() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().join("explosion");
        std::fs::create_dir(&dir).unwrap();
        for i in 1..=64u32 {
            RgbaImage::from_pixel(8, 8, Rgba([i as u8, 0, 0, 255])).save(dir.join(format!("explosion_{i:04}.png"))).unwrap();
        }
        let out = d.path().join("T_Explosion_SubUV.png");
        let (r, sheet) = quiet_ctx(|ctx| build(&FlipbookOptions { paths: vec![dir.to_string_lossy().into()], out: out.to_string_lossy().into(), ..Default::default() }, ctx));
        assert!(r.write.ok, "{:?}", r.write.issues);
        assert_eq!((r.cols, r.rows, r.frames, r.width), (8, 8, 64, 64));
        let s = sheet.unwrap();
        assert_eq!(s.get_pixel(0, 0)[0], 1);
        assert_eq!(s.get_pixel(8 * 7, 0)[0], 8, "row 1 ends with frame 8");
        assert_eq!(s.get_pixel(0, 8)[0], 9, "row 2 starts with frame 9");
        assert_eq!(s.get_pixel(63, 63)[0], 64);
        let j: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(out.with_extension("json")).unwrap()).unwrap();
        assert_eq!((j["cols"].as_u64(), j["frames"].as_u64()), (Some(8), Some(64)));

        assert_eq!(select(&(1..=10).collect::<Vec<_>>(), 2, 9, 3), [2, 5, 8]);
        assert_eq!(suggest_grid(10), (4, 3));
    }
}

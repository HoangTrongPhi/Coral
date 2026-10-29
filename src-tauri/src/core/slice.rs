//! Cutting an existing sheet: a grid (cell size, offset, spacing; empty cells dropped) or automatic
//! "islands" (8-connected alpha components, nearby boxes merged, ordered in rows). The result is
//! written as separate PNGs or as metadata for the uncut sheet (JSON Hash / Unity meta).

use std::path::{Path, PathBuf};
use std::time::Instant;

use image::RgbaImage;
use serde::{Deserialize, Serialize};

use super::export::{self, Output};
use super::pipeline::{FrameRect, PackedSprite, Packed, PageInfo};
use super::project::{ExportTarget, Project, Target};
use super::trim::TrimInfo;
use super::{image_io, Issue, RunCtx, WriteReport, Writer};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SliceOptions {
    Grid {
        cell_w: u32,
        cell_h: u32,
        #[serde(default)]
        offset: [u32; 2],
        #[serde(default)]
        spacing: [u32; 2],
        /// Keep cells with no visible pixel.
        #[serde(default)]
        keep_empty: bool,
    },
    Auto {
        /// Boxes this close (px) become one sprite.
        #[serde(default = "two")]
        merge: u32,
        #[serde(default)]
        alpha_threshold: u8,
        /// Islands smaller than this many pixels (dust) are ignored.
        #[serde(default = "four")]
        min_pixels: u32,
    },
}

fn two() -> u32 {
    2
}
fn four() -> u32 {
    4
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliceRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SliceResult {
    pub width: u32,
    pub height: u32,
    pub rects: Vec<SliceRect>,
    pub suggested_names: Vec<String>,
}

fn has_pixels(img: &RgbaImage, r: &SliceRect, threshold: u8) -> bool {
    (r.y..r.y + r.h).any(|y| (r.x..r.x + r.w).any(|x| img.get_pixel(x, y)[3] > threshold))
}

pub fn grid(img: &RgbaImage, cw: u32, ch: u32, offset: [u32; 2], spacing: [u32; 2], keep_empty: bool) -> Vec<SliceRect> {
    let (w, h) = img.dimensions();
    let mut out = Vec::new();
    if cw == 0 || ch == 0 {
        return out;
    }
    let mut y = offset[1];
    while y + ch <= h {
        let mut x = offset[0];
        while x + cw <= w {
            let r = SliceRect { x, y, w: cw, h: ch };
            if keep_empty || has_pixels(img, &r, 0) {
                out.push(r);
            }
            x += cw + spacing[0];
        }
        y += ch + spacing[1];
    }
    out
}

/// Connected components (8-neighbour) of alpha > threshold → boxes, merged when ≤ `merge` px apart,
/// sorted in reading order (rows grouped with a tolerance of half the average height).
pub fn auto(img: &RgbaImage, merge: u32, threshold: u8, min_pixels: u32) -> Vec<SliceRect> {
    let (w, h) = img.dimensions();
    let mut label = vec![0u32; (w * h) as usize];
    let mut boxes: Vec<[u32; 5]> = Vec::new(); // x0 y0 x1 y1 count
    let mut stack = Vec::new();
    for sy in 0..h {
        for sx in 0..w {
            let i = (sy * w + sx) as usize;
            if label[i] != 0 || img.get_pixel(sx, sy)[3] <= threshold {
                continue;
            }
            let id = boxes.len() as u32 + 1;
            let mut b = [sx, sy, sx, sy, 0];
            label[i] = id;
            stack.push((sx, sy));
            while let Some((x, y)) = stack.pop() {
                b[0] = b[0].min(x);
                b[1] = b[1].min(y);
                b[2] = b[2].max(x);
                b[3] = b[3].max(y);
                b[4] += 1;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                        if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                            continue;
                        }
                        let j = (ny as u32 * w + nx as u32) as usize;
                        if label[j] == 0 && img.get_pixel(nx as u32, ny as u32)[3] > threshold {
                            label[j] = id;
                            stack.push((nx as u32, ny as u32));
                        }
                    }
                }
            }
            boxes.push(b);
        }
    }
    let mut rects: Vec<[u32; 4]> = boxes.iter().filter(|b| b[4] >= min_pixels).map(|b| [b[0], b[1], b[2], b[3]]).collect();
    // Merge boxes whose gap is ≤ merge, until nothing changes.
    let gap = |a: &[u32; 4], b: &[u32; 4]| {
        let gx = (b[0] as i64 - a[2] as i64 - 1).max(a[0] as i64 - b[2] as i64 - 1).max(0);
        let gy = (b[1] as i64 - a[3] as i64 - 1).max(a[1] as i64 - b[3] as i64 - 1).max(0);
        gx.max(gy)
    };
    loop {
        let mut merged = false;
        'outer: for i in 0..rects.len() {
            for j in i + 1..rects.len() {
                if gap(&rects[i], &rects[j]) <= merge as i64 {
                    let b = rects.remove(j);
                    let a = &mut rects[i];
                    *a = [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])];
                    merged = true;
                    break 'outer;
                }
            }
        }
        if !merged {
            break;
        }
    }
    let mut out: Vec<SliceRect> = rects.into_iter().map(|r| SliceRect { x: r[0], y: r[1], w: r[2] - r[0] + 1, h: r[3] - r[1] + 1 }).collect();
    reading_order(&mut out);
    out
}

/// Rows top to bottom (a rect joins a row when its top is within half the average height of the
/// row's first), left to right inside a row.
pub fn reading_order(rects: &mut Vec<SliceRect>) {
    if rects.is_empty() {
        return;
    }
    let avg_h = rects.iter().map(|r| r.h).sum::<u32>() as f32 / rects.len() as f32;
    let tol = avg_h / 2.0;
    rects.sort_by_key(|r| (r.y, r.x));
    let mut rows: Vec<Vec<SliceRect>> = Vec::new();
    for r in rects.drain(..) {
        match rows.last_mut() {
            Some(row) if (r.y as f32 - row[0].y as f32) <= tol => row.push(r),
            _ => rows.push(vec![r]),
        }
    }
    for mut row in rows {
        row.sort_by_key(|r| r.x);
        rects.extend(row);
    }
}

pub fn slice(img: &RgbaImage, opts: &SliceOptions) -> Vec<SliceRect> {
    match opts {
        SliceOptions::Grid { cell_w, cell_h, offset, spacing, keep_empty } => grid(img, *cell_w, *cell_h, *offset, *spacing, *keep_empty),
        SliceOptions::Auto { merge, alpha_threshold, min_pixels } => auto(img, *merge, *alpha_threshold, *min_pixels),
    }
}

/// `{base}_{i}` with `{base}` = the sheet's file name; `{i}` 0-based (`{i1}` 1-based, `{i:03}` padded).
pub fn names(pattern: &str, base: &str, n: usize) -> Vec<String> {
    (0..n)
        .map(|i| {
            let mut s = pattern.replace("{base}", base).replace("{i1}", &(i + 1).to_string());
            if let Some(start) = s.find("{i:0") {
                if let Some(end) = s[start..].find('}') {
                    let width: usize = s[start + 4..start + end].parse().unwrap_or(0);
                    s.replace_range(start..start + end + 1, &format!("{i:0width$}"));
                }
            }
            s.replace("{i}", &i.to_string())
        })
        .collect()
}

pub fn slice_file(path: &Path, opts: &SliceOptions) -> Result<SliceResult, String> {
    let img = image_io::load(path)?;
    let rects = slice(&img, opts);
    let base = super::sources::file_stem(path);
    Ok(SliceResult { width: img.width(), height: img.height(), suggested_names: names("{base}_{i}", &base, rects.len()), rects })
}

/// One cut sprite as the UI edits it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NamedRect {
    pub name: String,
    pub rect: SliceRect,
    #[serde(default)]
    pub pivot: Option<[f32; 2]>,
    /// `[left, top, right, bottom]` px.
    #[serde(default)]
    pub nine_slice: Option<[u32; 4]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SliceOutput {
    /// One PNG per rect in the output folder.
    #[default]
    Sprites,
    /// JSON Hash describing the uncut sheet.
    JsonHash,
    /// The sheet's own `.png.meta` (Unity slices it).
    UnityMeta,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplySlice {
    pub path: String,
    pub rects: Vec<NamedRect>,
    #[serde(default)]
    pub output: SliceOutput,
    /// Folder for sprites / JSON; the Unity meta goes next to the sheet.
    #[serde(default)]
    pub out_dir: String,
    #[serde(default)]
    pub dry_run: bool,
}

/// The sheet as a "packed" atlas of its own rects, for the metadata writers.
fn as_packed(img: &RgbaImage, rects: &[NamedRect]) -> Packed {
    Packed {
        pages: vec![PageInfo { index: 0, width: img.width(), height: img.height(), fill: 0.0, sprites: rects.len() }],
        sprites: rects
            .iter()
            .map(|r| PackedSprite {
                name: r.name.clone(),
                page: 0,
                frame: FrameRect { x: r.rect.x, y: r.rect.y, w: r.rect.w, h: r.rect.h },
                rotated: false,
                trim: TrimInfo::untrimmed(r.rect.w, r.rect.h),
                pivot: r.pivot.unwrap_or(super::pipeline::DEFAULT_PIVOT),
                nine_slice: r.nine_slice,
                alias_of: None,
            })
            .collect(),
        images: vec![img.clone()],
        ..Default::default()
    }
}

pub fn apply(a: &ApplySlice, ctx: &RunCtx) -> WriteReport {
    let started = Instant::now();
    let mut w = Writer::new(a.dry_run);
    let path = PathBuf::from(&a.path);
    let img = match image_io::load(&path) {
        Ok(i) => i,
        Err(e) => {
            w.issues.push(Issue::error("unreadable-image", format!("{}: {e}", path.display())).at(&path));
            return WriteReport::from_writer(w, started, false, vec![]);
        }
    };
    let mut seen = std::collections::HashSet::new();
    for r in &a.rects {
        if !seen.insert(r.name.as_str()) {
            w.issues.push(Issue::error("duplicate-name", format!("Two rects are named \"{}\"", r.name)));
        }
        if r.rect.w == 0 || r.rect.h == 0 || r.rect.x + r.rect.w > img.width() || r.rect.y + r.rect.h > img.height() {
            w.issues.push(Issue::error("rect-outside", format!("\"{}\" lies outside the {}×{} sheet", r.name, img.width(), img.height())));
        }
    }
    if super::has_errors(&w.issues) {
        return WriteReport::from_writer(w, started, false, vec![]);
    }
    let out = if a.out_dir.is_empty() { path.parent().unwrap_or(Path::new(".")).to_path_buf() } else { PathBuf::from(&a.out_dir) };
    match a.output {
        SliceOutput::Sprites => {
            for (i, r) in a.rects.iter().enumerate() {
                if ctx.cancelled() {
                    break;
                }
                (ctx.progress)(i as u64 + 1, a.rects.len() as u64, &r.name);
                let cut = image::imageops::crop_imm(&img, r.rect.x, r.rect.y, r.rect.w, r.rect.h).to_image();
                w.write(&out.join(format!("{}.png", r.name.replace('/', "_"))), "sprite", &image_io::encode_png(&cut), Some(&path));
            }
        }
        SliceOutput::JsonHash | SliceOutput::UnityMeta => {
            let base = super::sources::file_stem(&path);
            let mut project = Project::new(&base);
            project.pack.image_format = if crate::utils::paths::ext_lower(&path) == "tga" { image_io::ImageKind::Tga } else { image_io::ImageKind::Png };
            let packed = as_packed(&img, &a.rects);
            let mut o = Output::default();
            if a.output == SliceOutput::JsonHash {
                let doc = export::json::document(&project, &packed, 0, false);
                o.add(out.join(format!("{base}.json")), "json", (serde_json::to_string_pretty(&doc).unwrap_or_default() + "\n").into_bytes());
            } else {
                // The meta belongs to the sheet itself, wherever it is.
                let sheet_dir = path.parent().unwrap_or(Path::new("."));
                export::unity_meta::write(&ExportTarget::new(Target::Unity, sheet_dir.to_string_lossy()), &project, &packed, sheet_dir, &mut o);
                o.files.retain(|f| f.kind == "meta");
            }
            w.issues.extend(o.issues);
            for f in o.files {
                w.write(&f.path, f.kind, &f.bytes, Some(&path));
            }
        }
    }
    WriteReport::from_writer(w, started, ctx.cancelled(), vec![format!("{} sprite(s)", a.rects.len())])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::quiet_ctx;
    use image::Rgba;

    fn blob(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32) {
        for yy in y..y + h {
            for xx in x..x + w {
                img.put_pixel(xx, yy, Rgba([255, 255, 255, 255]));
            }
        }
    }

    #[test]
    fn auto_finds_six_shapes_in_reading_order() {
        let mut img = RgbaImage::new(64, 40);
        // Row 1 (tops slightly uneven), row 2; plus a shape split by a 1 px gap (merged) and dust.
        for (i, dy) in [0u32, 2, 1].iter().enumerate() {
            blob(&mut img, 2 + i as u32 * 20, 2 + dy, 10, 12);
        }
        for i in 0..3u32 {
            blob(&mut img, 4 + i as u32 * 20, 24, 8, 10);
        }
        blob(&mut img, 54, 24, 3, 10); // 2 px right of the third shape of row 2 (44..51): merged
        img.put_pixel(63, 0, Rgba([255, 255, 255, 255])); // dust
        let r = auto(&img, 2, 0, 4);
        assert_eq!(r.len(), 6, "{r:?}");
        assert_eq!((r[0].x, r[1].x, r[2].x), (2, 22, 42));
        assert!(r[3].y == 24 && r[3].x == 4 && r[5].x == 44 && r[5].w == 13, "row 2 with the merged piece: {:?}", r[5]);
    }

    #[test]
    fn grid_skips_empty_and_names() {
        let mut img = RgbaImage::new(64, 32);
        blob(&mut img, 0, 0, 4, 4);
        blob(&mut img, 40, 20, 4, 4);
        let g = grid(&img, 32, 16, [0, 0], [0, 0], false);
        assert_eq!(g, [SliceRect { x: 0, y: 0, w: 32, h: 16 }, SliceRect { x: 32, y: 16, w: 32, h: 16 }]);
        assert_eq!(grid(&img, 32, 16, [0, 0], [0, 0], true).len(), 4);
        assert_eq!(names("{base}_{i:02}", "hero", 2), ["hero_00", "hero_01"]);
        assert_eq!(names("{base}-{i1}", "hero", 1), ["hero-1"]);
    }

    #[test]
    fn apply_writes_sprites_and_metadata() {
        let d = tempfile::tempdir().unwrap();
        let mut img = RgbaImage::new(32, 16);
        blob(&mut img, 0, 0, 16, 16);
        let sheet = d.path().join("sheet.png");
        img.save(&sheet).unwrap();
        let rects = vec![
            NamedRect { name: "a".into(), rect: SliceRect { x: 0, y: 0, w: 16, h: 16 }, pivot: Some([0.5, 1.0]), nine_slice: None },
            NamedRect { name: "b".into(), rect: SliceRect { x: 16, y: 0, w: 16, h: 16 }, pivot: None, nine_slice: Some([2, 2, 2, 2]) },
        ];
        let out = d.path().join("out").to_string_lossy().into_owned();
        let r = quiet_ctx(|ctx| apply(&ApplySlice { path: sheet.to_string_lossy().into(), rects: rects.clone(), output: SliceOutput::Sprites, out_dir: out.clone(), dry_run: false }, ctx));
        assert!(r.ok && d.path().join("out/a.png").exists() && d.path().join("out/b.png").exists());
        let m = quiet_ctx(|ctx| apply(&ApplySlice { path: sheet.to_string_lossy().into(), rects: rects.clone(), output: SliceOutput::UnityMeta, out_dir: String::new(), dry_run: false }, ctx));
        assert!(m.ok, "{:?}", m.issues);
        let meta = std::fs::read_to_string(d.path().join("sheet.png.meta")).unwrap();
        assert!(meta.contains("name: a") && meta.contains("pivot: {x: 0.5, y: 0}") && meta.contains("border: {x: 2, y: 2, z: 2, w: 2}"));
        let j = quiet_ctx(|ctx| apply(&ApplySlice { path: sheet.to_string_lossy().into(), rects, output: SliceOutput::JsonHash, out_dir: out, dry_run: true }, ctx));
        assert!(j.ok && j.files.len() == 1 && !d.path().join("out/sheet.json").exists());
    }
}

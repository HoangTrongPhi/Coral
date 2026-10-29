//! Pixel-art tools: palettes (extract, `.hex` / `.gpl` / `.pal` files, remap to the nearest colour in
//! OKLab, swap tables), nearest-neighbour upscale and alpha bleed over files.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::Instant;

use image::{Rgba, RgbaImage};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use super::{bleed, collect_images, image_io, Issue, OutputMode, RunCtx, WriteReport, Writer};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaletteColor {
    /// `#rrggbb` or `#rrggbbaa`.
    pub hex: String,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaletteReport {
    pub colors: Vec<PaletteColor>,
    /// Opaque-ish pixels counted.
    pub pixels: u64,
    pub files: usize,
    pub issues: Vec<Issue>,
}

pub fn to_hex(c: [u8; 4]) -> String {
    if c[3] == 255 {
        format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
    } else {
        format!("#{:02x}{:02x}{:02x}{:02x}", c[0], c[1], c[2], c[3])
    }
}

pub fn parse_hex(s: &str) -> Option<[u8; 4]> {
    let h = s.trim().trim_start_matches('#');
    let v = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    match h.len() {
        6 => Some([v(0)?, v(2)?, v(4)?, 255]),
        8 => Some([v(0)?, v(2)?, v(4)?, v(6)?]),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PaletteSort {
    #[default]
    Frequency,
    Hue,
}

fn hue(c: [u8; 4]) -> (u32, u32) {
    let (r, g, b) = (c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / d) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    // Greys first, then by hue, then light to dark.
    (if d < 0.05 { 0 } else { 1 + ((h + 360.0) % 360.0) as u32 }, (1000.0 * (1.0 - max)) as u32)
}

/// Distinct colours (alpha 0 ignored) of all inputs.
pub fn extract(paths: &[String], sort: PaletteSort) -> PaletteReport {
    let (files, mut issues) = collect_images(paths, true);
    let counts: Vec<Result<HashMap<[u8; 4], u64>, (PathBuf, String)>> = files
        .par_iter()
        .map(|f| {
            let img = image_io::load(f).map_err(|e| (f.clone(), e))?;
            let mut m = HashMap::new();
            for p in img.pixels().filter(|p| p[3] > 0) {
                *m.entry(p.0).or_insert(0) += 1;
            }
            Ok(m)
        })
        .collect();
    let mut all: HashMap<[u8; 4], u64> = HashMap::new();
    for c in counts {
        match c {
            Ok(m) => {
                for (k, v) in m {
                    *all.entry(k).or_insert(0) += v;
                }
            }
            Err((f, e)) => issues.push(Issue::error("unreadable-image", format!("{}: {e}", f.display())).at(&f)),
        }
    }
    let pixels = all.values().sum();
    let mut v: Vec<([u8; 4], u64)> = all.into_iter().collect();
    match sort {
        PaletteSort::Frequency => v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0))),
        PaletteSort::Hue => v.sort_by(|a, b| hue(a.0).cmp(&hue(b.0)).then(a.0.cmp(&b.0))),
    }
    if v.len() > 4096 {
        issues.push(Issue::warn("too-many-colors", format!("{} colours: this does not look like pixel art; only the 4096 most frequent are listed", v.len())));
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(4096);
    }
    PaletteReport { colors: v.into_iter().map(|(c, n)| PaletteColor { hex: to_hex(c), count: n }).collect(), pixels, files: files.len(), issues }
}

// ---------------------------------------------------------------- palette files

/// Read `.hex` (Lospec: one `rrggbb` per line), `.gpl` (GIMP) or `.pal` (JASC-PAL).
pub fn read_palette(path: &Path) -> Result<Vec<[u8; 4]>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let ext = crate::utils::paths::ext_lower(path);
    let mut out = Vec::new();
    match ext.as_str() {
        "hex" | "txt" => {
            for l in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with(';')) {
                out.push(parse_hex(l).ok_or_else(|| format!("bad colour \"{l}\""))?);
            }
        }
        "gpl" => {
            if !text.trim_start().starts_with("GIMP Palette") {
                return Err("not a GIMP palette (no \"GIMP Palette\" header)".into());
            }
            for l in text.lines().skip(1).map(str::trim) {
                if l.is_empty() || l.starts_with('#') || l.starts_with("Name:") || l.starts_with("Columns:") {
                    continue;
                }
                let n: Vec<u8> = l.split_whitespace().take(3).filter_map(|v| v.parse().ok()).collect();
                if n.len() == 3 {
                    out.push([n[0], n[1], n[2], 255]);
                }
            }
        }
        "pal" => {
            let mut lines = text.lines().map(str::trim);
            if lines.next() != Some("JASC-PAL") {
                return Err("not a JASC palette (no \"JASC-PAL\" header)".into());
            }
            lines.next();
            let count: usize = lines.next().and_then(|l| l.parse().ok()).ok_or("bad colour count")?;
            for l in lines.take(count) {
                let n: Vec<u8> = l.split_whitespace().filter_map(|v| v.parse().ok()).collect();
                if n.len() >= 3 {
                    out.push([n[0], n[1], n[2], *n.get(3).unwrap_or(&255)]);
                }
            }
        }
        other => return Err(format!("unknown palette type .{other} (use .hex, .gpl or .pal)")),
    }
    if out.is_empty() {
        return Err("the palette has no colours".into());
    }
    Ok(out)
}

pub fn palette_text(colors: &[[u8; 4]], ext: &str, name: &str) -> Result<String, String> {
    Ok(match ext {
        "hex" | "txt" => colors.iter().map(|c| format!("{:02x}{:02x}{:02x}\n", c[0], c[1], c[2])).collect(),
        "gpl" => {
            let mut s = format!("GIMP Palette\nName: {name}\nColumns: 8\n#\n");
            for c in colors {
                s.push_str(&format!("{:>3} {:>3} {:>3}\t{}\n", c[0], c[1], c[2], to_hex(*c)));
            }
            s
        }
        "pal" => {
            let mut s = format!("JASC-PAL\n0100\n{}\n", colors.len());
            for c in colors {
                s.push_str(&format!("{} {} {}\n", c[0], c[1], c[2]));
            }
            s
        }
        other => return Err(format!("unknown palette type .{other}")),
    })
}

// ---------------------------------------------------------------- OKLab

fn srgb_to_linear(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Björn Ottosson's OKLab.
pub fn oklab(c: [u8; 4]) -> [f32; 3] {
    let (r, g, b) = (srgb_to_linear(c[0]), srgb_to_linear(c[1]), srgb_to_linear(c[2]));
    let l = (0.412_221_47 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    [0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s, 1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s, 0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s]
}

pub fn nearest(c: [u8; 4], palette: &[([u8; 4], [f32; 3])]) -> [u8; 4] {
    let lab = oklab(c);
    let d = |p: &[f32; 3]| (p[0] - lab[0]).powi(2) + (p[1] - lab[1]).powi(2) + (p[2] - lab[2]).powi(2);
    palette.iter().min_by(|a, b| d(&a.1).total_cmp(&d(&b.1))).map(|p| p.0).unwrap_or(c)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PaletteSource {
    File { path: String },
    Colors { colors: Vec<String> },
}

impl PaletteSource {
    pub fn load(&self) -> Result<Vec<[u8; 4]>, String> {
        match self {
            PaletteSource::File { path } => read_palette(Path::new(path)),
            PaletteSource::Colors { colors } => colors.iter().map(|c| parse_hex(c).ok_or(format!("bad colour {c}"))).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemapOptions {
    pub paths: Vec<String>,
    pub palette: PaletteSource,
    pub output: OutputMode,
    #[serde(default)]
    pub dry_run: bool,
}

/// Remap one image; returns the number of changed pixels and which colour went where.
pub fn remap_image(img: &mut RgbaImage, palette: &[([u8; 4], [f32; 3])], merges: &mut BTreeMap<String, String>) -> u64 {
    let mut cache: HashMap<[u8; 3], [u8; 4]> = HashMap::new();
    let mut changed = 0;
    for p in img.pixels_mut() {
        if p[3] == 0 {
            continue;
        }
        let key = [p[0], p[1], p[2]];
        let to = *cache.entry(key).or_insert_with(|| nearest([p[0], p[1], p[2], 255], palette));
        if [to[0], to[1], to[2]] != key {
            changed += 1;
            merges.entry(to_hex([p[0], p[1], p[2], 255])).or_insert_with(|| to_hex([to[0], to[1], to[2], 255]));
            *p = Rgba([to[0], to[1], to[2], p[3]]);
        }
    }
    changed
}

/// Writes `files` processed by `f` (in place: `.bak` first). `f` returns the new image and a note.
fn batch(paths: &[String], output: &OutputMode, dry_run: bool, suffix: &str, ctx: &RunCtx, f: &(dyn Fn(RgbaImage) -> (RgbaImage, Option<String>) + Sync)) -> (Writer, Vec<String>) {
    let (files, issues) = collect_images(paths, true);
    let mut w = Writer::new(dry_run);
    w.issues = issues;
    let total = files.len() as u64;
    let results: Vec<(PathBuf, Result<(RgbaImage, Option<String>), String>)> = files.par_iter().filter(|_| !ctx.cancelled()).map(|p| (p.clone(), image_io::load(p).map(f))).collect();
    let mut notes = Vec::new();
    for (i, (p, r)) in results.into_iter().enumerate() {
        (ctx.progress)(i as u64 + 1, total, &p.to_string_lossy());
        match r {
            Ok((img, note)) => {
                let target = output.target(&p, if output.in_place() { "" } else { suffix }, None);
                if output.in_place() && !w.backup(&p) {
                    continue;
                }
                let kind = if crate::utils::paths::ext_lower(&target) == "tga" { image_io::ImageKind::Tga } else { image_io::ImageKind::Png };
                match image_io::encode(&img, kind) {
                    Ok(b) => {
                        w.write(&target, "image", &b, Some(&p));
                    }
                    Err(e) => w.issues.push(Issue::error("encode-failed", e).at(&p)),
                }
                if let Some(n) = note {
                    notes.push(n);
                }
            }
            Err(e) => w.issues.push(Issue::error("unreadable-image", format!("{}: {e}", p.display())).at(&p)),
        }
    }
    (w, notes)
}

pub fn remap(o: &RemapOptions, ctx: &RunCtx) -> WriteReport {
    let started = Instant::now();
    let palette = match o.palette.load() {
        Ok(p) => p,
        Err(e) => {
            let mut w = Writer::new(o.dry_run);
            w.issues.push(Issue::error("bad-palette", e));
            return WriteReport::from_writer(w, started, false, vec![]);
        }
    };
    let pal: Vec<([u8; 4], [f32; 3])> = palette.iter().map(|&c| (c, oklab(c))).collect();
    let merges = std::sync::Mutex::new(BTreeMap::new());
    let (w, mut notes) = batch(&o.paths, &o.output, o.dry_run, "_remap", ctx, &|mut img| {
        let mut m = BTreeMap::new();
        let n = remap_image(&mut img, &pal, &mut m);
        merges.lock().unwrap().extend(m);
        (img, Some(format!("{n} pixel(s) changed")))
    });
    let merges = merges.into_inner().unwrap();
    let total: u64 = notes.iter().filter_map(|n| n.split_whitespace().next()?.parse::<u64>().ok()).sum();
    notes = vec![format!("{total} pixel(s) changed in {} file(s), {} colour(s) mapped onto the {}-colour palette", w.files.iter().filter(|f| f.kind == "image").count(), merges.len(), palette.len())];
    notes.extend(merges.iter().take(50).map(|(a, b)| format!("{a} → {b}")));
    WriteReport::from_writer(w, started, ctx.cancelled(), notes)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapOptions {
    pub paths: Vec<String>,
    /// Each table makes `<name>_<variant>.png`.
    pub tables: Vec<super::project::SwapTable>,
    pub output: OutputMode,
    #[serde(default)]
    pub dry_run: bool,
}

pub fn swap(o: &SwapOptions, ctx: &RunCtx) -> WriteReport {
    let started = Instant::now();
    let mut all = Writer::new(o.dry_run);
    let mut notes = Vec::new();
    for t in &o.tables {
        let map: Result<HashMap<[u8; 3], [u8; 4]>, String> = t.map.iter().map(|[a, b]| Ok(((parse_hex(a).map(|c| [c[0], c[1], c[2]]).ok_or(format!("bad colour {a}"))?), parse_hex(b).ok_or(format!("bad colour {b}"))?))).collect();
        let map = match map {
            Ok(m) => m,
            Err(e) => {
                all.issues.push(Issue::error("bad-swap", format!("{}: {e}", t.variant)));
                continue;
            }
        };
        let suffix = format!("_{}", t.variant);
        // Variants never overwrite the source: "in place" puts them next to it.
        let out = if o.output.in_place() { OutputMode::Folder { path: String::new() } } else { o.output.clone() };
        let (w, n) = batch(&o.paths, &out, o.dry_run, &suffix, ctx, &|mut img| {
            let mut changed = 0u64;
            for p in img.pixels_mut() {
                if let Some(to) = map.get(&[p[0], p[1], p[2]]) {
                    *p = Rgba([to[0], to[1], to[2], ((p[3] as u32 * to[3] as u32) / 255) as u8]);
                    changed += 1;
                }
            }
            (img, Some(changed.to_string()))
        });
        let changed: u64 = n.iter().filter_map(|v| v.parse::<u64>().ok()).sum();
        notes.push(format!("{}: {changed} pixel(s) swapped", t.variant));
        all.files.extend(w.files);
        all.issues.extend(w.issues);
    }
    WriteReport::from_writer(all, started, ctx.cancelled(), notes)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpscaleOptions {
    pub paths: Vec<String>,
    pub factor: u32,
    pub output: OutputMode,
    #[serde(default)]
    pub dry_run: bool,
}

pub fn upscale(o: &UpscaleOptions, ctx: &RunCtx) -> WriteReport {
    let started = Instant::now();
    let f = o.factor.clamp(2, 8);
    let (w, _) = batch(&o.paths, &o.output, o.dry_run, &format!("_x{f}"), ctx, &|img| {
        let (iw, ih) = img.dimensions();
        (image::imageops::resize(&img, iw * f, ih * f, image::imageops::FilterType::Nearest), None)
    });
    WriteReport::from_writer(w, started, ctx.cancelled(), vec![format!("×{f} nearest neighbour")])
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BleedOptions {
    pub paths: Vec<String>,
    pub output: OutputMode,
    #[serde(default)]
    pub dry_run: bool,
}

pub fn bleed_files(o: &BleedOptions, ctx: &RunCtx) -> WriteReport {
    let started = Instant::now();
    let (w, _) = batch(&o.paths, &o.output, o.dry_run, "", ctx, &|mut img| {
        bleed::alpha_bleed(&mut img, 0);
        (img, None)
    });
    WriteReport::from_writer(w, started, ctx.cancelled(), vec![])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::quiet_ctx;

    fn four_colours(dir: &Path) -> PathBuf {
        let mut img = RgbaImage::new(4, 1);
        for (x, c) in [[250, 10, 10, 255], [10, 240, 10, 255], [12, 12, 245, 255], [128, 128, 128, 255]].iter().enumerate() {
            img.put_pixel(x as u32, 0, Rgba(*c));
        }
        let p = dir.join("in/four.png");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        img.save(&p).unwrap();
        p
    }

    #[test]
    fn remap_to_a_known_palette() {
        let d = tempfile::tempdir().unwrap();
        let src = four_colours(d.path());
        let pal = d.path().join("rgb.hex");
        std::fs::write(&pal, "ff0000\n00ff00\n0000ff\n").unwrap();
        let out = d.path().join("out").to_string_lossy().into_owned();
        let r = quiet_ctx(|ctx| remap(&RemapOptions { paths: vec![src.to_string_lossy().into()], palette: PaletteSource::File { path: pal.to_string_lossy().into() }, output: OutputMode::Folder { path: out.clone() }, dry_run: false }, ctx));
        assert!(r.ok, "{:?}", r.issues);
        let img = image_io::load(&PathBuf::from(&out).join("four_remap.png")).unwrap();
        assert_eq!(img.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(img.get_pixel(1, 0).0, [0, 255, 0, 255]);
        assert_eq!(img.get_pixel(2, 0).0, [0, 0, 255, 255]);
        assert!(r.notes[0].starts_with("4 pixel(s) changed"), "{:?}", r.notes);
        assert_eq!(image_io::load(&src).unwrap().get_pixel(0, 0)[0], 250, "source untouched");
    }

    #[test]
    fn palette_files_round_trip_and_extract() {
        let d = tempfile::tempdir().unwrap();
        let colors = vec![[1, 2, 3, 255], [200, 100, 50, 255]];
        for ext in ["hex", "gpl", "pal"] {
            let p = d.path().join(format!("p.{ext}"));
            std::fs::write(&p, palette_text(&colors, ext, "test").unwrap()).unwrap();
            assert_eq!(read_palette(&p).unwrap(), colors, "{ext}");
        }
        let src = four_colours(d.path());
        let r = extract(&[src.to_string_lossy().into()], PaletteSort::Hue);
        assert_eq!((r.colors.len(), r.pixels), (4, 4));
        assert_eq!(r.colors[0].hex, "#808080", "greys first");
        assert_eq!(parse_hex("#11223344"), Some([0x11, 0x22, 0x33, 0x44]));
    }

    #[test]
    fn swap_upscale_and_in_place_bleed() {
        let d = tempfile::tempdir().unwrap();
        let src = four_colours(d.path());
        let table = super::super::project::SwapTable { variant: "red".into(), map: vec![["#0af00a".into(), "#ff0000".into()]] };
        let r = quiet_ctx(|ctx| swap(&SwapOptions { paths: vec![src.to_string_lossy().into()], tables: vec![table], output: OutputMode::InPlace, dry_run: false }, ctx));
        assert!(r.ok, "{:?}", r.issues);
        let v = image_io::load(&d.path().join("in/four_red.png")).unwrap();
        assert_eq!(v.get_pixel(1, 0).0, [255, 0, 0, 255]);
        let out = d.path().join("big").to_string_lossy().into_owned();
        let u = quiet_ctx(|ctx| upscale(&UpscaleOptions { paths: vec![src.to_string_lossy().into()], factor: 4, output: OutputMode::Folder { path: out.clone() }, dry_run: false }, ctx));
        assert!(u.ok);
        assert_eq!(image_io::load(&PathBuf::from(out).join("four_x4.png")).unwrap().dimensions(), (16, 4));
        let b = quiet_ctx(|ctx| bleed_files(&BleedOptions { paths: vec![src.to_string_lossy().into()], output: OutputMode::InPlace, dry_run: false }, ctx));
        assert!(b.ok && b.files.iter().any(|f| f.kind == "backup") && src.with_extension("png.bak").exists());
    }
}

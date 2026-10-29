//! `run_pack`: sources → trim → dedupe → pack → compose (extrude, bleed) → page images → exports.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use image::{imageops, RgbaImage};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use super::export;
use super::image_io;
use super::pack::{self, Item};
use super::project::{Animation, Project, Target, TrimMode};
use super::sources::{self, SourceCache, SourceRef};
use super::trim::{self, TrimInfo};
use super::{bleed, has_errors, normal2d, FileWrite, Issue, RunCtx, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// A sprite as the source gave it (Pack module list, `scan_sources`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteInfo {
    pub name: String,
    pub trim: TrimInfo,
    /// xxh3 of the trimmed pixels, hex.
    pub hash: String,
    pub origin: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias_of: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u32>,
    pub pivot: [f32; 2],
    pub nine_slice: Option<[u32; 4]>,
}

/// A sprite in the atlas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackedSprite {
    pub name: String,
    pub page: usize,
    /// Pixels in the page; `w`/`h` unrotated.
    pub frame: FrameRect,
    pub rotated: bool,
    pub trim: TrimInfo,
    pub pivot: [f32; 2],
    pub nine_slice: Option<[u32; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias_of: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub index: usize,
    pub width: u32,
    pub height: u32,
    /// Sprite pixels / page pixels, 0–1.
    pub fill: f32,
    pub sprites: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackReport {
    pub name: String,
    pub pages: Vec<PageInfo>,
    pub sprites: Vec<PackedSprite>,
    pub animations: Vec<Animation>,
    /// Sprites sharing the pixels of another one.
    pub aliases: usize,
    pub files: Vec<FileWrite>,
    pub issues: Vec<Issue>,
    pub dry_run: bool,
    pub elapsed_ms: u64,
    pub ok: bool,
    pub cancelled: bool,
}

/// A packed atlas in memory: what exporters and previews read.
#[derive(Clone, Default)]
pub struct Packed {
    pub pages: Vec<PageInfo>,
    pub sprites: Vec<PackedSprite>,
    pub animations: Vec<Animation>,
    pub images: Vec<RgbaImage>,
    pub normals: Vec<RgbaImage>,
    /// Encoded page images in the project's format (written by every export target).
    pub page_bytes: Vec<Vec<u8>>,
    pub normal_bytes: Vec<Vec<u8>>,
    pub issues: Vec<Issue>,
    pub aliases: usize,
    pub cancelled: bool,
}

struct Prepared {
    info: SpriteInfo,
    image: RgbaImage,
}

/// Default pivot: centre.
pub const DEFAULT_PIVOT: [f32; 2] = [0.5, 0.5];

/// Scan + trim + dedupe, without packing (the sprite list of the Pack module).
fn prepare(project: &Project, cache: &SourceCache, ctx: &RunCtx) -> (Vec<Prepared>, Vec<Animation>, Vec<Issue>) {
    let scan = sources::scan(project, cache, ctx);
    let threshold = project.pack.trim_threshold;
    let trim_on = project.pack.trim == TrimMode::Alpha;
    let mut prepared: Vec<Prepared> = scan
        .sprites
        .par_iter()
        .map(|s| {
            let (image, t) = if trim_on { trim::trim(&s.image, threshold) } else { ((*s.image).clone(), TrimInfo::untrimmed(s.image.width(), s.image.height())) };
            let o = project.sprites.get(&s.name);
            let pivot = o.and_then(|o| o.pivot).or(s.pivot).unwrap_or(DEFAULT_PIVOT);
            let nine_slice = o.and_then(|o| o.nine_slice).or(s.nine_slice);
            Prepared {
                info: SpriteInfo { name: s.name.clone(), trim: t, hash: format!("{:016x}", image_io::hash(&image)), origin: s.origin.clone(), alias_of: None, duration_ms: s.duration_ms, pivot, nine_slice },
                image,
            }
        })
        .collect();
    let mut issues = scan.issues;
    if project.pack.dedupe {
        let mut first: HashMap<String, String> = HashMap::new();
        for p in &mut prepared {
            match first.get(&p.info.hash) {
                Some(orig) => p.info.alias_of = Some(orig.clone()),
                None => {
                    first.insert(p.info.hash.clone(), p.info.name.clone());
                }
            }
        }
    }
    for p in &prepared {
        if p.image.dimensions() == (1, 1) && p.image.get_pixel(0, 0)[3] == 0 && trim_on {
            issues.push(Issue::warn("empty-sprite", format!("Sprite \"{}\" is fully transparent", p.info.name)).at(&p.info.origin.path));
        }
    }
    (prepared, scan.animations, issues)
}

pub fn scan_sprites(project: &Project, cache: &SourceCache, ctx: &RunCtx) -> (Vec<SpriteInfo>, Vec<Animation>, Vec<Issue>) {
    let (p, a, i) = prepare(project, cache, ctx);
    (p.into_iter().map(|p| p.info).collect(), a, i)
}

/// Copy the outermost pixels of `r` outward by `n` (edges and corners).
fn extrude(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, n: u32) {
    let (pw, ph) = img.dimensions();
    for k in 1..=n {
        for xx in x..x + w {
            if y >= k {
                let p = *img.get_pixel(xx, y);
                img.put_pixel(xx, y - k, p);
            }
            if y + h - 1 + k < ph {
                let p = *img.get_pixel(xx, y + h - 1);
                img.put_pixel(xx, y + h - 1 + k, p);
            }
        }
    }
    for yy in y.saturating_sub(n)..(y + h + n).min(ph) {
        for k in 1..=n {
            if x >= k {
                let p = *img.get_pixel(x, yy);
                img.put_pixel(x - k, yy, p);
            }
            if x + w - 1 + k < pw {
                let p = *img.get_pixel(x + w - 1, yy);
                img.put_pixel(x + w - 1 + k, yy, p);
            }
        }
    }
}

/// Pack in memory (no files): the preview and the input of every exporter.
pub fn pack(project: &Project, cache: &SourceCache, ctx: &RunCtx) -> Packed {
    let s = &project.pack;
    let (prepared, animations, mut issues) = prepare(project, cache, ctx);
    let mut out = Packed { animations, ..Default::default() };
    if ctx.cancelled() {
        out.cancelled = true;
        out.issues = issues;
        return out;
    }
    if prepared.is_empty() {
        issues.push(Issue::warn("no-sprites", "Nothing to pack: add a folder, an Aseprite, PSD or GIF file"));
        out.issues = issues;
        return out;
    }
    (ctx.progress)(0, 3, "Packing");
    let unique: Vec<usize> = (0..prepared.len()).filter(|&i| prepared[i].info.alias_of.is_none()).collect();
    let items: Vec<Item> = unique.iter().map(|&i| Item { id: i, w: prepared[i].image.width(), h: prepared[i].image.height(), key: prepared[i].info.name.clone() }).collect();
    let layout = pack::layout(&items, s);
    for &id in &layout.rejected {
        let p = &prepared[id];
        let msg = if p.image.width() + 2 * s.extrude > s.max_size || p.image.height() + 2 * s.extrude > s.max_size {
            format!("Sprite \"{}\" ({}×{}) is larger than the {} px atlas limit", p.info.name, p.image.width(), p.image.height(), s.max_size)
        } else {
            format!("Sprite \"{}\" does not fit in one {} px page: turn on multiple pages or raise the size limit", p.info.name, s.max_size)
        };
        issues.push(Issue::error("atlas-overflow", msg).at(&p.info.origin.path));
    }
    let by_id: HashMap<usize, pack::Placed> = layout.placed.iter().map(|p| (p.id, *p)).collect();
    let name_index: HashMap<&str, usize> = prepared.iter().enumerate().map(|(i, p)| (p.info.name.as_str(), i)).collect();

    // Sprites (aliases point at their original's placement).
    for p in &prepared {
        let src = p.info.alias_of.as_deref().and_then(|a| name_index.get(a)).copied().unwrap_or_else(|| name_index[p.info.name.as_str()]);
        let Some(pl) = by_id.get(&src) else { continue };
        out.sprites.push(PackedSprite {
            name: p.info.name.clone(),
            page: pl.page,
            frame: FrameRect { x: pl.x, y: pl.y, w: p.image.width(), h: p.image.height() },
            rotated: pl.rotated,
            trim: p.info.trim,
            pivot: p.info.pivot,
            nine_slice: p.info.nine_slice,
            alias_of: p.info.alias_of.clone(),
        });
    }
    out.aliases = out.sprites.iter().filter(|s| s.alias_of.is_some()).count();

    // Compose pages.
    (ctx.progress)(1, 3, "Composing");
    let normal_settings = s.normal.clone().unwrap_or_default();
    let normal_tiles: HashMap<usize, RgbaImage> = if s.normal_maps {
        unique.par_iter().map(|&i| (i, normal2d::normal_map(&prepared[i].image, &normal_settings))).collect()
    } else {
        HashMap::new()
    };
    for (pi, size) in layout.pages.iter().enumerate() {
        let mut img = RgbaImage::new(size.w, size.h);
        let mut nimg = s.normal_maps.then(|| RgbaImage::from_pixel(size.w, size.h, image::Rgba([128, 128, 255, 0])));
        let mut area = 0u64;
        let mut count = 0;
        for pl in layout.placed.iter().filter(|p| p.page == pi) {
            let tile = &prepared[pl.id].image;
            let tile = if pl.rotated { imageops::rotate90(tile) } else { tile.clone() };
            imageops::replace(&mut img, &tile, pl.x as i64, pl.y as i64);
            if s.extrude > 0 {
                extrude(&mut img, pl.x, pl.y, tile.width(), tile.height(), s.extrude);
            }
            if let (Some(n), Some(nt)) = (nimg.as_mut(), normal_tiles.get(&pl.id)) {
                let nt = if pl.rotated { imageops::rotate90(nt) } else { nt.clone() };
                imageops::replace(n, &nt, pl.x as i64, pl.y as i64);
                if s.extrude > 0 {
                    extrude(n, pl.x, pl.y, nt.width(), nt.height(), s.extrude);
                }
            }
            area += tile.width() as u64 * tile.height() as u64;
            count += 1;
        }
        if s.alpha_bleed {
            bleed::alpha_bleed(&mut img, (s.padding + 2 * s.extrude + 2).max(4));
        }
        if s.premultiply {
            bleed::premultiply(&mut img);
        }
        let aliases_here = out.sprites.iter().filter(|sp| sp.page == pi && sp.alias_of.is_some()).count();
        out.pages.push(PageInfo { index: pi, width: size.w, height: size.h, fill: area as f32 / (size.w as f32 * size.h as f32).max(1.0), sprites: count + aliases_here });
        out.images.push(img);
        if let Some(n) = nimg {
            out.normals.push(n);
        }
    }
    (ctx.progress)(2, 3, "Encoding");
    let kind = s.image_format;
    out.page_bytes = out.images.par_iter().map(|i| image_io::encode(i, kind).unwrap_or_default()).collect();
    out.normal_bytes = out.normals.par_iter().map(|i| image_io::encode(i, kind).unwrap_or_default()).collect();
    (ctx.progress)(3, 3, "Packed");
    out.issues = issues;
    out
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PackOptions {
    pub dry_run: bool,
    /// Only these targets (CLI `--target`); empty = every export of the project.
    pub targets: Vec<Target>,
}

/// Pack, then write every export target (or list the files in a dry run).
pub fn run_pack(project: &Project, opts: &PackOptions, cache: &SourceCache, ctx: &RunCtx) -> (PackReport, Packed) {
    let started = Instant::now();
    let packed = pack(project, cache, ctx);
    let mut w = Writer::new(opts.dry_run);
    w.issues = packed.issues.clone();
    let exports: Vec<_> = project.exports.iter().filter(|e| opts.targets.is_empty() || opts.targets.contains(&e.target)).collect();
    if project.exports.is_empty() {
        w.issues.push(Issue::warn("no-exports", "No export target: add one (JSON, Godot, Unity, Spine) to write files"));
    } else if exports.is_empty() {
        w.issues.push(Issue::warn("no-exports", "None of the requested targets is set up in this project"));
    }
    // A broken pack (overflow, duplicate names) writes nothing: half an atlas is worse than none.
    let blocked = has_errors(&packed.issues) || packed.pages.is_empty() || packed.cancelled;
    if !blocked {
        let mut written: Vec<PathBuf> = Vec::new();
        for e in exports {
            let o = export::run(e, project, &packed);
            w.issues.extend(o.issues);
            for f in o.files {
                // Two targets sharing a folder write the atlas once.
                if written.contains(&f.path) {
                    continue;
                }
                w.write(&f.path, f.kind, &f.bytes, None);
                written.push(f.path);
            }
        }
    } else if !project.exports.is_empty() && !packed.cancelled && !packed.pages.is_empty() {
        w.issues.push(Issue::info("export-skipped", "Nothing was written because of the errors above"));
    }
    let ok = !packed.cancelled && !has_errors(&w.issues);
    let report = PackReport {
        name: project.name.clone(),
        pages: packed.pages.clone(),
        sprites: packed.sprites.clone(),
        animations: packed.animations.clone(),
        aliases: packed.aliases,
        files: w.files,
        issues: w.issues,
        dry_run: opts.dry_run,
        elapsed_ms: started.elapsed().as_millis() as u64,
        ok,
        cancelled: packed.cancelled,
    };
    (report, packed)
}

/// One line for logs, the watch pill and the command line.
pub fn summary(r: &PackReport) -> String {
    let pages: Vec<String> = r.pages.iter().map(|p| format!("{}×{} ({:.0}%)", p.width, p.height, p.fill * 100.0)).collect();
    let files = r.files.len();
    let verb = if r.dry_run { "would write" } else { "wrote" };
    format!(
        "{} sprite(s){} on {} page(s) {}; {verb} {files} file(s)",
        r.sprites.len(),
        if r.aliases > 0 { format!(" ({} alias)", r.aliases) } else { String::new() },
        r.pages.len(),
        pages.join(", ")
    )
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::core::project::{Hitbox, LoopMode};

    /// The hero of spec §5: `hero_idle_0` trimmed 30×44 at (1,4) of 32×48 at (0,0) with a bottom-centre
    /// pivot, `hero_idle_1` untrimmed at (32,0), a 64×64 page, `idle` at 8 fps, one hitbox.
    pub fn hero_packed() -> (Project, Packed) {
        let mut project = Project::new("hero");
        project.hitboxes.insert("hero_idle_1".into(), vec![Hitbox { kind: "hitbox".into(), rect: [12, 4, 20, 10], name: Some("sword".into()) }]);
        let packed = Packed {
            pages: vec![PageInfo { index: 0, width: 64, height: 64, fill: 0.7, sprites: 2 }],
            sprites: vec![
                PackedSprite { name: "hero_idle_0".into(), page: 0, frame: FrameRect { x: 0, y: 0, w: 30, h: 44 }, rotated: false, trim: TrimInfo { source_w: 32, source_h: 48, x: 1, y: 4, w: 30, h: 44 }, pivot: [0.5, 1.0], nine_slice: None, alias_of: None },
                PackedSprite { name: "hero_idle_1".into(), page: 0, frame: FrameRect { x: 32, y: 0, w: 32, h: 48 }, rotated: false, trim: TrimInfo::untrimmed(32, 48), pivot: [0.5, 1.0], nine_slice: None, alias_of: None },
            ],
            animations: vec![Animation { name: "idle".into(), frames: vec!["hero_idle_0".into(), "hero_idle_1".into()], fps: 8.0, loop_mode: LoopMode::Loop, durations_ms: None }],
            images: vec![RgbaImage::new(64, 64)],
            page_bytes: vec![vec![1, 2, 3]],
            ..Default::default()
        };
        (project, packed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::project::{ExportTarget, Source};
    use crate::core::quiet_ctx;
    use crate::core::sources::test_support::*;

    fn project_with(dir: &std::path::Path) -> Project {
        let mut p = Project::new("hero");
        p.sources = vec![Source::Folder { path: dir.join("art").to_string_lossy().into_owned(), recursive: true, include: vec!["*.png".into()] }];
        p
    }

    #[test]
    fn packs_trims_dedupes_and_writes_exports() {
        let d = tempfile::tempdir().unwrap();
        for i in 0..4 {
            write_png(&d.path().join(format!("art/hero_run_{i}.png")), &shape(32, 48, 1 + i, 4, 30 - i, 44, [200, 40 * i as u8, 0, 255]));
        }
        // Same pixels, different place in the frame: an alias with its own offsets.
        write_png(&d.path().join("art/hero_copy.png"), &shape(32, 48, 0, 0, 30, 44, [200, 0, 0, 255]));
        let mut p = project_with(d.path());
        p.exports = vec![ExportTarget::new(Target::JsonHash, d.path().join("build").to_string_lossy()), ExportTarget::new(Target::Spine, d.path().join("build").to_string_lossy())];
        let cache = SourceCache::default();

        let (dry, _) = quiet_ctx(|ctx| run_pack(&p, &PackOptions { dry_run: true, targets: vec![] }, &cache, ctx));
        assert!(dry.ok, "{:?}", dry.issues);
        assert!(!d.path().join("build").exists(), "dry run writes nothing");
        let names: Vec<String> = dry.files.iter().map(|f| PathBuf::from(&f.path).file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(names, ["hero.png", "hero.json", "hero.atlas"], "atlas once for both targets");

        let (r, packed) = quiet_ctx(|ctx| run_pack(&p, &PackOptions::default(), &cache, ctx));
        assert!(r.ok && r.files.iter().all(|f| f.status == crate::core::WriteStatus::Written));
        assert_eq!((r.sprites.len(), r.aliases, r.pages.len()), (5, 1, 1));
        // Names in natural order: hero_copy is the original, hero_run_0 the alias.
        let orig = r.sprites.iter().find(|s| s.name == "hero_copy").unwrap();
        let alias = r.sprites.iter().find(|s| s.name == "hero_run_0").unwrap();
        assert_eq!(alias.alias_of.as_deref(), Some("hero_copy"));
        assert_eq!((alias.frame, alias.trim.x, orig.trim.x), (orig.frame, 1, 0));
        let page = &packed.images[0];
        let fr = orig.frame;
        assert_eq!(page.get_pixel(fr.x, fr.y)[0], 200, "pixels at the frame");
        assert_eq!(page.get_pixel(fr.x - 1, fr.y)[3], 255, "extruded 1 px");
        let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(d.path().join("build/hero.json")).unwrap()).unwrap();
        assert_eq!(json["animations"]["hero_run"].as_array().unwrap().len(), 4);
        assert!(summary(&r).starts_with("5 sprite(s) (1 alias) on 1 page(s)"));
    }

    #[test]
    fn overflow_blocks_writing() {
        let d = tempfile::tempdir().unwrap();
        write_png(&d.path().join("art/big.png"), &shape(80, 80, 0, 0, 80, 80, [1, 1, 1, 255]));
        let mut p = project_with(d.path());
        p.pack.max_size = 64;
        p.exports = vec![ExportTarget::new(Target::JsonHash, d.path().join("build").to_string_lossy())];
        let (r, _) = quiet_ctx(|ctx| run_pack(&p, &PackOptions::default(), &SourceCache::default(), ctx));
        assert!(!r.ok && r.issues.iter().any(|i| i.code == "atlas-overflow"));
        assert!(r.files.is_empty());
    }

    #[test]
    #[ignore = "benchmark: cargo test --release -- --ignored bench"]
    fn bench_watch_repack_200_sprites() {
        let d = tempfile::tempdir().unwrap();
        for i in 0..200 {
            write_png(&d.path().join(format!("art/s_{i}.png")), &shape(64, 64, i % 8, 0, 56, 60, [i as u8, 0, 0, 255]));
        }
        let p = project_with(d.path());
        let cache = SourceCache::default();
        quiet_ctx(|ctx| pack(&p, &cache, ctx));
        let t = Instant::now();
        let packed = quiet_ctx(|ctx| pack(&p, &cache, ctx));
        let ms = t.elapsed().as_millis();
        println!("re-pack of 200 cached sprites: {ms} ms, {} page(s)", packed.pages.len());
        assert!(ms < 1000, "{ms} ms");
    }
}

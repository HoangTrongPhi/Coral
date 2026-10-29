//! Sources → sprites: folders of images, Aseprite files, PSD layers, GIF frames.
//!
//! Decoded files are cached by `(path, mtime, size)` (`SourceCache`), so a watch re-run only decodes
//! what changed.

pub mod aseprite;
pub mod folder;
pub mod gif;
pub mod psd;

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use image::RgbaImage;
use serde::{Deserialize, Serialize};

use super::anim;
use super::project::{Animation, Project, Source};
use super::{natural, Issue, RunCtx};

/// Where a sprite came from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRef {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SourceSprite {
    pub name: String,
    pub image: Arc<RgbaImage>,
    pub origin: SourceRef,
    pub duration_ms: Option<u32>,
    /// From the source itself (an Aseprite slice); the project's override wins.
    pub pivot: Option<[f32; 2]>,
    pub nine_slice: Option<[u32; 4]>,
}

/// What one source yields.
#[derive(Debug, Clone, Default)]
pub struct Loaded {
    pub sprites: Vec<SourceSprite>,
    pub animations: Vec<Animation>,
    pub issues: Vec<Issue>,
}

type Stamp = (u64, u64);

/// Decoded sources, keyed by file and what was asked of it (layer selection…).
#[derive(Default)]
pub struct SourceCache {
    entries: Mutex<HashMap<(PathBuf, String), (Stamp, Arc<Loaded>)>>,
}

impl SourceCache {
    /// Cached result for `path` + `variant`, or `load()` when the file changed.
    pub fn get_or_load(&self, path: &Path, variant: &str, load: impl FnOnce() -> Loaded) -> Arc<Loaded> {
        let stamp = super::image_io::stamp(path);
        let key = (path.to_path_buf(), variant.to_string());
        if let Some(stamp) = stamp {
            if let Some((s, l)) = self.entries.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
                if *s == stamp {
                    return l.clone();
                }
            }
        }
        let loaded = Arc::new(load());
        if let Some(stamp) = stamp {
            // Failed decodes are not cached: the user is probably fixing the file.
            if loaded.issues.iter().all(|i| i.severity != super::Severity::Error) {
                self.entries.lock().unwrap_or_else(|e| e.into_inner()).insert(key, (stamp, loaded.clone()));
            }
        }
        loaded
    }

    pub fn len(&self) -> usize {
        self.entries.lock().map(|e| e.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Every sprite of a project, its animations (automatic + the project's) and the problems found.
#[derive(Debug, Clone, Default)]
pub struct Scan {
    pub sprites: Vec<SourceSprite>,
    pub animations: Vec<Animation>,
    pub issues: Vec<Issue>,
}

/// File name without the extension, made safe for sprite names.
pub fn clean_name(s: &str) -> String {
    let out: String = s.trim().chars().map(|c| if c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '/') { c } else { '_' }).collect();
    if out.is_empty() {
        "_".into()
    } else {
        out
    }
}

pub fn file_stem(p: &Path) -> String {
    clean_name(&p.file_stem().unwrap_or_default().to_string_lossy())
}

pub fn load_source(src: &Source, cache: &SourceCache, ctx: &RunCtx) -> Arc<Loaded> {
    let path = Path::new(src.path());
    match src {
        Source::Folder { recursive, include, .. } => Arc::new(folder::load(path, *recursive, include, cache, ctx)),
        Source::Aseprite { layers, .. } => {
            let variant = serde_json::to_string(layers).unwrap_or_default();
            cache.get_or_load(path, &variant, || aseprite::load(path, layers))
        }
        Source::Psd { mode, .. } => cache.get_or_load(path, &format!("{mode:?}"), || psd::load(path, *mode)),
        Source::Gif { .. } => cache.get_or_load(path, "gif", || gif::load(path)),
    }
}

/// All sources of the project in order. Duplicate sprite names are errors (never renamed silently).
pub fn scan(project: &Project, cache: &SourceCache, ctx: &RunCtx) -> Scan {
    let mut scan = Scan::default();
    let mut auto_anims: Vec<Animation> = Vec::new();
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    let total = project.sources.len() as u64;
    for (i, src) in project.sources.iter().enumerate() {
        if ctx.cancelled() {
            break;
        }
        (ctx.progress)(i as u64, total, src.path());
        let loaded = load_source(src, cache, ctx);
        scan.issues.extend(loaded.issues.iter().cloned());
        for s in &loaded.sprites {
            if let Some(first) = seen.get(&s.name) {
                scan.issues.push(Issue::error("duplicate-name", format!("Sprite name \"{}\" comes from both {} and {}", s.name, first, s.origin.path)).at(&s.origin.path));
                continue;
            }
            seen.insert(s.name.clone(), s.origin.path.clone());
            scan.sprites.push(s.clone());
        }
        for a in &loaded.animations {
            if auto_anims.iter().any(|b| b.name == a.name) {
                let prefixed = format!("{}_{}", file_stem(Path::new(src.path())), a.name);
                scan.issues.push(Issue::warn("duplicate-animation", format!("Animation \"{}\" exists twice; the one from {} is named \"{prefixed}\"", a.name, src.path())));
                auto_anims.push(Animation { name: prefixed, ..a.clone() });
            } else {
                auto_anims.push(a.clone());
            }
        }
    }
    (ctx.progress)(total, total, "Sources read");
    let mut anims = anim::merge(auto_anims, project);
    for a in &mut anims {
        let before = a.frames.len();
        a.frames.retain(|f| seen.contains_key(f));
        if a.frames.len() < before {
            scan.issues.push(Issue::warn("missing-frame", format!("Animation \"{}\": {} frame(s) not found among the sprites", a.name, before - a.frames.len())));
            a.durations_ms = None;
        }
    }
    anims.retain(|a| !a.frames.is_empty());
    scan.animations = anims;
    for name in project.sprites.keys().chain(project.hitboxes.keys()) {
        if !seen.contains_key(name) {
            scan.issues.push(Issue::info("stale-override", format!("Settings for sprite \"{name}\" but no such sprite (renamed or removed?)")));
        }
    }
    scan.sprites.sort_by(|a, b| natural::cmp(&a.name, &b.name));
    scan
}

/// Composite `top` over `bottom` at (dx, dy), normal blend with an extra opacity (0–255).
pub fn blend_over(bottom: &mut RgbaImage, top: &RgbaImage, dx: i32, dy: i32, opacity: u8) {
    let (bw, bh) = bottom.dimensions();
    for (x, y, p) in top.enumerate_pixels() {
        let (tx, ty) = (x as i32 + dx, y as i32 + dy);
        if tx < 0 || ty < 0 || tx >= bw as i32 || ty >= bh as i32 {
            continue;
        }
        let sa = p[3] as u32 * opacity as u32 / 255;
        if sa == 0 {
            continue;
        }
        let d = bottom.get_pixel_mut(tx as u32, ty as u32);
        let da = d[3] as u32;
        let out_a = sa + da * (255 - sa) / 255;
        for c in 0..3 {
            let v = (p[c] as u32 * sa + d[c] as u32 * da * (255 - sa) / 255) / out_a.max(1);
            d[c] = v.min(255) as u8;
        }
        d[3] = out_a.min(255) as u8;
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use image::{Rgba, RgbaImage};

    /// A `w`×`h` transparent picture with a filled `rw`×`rh` rect at (`x`, `y`).
    pub fn shape(w: u32, h: u32, x: u32, y: u32, rw: u32, rh: u32, c: [u8; 4]) -> RgbaImage {
        let mut img = RgbaImage::new(w, h);
        for yy in y..(y + rh).min(h) {
            for xx in x..(x + rw).min(w) {
                img.put_pixel(xx, yy, Rgba(c));
            }
        }
        img
    }

    pub fn write_png(path: &std::path::Path, img: &RgbaImage) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        img.save(path).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use crate::core::project::Source;
    use crate::core::quiet_ctx;

    #[test]
    fn duplicates_are_errors_and_cache_skips_unchanged_files() {
        let d = tempfile::tempdir().unwrap();
        write_png(&d.path().join("a/hero_0.png"), &shape(8, 8, 1, 1, 4, 4, [255, 0, 0, 255]));
        write_png(&d.path().join("a/hero_1.png"), &shape(8, 8, 2, 2, 4, 4, [255, 0, 0, 255]));
        write_png(&d.path().join("b/hero_0.png"), &shape(8, 8, 0, 0, 2, 2, [0, 255, 0, 255]));
        let folder = |p: &str| Source::Folder { path: d.path().join(p).to_string_lossy().into_owned(), recursive: true, include: vec!["*.png".into()] };
        let mut project = Project::new("t");
        project.sources = vec![folder("a"), folder("b")];
        let cache = SourceCache::default();
        let s = quiet_ctx(|ctx| scan(&project, &cache, ctx));
        assert_eq!(s.sprites.len(), 2);
        assert!(s.issues.iter().any(|i| i.code == "duplicate-name"));
        assert_eq!(s.animations.len(), 1, "hero_0, hero_1 → hero");
        assert_eq!(cache.len(), 3);
        let again = quiet_ctx(|ctx| scan(&project, &cache, ctx));
        assert!(Arc::ptr_eq(&s.sprites[0].image, &again.sprites[0].image), "decoded once");
    }

    #[test]
    fn blends_normally() {
        let mut bottom = shape(1, 1, 0, 0, 1, 1, [0, 0, 255, 255]);
        blend_over(&mut bottom, &shape(1, 1, 0, 0, 1, 1, [255, 0, 0, 255]), 0, 0, 128);
        let p = bottom.get_pixel(0, 0);
        assert!(p[0] > 120 && p[0] < 135 && p[2] > 120 && p[3] == 255, "{p:?}");
    }
}

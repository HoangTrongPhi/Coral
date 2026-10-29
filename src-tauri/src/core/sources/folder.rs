//! A folder of PNG / TGA (and GIF) files. Names: path below the folder without the extension,
//! sub-folders as `/` (`fx/spark_0`). Numbered names become animations.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rayon::prelude::*;

use super::{Loaded, SourceCache, SourceRef, SourceSprite};
use crate::core::{anim, glob_set, image_io, natural, project, Issue, RunCtx};

pub fn files(root: &Path, recursive: bool, include: &[String]) -> Result<Vec<PathBuf>, Issue> {
    if !root.is_dir() {
        return Err(Issue::error("source-not-found", format!("Folder not found: {}", root.display())).at(root));
    }
    let set = glob_set(include).map_err(|e| Issue::error("bad-pattern", e.to_string()))?;
    let mut v: Vec<PathBuf> = walkdir::WalkDir::new(root)
        .max_depth(if recursive { usize::MAX } else { 1 })
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file() && image_io::is_image(e.path()))
        .filter(|e| include.is_empty() || set.is_match(e.file_name()) || set.is_match(e.path().strip_prefix(root).unwrap_or(e.path())))
        .map(|e| e.into_path())
        .collect();
    v.sort_by(|a, b| natural::cmp_path(a, b));
    Ok(v)
}

pub fn sprite_name(root: &Path, file: &Path) -> String {
    let rel = file.strip_prefix(root).unwrap_or(file).with_extension("");
    let parts: Vec<String> = rel.components().map(|c| super::clean_name(&c.as_os_str().to_string_lossy())).collect();
    parts.join("/")
}

pub fn load(root: &Path, recursive: bool, include: &[String], cache: &SourceCache, ctx: &RunCtx) -> Loaded {
    let files = match files(root, recursive, include) {
        Ok(f) => f,
        Err(issue) => return Loaded { issues: vec![issue], ..Default::default() },
    };
    if files.is_empty() {
        return Loaded { issues: vec![Issue::warn("empty-source", format!("No images matching {} in {}", include.join(", "), root.display())).at(root)], ..Default::default() };
    }
    let parts: Vec<Arc<Loaded>> = files
        .par_iter()
        .filter(|_| !ctx.cancelled())
        .map(|f| {
            let name = sprite_name(root, f);
            if image_io::is_image(f) && crate::utils::paths::ext_lower(f) == "gif" {
                // An animated GIF in the folder: its frames, named after the file.
                return cache.get_or_load(f, "gif-in-folder", || super::gif::load_named(f, &name));
            }
            cache.get_or_load(f, "image", || match image_io::load(f) {
                Ok(img) => Loaded {
                    sprites: vec![SourceSprite { name, image: Arc::new(img), origin: SourceRef { path: f.to_string_lossy().into_owned(), frame: None, layer: None }, duration_ms: None, pivot: None, nine_slice: None }],
                    ..Default::default()
                },
                Err(e) => Loaded { issues: vec![Issue::error("unreadable-image", format!("{}: {e}", f.display())).at(f)], ..Default::default() },
            })
        })
        .collect();
    let mut out = Loaded::default();
    for p in parts {
        out.sprites.extend(p.sprites.iter().cloned());
        out.animations.extend(p.animations.iter().cloned());
        out.issues.extend(p.issues.iter().cloned());
    }
    let still: Vec<String> = out.sprites.iter().filter(|s| s.origin.frame.is_none()).map(|s| s.name.clone()).collect();
    out.animations.extend(anim::from_names(&still, project::default_fps()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::quiet_ctx;
    use crate::core::sources::test_support::*;

    #[test]
    fn names_with_subfolders_and_include() {
        let d = tempfile::tempdir().unwrap();
        let img = shape(4, 4, 0, 0, 2, 2, [1, 2, 3, 255]);
        for f in ["hero_idle_0.png", "hero_idle_1.png", "fx/spark_0.png", "fx/spark_1.tga", "notes.txt"] {
            if f.ends_with(".txt") {
                std::fs::write(d.path().join(f), "x").unwrap();
            } else {
                write_png(&d.path().join(f), &img);
            }
        }
        let cache = SourceCache::default();
        let all = quiet_ctx(|ctx| load(d.path(), true, &["*.png".into(), "*.tga".into()], &cache, ctx));
        let mut names: Vec<&str> = all.sprites.iter().map(|s| s.name.as_str()).collect();
        names.sort();
        assert_eq!(names, ["fx/spark_0", "fx/spark_1", "hero_idle_0", "hero_idle_1"]);
        assert_eq!(all.animations.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), ["fx/spark", "hero_idle"]);
        let flat = quiet_ctx(|ctx| load(d.path(), false, &["*.png".into()], &cache, ctx));
        assert_eq!(flat.sprites.len(), 2);
        let missing = quiet_ctx(|ctx| load(&d.path().join("nope"), true, &[], &cache, ctx));
        assert_eq!(missing.issues[0].code, "source-not-found");
    }
}

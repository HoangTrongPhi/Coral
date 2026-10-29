//! Spine / libGDX `.atlas` (the classic text format every Spine runtime and libGDX's TextureAtlas
//! read). One block per page; `offset` uses the bottom-left origin of the untrimmed sprite:
//! `offset.y = sourceH − trim.y − h`. Rotated regions: `rotate: true`, `size` unrotated.
//! *Verify*: load with spine-runtimes or libGDX (docs/PROGRESS.md).

use std::fmt::Write as _;
use std::path::Path;

use super::{page_file, page_images, Output};
use crate::core::pipeline::Packed;
use crate::core::project::{ExportTarget, FilterMode, Project};

pub fn atlas_text(project: &Project, packed: &Packed, filter: FilterMode) -> String {
    let ext = project.pack.image_format.ext();
    let n = packed.pages.len();
    let f = if filter == FilterMode::Point { "Nearest,Nearest" } else { "Linear,Linear" };
    let mut t = String::new();
    for (p, page) in packed.pages.iter().enumerate() {
        if p > 0 {
            t.push('\n');
        }
        let _ = write!(t, "{}\nsize: {},{}\nformat: RGBA8888\nfilter: {f}\nrepeat: none\n", page_file(project, p, n, ext), page.width, page.height);
        for s in packed.sprites.iter().filter(|s| s.page == p) {
            let tr = &s.trim;
            let _ = write!(
                t,
                "{}\n  rotate: {}\n  xy: {}, {}\n  size: {}, {}\n  orig: {}, {}\n  offset: {}, {}\n  index: -1\n",
                s.name,
                s.rotated,
                s.frame.x,
                s.frame.y,
                s.frame.w,
                s.frame.h,
                tr.source_w,
                tr.source_h,
                tr.x,
                tr.source_h - tr.y - tr.h
            );
        }
    }
    t
}

pub fn write(target: &ExportTarget, project: &Project, packed: &Packed, out: &Path, o: &mut Output) {
    page_images(project, packed, out, o);
    let filter = target.filter.unwrap_or(FilterMode::Bilinear);
    o.add(out.join(format!("{}.atlas", project.name)), "atlas-text", atlas_text(project, packed, filter).into_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::pipeline::test_support::hero_packed;

    #[test]
    fn spec_layout() {
        let (project, packed) = hero_packed();
        let t = atlas_text(&project, &packed, FilterMode::Bilinear);
        assert!(t.starts_with("hero.png\nsize: 64,64\nformat: RGBA8888\nfilter: Linear,Linear\nrepeat: none\nhero_idle_0\n  rotate: false\n  xy: 0, 0\n  size: 30, 44\n  orig: 32, 48\n  offset: 1, 0\n  index: -1\n"), "{t}");
    }
}

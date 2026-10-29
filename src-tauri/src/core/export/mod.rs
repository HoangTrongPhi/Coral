//! Metadata writers. Each target turns a packed atlas into files (bytes in memory); the pipeline
//! writes them (or lists them in a dry run). Formats: spec §5 and docs/FORMATS.md.

pub mod godot;
pub mod json;
pub mod spine;
pub mod unity_meta;

use std::path::{Path, PathBuf};

use crate::core::pipeline::Packed;
use crate::core::project::{ExportTarget, Project};
use crate::core::Issue;

/// One file an exporter produces.
pub struct OutFile {
    pub path: PathBuf,
    pub kind: &'static str,
    pub bytes: Vec<u8>,
}

#[derive(Default)]
pub struct Output {
    pub files: Vec<OutFile>,
    pub issues: Vec<Issue>,
}

impl Output {
    pub fn add(&mut self, path: PathBuf, kind: &'static str, bytes: Vec<u8>) {
        self.files.push(OutFile { path, kind, bytes });
    }
}

/// Atlas page image file names: `hero.png`, or `hero-0.png`, `hero-1.png`… for several pages.
pub fn page_file(project: &Project, page: usize, pages: usize, ext: &str) -> String {
    if pages <= 1 {
        format!("{}.{ext}", project.name)
    } else {
        format!("{}-{page}.{ext}", project.name)
    }
}

/// Copy every atlas page (and normal-map page) into `out`.
pub fn page_images(project: &Project, packed: &Packed, out: &Path, o: &mut Output) {
    let ext = project.pack.image_format.ext();
    let n = packed.pages.len();
    for (i, bytes) in packed.page_bytes.iter().enumerate() {
        o.add(out.join(page_file(project, i, n, ext)), "atlas", bytes.clone());
    }
    for (i, bytes) in packed.normal_bytes.iter().enumerate() {
        let name = page_file(project, i, n, ext).replacen(&format!(".{ext}"), &format!("_n.{ext}"), 1);
        o.add(out.join(name), "normal-atlas", bytes.clone());
    }
}

/// Files for one export target.
pub fn run(target: &ExportTarget, project: &Project, packed: &Packed) -> Output {
    use crate::core::project::Target::*;
    let out = PathBuf::from(&target.out);
    let mut o = Output::default();
    if target.out.trim().is_empty() {
        o.issues.push(Issue::error("export-no-folder", format!("{}: no output folder", target.target.label())));
        return o;
    }
    match target.target {
        JsonHash => json::write(target, project, packed, &out, false, &mut o),
        JsonArray => json::write(target, project, packed, &out, true, &mut o),
        Godot => godot::write(target, project, packed, &out, &mut o),
        Unity => unity_meta::write(target, project, packed, &out, &mut o),
        Spine => spine::write(target, project, packed, &out, &mut o),
    }
    o
}

/// `8.0`, `0.5`, `12.5`: floats as Godot and the JSON samples write them.
pub fn fmt_f(v: f32) -> String {
    if v.fract() == 0.0 && v.abs() < 1e9 {
        format!("{v:.1}")
    } else {
        let s = format!("{v:.4}");
        s.trim_end_matches('0').to_string()
    }
}

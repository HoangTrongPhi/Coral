//! Engines: plain functions over files, no `tauri` imports, so the commands, the command line
//! (`cli.rs`) and the tests all call the same code.

pub mod anim;
pub mod bleed;
pub mod export;
pub mod flipbook;
pub mod image_io;
pub mod natural;
pub mod normal2d;
pub mod pack;
pub mod palette;
pub mod pipeline;
pub mod project;
mod samples;
pub mod slice;
pub mod sources;
pub mod trim;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};

/// Progress and cancel for one long operation. Engines report `(done, total, message)` and check
/// `cancelled()` between units of work (each file, each frame…).
pub struct RunCtx<'a> {
    pub progress: &'a (dyn Fn(u64, u64, &str) + Sync),
    pub cancel: &'a AtomicBool,
}

impl RunCtx<'_> {
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

/// A context that reports nothing and is never cancelled (tests, quick calls).
pub fn quiet_ctx<R>(f: impl FnOnce(&RunCtx) -> R) -> R {
    let cancel = AtomicBool::new(false);
    f(&RunCtx { progress: &|_, _, _| {}, cancel: &cancel })
}

/// Errors that stop a whole operation. A single unreadable file is an `Issue` instead.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("{0}")]
    Input(String),
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
}

impl CoreError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        CoreError::Io { path: path.into(), source }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warn,
    Error,
}

/// Something a report wants the user to see; the job carries on past it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub severity: Severity,
    /// Stable kebab-case id, e.g. `duplicate-name`.
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl Issue {
    pub fn new(severity: Severity, code: &str, message: impl Into<String>) -> Self {
        Issue { severity, code: code.into(), message: message.into(), path: None }
    }
    pub fn error(code: &str, message: impl Into<String>) -> Self {
        Self::new(Severity::Error, code, message)
    }
    pub fn warn(code: &str, message: impl Into<String>) -> Self {
        Self::new(Severity::Warn, code, message)
    }
    pub fn info(code: &str, message: impl Into<String>) -> Self {
        Self::new(Severity::Info, code, message)
    }

    pub fn at(mut self, path: impl AsRef<Path>) -> Self {
        self.path = Some(path.as_ref().to_string_lossy().into_owned());
        self
    }
}

/// Issues at or above `level`.
pub fn count_at_least(issues: &[Issue], level: Severity) -> usize {
    issues.iter().filter(|i| i.severity >= level).count()
}

pub fn has_errors(issues: &[Issue]) -> bool {
    issues.iter().any(|i| i.severity == Severity::Error)
}

pub(crate) fn glob_set(patterns: &[String]) -> Result<GlobSet, CoreError> {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        b.add(Glob::new(p).map_err(|e| CoreError::Input(format!("Bad pattern {p}: {e}")))?);
    }
    b.build().map_err(|e| CoreError::Input(e.to_string()))
}

/// One file a write operation wrote, or would write in a dry run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileWrite {
    pub path: String,
    /// What it is: `atlas`, `json`, `tres`, `meta`, `atlas-text`, `sprite`, `image`, `backup`, …
    pub kind: String,
    pub status: WriteStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WriteStatus {
    Planned,
    Written,
    Skipped,
    Failed,
}

/// Collects planned or written files: in a dry run nothing touches the disk.
pub struct Writer {
    pub dry_run: bool,
    pub files: Vec<FileWrite>,
    pub issues: Vec<Issue>,
}

impl Writer {
    pub fn new(dry_run: bool) -> Self {
        Writer { dry_run, files: Vec::new(), issues: Vec::new() }
    }

    /// Atomic write (`<file>.tmp` then rename), or only the plan in a dry run.
    pub fn write(&mut self, path: &Path, kind: &str, bytes: &[u8], source: Option<&Path>) -> bool {
        let mut fw = FileWrite {
            path: path.to_string_lossy().into_owned(),
            kind: kind.into(),
            status: WriteStatus::Planned,
            bytes: Some(bytes.len() as u64),
            source: source.map(|s| s.to_string_lossy().into_owned()),
            note: None,
        };
        let ok = if self.dry_run {
            true
        } else {
            match crate::utils::paths::write_atomic(path, bytes) {
                Ok(()) => {
                    fw.status = WriteStatus::Written;
                    true
                }
                Err(e) => {
                    fw.status = WriteStatus::Failed;
                    fw.note = Some(e.to_string());
                    self.issues.push(Issue::error("write-failed", format!("Cannot write {}: {e}", path.display())).at(path));
                    false
                }
            }
        };
        self.files.push(fw);
        ok
    }

    /// Before overwriting a user's file in place: copy it to `<file>.bak` (kept if it exists).
    pub fn backup(&mut self, path: &Path) -> bool {
        let mut bak = path.as_os_str().to_owned();
        bak.push(".bak");
        let bak = PathBuf::from(bak);
        if bak.exists() || !path.exists() {
            return true;
        }
        let mut fw = FileWrite { path: bak.to_string_lossy().into_owned(), kind: "backup".into(), status: WriteStatus::Planned, bytes: None, source: Some(path.to_string_lossy().into_owned()), note: None };
        let ok = self.dry_run
            || match std::fs::copy(path, &bak) {
                Ok(n) => {
                    fw.status = WriteStatus::Written;
                    fw.bytes = Some(n);
                    true
                }
                Err(e) => {
                    fw.status = WriteStatus::Failed;
                    self.issues.push(Issue::error("backup-failed", format!("Cannot back up {}: {e}", path.display())).at(path));
                    false
                }
            };
        self.files.push(fw);
        ok
    }

    pub fn skip(&mut self, path: &Path, kind: &str, note: impl Into<String>) {
        self.files.push(FileWrite { path: path.to_string_lossy().into_owned(), kind: kind.into(), status: WriteStatus::Skipped, bytes: None, source: None, note: Some(note.into()) });
    }
}

/// The result of a batch that writes files (pixel tools, slicing, normal maps, bleed).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteReport {
    pub files: Vec<FileWrite>,
    pub issues: Vec<Issue>,
    pub dry_run: bool,
    pub elapsed_ms: u64,
    pub ok: bool,
    pub cancelled: bool,
    /// Tool-specific one-liners (e.g. "1 204 pixels changed").
    #[serde(default)]
    pub notes: Vec<String>,
}

impl WriteReport {
    pub fn from_writer(w: Writer, started: std::time::Instant, cancelled: bool, notes: Vec<String>) -> Self {
        let ok = !cancelled && !has_errors(&w.issues);
        WriteReport { files: w.files, issues: w.issues, dry_run: w.dry_run, elapsed_ms: started.elapsed().as_millis() as u64, ok, cancelled, notes }
    }
}

/// Where a batch tool writes: a separate folder (default) or over the inputs (`.bak` first).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum OutputMode {
    Folder { path: String },
    InPlace,
}

impl OutputMode {
    /// Target for `input`, with `suffix` before the extension and `ext` replacing it (if given).
    pub fn target(&self, input: &Path, suffix: &str, ext: Option<&str>) -> PathBuf {
        let stem = input.file_stem().unwrap_or_default().to_string_lossy();
        let ext = ext.map(str::to_string).unwrap_or_else(|| input.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_else(|| "png".into()));
        let name = format!("{stem}{suffix}.{ext}");
        match self {
            OutputMode::Folder { path } if !path.is_empty() => PathBuf::from(path).join(name),
            // No folder: next to the input.
            _ => input.with_file_name(name),
        }
    }

    pub fn in_place(&self) -> bool {
        matches!(self, OutputMode::InPlace)
    }
}

/// Image files under the given paths (files as given, folders walked), naturally sorted.
pub fn collect_images(paths: &[String], recursive: bool) -> (Vec<PathBuf>, Vec<Issue>) {
    let mut out = Vec::new();
    let mut issues = Vec::new();
    for p in paths {
        let p = PathBuf::from(p);
        if p.is_dir() {
            let walker = walkdir::WalkDir::new(&p).max_depth(if recursive { usize::MAX } else { 1 });
            let mut found: Vec<PathBuf> = walker.into_iter().flatten().filter(|e| e.file_type().is_file() && image_io::is_image(e.path())).map(|e| e.into_path()).collect();
            found.sort_by(|a, b| natural::cmp_path(a, b));
            out.extend(found);
        } else if p.is_file() {
            if image_io::is_image(&p) {
                out.push(p);
            } else {
                issues.push(Issue::warn("not-an-image", format!("Not a PNG/TGA/GIF image: {}", p.display())).at(&p));
            }
        } else {
            issues.push(Issue::error("not-found", format!("Not found: {}", p.display())).at(&p));
        }
    }
    (out, issues)
}

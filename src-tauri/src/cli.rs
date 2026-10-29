//! Command line, same exe as the app (`coral pack hero.coral.json --report r.json`). Runs without a
//! window, for build scripts and CI. Exit codes: 0 done and nothing at the `--fail-on` level, 1 issues
//! at that level (an atlas that does not fit, a source error, duplicate names…), 2 bad arguments, a
//! broken project file or missing input.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc};

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Serialize;

use crate::commands::watch_cmd::{PackWatch, WatchAction};
use crate::core::flipbook::{self, FlipbookOptions};
use crate::core::image_io::ImageKind;
use crate::core::normal2d::{self, Convention, HeightSource, NormalOptions, NormalSettings};
use crate::core::palette::{self, BleedOptions, PaletteSort, PaletteSource, RemapOptions, UpscaleOptions};
use crate::core::pipeline::{run_pack, summary, PackOptions, PackReport};
use crate::core::project::{Project, Target};
use crate::core::slice::{self, ApplySlice, NamedRect, SliceOptions, SliceOutput};
use crate::core::sources::SourceCache;
use crate::core::{count_at_least, FileWrite, Issue, OutputMode, RunCtx, Severity, WriteReport, WriteStatus};
use crate::utils::watch::Watcher;

const APP: &str = "Coral";
const EXAMPLES: &str = "\
Examples:
  coral pack hero.coral.json --report pack.json
  coral pack hero.coral.json --target godot --dry-run
  coral pack hero.coral.json --watch
  coral slice sheet.png --grid 32x32 --out sprites
  coral slice sheet.png --auto --merge 2 --meta unity
  coral flipbook fx/explosion --cols 8 --rows 8 --cell 256 --out T_Explosion_SubUV.png --json
  coral bleed atlas.png --in-place
  coral palette extract art --out art.hex
  coral palette remap art --palette endesga-32.hex --out art_remapped
  coral upscale icons --factor 4 --out icons_x4
  coral normal sprites --strength 2 --source alpha-bevel --out normals

Exit codes: 0 done, 1 issues at the --fail-on level, 2 bad arguments or input.
In cmd.exe run `start /wait coral.exe ...` to wait for the exit code.";

/// Subcommands the exe answers on the command line; any other first argument opens the window
/// (e.g. a `.coral.json` from Explorer's \"Open with\").
const SUBCOMMANDS: &[&str] = &["pack", "slice", "flipbook", "bleed", "palette", "upscale", "normal", "help"];

#[derive(Parser, Debug)]
#[command(name = "coral", version, about = "Coral — Sprite atlas & 2D animation toolkit", after_help = EXAMPLES)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Pack a project's sprites into atlases and write its exports.
    Pack(PackArgs),
    /// Cut a sheet into sprites (grid or automatic) or describe it with metadata.
    Slice(SliceArgs),
    /// Lay a numbered frame sequence out on an even grid (VFX SubUV).
    Flipbook(FlipbookArgs),
    /// Colour transparent pixels with their neighbours (alpha stays 0).
    Bleed(BleedArgs),
    /// Extract or apply palettes.
    #[command(subcommand)]
    Palette(PaletteCmd),
    /// Nearest-neighbour upscale (pixel art).
    Upscale(UpscaleArgs),
    /// 2D normal maps from luminance or an alpha bevel.
    Normal(NormalArgs),
}

/// Flags every subcommand has.
#[derive(Args, Debug, Clone)]
pub struct Common {
    /// Write the whole result (the same structure the app shows) as JSON.
    #[arg(long, value_name = "FILE")]
    pub report: Option<PathBuf>,
    /// Issue level that makes the exit code 1.
    #[arg(long, value_enum, default_value_t = FailOn::Error)]
    pub fail_on: FailOn,
    /// Only the summary line and errors.
    #[arg(long)]
    pub quiet: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailOn {
    Warn,
    Error,
}

impl FailOn {
    pub fn severity(self) -> Severity {
        match self {
            FailOn::Warn => Severity::Warn,
            FailOn::Error => Severity::Error,
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetArg {
    JsonHash,
    JsonArray,
    Godot,
    Unity,
    Spine,
    All,
}

#[derive(Args, Debug)]
pub struct PackArgs {
    /// The project file (`*.coral.json`).
    pub project: PathBuf,
    /// Only this export target (repeatable); default: all of the project's.
    #[arg(long, value_enum)]
    pub target: Vec<TargetArg>,
    /// Pack, but only list the files that would be written.
    #[arg(long)]
    pub dry_run: bool,
    /// Pack once, then again whenever a source or the project changes; Ctrl+C stops (exit 0).
    #[arg(long)]
    pub watch: bool,
    #[command(flatten)]
    pub common: Common,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaArg {
    JsonHash,
    Unity,
}

#[derive(Args, Debug)]
pub struct SliceArgs {
    pub sheet: PathBuf,
    /// Cell size, e.g. `32x32`.
    #[arg(long, value_name = "WxH", conflicts_with = "auto")]
    pub grid: Option<String>,
    /// Grid offset `x,y`.
    #[arg(long, value_name = "X,Y", default_value = "0,0")]
    pub offset: String,
    /// Grid spacing `x,y`.
    #[arg(long, value_name = "X,Y", default_value = "0,0")]
    pub spacing: String,
    /// Find separate shapes automatically.
    #[arg(long)]
    pub auto: bool,
    /// With --auto: shapes this close (px) become one sprite.
    #[arg(long, default_value_t = 2)]
    pub merge: u32,
    /// Sprite names; `{base}` = sheet name, `{i}` index, `{i:03}` padded, `{i1}` from 1.
    #[arg(long, default_value = "{base}_{i}")]
    pub names: String,
    /// Write metadata for the uncut sheet instead of separate PNGs.
    #[arg(long, value_enum)]
    pub meta: Option<MetaArg>,
    /// Output folder (sprites, JSON). The Unity meta goes next to the sheet.
    #[arg(long)]
    pub out: Option<PathBuf>,
    #[arg(long)]
    pub dry_run: bool,
    #[command(flatten)]
    pub common: Common,
}

#[derive(Args, Debug)]
pub struct FlipbookArgs {
    pub folder: PathBuf,
    #[arg(long, default_value_t = 0)]
    pub cols: u32,
    #[arg(long, default_value_t = 0)]
    pub rows: u32,
    /// Cell size (px, longer side); default: the frames' size.
    #[arg(long, default_value_t = 0)]
    pub cell: u32,
    /// First frame (1-based).
    #[arg(long, default_value_t = 0)]
    pub from: usize,
    /// Last frame (1-based).
    #[arg(long, default_value_t = 0)]
    pub to: usize,
    #[arg(long, default_value_t = 1)]
    pub step: usize,
    #[arg(long, default_value_t = 30.0)]
    pub fps: f32,
    #[arg(long, default_value_t = 4096)]
    pub max_size: u32,
    /// Output image (.png or .tga).
    #[arg(long)]
    pub out: PathBuf,
    /// Also write `<out>.json` with cols, rows, frames, fps and cell.
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[command(flatten)]
    pub common: Common,
}

#[derive(Args, Debug, Clone)]
pub struct Output {
    /// Output folder.
    #[arg(long, conflicts_with = "in_place")]
    pub out: Option<PathBuf>,
    /// Overwrite the inputs (a `.bak` copy is made first).
    #[arg(long)]
    pub in_place: bool,
}

impl Output {
    fn mode(&self) -> Result<OutputMode, String> {
        match (&self.out, self.in_place) {
            (Some(o), false) => Ok(OutputMode::Folder { path: o.to_string_lossy().into_owned() }),
            (None, true) => Ok(OutputMode::InPlace),
            _ => Err("Give --out <folder> or --in-place".into()),
        }
    }
}

#[derive(Args, Debug)]
pub struct BleedArgs {
    pub input: Vec<PathBuf>,
    #[command(flatten)]
    pub output: Output,
    #[arg(long)]
    pub dry_run: bool,
    #[command(flatten)]
    pub common: Common,
}

#[derive(Subcommand, Debug)]
pub enum PaletteCmd {
    /// Write the colours used by the images as a palette file (.hex, .gpl or .pal).
    Extract {
        input: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
        /// Order: by frequency or by hue.
        #[arg(long, value_enum, default_value_t = SortArg::Frequency)]
        sort: SortArg,
        #[command(flatten)]
        common: Common,
    },
    /// Move every colour to the nearest palette colour (OKLab).
    Remap {
        input: Vec<PathBuf>,
        #[arg(long)]
        palette: PathBuf,
        #[command(flatten)]
        output: Output,
        #[arg(long)]
        dry_run: bool,
        #[command(flatten)]
        common: Common,
    },
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortArg {
    Frequency,
    Hue,
}

#[derive(Args, Debug)]
pub struct UpscaleArgs {
    pub input: Vec<PathBuf>,
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u32).range(2..=8))]
    pub factor: u32,
    #[command(flatten)]
    pub output: Output,
    #[arg(long)]
    pub dry_run: bool,
    #[command(flatten)]
    pub common: Common,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceArg {
    Luminance,
    AlphaBevel,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConventionArg {
    Opengl,
    Directx,
}

#[derive(Args, Debug)]
pub struct NormalArgs {
    pub input: Vec<PathBuf>,
    #[arg(long, default_value_t = 2.0)]
    pub strength: f32,
    #[arg(long, value_enum, default_value_t = ConventionArg::Opengl)]
    pub convention: ConventionArg,
    #[arg(long, value_enum, default_value_t = SourceArg::Luminance)]
    pub source: SourceArg,
    /// Gaussian blur of the height (px).
    #[arg(long, default_value_t = 0.0)]
    pub blur: f32,
    /// Alpha bevel width (px).
    #[arg(long, default_value_t = 4.0)]
    pub bevel: f32,
    #[command(flatten)]
    pub output: Output,
    #[arg(long)]
    pub dry_run: bool,
    #[command(flatten)]
    pub common: Common,
}

/// True when the process was started for the command line rather than the app window.
pub fn wants_cli(args: &[String]) -> bool {
    args.get(1).is_some_and(|a| SUBCOMMANDS.contains(&a.as_str()) || matches!(a.as_str(), "--help" | "-h" | "--version" | "-V"))
}

/// Release builds use the Windows GUI subsystem: borrow the calling terminal's console for output.
fn attach_console() {
    #[cfg(windows)]
    {
        extern "system" {
            fn AttachConsole(process_id: u32) -> i32;
        }
        const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
        unsafe {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}

// ---------------------------------------------------------------- output

/// Console output in the suite's format: a title line, one `OK  `/`WARN`/`FAIL`/`SKIP` line per step
/// with at most 20 detail lines (problems first), and a closing line.
pub struct Out {
    pub quiet: bool,
}

impl Out {
    pub fn title(&self, command: &str, target: &Path) {
        if !self.quiet {
            println!("{APP} {} — {command} on {}", env!("CARGO_PKG_VERSION"), target.display());
        }
    }

    pub fn step(&self, mark: &str, title: &str, summary: &str, details: &[String]) {
        println!("{mark} {title}: {summary}");
        let mut lines: Vec<&String> = details.iter().filter(|d| d.trim_start().starts_with('!')).collect();
        if !self.quiet {
            lines.extend(details.iter().filter(|d| !d.trim_start().starts_with('!')));
        }
        for d in lines.iter().take(20) {
            println!("       {d}");
        }
        if lines.len() > 20 {
            println!("       … {} more line(s) in the report", lines.len() - 20);
        }
    }

    pub fn finish(&self, ok: bool, elapsed_ms: u64) {
        println!("{} in {elapsed_ms} ms", if ok { "Finished" } else { "Stopped after a failed step" });
    }
}

/// `! message (path)` for errors, `message (path)` otherwise.
pub fn issue_lines(issues: &[Issue]) -> Vec<String> {
    issues
        .iter()
        .map(|i| {
            let at = i.path.as_deref().map(|p| format!(" ({p})")).unwrap_or_default();
            let bang = if i.severity == Severity::Error { "! " } else { "" };
            format!("{bang}{}{at}", i.message)
        })
        .collect()
}

fn file_lines(files: &[FileWrite]) -> Vec<String> {
    files
        .iter()
        .map(|f| {
            let st = match f.status {
                WriteStatus::Planned => "would write",
                WriteStatus::Written => "wrote",
                WriteStatus::Skipped => "skipped",
                WriteStatus::Failed => "! failed",
            };
            format!("{st} {}", f.path)
        })
        .collect()
}

pub fn mark_for(issues: &[Issue], cancelled: bool) -> &'static str {
    if cancelled {
        "SKIP"
    } else if count_at_least(issues, Severity::Error) > 0 {
        "FAIL"
    } else if count_at_least(issues, Severity::Warn) > 0 {
        "WARN"
    } else {
        "OK  "
    }
}

pub fn write_report<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    crate::utils::paths::write_atomic(path, json.as_bytes()).map_err(|e| format!("Cannot write report {}: {e}", path.display()))
}

/// Report file, exit code from the issues.
fn finish<T: Serialize>(common: &Common, out: &Out, value: &T, issues: &[Issue], elapsed_ms: u64) -> i32 {
    if let Some(p) = &common.report {
        if let Err(e) = write_report(p, value) {
            eprintln!("{e}");
            return 1;
        }
        if !out.quiet {
            println!("Report written to {}", p.display());
        }
    }
    let failed = count_at_least(issues, common.fail_on.severity()) > 0;
    out.finish(!failed, elapsed_ms);
    i32::from(failed)
}

fn write_step(title: &str, r: &WriteReport, common: &Common) -> i32 {
    let out = Out { quiet: common.quiet };
    let verb = if r.dry_run { "would write" } else { "wrote" };
    let mut details = issue_lines(&r.issues);
    details.extend(r.notes.iter().cloned());
    details.extend(file_lines(&r.files));
    out.step(mark_for(&r.issues, r.cancelled), title, &format!("{verb} {} file(s)", r.files.iter().filter(|f| f.kind != "backup").count()), &details);
    finish(common, &out, r, &r.issues, r.elapsed_ms)
}

fn paths(v: &[PathBuf]) -> Vec<String> {
    v.iter().map(|p| p.to_string_lossy().into_owned()).collect()
}

/// Inputs must exist: a typo is a usage error (2), not an empty run.
fn require(inputs: &[PathBuf]) -> Result<(), i32> {
    if inputs.is_empty() {
        eprintln!("No input given");
        return Err(2);
    }
    if let Some(p) = inputs.iter().find(|p| !p.exists()) {
        eprintln!("Not found: {}", p.display());
        return Err(2);
    }
    Ok(())
}

fn quiet_run<T>(f: impl FnOnce(&RunCtx) -> T) -> T {
    crate::core::quiet_ctx(f)
}

// ---------------------------------------------------------------- commands

pub fn run(args: &[String]) -> i32 {
    attach_console();
    let cli = match Cli::try_parse_from(args) {
        Ok(c) => c,
        Err(e) => {
            let code = if e.use_stderr() { 2 } else { 0 };
            let _ = e.print();
            return code;
        }
    };
    match cli.command {
        Command::Pack(a) => pack(a),
        Command::Slice(a) => slice_cmd(a),
        Command::Flipbook(a) => flipbook_cmd(a),
        Command::Bleed(a) => {
            if let Err(c) = require(&a.input) {
                return c;
            }
            let output = match a.output.mode() {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("{e}");
                    return 2;
                }
            };
            Out { quiet: a.common.quiet }.title("bleed", &a.input[0]);
            let r = quiet_run(|ctx| palette::bleed_files(&BleedOptions { paths: paths(&a.input), output, dry_run: a.dry_run }, ctx));
            write_step("Alpha bleed", &r, &a.common)
        }
        Command::Palette(p) => palette_cmd(p),
        Command::Upscale(a) => {
            if let Err(c) = require(&a.input) {
                return c;
            }
            let output = match a.output.mode() {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("{e}");
                    return 2;
                }
            };
            Out { quiet: a.common.quiet }.title("upscale", &a.input[0]);
            let r = quiet_run(|ctx| palette::upscale(&UpscaleOptions { paths: paths(&a.input), factor: a.factor, output, dry_run: a.dry_run }, ctx));
            write_step(&format!("Upscale ×{}", a.factor), &r, &a.common)
        }
        Command::Normal(a) => {
            if let Err(c) = require(&a.input) {
                return c;
            }
            let output = match a.output.mode() {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("{e}");
                    return 2;
                }
            };
            let settings = NormalSettings {
                source: if a.source == SourceArg::AlphaBevel { HeightSource::AlphaBevel } else { HeightSource::Luminance },
                strength: a.strength,
                blur: a.blur,
                bevel: a.bevel,
                convention: if a.convention == ConventionArg::Directx { Convention::Directx } else { Convention::Opengl },
                invert: false,
            };
            Out { quiet: a.common.quiet }.title("normal", &a.input[0]);
            let (r, _) = quiet_run(|ctx| normal2d::run(&NormalOptions { paths: paths(&a.input), settings, output, dry_run: a.dry_run }, ctx));
            write_step("Normal maps", &r, &a.common)
        }
    }
}

fn pack_targets(t: &[TargetArg]) -> Vec<Target> {
    if t.is_empty() || t.contains(&TargetArg::All) {
        return Vec::new();
    }
    t.iter()
        .map(|t| match t {
            TargetArg::JsonHash => Target::JsonHash,
            TargetArg::JsonArray => Target::JsonArray,
            TargetArg::Godot => Target::Godot,
            TargetArg::Unity => Target::Unity,
            _ => Target::Spine,
        })
        .collect()
}

fn print_pack(r: &PackReport, quiet: bool) {
    let out = Out { quiet };
    let mut details = issue_lines(&r.issues);
    details.extend(r.pages.iter().map(|p| format!("page {}: {}×{}, {} sprite(s), {:.0}% filled", p.index, p.width, p.height, p.sprites, p.fill * 100.0)));
    details.extend(file_lines(&r.files));
    out.step(mark_for(&r.issues, r.cancelled), "Pack", &summary(r), &details);
}

fn pack(a: PackArgs) -> i32 {
    let project = match Project::load(&a.project) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let out = Out { quiet: a.common.quiet };
    out.title("pack", &a.project);
    let options = PackOptions { dry_run: a.dry_run, targets: pack_targets(&a.target) };
    let cache = Arc::new(SourceCache::default());
    let (r, _) = quiet_run(|ctx| run_pack(&project, &options, &cache, ctx));
    print_pack(&r, a.common.quiet);
    let first = finish(&a.common, &out, &r, &r.issues, r.elapsed_ms);
    if !a.watch {
        return first;
    }

    let action = WatchAction::PackProject { project: project.clone(), project_path: Some(a.project.to_string_lossy().into_owned()) };
    let mut config = action.default_config();
    config.ignore.extend(a.common.report.iter().cloned());
    let job = PackWatch::new(project, Some(a.project.clone()), cache);
    let common = a.common.clone();
    let never = AtomicBool::new(false);
    let watch = Watcher::start(1, config, false, |_| {}, move |changed| {
        let (s, report) = job.run(changed, &never);
        match (&report, &s.error) {
            (Some(r), _) => {
                println!("— {} change(s): re-packed", changed.len());
                print_pack(r, common.quiet);
                let _ = finish(&common, &Out { quiet: common.quiet }, r, &r.issues, r.elapsed_ms);
            }
            (None, Some(e)) => eprintln!("! {e}"),
            _ => {}
        }
        s
    });
    let _watch = match watch {
        Ok(w) => w,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    println!("Watching the sources of {} — Ctrl+C to stop", a.project.display());
    let (tx, rx) = mpsc::channel();
    if let Err(e) = ctrlc::set_handler(move || {
        let _ = tx.send(());
    }) {
        eprintln!("{e}");
        return 1;
    }
    let _ = rx.recv();
    0
}

fn pair(s: &str, sep: char) -> Option<[u32; 2]> {
    let (a, b) = s.split_once(sep)?;
    Some([a.trim().parse().ok()?, b.trim().parse().ok()?])
}

fn slice_cmd(a: SliceArgs) -> i32 {
    if let Err(c) = require(std::slice::from_ref(&a.sheet)) {
        return c;
    }
    let options = match (&a.grid, a.auto) {
        (Some(g), false) => match (pair(&g.to_lowercase(), 'x'), pair(&a.offset, ','), pair(&a.spacing, ',')) {
            (Some([w, h]), Some(offset), Some(spacing)) => SliceOptions::Grid { cell_w: w, cell_h: h, offset, spacing, keep_empty: false },
            _ => {
                eprintln!("--grid is WxH (e.g. 32x32); --offset and --spacing are X,Y");
                return 2;
            }
        },
        (None, true) => SliceOptions::Auto { merge: a.merge, alpha_threshold: 0, min_pixels: 4 },
        _ => {
            eprintln!("Give --grid WxH or --auto");
            return 2;
        }
    };
    let out = Out { quiet: a.common.quiet };
    out.title("slice", &a.sheet);
    let res = match slice::slice_file(&a.sheet, &options) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}: {e}", a.sheet.display());
            return 2;
        }
    };
    let base = crate::core::sources::file_stem(&a.sheet);
    let names = slice::names(&a.names, &base, res.rects.len());
    let rects: Vec<NamedRect> = res.rects.iter().zip(names).map(|(r, n)| NamedRect { name: n, rect: *r, pivot: None, nine_slice: None }).collect();
    let output = match a.meta {
        None => SliceOutput::Sprites,
        Some(MetaArg::JsonHash) => SliceOutput::JsonHash,
        Some(MetaArg::Unity) => SliceOutput::UnityMeta,
    };
    if output == SliceOutput::Sprites && a.out.is_none() {
        eprintln!("Give --out <folder> for the sprites (or --meta json-hash|unity)");
        return 2;
    }
    let r = quiet_run(|ctx| slice::apply(&ApplySlice { path: a.sheet.to_string_lossy().into(), rects, output, out_dir: a.out.as_ref().map(|o| o.to_string_lossy().into_owned()).unwrap_or_default(), dry_run: a.dry_run }, ctx));
    write_step(&format!("Slice ({} rect(s) in {}×{})", res.rects.len(), res.width, res.height), &r, &a.common)
}

fn flipbook_cmd(a: FlipbookArgs) -> i32 {
    if let Err(c) = require(std::slice::from_ref(&a.folder)) {
        return c;
    }
    let format = if crate::utils::paths::ext_lower(&a.out) == "tga" { ImageKind::Tga } else { ImageKind::Png };
    let o = FlipbookOptions { paths: vec![a.folder.to_string_lossy().into()], from: a.from, to: a.to, step: a.step, cols: a.cols, rows: a.rows, cell: a.cell, max_size: a.max_size, fps: a.fps, format, out: a.out.to_string_lossy().into(), json: a.json, dry_run: a.dry_run };
    let out = Out { quiet: a.common.quiet };
    out.title("flipbook", &a.folder);
    let (r, _) = quiet_run(|ctx| flipbook::build(&o, ctx));
    let mut details = issue_lines(&r.write.issues);
    details.extend(r.write.notes.iter().cloned());
    details.extend(file_lines(&r.write.files));
    let verb = if r.write.dry_run { "would write" } else { "wrote" };
    out.step(mark_for(&r.write.issues, r.write.cancelled), "Flipbook", &format!("{} frame(s) on {}×{} cells of {}×{} ({}×{} px); {verb} {} file(s)", r.frames, r.cols, r.rows, r.cell[0], r.cell[1], r.width, r.height, r.write.files.len()), &details);
    finish(&a.common, &out, &r, &r.write.issues, r.write.elapsed_ms)
}

fn palette_cmd(p: PaletteCmd) -> i32 {
    match p {
        PaletteCmd::Extract { input, out: file, sort, common } => {
            if let Err(c) = require(&input) {
                return c;
            }
            let out = Out { quiet: common.quiet };
            out.title("palette extract", &input[0]);
            let started = std::time::Instant::now();
            let r = palette::extract(&paths(&input), if sort == SortArg::Hue { PaletteSort::Hue } else { PaletteSort::Frequency });
            let colors: Vec<[u8; 4]> = r.colors.iter().filter_map(|c| palette::parse_hex(&c.hex)).collect();
            let mut issues = r.issues.clone();
            match palette::palette_text(&colors, &crate::utils::paths::ext_lower(&file), &crate::core::sources::file_stem(&file)) {
                Ok(t) => {
                    if let Err(e) = crate::utils::paths::write_atomic(&file, t.as_bytes()) {
                        issues.push(Issue::error("write-failed", format!("{}: {e}", file.display())));
                    }
                }
                Err(e) => {
                    eprintln!("{e}");
                    return 2;
                }
            }
            let mut details = issue_lines(&issues);
            details.extend(r.colors.iter().map(|c| format!("{}  {}", c.hex, c.count)));
            out.step(mark_for(&issues, false), "Palette", &format!("{} colour(s) from {} file(s) → {}", r.colors.len(), r.files, file.display()), &details);
            finish(&common, &out, &r, &issues, started.elapsed().as_millis() as u64)
        }
        PaletteCmd::Remap { input, palette: pal, output, dry_run, common } => {
            if let Err(c) = require(&input).and_then(|_| require(std::slice::from_ref(&pal))) {
                return c;
            }
            let output = match output.mode() {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("{e}");
                    return 2;
                }
            };
            if let Err(e) = palette::read_palette(&pal) {
                eprintln!("{}: {e}", pal.display());
                return 2;
            }
            Out { quiet: common.quiet }.title("palette remap", &input[0]);
            let r = quiet_run(|ctx| palette::remap(&RemapOptions { paths: paths(&input), palette: PaletteSource::File { path: pal.to_string_lossy().into() }, output, dry_run }, ctx));
            write_step("Palette remap", &r, &common)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::sources::test_support::{shape, write_png};

    fn args(list: &[&str]) -> Vec<String> {
        std::iter::once("coral").chain(list.iter().copied()).map(String::from).collect()
    }

    #[test]
    fn parses_flags() {
        assert!(!wants_cli(&args(&[])));
        assert!(!wants_cli(&args(&["D:/hero.coral.json"])), "Open with → window");
        assert!(["pack", "slice", "flipbook", "bleed", "palette", "upscale", "normal", "--help", "--version"].iter().all(|c| wants_cli(&args(&[c]))));

        let c = Cli::try_parse_from(args(&["pack", "hero.coral.json", "--target", "godot", "--target", "unity", "--dry-run", "--report", "r.json", "--watch", "--fail-on", "warn", "--quiet"])).unwrap();
        let Command::Pack(p) = c.command else { panic!() };
        assert_eq!(pack_targets(&p.target), [Target::Godot, Target::Unity]);
        assert!(p.dry_run && p.watch && p.common.quiet && p.common.fail_on == FailOn::Warn);
        assert!(pack_targets(&[TargetArg::All]).is_empty());

        let s = Cli::try_parse_from(args(&["slice", "s.png", "--grid", "32x16", "--offset", "1,2", "--out", "o"])).unwrap();
        assert!(matches!(s.command, Command::Slice(SliceArgs { ref grid, .. }) if grid.as_deref() == Some("32x16")));
        assert!(Cli::try_parse_from(args(&["slice", "s.png", "--grid", "8x8", "--auto"])).is_err(), "grid xor auto");
        assert!(Cli::try_parse_from(args(&["bleed", "a.png", "--out", "o", "--in-place"])).is_err(), "out xor in-place");
        assert!(Cli::try_parse_from(args(&["upscale", "a.png", "--factor", "9", "--out", "o"])).is_err(), "factor 2..8");
        assert!(Cli::try_parse_from(args(&["pack"])).is_err(), "project required");
        assert!(Cli::try_parse_from(args(&["pack", "p", "--target", "nope"])).is_err());
        assert!(Cli::try_parse_from(args(&["pack", "p", "--report"])).is_err(), "value missing");
        assert!(Cli::try_parse_from(args(&["palette", "remap", "a.png", "--palette", "p.hex", "--out", "o"])).is_ok());
        assert!(Cli::try_parse_from(args(&["normal", "a.png", "--convention", "directx", "--source", "alpha-bevel", "--out", "o"])).is_ok());
        assert_eq!(pair("32x16", 'x'), Some([32, 16]));
    }

    #[test]
    fn runs_pack() {
        let d = tempfile::tempdir().unwrap();
        for i in 0..3 {
            write_png(&d.path().join(format!("art/hero_idle_{i}.png")), &shape(32, 48, 2 + i, 4, 20, 40, [200, 50, 50, 255]));
        }
        let project = d.path().join("hero.coral.json");
        std::fs::write(&project, r#"{ "format": "coral.project", "version": 1, "name": "hero",
            "sources": [{ "kind": "folder", "path": "art" }],
            "exports": [{ "target": "jsonHash", "out": "build" }, { "target": "spine", "out": "build/spine" }] }"#).unwrap();
        let report = d.path().join("r.json");
        let p = project.to_string_lossy().into_owned();
        assert_eq!(run(&args(&["pack", &p, "--report", &report.to_string_lossy()])), 0);
        assert!(d.path().join("build/hero.png").exists() && d.path().join("build/hero.json").exists() && d.path().join("build/spine/hero.atlas").exists());
        let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&report).unwrap()).unwrap();
        assert_eq!(json["sprites"].as_array().unwrap().len(), 3);
        assert_eq!(json["animations"][0]["name"], "hero_idle");

        // Only one target, dry run: nothing new is written.
        std::fs::remove_dir_all(d.path().join("build")).unwrap();
        assert_eq!(run(&args(&["pack", &p, "--target", "spine", "--dry-run"])), 0);
        assert!(!d.path().join("build").exists());

        // Too small an atlas: exit 1; a broken project file or a missing one: exit 2.
        std::fs::write(&project, r#"{ "name": "hero", "sources": [{ "kind": "folder", "path": "art" }], "pack": { "maxSize": 16, "multiPage": false }, "exports": [{ "target": "jsonHash", "out": "build" }] }"#).unwrap();
        assert_eq!(run(&args(&["pack", &p])), 1);
        std::fs::write(&project, "{ not json").unwrap();
        assert_eq!(run(&args(&["pack", &p])), 2);
        assert_eq!(run(&args(&["pack", &d.path().join("missing.coral.json").to_string_lossy()])), 2);
    }

    #[test]
    fn runs_tools() {
        let d = tempfile::tempdir().unwrap();
        let mut sheet = image::RgbaImage::new(32, 16);
        image::imageops::replace(&mut sheet, &shape(16, 16, 2, 2, 10, 10, [9, 9, 9, 255]), 0, 0);
        image::imageops::replace(&mut sheet, &shape(16, 16, 2, 2, 10, 10, [9, 9, 9, 255]), 16, 0);
        let s = d.path().join("sheet.png");
        sheet.save(&s).unwrap();
        let s = s.to_string_lossy().into_owned();
        let out = d.path().join("out").to_string_lossy().into_owned();
        assert_eq!(run(&args(&["slice", &s, "--grid", "16x16", "--out", &out])), 0);
        assert!(d.path().join("out/sheet_0.png").exists() && d.path().join("out/sheet_1.png").exists());
        assert_eq!(run(&args(&["slice", &s, "--auto", "--meta", "unity"])), 0);
        assert!(d.path().join("sheet.png.meta").exists());
        assert_eq!(run(&args(&["slice", &s])), 2, "grid or auto required");
        assert_eq!(run(&args(&["upscale", &s, "--factor", "2", "--out", &out])), 0);
        assert!(d.path().join("out/sheet_x2.png").exists());
        let hex = d.path().join("p.hex").to_string_lossy().into_owned();
        assert_eq!(run(&args(&["palette", "extract", &s, "--out", &hex])), 0);
        assert_eq!(std::fs::read_to_string(&hex).unwrap(), "090909\n");
        assert_eq!(run(&args(&["palette", "remap", &s, "--palette", &hex, "--out", &out])), 0);
        assert_eq!(run(&args(&["normal", &s, "--out", &out])), 0);
        assert!(d.path().join("out/sheet_n.png").exists());
        assert_eq!(run(&args(&["bleed", &s, "--in-place"])), 0);
        assert!(d.path().join("sheet.png.bak").exists());
        assert_eq!(run(&args(&["bleed", &s])), 2, "--out or --in-place");
        assert_eq!(run(&args(&["normal", &d.path().join("nope.png").to_string_lossy(), "--out", &out])), 2);
        let fb = d.path().join("fx.png").to_string_lossy().into_owned();
        assert_eq!(run(&args(&["flipbook", &out, "--out", &fb, "--json", "--cols", "2"])), 0);
        assert!(d.path().join("fx.json").exists());
    }
}

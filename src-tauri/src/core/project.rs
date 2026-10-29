//! `*.coral.json` (`format: "coral.project"`, `version: 1`), see docs/FORMATS.md.
//!
//! In memory every path is absolute: `load` resolves them against the project file's folder and
//! `to_json` writes them relative to the file again, so a project moves with its repository.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::image_io::ImageKind;
use super::natural;
use super::CoreError;
use crate::utils::paths::{relativize, resolve};

pub const FORMAT: &str = "coral.project";
pub const VERSION: u32 = 1;
pub const EXTENSION: &str = ".coral.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Project {
    pub format: String,
    pub version: u32,
    pub name: String,
    pub sources: Vec<Source>,
    /// Per-sprite overrides, key = sprite name.
    pub sprites: BTreeMap<String, SpriteOverride>,
    /// Hand-made or edited animations; they replace automatic ones of the same name.
    pub animations: Vec<Animation>,
    /// Boxes per sprite, in source (untrimmed) pixels, origin top-left.
    pub hitboxes: BTreeMap<String, Vec<Hitbox>>,
    pub pack: PackSettings,
    pub exports: Vec<ExportTarget>,
    /// Saved palette swap tables (Pixel module).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub swaps: Vec<SwapTable>,
    pub watch: WatchSettings,
}

impl Default for Project {
    fn default() -> Self {
        Project {
            format: FORMAT.into(),
            version: VERSION,
            name: "atlas".into(),
            sources: Vec::new(),
            sprites: BTreeMap::new(),
            animations: Vec::new(),
            hitboxes: BTreeMap::new(),
            pack: PackSettings::default(),
            exports: Vec::new(),
            swaps: Vec::new(),
            watch: WatchSettings::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Source {
    Folder {
        path: String,
        #[serde(default = "yes")]
        recursive: bool,
        #[serde(default = "default_include")]
        include: Vec<String>,
    },
    Aseprite {
        path: String,
        #[serde(default)]
        layers: LayerSelection,
    },
    Psd {
        path: String,
        #[serde(default)]
        mode: PsdMode,
    },
    Gif {
        path: String,
    },
}

fn yes() -> bool {
    true
}
fn default_include() -> Vec<String> {
    vec!["*.png".into(), "*.tga".into()]
}

impl Source {
    pub fn path(&self) -> &str {
        match self {
            Source::Folder { path, .. } | Source::Aseprite { path, .. } | Source::Psd { path, .. } | Source::Gif { path } => path,
        }
    }
    fn path_mut(&mut self) -> &mut String {
        match self {
            Source::Folder { path, .. } | Source::Aseprite { path, .. } | Source::Psd { path, .. } | Source::Gif { path } => path,
        }
    }

    /// The source a dropped path becomes: a folder, or a file by its extension.
    pub fn for_path(path: &Path) -> Option<Source> {
        let p = path.to_string_lossy().into_owned();
        if path.is_dir() {
            return Some(Source::Folder { path: p, recursive: true, include: default_include() });
        }
        match crate::utils::paths::ext_lower(path).as_str() {
            "aseprite" | "ase" => Some(Source::Aseprite { path: p, layers: LayerSelection::default() }),
            "psd" => Some(Source::Psd { path: p, mode: PsdMode::Layers }),
            "gif" => Some(Source::Gif { path: p }),
            _ => None,
        }
    }
}

/// `"visible"` (all visible layers merged), `"all"` or a list of layer names (one sprite per layer).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LayerSelection {
    Mode(LayerMode),
    Names(Vec<String>),
}

impl Default for LayerSelection {
    fn default() -> Self {
        LayerSelection::Mode(LayerMode::Visible)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayerMode {
    /// Visible layers merged into one picture per frame.
    Visible,
    /// Every layer, hidden ones too, merged.
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PsdMode {
    /// One sprite per visible pixel layer.
    #[default]
    Layers,
    /// The flattened image as one sprite.
    Composite,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SpriteOverride {
    /// Normalised in the untrimmed sprite, origin top-left: `[0.5, 1.0]` = bottom centre.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pivot: Option<[f32; 2]>,
    /// 9-slice borders in px: `[left, top, right, bottom]`.
    pub nine_slice: Option<[u32; 4]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoopMode {
    #[default]
    Loop,
    Once,
    Pingpong,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Animation {
    pub name: String,
    pub frames: Vec<String>,
    #[serde(default = "default_fps")]
    pub fps: f32,
    #[serde(default, rename = "loop")]
    pub loop_mode: LoopMode,
    /// Per-frame milliseconds (from Aseprite or edited); `null` = every frame `1000 / fps`.
    #[serde(default)]
    pub durations_ms: Option<Vec<u32>>,
}

pub fn default_fps() -> f32 {
    12.0
}

impl Animation {
    pub fn duration_ms(&self, i: usize) -> f32 {
        self.durations_ms.as_ref().and_then(|d| d.get(i)).map(|&d| d as f32).unwrap_or(1000.0 / self.fps.max(0.01))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hitbox {
    /// `hitbox`, `hurtbox` or `custom:<name>`.
    pub kind: String,
    /// `[x, y, w, h]` in the untrimmed sprite, origin top-left.
    pub rect: [i32; 4],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrimMode {
    None,
    #[default]
    Alpha,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Heuristic {
    #[default]
    BestShortSideFit,
    BestLongSideFit,
    BestAreaFit,
    BottomLeft,
    ContactPoint,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PackSettings {
    pub max_size: u32,
    pub power_of_two: bool,
    pub square: bool,
    pub allow_rotation: bool,
    /// Empty pixels between sprites.
    pub padding: u32,
    /// Edge pixels repeated outward around each sprite (stops bleeding with filtering).
    pub extrude: u32,
    pub trim: TrimMode,
    /// Alpha at or below this counts as empty when trimming.
    pub trim_threshold: u8,
    pub alpha_bleed: bool,
    pub dedupe: bool,
    pub premultiply: bool,
    pub heuristic: Heuristic,
    pub multi_page: bool,
    pub image_format: ImageKind,
    /// Also pack a normal-map atlas with the same rects (`<name>_n.png`).
    pub normal_maps: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normal: Option<super::normal2d::NormalSettings>,
}

impl Default for PackSettings {
    fn default() -> Self {
        PackSettings {
            max_size: 2048,
            power_of_two: true,
            square: false,
            allow_rotation: false,
            padding: 2,
            extrude: 1,
            trim: TrimMode::Alpha,
            trim_threshold: 0,
            alpha_bleed: true,
            dedupe: true,
            premultiply: false,
            heuristic: Heuristic::BestShortSideFit,
            multi_page: true,
            image_format: ImageKind::Png,
            normal_maps: false,
            normal: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Target {
    JsonHash,
    JsonArray,
    Godot,
    Unity,
    Spine,
}

impl Target {
    pub const ALL: [Target; 5] = [Target::JsonHash, Target::JsonArray, Target::Godot, Target::Unity, Target::Spine];
    pub fn label(self) -> &'static str {
        match self {
            Target::JsonHash => "JSON Hash",
            Target::JsonArray => "JSON Array",
            Target::Godot => "Godot SpriteFrames",
            Target::Unity => "Unity .meta",
            Target::Spine => "Spine / libGDX atlas",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterMode {
    /// Nearest neighbour (pixel art).
    #[default]
    Point,
    Bilinear,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportTarget {
    pub target: Target,
    /// Folder the files go to.
    pub out: String,
    /// Godot: folder of `project.godot` (else searched upward from `out`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub godot_project: Option<String>,
    /// Godot: also one `AtlasTexture` `.tres` per sprite (UI sprites).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub atlas_textures: bool,
    /// Unity: pixels per unit (default 100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pixels_per_unit: Option<f32>,
    /// Unity: texture filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<FilterMode>,
    /// JSON Hash: `.paper2dsprites` extension for Unreal's Paper2D importer.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub paper2d: bool,
}

impl ExportTarget {
    pub fn new(target: Target, out: impl Into<String>) -> Self {
        ExportTarget { target, out: out.into(), godot_project: None, atlas_textures: false, pixels_per_unit: None, filter: None, paper2d: false }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WatchSettings {
    pub auto_start: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapTable {
    pub variant: String,
    /// `["#rrggbb(aa)", "#rrggbb(aa)"]` pairs, from → to.
    pub map: Vec<[String; 2]>,
}

impl Project {
    pub fn new(name: &str) -> Self {
        Project { name: name.into(), ..Default::default() }
    }

    /// Read, migrate and resolve (absolute paths).
    pub fn load(path: &Path) -> Result<Project, CoreError> {
        let text = std::fs::read_to_string(path).map_err(|e| CoreError::io(path, e))?;
        let mut p = Self::from_json(&text).map_err(|e| CoreError::Input(format!("{}: {e}", path.display())))?;
        p.resolve(path.parent().unwrap_or(Path::new(".")));
        Ok(p)
    }

    /// Parse + migrate; paths stay as written.
    pub fn from_json(text: &str) -> Result<Project, String> {
        let mut v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("not valid JSON ({e})"))?;
        migrate(&mut v)?;
        serde_json::from_value(v).map_err(|e| format!("not a Coral project ({e})"))
    }

    fn map_paths(&mut self, f: &dyn Fn(&str) -> String) {
        for s in &mut self.sources {
            let p = s.path_mut();
            *p = f(p);
        }
        for e in &mut self.exports {
            e.out = f(&e.out);
            if let Some(g) = &mut e.godot_project {
                *g = f(g);
            }
        }
    }

    pub fn resolve(&mut self, base: &Path) {
        self.map_paths(&|p| if p.is_empty() { String::new() } else { resolve(base, p).to_string_lossy().into_owned() });
    }

    /// JSON as saved next to `file`: relative paths, 2-space indent, animations in natural order.
    pub fn to_json(&self, file: &Path) -> String {
        let base = file.parent().unwrap_or(Path::new("."));
        let mut p = self.clone();
        p.format = FORMAT.into();
        p.version = VERSION;
        p.map_paths(&|s| if s.is_empty() || !Path::new(s).is_absolute() { s.replace('\\', "/") } else { relativize(base, Path::new(s)) });
        p.animations.sort_by(|a, b| natural::cmp(&a.name, &b.name));
        let mut s = serde_json::to_string_pretty(&p).expect("project serializes");
        s.push('\n');
        s
    }

    pub fn save(&self, file: &Path) -> Result<(), CoreError> {
        crate::utils::paths::write_atomic(file, self.to_json(file).as_bytes()).map_err(|e| CoreError::io(file, e))
    }
}

/// Bring an older document up to `VERSION`; never refuse silently.
fn migrate(v: &mut serde_json::Value) -> Result<(), String> {
    let obj = v.as_object_mut().ok_or("not a JSON object")?;
    if let Some(f) = obj.get("format").and_then(|f| f.as_str()) {
        if f != FORMAT {
            return Err(format!("format is \"{f}\", expected \"{FORMAT}\""));
        }
    }
    let version = obj.get("version").and_then(|v| v.as_u64()).unwrap_or(0);
    if version > VERSION as u64 {
        return Err(format!("version {version} was written by a newer Coral (this one reads up to {VERSION})"));
    }
    // Version 0 (drafts without a version): `sources` could be plain folder strings.
    if version == 0 {
        if let Some(serde_json::Value::Array(sources)) = obj.get_mut("sources") {
            for s in sources.iter_mut() {
                if let serde_json::Value::String(p) = s {
                    *s = serde_json::json!({ "kind": "folder", "path": p });
                }
            }
        }
    }
    obj.insert("format".into(), FORMAT.into());
    obj.insert("version".into(), VERSION.into());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) const SPEC_EXAMPLE: &str = r#"{
      "format": "coral.project", "version": 1, "name": "hero",
      "sources": [
        { "kind": "folder", "path": "art/hero", "recursive": true, "include": ["*.png"] },
        { "kind": "aseprite", "path": "art/hero_fx.aseprite", "layers": "visible" },
        { "kind": "aseprite", "path": "art/hero_fx2.aseprite", "layers": ["Body", "FX"] },
        { "kind": "psd", "path": "art/ui.psd", "mode": "layers" }
      ],
      "sprites": { "hero_idle_0": { "pivot": [0.5, 1.0], "nineSlice": null } },
      "animations": [ { "name": "idle", "frames": ["hero_idle_0", "hero_idle_1"], "fps": 8, "loop": "loop", "durationsMs": null } ],
      "hitboxes": { "hero_attack_3": [ { "kind": "hitbox", "rect": [12, 4, 20, 10], "name": "sword" } ] },
      "pack": { "maxSize": 2048, "powerOfTwo": true, "square": false, "allowRotation": false, "padding": 2, "extrude": 1,
                "trim": "alpha", "trimThreshold": 0, "alphaBleed": true, "dedupe": true, "premultiply": false,
                "heuristic": "bestShortSideFit", "multiPage": true },
      "exports": [
        { "target": "jsonHash", "out": "build/hero" },
        { "target": "godot", "out": "../game/assets/hero", "godotProject": "../game" },
        { "target": "unity", "out": "../UnityGame/Assets/Art/hero" },
        { "target": "spine", "out": "build/spine/hero" }
      ],
      "watch": { "autoStart": false },
      "someFutureField": 1
    }"#;

    #[test]
    fn reads_the_spec_example() {
        let p = Project::from_json(SPEC_EXAMPLE).unwrap();
        assert_eq!(p.name, "hero");
        assert_eq!(p.sources.len(), 4);
        assert!(matches!(&p.sources[2], Source::Aseprite { layers: LayerSelection::Names(n), .. } if n == &["Body", "FX"]));
        assert!(matches!(&p.sources[1], Source::Aseprite { layers: LayerSelection::Mode(LayerMode::Visible), .. }));
        assert_eq!(p.sprites["hero_idle_0"].pivot, Some([0.5, 1.0]));
        assert_eq!(p.animations[0].fps, 8.0);
        assert_eq!(p.hitboxes["hero_attack_3"][0].rect, [12, 4, 20, 10]);
        assert_eq!(p.exports[1].godot_project.as_deref(), Some("../game"));
        assert_eq!(p.pack.image_format, ImageKind::Png);
    }

    #[test]
    fn round_trip_keeps_paths_relative() {
        let d = tempfile::tempdir().unwrap();
        let file = d.path().join("proj").join("hero.coral.json");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, SPEC_EXAMPLE).unwrap();
        let p = Project::load(&file).unwrap();
        assert!(Path::new(p.sources[0].path()).is_absolute());
        assert!(p.exports[1].out.replace('\\', "/").ends_with("game/assets/hero"));
        p.save(&file).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("\"path\": \"art/hero\"") && text.contains("\"out\": \"../game/assets/hero\""), "{text}");
        assert!(!text.contains("someFutureField"));
        assert_eq!(Project::load(&file).unwrap(), p, "round trip");
    }

    #[test]
    fn migrates_and_refuses_loudly() {
        let p = Project::from_json(r#"{ "name": "old", "sources": ["art"] }"#).unwrap();
        assert_eq!((p.version, p.sources[0].path()), (1, "art"));
        assert!(Project::from_json(r#"{ "format": "coral.project", "version": 9 }"#).unwrap_err().contains("newer Coral"));
        assert!(Project::from_json(r#"{ "format": "other.thing" }"#).is_err());
        assert!(Project::from_json("[]").is_err());
    }
}

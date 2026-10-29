//! Tauri IPC handlers. Each submodule wraps `core` engines; handlers stay thin and run blocking work
//! on the async runtime's blocking pool, as jobs (one per kind, cancellable, `coral://progress`).

pub mod app_cmd;
pub mod project_cmd;
pub mod tools_cmd;
pub mod watch_cmd;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Runtime};

use crate::core::pipeline::Packed;
use crate::core::sources::SourceCache;
use crate::core::RunCtx;
use crate::utils::jobs::{progress_emitter, JobRegistry};

/// Event name; keep in sync with `EVENTS.progress` in `src/lib/ipc.ts`.
pub const PROGRESS_EVENT: &str = "coral://progress";

/// Shared between commands and watches: decoded sources and the last previews.
#[derive(Default)]
pub struct CoralState {
    pub cache: Arc<SourceCache>,
    /// PNG bytes by key: `atlas:<page>`, `normal:<page>`, `flipbook`, `normal`.
    pub previews: Mutex<HashMap<String, Vec<u8>>>,
    pub last_pack: Mutex<Option<Packed>>,
}

impl CoralState {
    pub fn set_preview(&self, key: &str, png: Vec<u8>) {
        self.previews.lock().unwrap_or_else(|e| e.into_inner()).insert(key.to_string(), png);
    }
}

/// Run `f` as a job of `kind` on the blocking pool with progress events and cancel.
pub async fn run_job<R: Runtime, T: Send + 'static>(app: &AppHandle<R>, jobs: &JobRegistry, kind: &str, f: impl FnOnce(&RunCtx) -> T + Send + 'static) -> Result<T, String> {
    let job = jobs.start(kind)?;
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let progress = progress_emitter(&handle, PROGRESS_EVENT, job.id);
        let r = f(&RunCtx { progress: &progress, cancel: &job.cancel });
        drop(job);
        r
    })
    .await
    .map_err(|e| e.to_string())
}

/// The JSON the frontend sends (see `src/types/coral.ts`) must decode into the command inputs.
#[cfg(test)]
mod contract_tests {
    use super::watch_cmd::{WatchAction, WatchRequest};
    use crate::core::flipbook::FlipbookOptions;
    use crate::core::normal2d::{Convention, HeightSource, NormalOptions};
    use crate::core::palette::{PaletteSource, RemapOptions, SwapOptions, UpscaleOptions};
    use crate::core::pipeline::PackOptions;
    use crate::core::project::{LayerSelection, Project, Source, Target};
    use crate::core::slice::{ApplySlice, SliceOptions, SliceOutput};
    use crate::core::OutputMode;
    use crate::utils::watch::WatchConfig;
    use serde_json::json;

    /// As `newProject()` + edits in `src/stores/projectStore.ts` build it.
    fn ts_project() -> serde_json::Value {
        json!({
            "format": "coral.project", "version": 1, "name": "hero",
            "sources": [
                { "kind": "folder", "path": "D:/art/hero", "recursive": true, "include": ["*.png"] },
                { "kind": "aseprite", "path": "D:/art/fx.aseprite", "layers": ["Body"] },
                { "kind": "psd", "path": "D:/art/ui.psd", "mode": "composite" },
                { "kind": "gif", "path": "D:/art/blink.gif" }
            ],
            "sprites": { "hero_idle_0": { "pivot": [0.5, 1], "nineSlice": [2, 2, 2, 2] } },
            "animations": [{ "name": "idle", "frames": ["hero_idle_0"], "fps": 8, "loop": "pingpong", "durationsMs": [100] }],
            "hitboxes": { "hero_idle_0": [{ "kind": "custom:shield", "rect": [0, 0, 4, 4] }] },
            "pack": { "maxSize": 1024, "powerOfTwo": false, "square": false, "allowRotation": true, "padding": 0, "extrude": 0,
                      "trim": "none", "trimThreshold": 8, "alphaBleed": false, "dedupe": false, "premultiply": true,
                      "heuristic": "contactPoint", "multiPage": false, "imageFormat": "tga", "normalMaps": true,
                      "normal": { "source": "alphaBevel", "strength": 3, "blur": 1, "bevel": 6, "convention": "directx", "invert": false } },
            "exports": [
                { "target": "jsonHash", "out": "D:/build", "paper2d": true },
                { "target": "jsonArray", "out": "D:/build" },
                { "target": "godot", "out": "D:/game/assets", "godotProject": "D:/game", "atlasTextures": true },
                { "target": "unity", "out": "D:/u/Assets", "pixelsPerUnit": 32, "filter": "point" },
                { "target": "spine", "out": "D:/spine", "filter": "bilinear" }
            ],
            "watch": { "autoStart": false }
        })
    }

    #[test]
    fn project_from_ts() {
        let p: Project = serde_json::from_value(ts_project()).unwrap();
        assert_eq!(p.sources.len(), 4);
        assert!(matches!(&p.sources[1], Source::Aseprite { layers: LayerSelection::Names(n), .. } if n == &["Body"]));
        assert_eq!(p.pack.normal.as_ref().unwrap().convention, Convention::Directx);
        assert_eq!(p.pack.normal.as_ref().unwrap().source, HeightSource::AlphaBevel);
        assert_eq!(p.exports[3].pixels_per_unit, Some(32.0));
        assert!(p.exports[0].paper2d && p.exports[2].atlas_textures);
        let minimal: Project = serde_json::from_value(json!({ "name": "x" })).unwrap();
        assert_eq!((minimal.pack.max_size, minimal.pack.padding, minimal.pack.dedupe), (2048, 2, true));

        let o: PackOptions = serde_json::from_value(json!({ "dryRun": true, "targets": ["godot", "unity"] })).unwrap();
        assert!(o.dry_run && o.targets == [Target::Godot, Target::Unity]);
        assert!(!serde_json::from_value::<PackOptions>(json!({})).unwrap().dry_run);
    }

    #[test]
    fn tool_options_from_ts() {
        let g: SliceOptions = serde_json::from_value(json!({ "kind": "grid", "cellW": 32, "cellH": 32, "offset": [0, 0], "spacing": [1, 1], "keepEmpty": false })).unwrap();
        assert!(matches!(g, SliceOptions::Grid { cell_w: 32, spacing: [1, 1], .. }));
        let a: SliceOptions = serde_json::from_value(json!({ "kind": "auto", "merge": 3 })).unwrap();
        assert!(matches!(a, SliceOptions::Auto { merge: 3, min_pixels: 4, .. }));
        let ap: ApplySlice = serde_json::from_value(json!({ "path": "D:/s.png", "rects": [{ "name": "a", "rect": { "x": 0, "y": 0, "w": 4, "h": 4 }, "pivot": [0.5, 0.5], "nineSlice": null }], "output": "unityMeta", "outDir": "", "dryRun": true })).unwrap();
        assert_eq!(ap.output, SliceOutput::UnityMeta);

        let f: FlipbookOptions = serde_json::from_value(json!({ "paths": ["D:/fx"], "from": 1, "to": 64, "step": 1, "cols": 8, "rows": 8, "cell": 256, "out": "D:/fx.png", "json": true, "dryRun": false })).unwrap();
        assert_eq!((f.cols, f.max_size, f.fps), (8, 4096, 30.0));

        let r: RemapOptions = serde_json::from_value(json!({ "paths": ["D:/a.png"], "palette": { "kind": "colors", "colors": ["#ff0000"] }, "output": { "kind": "folder", "path": "D:/out" }, "dryRun": true })).unwrap();
        assert!(matches!(r.palette, PaletteSource::Colors { .. }) && matches!(r.output, OutputMode::Folder { .. }));
        let s: SwapOptions = serde_json::from_value(json!({ "paths": [], "tables": [{ "variant": "red", "map": [["#000000", "#ff0000"]] }], "output": { "kind": "inPlace" } })).unwrap();
        assert!(s.output.in_place() && !s.dry_run);
        let u: UpscaleOptions = serde_json::from_value(json!({ "paths": ["a"], "factor": 4, "output": { "kind": "folder", "path": "o" } })).unwrap();
        assert_eq!(u.factor, 4);
        let n: NormalOptions = serde_json::from_value(json!({ "paths": ["a"], "settings": { "strength": 1.5 }, "output": { "kind": "folder", "path": "o" } })).unwrap();
        assert_eq!((n.settings.strength, n.settings.bevel), (1.5, 4.0));
    }

    #[test]
    fn watch_request_from_ts() {
        let r: WatchRequest = serde_json::from_value(json!({ "action": { "kind": "packProject", "project": ts_project(), "projectPath": "D:/hero.coral.json" } })).unwrap();
        let WatchAction::PackProject { project, project_path } = &r.action;
        assert_eq!((project.name.as_str(), project_path.as_deref()), ("hero", Some("D:/hero.coral.json")));
        let cfg = r.action.default_config();
        assert_eq!(cfg.paths.len(), 5, "4 sources + the project file");
        assert_eq!(cfg.ignore.len(), 5, "every export folder");
        let c: WatchConfig = serde_json::from_value(json!({ "paths": ["D:/art"], "debounceMs": 250 })).unwrap();
        assert_eq!((c.debounce_ms, c.settle_ms), (250, 300));
    }
}

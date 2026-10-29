//! End-to-end IPC tests: the real command set (`configure`) on Tauri's mock runtime, invoked
//! with the same JSON the frontend stores send (`src/stores/*Store.ts` via `src/lib/ipc.ts`).

use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{json, Value};
use tauri::ipc::{CallbackFn, InvokeBody, InvokeResponseBody};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::{App, Listener, Manager, WebviewWindow, WebviewWindowBuilder};

use crate::commands::PROGRESS_EVENT;
use crate::core::sources::test_support::{shape, write_png};
use crate::utils::jobs::JobRegistry;
use crate::utils::watch::WATCH_EVENT;

pub(crate) struct Harness {
    pub app: App<MockRuntime>,
    webview: WebviewWindow<MockRuntime>,
}

#[allow(dead_code)]
impl Harness {
    pub fn new() -> Self {
        let app = super::configure(mock_builder()).build(mock_context(noop_assets())).expect("build mock app");
        let webview = WebviewWindowBuilder::new(&app, "main", Default::default()).build().expect("main window");
        Harness { app, webview }
    }

    /// `invoke(cmd, args)` exactly as `@tauri-apps/api/core` would send it.
    pub fn invoke(&self, cmd: &str, args: Value) -> Result<Value, Value> {
        self.invoke_body(cmd, args).map(|body| body.deserialize::<Value>().expect("JSON response"))
    }

    /// For commands that answer with an ArrayBuffer (`tauri::ipc::Response`).
    pub fn invoke_raw(&self, cmd: &str, args: Value) -> Result<Vec<u8>, Value> {
        match self.invoke_body(cmd, args)? {
            InvokeResponseBody::Raw(bytes) => Ok(bytes),
            InvokeResponseBody::Json(j) => panic!("{cmd}: expected raw bytes, got JSON {j}"),
        }
    }

    fn invoke_body(&self, cmd: &str, args: Value) -> Result<InvokeResponseBody, Value> {
        let url = if cfg!(windows) { "http://tauri.localhost" } else { "tauri://localhost" };
        get_ipc_response(
            &self.webview,
            InvokeRequest {
                cmd: cmd.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: url.parse().unwrap(),
                body: InvokeBody::Json(args),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_string(),
            },
        )
    }

    pub fn ok(&self, cmd: &str, args: Value) -> Value {
        self.invoke(cmd, args).unwrap_or_else(|e| panic!("{cmd} failed: {e}"))
    }

    /// Collect payloads of a backend event.
    pub fn listen(&self, event: &str) -> mpsc::Receiver<Value> {
        let (tx, rx) = mpsc::channel();
        self.app.listen_any(event, move |e| {
            let _ = tx.send(serde_json::from_str(e.payload()).unwrap_or(Value::Null));
        });
        rx
    }
}

pub(crate) fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// `art/hero_idle_0..2.png` (32×48 with transparent margins) and a project exporting JSON + Spine.
fn hero_dir() -> (tempfile::TempDir, Value) {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..3 {
        write_png(&dir.path().join(format!("art/hero_idle_{i}.png")), &shape(32, 48, 1 + i, 4, 24, 40, [220, 60, 40, 255]));
    }
    let project = json!({
        "format": "coral.project", "version": 1, "name": "hero",
        "sources": [{ "kind": "folder", "path": s(&dir.path().join("art")), "recursive": true, "include": ["*.png"] }],
        "sprites": {}, "animations": [], "hitboxes": {},
        "pack": { "maxSize": 512, "padding": 2, "extrude": 1 },
        "exports": [{ "target": "jsonHash", "out": s(&dir.path().join("build")) }, { "target": "spine", "out": s(&dir.path().join("build")) }],
        "watch": { "autoStart": false }
    });
    (dir, project)
}

#[test]
fn app_info_log_and_settings() {
    let h = Harness::new();
    let info = h.ok("app_info", json!({}));
    assert!(info["version"].as_str().is_some_and(|v| !v.is_empty()));
    assert_eq!(info["debug"], cfg!(debug_assertions));
    assert!(info["abyss"]["managed"].is_boolean());
    assert_eq!(h.ok("ui_log", json!({ "level": "warn", "message": "from the UI" })), Value::Null);
    assert!(h.invoke("open_url", json!({ "url": "file:///C:/x" })).is_err());
    assert!(h.invoke("set_settings", json!({ "settings": [1] })).is_err(), "object only");
    assert!(h.ok("get_settings", json!({})).is_object());
}

#[test]
fn drop_scan_preview_and_export() {
    let h = Harness::new();
    let progress = h.listen(PROGRESS_EVENT);
    let (dir, project) = hero_dir();

    // Drop (dropRouter → projectStore.addPaths).
    let drop = h.ok("sources_for_paths", json!({ "paths": [s(&dir.path().join("art")), s(&dir.path().join("art/hero_idle_0.png")), s(&dir.path().join("notes.txt"))] }));
    assert_eq!(drop["sources"][0]["kind"], "folder");
    assert_eq!(drop["sources"][1]["recursive"], false, "a loose PNG brings its folder");
    assert_eq!(drop["ignored"].as_array().unwrap().len(), 1);

    let scan = h.ok("scan_sources", json!({ "project": project }));
    assert_eq!(scan["sprites"].as_array().unwrap().len(), 3);
    assert_eq!(scan["sprites"][0]["trim"], json!({ "sourceW": 32, "sourceH": 48, "x": 1, "y": 4, "w": 24, "h": 40 }));
    assert_eq!(scan["animations"][0]["name"], "hero_idle");

    let thumb = h.invoke_raw("sprite_thumbnail", json!({ "project": project, "name": "hero_idle_1", "maxSize": 16 })).unwrap();
    assert_eq!(&thumb[1..4], b"PNG");

    let preview = h.ok("pack_preview", json!({ "project": project }));
    assert!(preview["ok"].as_bool().unwrap(), "{preview}");
    assert!(preview["files"].as_array().unwrap().iter().all(|f| f["status"] == "planned"));
    assert!(!dir.path().join("build").exists(), "preview writes nothing");
    let png = h.invoke_raw("preview_png", json!({ "key": "atlas:0" })).unwrap();
    assert_eq!(&png[1..4], b"PNG");
    assert!(h.invoke("preview_png", json!({ "key": "atlas:9" })).is_err());
    assert!(progress.try_iter().any(|p| p["jobId"].as_u64().is_some() && p["total"] == 3));

    let r = h.ok("pack_and_export", json!({ "project": project, "options": { "dryRun": false } }));
    assert!(r["ok"].as_bool().unwrap());
    assert_eq!(r["files"].as_array().unwrap().len(), 3, "png once + json + atlas");
    assert!(dir.path().join("build/hero.json").exists() && dir.path().join("build/hero.atlas").exists());

    // Save → load: relative paths on disk, absolute in memory.
    let file = dir.path().join("hero.coral.json");
    let saved = h.ok("save_project", json!({ "path": s(&dir.path().join("hero")), "project": project }));
    assert!(saved.as_str().unwrap().ends_with("hero.coral.json"));
    assert!(fs::read_to_string(&file).unwrap().contains(r#""path": "art""#));
    let loaded = h.ok("load_project", json!({ "path": s(&file) }));
    assert_eq!(loaded["sprites"].as_array().unwrap().len(), 3);
    assert!(Path::new(loaded["project"]["sources"][0]["path"].as_str().unwrap()).is_absolute());
    assert!(h.invoke("load_project", json!({ "path": s(&dir.path().join("nope.coral.json")) })).is_err());
}

#[test]
fn tools_through_ipc() {
    let h = Harness::new();
    let (dir, _) = hero_dir();
    let img = s(&dir.path().join("art/hero_idle_0.png"));
    let sl = h.ok("slice_sheet", json!({ "path": img, "options": { "kind": "auto", "merge": 2 } }));
    assert_eq!(sl["rects"].as_array().unwrap().len(), 1);
    assert_eq!(sl["suggestedNames"][0], "hero_idle_0_0");
    let out = s(&dir.path().join("out"));
    let up = h.ok("upscale_nearest", json!({ "options": { "paths": [img], "factor": 2, "output": { "kind": "folder", "path": out }, "dryRun": true } }));
    assert_eq!(up["files"][0]["status"], "planned");
    let pal = h.ok("palette_extract", json!({ "paths": [s(&dir.path().join("art"))] }));
    assert_eq!(pal["colors"][0]["hex"], "#dc3c28");
    let n = h.invoke_raw("normal_preview", json!({ "path": img, "settings": { "source": "alphaBevel" } })).unwrap();
    assert_eq!(&n[1..4], b"PNG");
    let fb = h.ok("build_flipbook", json!({ "options": { "paths": [s(&dir.path().join("art"))], "out": s(&dir.path().join("fx.png")), "dryRun": true } }));
    assert_eq!((fb["cols"].as_u64(), fb["rows"].as_u64()), (Some(2), Some(2)));
    assert!(h.invoke_raw("preview_png", json!({ "key": "flipbook" })).is_ok());
    assert!(h.invoke_raw("image_png", json!({ "path": img })).is_ok());
}

#[test]
fn one_job_per_kind_and_cancel() {
    let h = Harness::new();
    let jobs = h.app.state::<JobRegistry>();
    let running = jobs.start("pack").unwrap();
    let (_dir, project) = hero_dir();
    let err = h.invoke("pack_preview", json!({ "project": project })).unwrap_err();
    assert_eq!(err, "Pack is already running");
    assert_eq!(h.ok("cancel_job", json!({ "jobId": running.id })), true);
    assert!(running.cancel.load(std::sync::atomic::Ordering::Relaxed));
    drop(running);
    assert_eq!(h.ok("cancel_job", json!({ "jobId": 999 })), false);
    assert!(h.invoke("pack_preview", json!({ "project": project })).is_ok());
}

#[test]
fn watch_repacks_on_change() {
    let h = Harness::new();
    let status = h.listen(WATCH_EVENT);
    let (dir, project) = hero_dir();
    let request = json!({ "request": {
        "action": { "kind": "packProject", "project": project },
        "config": { "paths": [s(&dir.path().join("art"))], "ignore": [s(&dir.path().join("build"))], "debounceMs": 100, "settleMs": 50 }
    } });
    let id = h.ok("watch_start", request).as_u64().unwrap();

    let idle_after_run = |rx: &mpsc::Receiver<Value>| loop {
        let s = rx.recv_timeout(Duration::from_secs(5)).expect("watch://status within 5 s");
        if s["state"] == "idle" && s["lastRunAt"].is_u64() {
            return s;
        }
    };
    let first = idle_after_run(&status);
    assert_eq!(first["watchId"], id);
    assert!(first["lastSummary"].as_str().unwrap().starts_with("3 sprite(s)"), "{first}");
    assert!(dir.path().join("build/hero.png").exists());

    write_png(&dir.path().join("art/hero_idle_3.png"), &shape(32, 48, 0, 0, 30, 40, [10, 200, 10, 255]));
    let second = idle_after_run(&status);
    assert!(second["lastSummary"].as_str().unwrap().starts_with("4 sprite(s)"), "{second}");
    assert!(second["changedPaths"].as_array().unwrap().iter().any(|p| p.as_str().unwrap().ends_with("hero_idle_3.png")));

    assert_eq!(h.ok("watch_list", json!({})).as_array().unwrap().len(), 1);
    assert_eq!(h.ok("watch_stop", json!({ "watchId": id })), true);
    assert_eq!(h.ok("watch_stop", json!({ "watchId": id })), false);
}

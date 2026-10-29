//! JSON Hash / JSON Array, as TexturePacker writes them (Phaser 3, PixiJS read them), plus a
//! `meta.coral` block with fps, loop modes, frame durations, hitboxes and 9-slice borders.
//! Several pages: one `<name>-<i>.json` per page and `meta.related_multi_packs`.
//! Rotated sprites: `frame.w/h` are the unrotated size, the atlas holds them turned 90° clockwise.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{json, Map, Value};

use super::{page_file, page_images, Output};
use crate::core::pipeline::Packed;
use crate::core::project::{ExportTarget, LoopMode, Project};

fn frame_json(s: &crate::core::pipeline::PackedSprite) -> Value {
    json!({
        "frame": { "x": s.frame.x, "y": s.frame.y, "w": s.frame.w, "h": s.frame.h },
        "rotated": s.rotated,
        "trimmed": s.trim.trimmed(),
        "spriteSourceSize": { "x": s.trim.x, "y": s.trim.y, "w": s.trim.w, "h": s.trim.h },
        "sourceSize": { "w": s.trim.source_w, "h": s.trim.source_h },
        "pivot": { "x": round4(s.pivot[0]), "y": round4(s.pivot[1]) }
    })
}

fn round4(v: f32) -> f64 {
    (v as f64 * 10000.0).round() / 10000.0
}

/// The `meta.coral` block (same on every page).
pub fn coral_meta(project: &Project, packed: &Packed) -> Value {
    let mut fps = Map::new();
    let mut loops = Map::new();
    let mut durations = Map::new();
    for a in &packed.animations {
        fps.insert(a.name.clone(), json!(round4(a.fps)));
        loops.insert(a.name.clone(), json!(match a.loop_mode {
            LoopMode::Loop => "loop",
            LoopMode::Once => "once",
            LoopMode::Pingpong => "pingpong",
        }));
        if let Some(d) = &a.durations_ms {
            durations.insert(a.name.clone(), json!(d));
        }
    }
    let mut hitboxes = Map::new();
    for (name, boxes) in &project.hitboxes {
        if packed.sprites.iter().any(|s| &s.name == name) && !boxes.is_empty() {
            hitboxes.insert(name.clone(), json!(boxes.iter().map(|b| {
                let mut v = json!({ "kind": b.kind, "x": b.rect[0], "y": b.rect[1], "w": b.rect[2], "h": b.rect[3] });
                if let Some(n) = &b.name { v["name"] = json!(n); }
                v
            }).collect::<Vec<_>>()));
        }
    }
    let mut nine = Map::new();
    for s in &packed.sprites {
        if let Some([l, t, r, b]) = s.nine_slice {
            nine.insert(s.name.clone(), json!({ "left": l, "top": t, "right": r, "bottom": b }));
        }
    }
    json!({ "fps": fps, "loop": loops, "durationsMs": durations, "hitboxes": hitboxes, "nineSlice": nine })
}

pub fn document(project: &Project, packed: &Packed, page: usize, array: bool) -> Value {
    let n = packed.pages.len();
    let ext = project.pack.image_format.ext();
    let sprites: Vec<_> = packed.sprites.iter().filter(|s| s.page == page).collect();
    let frames = if array {
        Value::Array(sprites.iter().map(|s| {
            let mut v = json!({ "filename": s.name });
            v.as_object_mut().unwrap().extend(frame_json(s).as_object().unwrap().clone());
            v
        }).collect())
    } else {
        Value::Object(sprites.iter().map(|s| (s.name.clone(), frame_json(s))).collect())
    };
    let animations: BTreeMap<&str, &Vec<String>> = packed.animations.iter().map(|a| (a.name.as_str(), &a.frames)).collect();
    let p = &packed.pages[page];
    let mut meta = json!({
        "app": "https://github.com/HoangTrongPhi/Coral",
        "version": "1.0",
        "image": page_file(project, page, n, ext),
        "format": "RGBA8888",
        "size": { "w": p.width, "h": p.height },
        "scale": "1",
        "coral": coral_meta(project, packed),
    });
    if project.pack.premultiply {
        meta["premultiplyAlpha"] = json!(true);
    }
    if n > 1 {
        let json_ext = "json";
        meta["related_multi_packs"] = json!((0..n).filter(|&i| i != page).map(|i| page_file(project, i, n, json_ext)).collect::<Vec<_>>());
    }
    json!({ "frames": frames, "animations": animations, "meta": meta })
}

pub fn write(target: &ExportTarget, project: &Project, packed: &Packed, out: &Path, array: bool, o: &mut Output) {
    page_images(project, packed, out, o);
    let n = packed.pages.len();
    let ext = if target.paper2d && !array { "paper2dsprites" } else { "json" };
    for page in 0..n {
        let doc = document(project, packed, page, array);
        let mut text = serde_json::to_string_pretty(&doc).expect("json");
        text.push('\n');
        o.add(out.join(page_file(project, page, n, ext)), "json", text.into_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::pipeline::test_support::hero_packed;

    #[test]
    fn matches_the_hand_written_sample() {
        let (project, packed) = hero_packed();
        let doc = document(&project, &packed, 0, false);
        let expected = json!({
            "frame": {"x": 0, "y": 0, "w": 30, "h": 44}, "rotated": false, "trimmed": true,
            "spriteSourceSize": {"x": 1, "y": 4, "w": 30, "h": 44}, "sourceSize": {"w": 32, "h": 48},
            "pivot": {"x": 0.5, "y": 1.0}
        });
        assert_eq!(doc["frames"]["hero_idle_0"], expected);
        assert_eq!(doc["animations"]["idle"], json!(["hero_idle_0", "hero_idle_1"]));
        assert_eq!(doc["meta"]["image"], "hero.png");
        assert_eq!(doc["meta"]["format"], "RGBA8888");
        assert_eq!(doc["meta"]["coral"]["fps"]["idle"], json!(8.0));
        assert_eq!(doc["meta"]["coral"]["hitboxes"]["hero_idle_1"][0], json!({ "kind": "hitbox", "x": 12, "y": 4, "w": 20, "h": 10, "name": "sword" }));
        assert!(doc["meta"].get("related_multi_packs").is_none());

        let arr = document(&project, &packed, 0, true);
        assert_eq!(arr["frames"][0]["filename"], "hero_idle_0");
        assert_eq!(arr["frames"][0]["frame"], expected["frame"]);
    }
}

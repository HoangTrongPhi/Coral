//! Godot 4 `SpriteFrames` resource (`.tres`, `format=3`) with one `AtlasTexture` per frame, and
//! optionally one `AtlasTexture` `.tres` per sprite (UI). Spec §5.2.
//!
//! - `res://` paths are relative to the folder holding `project.godot` (`godotProject`, else searched
//!   upward from the output folder).
//! - `margin = Rect2(trim.x, trim.y, sourceW − w, sourceH − h)`: Godot offsets the region by the
//!   margin's position and sizes the texture as region + margin, so trimmed sprites keep their frame.
//! - Frame `duration` is relative: `durationMs / (1000 / fps)`. Ping-pong is unrolled, `once` → no loop.
//! - Rotated sprites cannot be expressed: error, turn `allowRotation` off.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use super::{fmt_f, page_file, page_images, Output};
use crate::core::anim::expand_pingpong;
use crate::core::pipeline::{Packed, PackedSprite};
use crate::core::project::{ExportTarget, LoopMode, Project};
use crate::core::Issue;

pub fn find_godot_project(start: &Path) -> Option<PathBuf> {
    let mut cur = Some(start);
    while let Some(dir) = cur {
        if dir.join("project.godot").is_file() {
            return Some(dir.to_path_buf());
        }
        cur = dir.parent();
    }
    None
}

/// `res://` path of `file` in the Godot project at `root`, or None when outside it.
pub fn res_path(root: &Path, file: &Path) -> Option<String> {
    let rel = crate::utils::paths::normalize(file).strip_prefix(crate::utils::paths::normalize(root)).ok()?.to_path_buf();
    Some(format!("res://{}", rel.to_string_lossy().replace('\\', "/")))
}

fn sub_id(name: &str, used: &mut Vec<String>) -> String {
    let clean: String = name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    let mut id = format!("AtlasTexture_{clean}");
    let mut n = 2;
    while used.contains(&id) {
        id = format!("AtlasTexture_{clean}_{n}");
        n += 1;
    }
    used.push(id.clone());
    id
}

fn region_lines(s: &PackedSprite) -> String {
    let mut t = format!("region = Rect2({}, {}, {}, {})\n", s.frame.x, s.frame.y, s.frame.w, s.frame.h);
    if s.trim.trimmed() {
        let _ = writeln!(t, "margin = Rect2({}, {}, {}, {})", s.trim.x, s.trim.y, s.trim.source_w - s.trim.w, s.trim.source_h - s.trim.h);
    }
    t
}

/// The SpriteFrames text for the given atlas `res://` paths (one per page).
pub fn sprite_frames(packed: &Packed, page_res: &[String]) -> String {
    // Frames per animation; without any animation, every sprite in a "default" one.
    let anims: Vec<(String, Vec<&PackedSprite>, f32, bool, Vec<f32>)> = if packed.animations.is_empty() {
        vec![("default".into(), packed.sprites.iter().collect(), 5.0, true, vec![1.0; packed.sprites.len()])]
    } else {
        packed
            .animations
            .iter()
            .map(|a| {
                let base = 1000.0 / a.fps.max(0.01);
                let mut frames: Vec<(&PackedSprite, f32)> = a.frames.iter().enumerate().filter_map(|(i, f)| packed.sprites.iter().find(|s| &s.name == f).map(|s| (s, a.duration_ms(i) / base))).collect();
                if a.loop_mode == LoopMode::Pingpong {
                    frames = expand_pingpong(&frames);
                }
                let (sprites, durs): (Vec<_>, Vec<_>) = frames.into_iter().unzip();
                (a.name.clone(), sprites, a.fps, a.loop_mode != LoopMode::Once, durs)
            })
            .collect()
    };

    // One sub-resource per distinct sprite used, in first-use order.
    let mut used_ids = Vec::new();
    let mut subs: Vec<(&str, String)> = Vec::new();
    for (_, sprites, ..) in &anims {
        for s in sprites {
            if !subs.iter().any(|(n, _)| *n == s.name) {
                subs.push((&s.name, sub_id(&s.name, &mut used_ids)));
            }
        }
    }
    let pages_used: Vec<usize> = (0..page_res.len()).filter(|p| packed.sprites.iter().any(|s| s.page == *p && subs.iter().any(|(n, _)| *n == s.name))).collect();

    let mut t = format!("[gd_resource type=\"SpriteFrames\" load_steps={} format=3]\n\n", pages_used.len() + subs.len() + 1);
    for &p in &pages_used {
        let _ = writeln!(t, "[ext_resource type=\"Texture2D\" path=\"{}\" id=\"{}_atlas\"]", page_res[p], p + 1);
    }
    t.push('\n');
    for (name, id) in &subs {
        let s = packed.sprites.iter().find(|s| s.name == *name).expect("listed");
        let _ = writeln!(t, "[sub_resource type=\"AtlasTexture\" id=\"{id}\"]\natlas = ExtResource(\"{}_atlas\")", s.page + 1);
        t.push_str(&region_lines(s));
        t.push('\n');
    }
    t.push_str("[resource]\nanimations = [");
    for (i, (name, sprites, fps, looped, durs)) in anims.iter().enumerate() {
        if i > 0 {
            t.push_str(", ");
        }
        t.push_str("{\n\"frames\": [");
        for (j, (s, d)) in sprites.iter().zip(durs).enumerate() {
            if j > 0 {
                t.push_str(", ");
            }
            let id = &subs.iter().find(|(n, _)| *n == s.name).expect("listed").1;
            let _ = write!(t, "{{\n\"duration\": {},\n\"texture\": SubResource(\"{id}\")\n}}", fmt_f(*d));
        }
        let _ = write!(t, "],\n\"loop\": {looped},\n\"name\": &\"{name}\",\n\"speed\": {}\n}}", fmt_f(*fps));
    }
    t.push_str("]\n");
    t
}

pub fn atlas_texture(s: &PackedSprite, res: &str) -> String {
    format!("[gd_resource type=\"AtlasTexture\" load_steps=2 format=3]\n\n[ext_resource type=\"Texture2D\" path=\"{res}\" id=\"1_atlas\"]\n\n[resource]\natlas = ExtResource(\"1_atlas\")\n{}", region_lines(s))
}

pub fn write(target: &ExportTarget, project: &Project, packed: &Packed, out: &Path, o: &mut Output) {
    if packed.sprites.iter().any(|s| s.rotated) {
        o.issues.push(Issue::error("godot-rotated", "Godot AtlasTexture cannot show rotated sprites: turn off \"Allow rotation\" for this project"));
        return;
    }
    let root = match target.godot_project.as_deref().filter(|g| !g.is_empty()) {
        Some(g) if Path::new(g).join("project.godot").is_file() => PathBuf::from(g),
        Some(g) => {
            o.issues.push(Issue::error("godot-project-not-found", format!("No project.godot in {g}")).at(g));
            return;
        }
        None => match find_godot_project(out) {
            Some(r) => r,
            None => {
                o.issues.push(Issue::error("godot-project-not-found", format!("No project.godot in {} or any folder above it: set the Godot project folder", out.display())).at(out));
                return;
            }
        },
    };
    let ext = project.pack.image_format.ext();
    let n = packed.pages.len();
    let mut res = Vec::new();
    for p in 0..n {
        match res_path(&root, &out.join(page_file(project, p, n, ext))) {
            Some(r) => res.push(r),
            None => {
                o.issues.push(Issue::error("godot-outside-project", format!("{} is not inside the Godot project {}", out.display(), root.display())).at(out));
                return;
            }
        }
    }
    page_images(project, packed, out, o);
    o.add(out.join(format!("{}.tres", project.name)), "tres", sprite_frames(packed, &res).into_bytes());
    if target.atlas_textures {
        for s in &packed.sprites {
            let file = format!("{}.tres", s.name.replace('/', "_"));
            o.add(out.join(file), "tres", atlas_texture(s, &res[s.page]).into_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::pipeline::test_support::hero_packed;

    #[test]
    fn sprite_frames_text_matches() {
        let (_, mut packed) = hero_packed();
        packed.animations[0].durations_ms = Some(vec![125, 250]);
        let t = sprite_frames(&packed, &["res://assets/hero/hero.png".into()]);
        let expected = "[gd_resource type=\"SpriteFrames\" load_steps=4 format=3]

[ext_resource type=\"Texture2D\" path=\"res://assets/hero/hero.png\" id=\"1_atlas\"]

[sub_resource type=\"AtlasTexture\" id=\"AtlasTexture_hero_idle_0\"]
atlas = ExtResource(\"1_atlas\")
region = Rect2(0, 0, 30, 44)
margin = Rect2(1, 4, 2, 4)

[sub_resource type=\"AtlasTexture\" id=\"AtlasTexture_hero_idle_1\"]
atlas = ExtResource(\"1_atlas\")
region = Rect2(32, 0, 32, 48)

[resource]
animations = [{
\"frames\": [{
\"duration\": 1.0,
\"texture\": SubResource(\"AtlasTexture_hero_idle_0\")
}, {
\"duration\": 2.0,
\"texture\": SubResource(\"AtlasTexture_hero_idle_1\")
}],
\"loop\": true,
\"name\": &\"idle\",
\"speed\": 8.0
}]
";
        assert_eq!(t, expected);
    }

    #[test]
    fn pingpong_once_and_project_lookup() {
        let (_, mut packed) = hero_packed();
        packed.animations[0].loop_mode = LoopMode::Once;
        assert!(sprite_frames(&packed, &["res://a.png".into()]).contains("\"loop\": false"));
        packed.animations[0].frames = vec!["hero_idle_0".into(), "hero_idle_1".into(), "hero_idle_0".into()];
        packed.animations[0].loop_mode = LoopMode::Pingpong;
        let t = sprite_frames(&packed, &["res://a.png".into()]);
        assert_eq!(t.matches("SubResource(").count(), 4, "0 1 0 → 0 1 0 1");

        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("project.godot"), "").unwrap();
        let out = d.path().join("assets/hero");
        std::fs::create_dir_all(&out).unwrap();
        assert_eq!(find_godot_project(&out).as_deref(), Some(d.path()));
        assert_eq!(res_path(d.path(), &out.join("hero.png")).unwrap(), "res://assets/hero/hero.png");
        assert!(res_path(&out, d.path()).is_none());
    }
}

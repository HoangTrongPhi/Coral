//! Writes `sample-assets/` for trying Coral by hand:
//! `cargo test --manifest-path src-tauri/Cargo.toml gen_sample_assets -- --ignored`.
//! Three animations (idle 4, run 8, attack 6 frames) as 32×48 PNGs with transparent margins,
//! a small Aseprite file with tags, a 6-shape sheet for Slice, a 16-frame flipbook sequence and
//! `sample.coral.json`.

#[cfg(test)]
mod tests {
    use std::path::Path;

    use image::{Rgba, RgbaImage};

    use crate::core::sources::aseprite::test_support::{write, TestLayer};

    fn rect(img: &mut RgbaImage, x: i32, y: i32, w: i32, h: i32, c: [u8; 4]) {
        for yy in y.max(0)..(y + h).min(img.height() as i32) {
            for xx in x.max(0)..(x + w).min(img.width() as i32) {
                img.put_pixel(xx as u32, yy as u32, Rgba(c));
            }
        }
    }

    fn circle(img: &mut RgbaImage, cx: f32, cy: f32, r: f32, c: [u8; 4]) {
        for (x, y, p) in img.enumerate_pixels_mut() {
            if (x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2) <= r * r {
                *p = Rgba(c);
            }
        }
    }

    fn hero(anim: &str, i: u32, n: u32) -> RgbaImage {
        let mut img = RgbaImage::new(32, 48);
        let t = i as f32 / n as f32 * std::f32::consts::TAU;
        let bob = if anim == "idle" { (t.sin() * 1.5).round() as i32 } else if anim == "run" { (i % 2) as i32 * 2 } else { 0 };
        let body = match anim {
            "idle" => [79, 140, 255, 255],
            "run" => [34, 197, 94, 255],
            _ => [249, 115, 22, 255],
        };
        rect(&mut img, 10, 44, 12, 2, [30, 41, 59, 255]);
        rect(&mut img, 10, 18 + bob, 12, 18, body);
        circle(&mut img, 16.0, 12.0 + bob as f32, 6.0, [252, 217, 182, 255]);
        let leg = if anim == "run" { (t.sin() * 4.0).round() as i32 } else { 0 };
        rect(&mut img, 11 + leg, 36 + bob, 4, 8 - bob, [51, 65, 85, 255]);
        rect(&mut img, 17 - leg, 36 + bob, 4, 8 - bob, [51, 65, 85, 255]);
        if anim == "attack" {
            let reach = [2, 6, 10, 10, 8, 4][i as usize];
            rect(&mut img, 22, 24, reach, 3, [226, 232, 240, 255]);
        }
        img
    }

    #[test]
    #[ignore = "writes sample-assets/: cargo test gen_sample_assets -- --ignored"]
    fn gen_sample_assets() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("sample-assets");
        let art = root.join("art").join("hero");
        std::fs::create_dir_all(&art).unwrap();
        for (anim, n) in [("idle", 4), ("run", 8), ("attack", 6)] {
            for i in 0..n {
                hero(anim, i, n).save(art.join(format!("hero_{anim}_{i}.png"))).unwrap();
            }
        }

        // Aseprite: two layers, tags "spark" (forward) and "fade" (ping-pong), a pivot slice.
        let frames: Vec<(u16, Vec<(u16, i16, i16, RgbaImage)>)> = (0..6)
            .map(|i| {
                let mut core = RgbaImage::new(24, 24);
                circle(&mut core, 12.0, 12.0, 3.0 + i as f32 * 1.5, [250, 204, 21, 255]);
                let mut glow = RgbaImage::new(24, 24);
                circle(&mut glow, 12.0, 12.0, 2.0 + i as f32, [255, 247, 174, 200]);
                (if i == 5 { 200 } else { 80 }, vec![(0, 0, 0, core), (1, 0, 0, glow)])
            })
            .collect();
        let ase = write(24, 24, &[TestLayer { name: "Core", visible: true }, TestLayer { name: "Glow", visible: true }], &frames, &[("spark", 0, 3, 0), ("fade", 4, 5, 2)], Some(([0, 0, 24, 24], [6, 6, 12, 12], [12, 12])));
        std::fs::create_dir_all(root.join("art/fx")).unwrap();
        std::fs::write(root.join("art/fx/spark.aseprite"), ase).unwrap();

        // Sheet for Slice: six shapes, two uneven rows.
        let mut sheet = RgbaImage::new(128, 64);
        for (i, (x, y)) in [(4, 3), (36, 5), (68, 4), (6, 36), (38, 35), (70, 38)].iter().enumerate() {
            rect(&mut sheet, *x, *y, 22, 24, [(40 * i) as u8 + 40, 180, 255 - (30 * i) as u8, 255]);
        }
        std::fs::create_dir_all(root.join("sheets")).unwrap();
        sheet.save(root.join("sheets/sheet.png")).unwrap();

        // Flipbook frames.
        let fx = root.join("fx/explosion");
        std::fs::create_dir_all(&fx).unwrap();
        for i in 1..=16u32 {
            let mut f = RgbaImage::new(64, 64);
            let a = (255.0 * (1.0 - i as f32 / 17.0)) as u8;
            circle(&mut f, 32.0, 32.0, 4.0 + i as f32 * 1.7, [250, 140, 30, a]);
            circle(&mut f, 32.0, 32.0, 2.0 + i as f32 * 0.9, [255, 250, 200, a]);
            f.save(fx.join(format!("explosion_{i:04}.png"))).unwrap();
        }

        std::fs::write(
            root.join("sample.coral.json"),
            r#"{
  "format": "coral.project",
  "version": 1,
  "name": "hero",
  "sources": [
    { "kind": "folder", "path": "art/hero", "recursive": true, "include": ["*.png"] },
    { "kind": "aseprite", "path": "art/fx/spark.aseprite", "layers": "visible" }
  ],
  "sprites": { "hero_idle_0": { "pivot": [0.5, 1.0], "nineSlice": null } },
  "animations": [],
  "hitboxes": { "hero_attack_3": [{ "kind": "hitbox", "rect": [22, 22, 10, 6], "name": "sword" }] },
  "pack": { "maxSize": 1024, "padding": 2, "extrude": 1 },
  "exports": [
    { "target": "jsonHash", "out": "build/json" },
    { "target": "spine", "out": "build/spine" }
  ],
  "watch": { "autoStart": false }
}
"#,
        )
        .unwrap();
        println!("sample assets in {}", root.display());
    }
}

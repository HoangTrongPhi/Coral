//! 2D normal maps for sprite lighting: a height field (luminance, or a bevel grown from the alpha
//! edge), optional Gaussian blur, Sobel slopes, `n = normalize(−dx·s, −dy·s, 1)`, RGB = n·0.5 + 0.5.
//!
//! Convention (docs/FORMATS.md#normal-maps): OpenGL = green up (Godot, Unity); DirectX flips green
//! (Unreal). Image y grows downward, so a height rising toward +x gives R < 128 and one rising toward
//! the bottom of the image gives G > 128 in OpenGL. Transparent pixels are flat (128, 128, 255).

use std::collections::VecDeque;
use std::path::Path;
use std::time::Instant;

use image::{Rgba, RgbaImage};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use super::{collect_images, image_io, OutputMode, RunCtx, WriteReport, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HeightSource {
    #[default]
    Luminance,
    AlphaBevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Convention {
    #[default]
    Opengl,
    Directx,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NormalSettings {
    pub source: HeightSource,
    pub strength: f32,
    /// Gaussian sigma in px; 0 = none.
    pub blur: f32,
    /// Alpha bevel width in px.
    pub bevel: f32,
    pub convention: Convention,
    /// Swap high and low (dark = raised).
    pub invert: bool,
}

impl Default for NormalSettings {
    fn default() -> Self {
        NormalSettings { source: HeightSource::Luminance, strength: 2.0, blur: 0.0, bevel: 4.0, convention: Convention::Opengl, invert: false }
    }
}

fn height_field(img: &RgbaImage, s: &NormalSettings) -> Vec<f32> {
    let (w, h) = img.dimensions();
    let mut hf: Vec<f32> = match s.source {
        HeightSource::Luminance => img.pixels().map(|p| (0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32) / 255.0 * (p[3] as f32 / 255.0)).collect(),
        HeightSource::AlphaBevel => {
            // Distance (in px) from the nearest transparent pixel or image edge, BFS, then clamp to bevel.
            let n = (w * h) as usize;
            let mut dist = vec![u32::MAX; n];
            let mut q = VecDeque::new();
            for y in 0..h {
                for x in 0..w {
                    let i = (y * w + x) as usize;
                    if img.get_pixel(x, y)[3] < 128 {
                        dist[i] = 0;
                        q.push_back((x, y));
                    } else if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                        dist[i] = 1;
                        q.push_back((x, y));
                    }
                }
            }
            while let Some((x, y)) = q.pop_front() {
                let d = dist[(y * w + x) as usize];
                for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let j = (ny as u32 * w + nx as u32) as usize;
                    if dist[j] > d + 1 {
                        dist[j] = d + 1;
                        q.push_back((nx as u32, ny as u32));
                    }
                }
            }
            let bevel = s.bevel.max(0.5);
            dist.into_iter().map(|d| (d as f32 / bevel).min(1.0)).collect()
        }
    };
    if s.invert {
        for v in &mut hf {
            *v = 1.0 - *v;
        }
    }
    if s.blur > 0.0 {
        hf = gaussian(&hf, w, h, s.blur);
    }
    hf
}

fn gaussian(src: &[f32], w: u32, h: u32, sigma: f32) -> Vec<f32> {
    let r = (sigma * 3.0).ceil() as i32;
    let k: Vec<f32> = (-r..=r).map(|i| (-(i * i) as f32 / (2.0 * sigma * sigma)).exp()).collect();
    let sum: f32 = k.iter().sum();
    let k: Vec<f32> = k.into_iter().map(|v| v / sum).collect();
    let (w, h) = (w as i32, h as i32);
    let at = |v: &[f32], x: i32, y: i32| v[(y.clamp(0, h - 1) * w + x.clamp(0, w - 1)) as usize];
    let mut tmp = vec![0.0; src.len()];
    for y in 0..h {
        for x in 0..w {
            tmp[(y * w + x) as usize] = (-r..=r).map(|i| at(src, x + i, y) * k[(i + r) as usize]).sum();
        }
    }
    let mut out = vec![0.0; src.len()];
    for y in 0..h {
        for x in 0..w {
            out[(y * w + x) as usize] = (-r..=r).map(|i| at(&tmp, x, y + i) * k[(i + r) as usize]).sum();
        }
    }
    out
}

/// Normal map of `img` (same size).
pub fn normal_map(img: &RgbaImage, s: &NormalSettings) -> RgbaImage {
    let (w, h) = img.dimensions();
    let hf = height_field(img, s);
    let (wi, hi) = (w as i32, h as i32);
    let at = |x: i32, y: i32| hf[(y.clamp(0, hi - 1) * wi + x.clamp(0, wi - 1)) as usize];
    let mut out = RgbaImage::new(w, h);
    for (x, y, p) in out.enumerate_pixels_mut() {
        let alpha = img.get_pixel(x, y)[3];
        if alpha == 0 {
            *p = Rgba([128, 128, 255, 0]);
            continue;
        }
        let (x, y) = (x as i32, y as i32);
        let dx = (at(x + 1, y - 1) + 2.0 * at(x + 1, y) + at(x + 1, y + 1)) - (at(x - 1, y - 1) + 2.0 * at(x - 1, y) + at(x - 1, y + 1));
        let dy = (at(x - 1, y + 1) + 2.0 * at(x, y + 1) + at(x + 1, y + 1)) - (at(x - 1, y - 1) + 2.0 * at(x, y - 1) + at(x + 1, y - 1));
        let (nx, ny, nz) = (-dx * s.strength, -dy * s.strength, 1.0f32);
        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        let (nx, mut ny, nz) = (nx / len, ny / len, nz / len);
        // `dy` grows downward in the image: with OpenGL's green-up, a slope rising toward the bottom
        // faces up-screen (G > 0.5).
        ny = -ny;
        if s.convention == Convention::Directx {
            ny = -ny;
        }
        let enc = |v: f32| ((v * 0.5 + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8;
        *p = Rgba([enc(nx), enc(ny), enc(nz), alpha]);
    }
    out
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalOptions {
    pub paths: Vec<String>,
    #[serde(default)]
    pub settings: NormalSettings,
    pub output: OutputMode,
    #[serde(default)]
    pub dry_run: bool,
}

/// `<name>_n.png` next to each input (or in the output folder).
pub fn run(o: &NormalOptions, ctx: &RunCtx) -> (WriteReport, Option<RgbaImage>) {
    let started = Instant::now();
    let (files, issues) = collect_images(&o.paths, true);
    let mut w = Writer::new(o.dry_run);
    w.issues = issues;
    let total = files.len() as u64;
    let results: Vec<(usize, Result<RgbaImage, String>)> = files.par_iter().enumerate().filter(|_| !ctx.cancelled()).map(|(i, f)| (i, image_io::load(f).map(|img| normal_map(&img, &o.settings)))).collect();
    let mut preview = None;
    for (i, r) in results {
        let f: &Path = &files[i];
        (ctx.progress)(i as u64 + 1, total, &f.to_string_lossy());
        match r {
            Ok(n) => {
                let target = o.output.target(f, "_n", Some("png"));
                w.write(&target, "image", &image_io::encode_png(&n), Some(f));
                if preview.is_none() {
                    preview = Some(n);
                }
            }
            Err(e) => w.issues.push(super::Issue::error("unreadable-image", format!("{}: {e}", f.display())).at(f)),
        }
    }
    (WriteReport::from_writer(w, started, ctx.cancelled(), vec![format!("{} normal map(s)", total)]), preview)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(horizontal: bool) -> RgbaImage {
        RgbaImage::from_fn(16, 16, |x, y| {
            let v = ((if horizontal { x } else { y }) * 16) as u8;
            Rgba([v, v, v, 255])
        })
    }

    #[test]
    fn slope_directions() {
        let s = NormalSettings::default();
        let n = normal_map(&ramp(true), &s);
        let p = n.get_pixel(8, 8);
        assert!(p[0] < 128 && (p[1] as i32 - 128).abs() <= 1 && p[2] > 128, "rising toward +x → R < 128 (OpenGL): {p:?}");
        let v = normal_map(&ramp(false), &s);
        assert!(v.get_pixel(8, 8)[1] > 128, "rising toward the bottom → G > 128 in OpenGL");
        let dx = normal_map(&ramp(false), &NormalSettings { convention: Convention::Directx, ..s.clone() });
        assert!(dx.get_pixel(8, 8)[1] < 128, "DirectX flips green");

        let mut sprite = RgbaImage::new(9, 9);
        for y in 1..8 {
            for x in 1..8 {
                sprite.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
        let bevel = normal_map(&sprite, &NormalSettings { source: HeightSource::AlphaBevel, bevel: 3.0, ..s });
        assert_eq!(*bevel.get_pixel(0, 0), Rgba([128, 128, 255, 0]), "transparent = flat");
        assert!(bevel.get_pixel(1, 4)[0] < 128, "left edge rises inward (+x)");
        assert!(bevel.get_pixel(7, 4)[0] > 128, "right edge rises toward −x");
    }
}

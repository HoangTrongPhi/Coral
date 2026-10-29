//! Reading and writing PNG / TGA, hashing pixels.

use std::io::Cursor;
use std::path::Path;

use image::{ImageFormat, RgbaImage};
use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::Xxh3;

/// Output image format of atlases and tools (Q2: PNG and TGA).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageKind {
    #[default]
    Png,
    Tga,
}

impl ImageKind {
    pub fn ext(self) -> &'static str {
        match self {
            ImageKind::Png => "png",
            ImageKind::Tga => "tga",
        }
    }
    fn format(self) -> ImageFormat {
        match self {
            ImageKind::Png => ImageFormat::Png,
            ImageKind::Tga => ImageFormat::Tga,
        }
    }
}

pub fn is_image(p: &Path) -> bool {
    matches!(p.extension().map(|e| e.to_string_lossy().to_lowercase()).as_deref(), Some("png" | "tga" | "gif"))
}

/// Decode a PNG / TGA (GIF: first frame) as RGBA8.
pub fn load(path: &Path) -> Result<RgbaImage, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let fmt = ImageFormat::from_path(path).ok();
    let img = match fmt {
        Some(f) => image::load_from_memory_with_format(&bytes, f),
        None => image::load_from_memory(&bytes),
    }
    .map_err(|e| e.to_string())?;
    Ok(img.to_rgba8())
}

pub fn encode(img: &RgbaImage, kind: ImageKind) -> Result<Vec<u8>, String> {
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, kind.format()).map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

pub fn encode_png(img: &RgbaImage) -> Vec<u8> {
    encode(img, ImageKind::Png).unwrap_or_default()
}

/// Hash of size + pixels (dedupe, cache keys).
pub fn hash(img: &RgbaImage) -> u64 {
    let mut h = Xxh3::new();
    h.update(&img.width().to_le_bytes());
    h.update(&img.height().to_le_bytes());
    h.update(img.as_raw());
    h.digest()
}

/// Same picture, at most `max` px on its longer side (nearest neighbour: pixel art stays crisp).
pub fn thumbnail(img: &RgbaImage, max: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    if w <= max && h <= max || max == 0 {
        return img.clone();
    }
    let s = max as f32 / w.max(h) as f32;
    let (tw, th) = (((w as f32 * s).round() as u32).max(1), ((h as f32 * s).round() as u32).max(1));
    image::imageops::resize(img, tw, th, image::imageops::FilterType::Nearest)
}

/// File modification time + size: the cache key of a decoded source.
pub fn stamp(p: &Path) -> Option<(u64, u64)> {
    let m = std::fs::metadata(p).ok()?;
    let t = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos() as u64;
    Some((t, m.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn png_and_tga_round_trip() {
        let mut img = RgbaImage::new(3, 2);
        img.put_pixel(1, 1, Rgba([10, 20, 30, 128]));
        let d = tempfile::tempdir().unwrap();
        for kind in [ImageKind::Png, ImageKind::Tga] {
            let p = d.path().join(format!("a.{}", kind.ext()));
            std::fs::write(&p, encode(&img, kind).unwrap()).unwrap();
            let back = load(&p).unwrap();
            assert_eq!(back, img, "{kind:?}");
        }
        assert_eq!(hash(&img), hash(&img.clone()));
        assert_eq!(thumbnail(&RgbaImage::new(200, 100), 50).dimensions(), (50, 25));
    }
}

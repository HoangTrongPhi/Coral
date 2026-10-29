//! Alpha trim: the smallest rect holding every pixel with alpha above the threshold.

use image::RgbaImage;
use serde::{Deserialize, Serialize};

/// Where the trimmed picture sits in the original one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrimInfo {
    pub source_w: u32,
    pub source_h: u32,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl TrimInfo {
    pub fn untrimmed(w: u32, h: u32) -> Self {
        TrimInfo { source_w: w, source_h: h, x: 0, y: 0, w, h }
    }
    pub fn trimmed(&self) -> bool {
        self.w != self.source_w || self.h != self.source_h
    }
}

/// Bounding box of alpha > `threshold`; `None` when the picture is empty.
pub fn alpha_bounds(img: &RgbaImage, threshold: u8) -> Option<TrimInfo> {
    let (w, h) = img.dimensions();
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, p) in img.enumerate_pixels() {
        if p[3] > threshold {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    (x0 != u32::MAX).then(|| TrimInfo { source_w: w, source_h: h, x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1 })
}

/// Trimmed picture + where it was. An empty picture keeps a 1×1 transparent pixel at (0, 0).
pub fn trim(img: &RgbaImage, threshold: u8) -> (RgbaImage, TrimInfo) {
    match alpha_bounds(img, threshold) {
        Some(t) if t.trimmed() => (image::imageops::crop_imm(img, t.x, t.y, t.w, t.h).to_image(), t),
        Some(t) => (img.clone(), t),
        None => {
            let (w, h) = img.dimensions();
            (RgbaImage::new(1, 1), TrimInfo { source_w: w, source_h: h, x: 0, y: 0, w: 1, h: 1 })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn trims_to_the_shape() {
        let mut img = RgbaImage::new(32, 48);
        for y in 7..17 {
            for x in 5..15 {
                img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        let (t, info) = trim(&img, 0);
        assert_eq!((info.x, info.y, info.w, info.h), (5, 7, 10, 10));
        assert_eq!((info.source_w, info.source_h), (32, 48));
        assert_eq!(t.dimensions(), (10, 10));
        // A faint pixel is kept at threshold 0 and dropped above it.
        img.put_pixel(0, 0, Rgba([0, 0, 0, 3]));
        assert_eq!(trim(&img, 0).1.x, 0);
        assert_eq!(trim(&img, 3).1.x, 5);
        let (e, ei) = trim(&RgbaImage::new(8, 8), 0);
        assert_eq!((e.dimensions(), ei.w), ((1, 1), 1));
    }
}

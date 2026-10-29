//! Photoshop: one sprite per visible pixel layer (`layers`, named after the layer) or the flattened
//! image (`composite`, named after the file). The `psd` crate reads pixel layers only: layer effects and
//! smart objects come out without their look (warn `psd-layer-unsupported`).

use std::path::Path;
use std::sync::Arc;

use image::RgbaImage;
use psd::Psd;

use super::{Loaded, SourceRef, SourceSprite};
use crate::core::project::PsdMode;
use crate::core::Issue;

fn layer_image(psd: &Psd, layer: &psd::PsdLayer) -> Option<RgbaImage> {
    let (w, h) = (psd.width(), psd.height());
    let px = layer.rgba();
    if px.len() == (w * h * 4) as usize {
        return RgbaImage::from_raw(w, h, px);
    }
    // Some versions return only the layer's own rect: place it on the canvas.
    let (lw, lh) = (layer.width() as u32, layer.height() as u32);
    let own = RgbaImage::from_raw(lw, lh, px)?;
    let mut canvas = RgbaImage::new(w, h);
    image::imageops::overlay(&mut canvas, &own, layer.layer_left() as i64, layer.layer_top() as i64);
    Some(canvas)
}

pub fn load(path: &Path, mode: PsdMode) -> Loaded {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => return Loaded { issues: vec![Issue::error("unreadable-psd", format!("{}: {e}", path.display())).at(path)], ..Default::default() },
    };
    // The decoder is third-party code on user files: a panic must not take the app down.
    let parsed = std::panic::catch_unwind(|| Psd::from_bytes(&bytes).map_err(|e| e.to_string()));
    let psd = match parsed {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => return Loaded { issues: vec![Issue::error("unreadable-psd", format!("{}: {e}", path.display())).at(path)], ..Default::default() },
        Err(_) => return Loaded { issues: vec![Issue::error("unreadable-psd", format!("{}: the PSD reader failed on this file (try saving with Maximize Compatibility)", path.display())).at(path)], ..Default::default() },
    };
    let origin = |layer: Option<String>| SourceRef { path: path.to_string_lossy().into_owned(), frame: None, layer };
    let mut out = Loaded::default();
    let sprite = |name: String, img: RgbaImage, layer: Option<String>| SourceSprite { name, image: Arc::new(img), origin: origin(layer), duration_ms: None, pivot: None, nine_slice: None };

    match mode {
        PsdMode::Composite => match RgbaImage::from_raw(psd.width(), psd.height(), psd.rgba()) {
            Some(img) => out.sprites.push(sprite(super::file_stem(path), img, None)),
            None => out.issues.push(Issue::error("unreadable-psd", format!("{}: no flattened image", path.display())).at(path)),
        },
        PsdMode::Layers => {
            let mut unsupported = 0;
            for layer in psd.layers() {
                if !layer.visible() {
                    continue;
                }
                if layer.width() == 0 || layer.height() == 0 {
                    unsupported += 1;
                    continue;
                }
                match std::panic::catch_unwind(|| layer_image(&psd, layer)) {
                    Ok(Some(img)) => out.sprites.push(sprite(super::clean_name(layer.name()), img, Some(layer.name().to_string()))),
                    _ => unsupported += 1,
                }
            }
            if unsupported > 0 {
                out.issues.push(Issue::warn("psd-layer-unsupported", format!("{}: {unsupported} layer(s) without pixels were skipped (adjustment, text or smart object layers: rasterize them in Photoshop)", path.display())).at(path));
            }
            if out.sprites.is_empty() && out.issues.is_empty() {
                out.issues.push(Issue::warn("empty-source", format!("{}: no visible pixel layers", path.display())).at(path));
            }
        }
    }
    out
}

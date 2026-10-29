//! Aseprite `.aseprite` / `.ase` reader, written against the published file format
//! (github.com/aseprite/aseprite/blob/main/docs/ase-file-specs.md) instead of the `asefile` crate,
//! which pins `image` 0.24 (a second copy of the codec stack in the binary).
//!
//! Read: RGBA / grayscale / indexed pixels, layers (visibility, opacity, groups), raw / linked /
//! compressed cels, tags (direction), frame durations, slices (9-slice centre, pivot), palettes.
//! Blend modes other than Normal are composited as Normal (warn `aseprite-blend-mode`); tilemap
//! layers are skipped (warn).
//!
//! Names (spec §3.2): `<file>_<tag>_<i>` for frames in a tag (a frame in two tags appears twice and is
//! deduplicated later), `<file>_<frame>` for frames outside tags; with a list of layer names, one sprite
//! per layer and frame: `<file>_<layer>_<frame>`.

use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use flate2::read::ZlibDecoder;
use image::{Rgba, RgbaImage};

use super::{blend_over, Loaded, SourceRef, SourceSprite};
use crate::core::project::{LayerMode, LayerSelection, LoopMode};
use crate::core::{anim, Issue};

#[derive(Debug, Clone)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    pub kind: u16,
    pub child_level: u16,
    pub blend: u16,
    pub opacity: u8,
    /// Visible including its parent groups.
    pub effective_visible: bool,
}

#[derive(Debug, Clone)]
pub struct Cel {
    pub layer: usize,
    pub x: i32,
    pub y: i32,
    pub opacity: u8,
    pub image: Arc<RgbaImage>,
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub name: String,
    pub from: usize,
    pub to: usize,
    /// 0 forward, 1 reverse, 2 ping-pong, 3 ping-pong reverse.
    pub direction: u8,
}

#[derive(Debug, Clone)]
pub struct Slice {
    pub name: String,
    pub bounds: [i32; 4],
    /// Centre rect relative to the slice: x, y, w, h.
    pub center: Option<[i32; 4]>,
    pub pivot: Option<[i32; 2]>,
}

#[derive(Debug, Clone, Default)]
pub struct Document {
    pub width: u32,
    pub height: u32,
    pub depth: u16,
    pub layers: Vec<Layer>,
    /// Per frame: duration and cels.
    pub frames: Vec<(u32, Vec<Cel>)>,
    pub tags: Vec<Tag>,
    pub slices: Vec<Slice>,
    pub palette: Vec<[u8; 4]>,
    pub transparent_index: u8,
    pub warnings: Vec<String>,
}

struct Rd<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Rd<'a> {
    fn need(&self, n: usize) -> Result<(), String> {
        if self.p + n > self.b.len() {
            Err("file is truncated".into())
        } else {
            Ok(())
        }
    }
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], String> {
        self.need(n)?;
        let s = &self.b[self.p..self.p + n];
        self.p += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.bytes(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        let b = self.bytes(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn i16(&mut self) -> Result<i16, String> {
        Ok(self.u16()? as i16)
    }
    fn u32(&mut self) -> Result<u32, String> {
        let b = self.bytes(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(self.u32()? as i32)
    }
    fn string(&mut self) -> Result<String, String> {
        let n = self.u16()? as usize;
        Ok(String::from_utf8_lossy(self.bytes(n)?).into_owned())
    }
}

fn pixels(doc: &Document, raw: &[u8], w: u32, h: u32) -> Result<RgbaImage, String> {
    let n = (w * h) as usize;
    let bpp = (doc.depth / 8) as usize;
    if raw.len() < n * bpp {
        return Err("cel pixels are truncated".into());
    }
    let mut img = RgbaImage::new(w, h);
    for (i, px) in img.pixels_mut().enumerate() {
        let s = &raw[i * bpp..i * bpp + bpp];
        *px = match doc.depth {
            32 => Rgba([s[0], s[1], s[2], s[3]]),
            16 => Rgba([s[0], s[0], s[0], s[1]]),
            _ => {
                if s[0] == doc.transparent_index {
                    Rgba([0, 0, 0, 0])
                } else {
                    Rgba(doc.palette.get(s[0] as usize).copied().unwrap_or([0, 0, 0, 255]))
                }
            }
        };
    }
    Ok(img)
}

pub fn parse(bytes: &[u8]) -> Result<Document, String> {
    let mut r = Rd { b: bytes, p: 0 };
    let _size = r.u32()?;
    if r.u16()? != 0xA5E0 {
        return Err("not an Aseprite file (bad magic number)".into());
    }
    let frames = r.u16()? as usize;
    let mut doc = Document { width: r.u16()? as u32, height: r.u16()? as u32, depth: r.u16()?, ..Default::default() };
    if !matches!(doc.depth, 8 | 16 | 32) {
        return Err(format!("unsupported colour depth {}", doc.depth));
    }
    let flags = r.u32()?;
    r.bytes(2 + 4 + 4)?; // speed (deprecated), reserved
    doc.transparent_index = r.u8()?;
    r.bytes(3)?;
    let _colors = r.u16()?;
    r.bytes(128 - r.p)?; // pixel ratio, grid, reserved
    let layer_uuids = flags & 4 != 0;

    // Cels that are links (type 1) refer to an earlier frame of the same layer; resolve after reading.
    let mut links: Vec<(usize, usize, usize)> = Vec::new(); // frame, cel slot, linked frame
    for f in 0..frames {
        let start = r.p;
        let frame_bytes = r.u32()? as usize;
        if r.u16()? != 0xF1FA {
            return Err(format!("frame {f}: bad frame magic number"));
        }
        let old_chunks = r.u16()? as usize;
        let duration = r.u16()? as u32;
        r.bytes(2)?;
        let new_chunks = r.u32()? as usize;
        let chunks = if new_chunks == 0 { old_chunks } else { new_chunks };
        let mut cels = Vec::new();
        for _ in 0..chunks {
            let cstart = r.p;
            let csize = r.u32()? as usize;
            let ctype = r.u16()?;
            if csize < 6 {
                return Err(format!("frame {f}: bad chunk size"));
            }
            r.need(csize - 6)?;
            let mut c = Rd { b: &bytes[r.p..cstart + csize], p: 0 };
            match ctype {
                0x0004 | 0x0011 => {
                    // Old palettes, used when no new palette chunk exists.
                    if doc.palette.is_empty() || ctype == 0x0004 && doc.palette.len() < 256 {
                        let packets = c.u16()?;
                        let mut idx = 0usize;
                        let mut pal = vec![[0, 0, 0, 255]; 256];
                        for _ in 0..packets {
                            idx += c.u8()? as usize;
                            let n = match c.u8()? {
                                0 => 256,
                                n => n as usize,
                            };
                            for _ in 0..n {
                                let rgb = c.bytes(3)?;
                                let k = if ctype == 0x0011 { 4 } else { 1 };
                                if idx < 256 {
                                    pal[idx] = [rgb[0].saturating_mul(k), rgb[1].saturating_mul(k), rgb[2].saturating_mul(k), 255];
                                }
                                idx += 1;
                            }
                        }
                        doc.palette = pal;
                    }
                }
                0x2019 => {
                    let size = c.u32()? as usize;
                    let first = c.u32()? as usize;
                    let last = c.u32()? as usize;
                    c.bytes(8)?;
                    if doc.palette.len() < size {
                        doc.palette.resize(size, [0, 0, 0, 255]);
                    }
                    for i in first..=last.min(size.saturating_sub(1)) {
                        let fl = c.u16()?;
                        let px = c.bytes(4)?;
                        doc.palette[i] = [px[0], px[1], px[2], px[3]];
                        if fl & 1 != 0 {
                            c.string()?;
                        }
                    }
                }
                0x2004 => {
                    let fl = c.u16()?;
                    let kind = c.u16()?;
                    let child_level = c.u16()?;
                    c.bytes(4)?;
                    let blend = c.u16()?;
                    let opacity = c.u8()?;
                    c.bytes(3)?;
                    let name = c.string()?;
                    let _ = layer_uuids;
                    doc.layers.push(Layer { name, visible: fl & 1 != 0, kind, child_level, blend, opacity: if flags & 1 != 0 { opacity } else { 255 }, effective_visible: true });
                }
                0x2005 => {
                    let layer = c.u16()? as usize;
                    let x = c.i16()? as i32;
                    let y = c.i16()? as i32;
                    let opacity = c.u8()?;
                    let kind = c.u16()?;
                    c.bytes(2 + 5)?; // z-index, reserved
                    match kind {
                        0 => {
                            let (w, h) = (c.u16()? as u32, c.u16()? as u32);
                            let raw = c.bytes((w * h) as usize * (doc.depth / 8) as usize)?;
                            cels.push(Cel { layer, x, y, opacity, image: Arc::new(pixels(&doc, raw, w, h)?) });
                        }
                        1 => {
                            let linked = c.u16()? as usize;
                            links.push((f, cels.len(), linked));
                            cels.push(Cel { layer, x, y, opacity, image: Arc::new(RgbaImage::new(0, 0)) });
                        }
                        2 => {
                            let (w, h) = (c.u16()? as u32, c.u16()? as u32);
                            let mut raw = Vec::new();
                            ZlibDecoder::new(&c.b[c.p..]).read_to_end(&mut raw).map_err(|e| format!("frame {f}: cel data: {e}"))?;
                            cels.push(Cel { layer, x, y, opacity, image: Arc::new(pixels(&doc, &raw, w, h)?) });
                        }
                        _ => doc.warnings.push(format!("frame {f}: tilemap cel skipped")),
                    }
                }
                0x2018 => {
                    let n = c.u16()?;
                    c.bytes(8)?;
                    for _ in 0..n {
                        let from = c.u16()? as usize;
                        let to = c.u16()? as usize;
                        let direction = c.u8()?;
                        c.bytes(2 + 6 + 3 + 1)?; // repeat, reserved, colour
                        let name = c.string()?;
                        doc.tags.push(Tag { name, from, to, direction });
                    }
                }
                0x2022 => {
                    let keys = c.u32()?;
                    let fl = c.u32()?;
                    c.u32()?;
                    let name = c.string()?;
                    let mut slice = Slice { name, bounds: [0; 4], center: None, pivot: None };
                    for k in 0..keys {
                        let _frame = c.u32()?;
                        let b = [c.i32()?, c.i32()?, c.u32()? as i32, c.u32()? as i32];
                        let center = if fl & 1 != 0 { Some([c.i32()?, c.i32()?, c.u32()? as i32, c.u32()? as i32]) } else { None };
                        let pivot = if fl & 2 != 0 { Some([c.i32()?, c.i32()?]) } else { None };
                        if k == 0 {
                            slice.bounds = b;
                            slice.center = center;
                            slice.pivot = pivot;
                        }
                    }
                    doc.slices.push(slice);
                }
                _ => {}
            }
            r.p = cstart + csize;
        }
        doc.frames.push((duration.max(1), cels));
        r.p = start + frame_bytes;
    }
    for (f, slot, linked) in links {
        let layer = doc.frames[f].1[slot].layer;
        let src = doc.frames.get(linked).and_then(|(_, cels)| cels.iter().find(|c| c.layer == layer)).map(|c| c.image.clone());
        match src {
            Some(img) => doc.frames[f].1[slot].image = img,
            None => doc.warnings.push(format!("frame {f}: linked cel points to a missing frame {linked}")),
        }
    }
    // Hidden groups hide their children.
    let mut hidden_at: Option<u16> = None;
    for l in &mut doc.layers {
        if let Some(level) = hidden_at {
            if l.child_level > level {
                l.effective_visible = false;
                continue;
            }
            hidden_at = None;
        }
        l.effective_visible = l.visible;
        if l.kind == 1 && !l.visible {
            hidden_at = Some(l.child_level);
        }
    }
    Ok(doc)
}

/// One frame with the chosen layers merged (bottom to top, layer and cel opacity).
pub fn composite(doc: &Document, frame: usize, include: &dyn Fn(usize, &Layer) -> bool) -> RgbaImage {
    let mut img = RgbaImage::new(doc.width, doc.height);
    let Some((_, cels)) = doc.frames.get(frame) else { return img };
    let mut ordered: Vec<&Cel> = cels.iter().collect();
    ordered.sort_by_key(|c| c.layer);
    for c in ordered {
        let Some(l) = doc.layers.get(c.layer) else { continue };
        if l.kind != 0 || !include(c.layer, l) {
            continue;
        }
        let opacity = (c.opacity as u32 * l.opacity as u32 / 255) as u8;
        blend_over(&mut img, &c.image, c.x, c.y, opacity);
    }
    img
}

pub fn load(path: &Path, layers: &LayerSelection) -> Loaded {
    let fail = |e: String| Loaded {
        issues: vec![Issue::error("unreadable-aseprite", format!("{}: {e} — if it opens in Aseprite, report it, or export a PNG sequence meanwhile", path.display())).at(path)],
        ..Default::default()
    };
    let doc = match std::fs::read(path).map_err(|e| e.to_string()).and_then(|b| parse(&b)) {
        Ok(d) => d,
        Err(e) => return fail(e),
    };
    let base = super::file_stem(path);
    let mut out = Loaded::default();
    for w in &doc.warnings {
        out.issues.push(Issue::warn("aseprite-unsupported", format!("{}: {w}", path.display())).at(path));
    }
    if doc.layers.iter().any(|l| l.kind == 0 && l.blend != 0 && l.effective_visible) {
        out.issues.push(Issue::warn("aseprite-blend-mode", format!("{}: layers with blend modes other than Normal are merged as Normal", path.display())).at(path));
    }

    // Pivot and 9-slice from the first slice, in canvas space.
    let (mut pivot, mut nine) = (None, None);
    if let Some(s) = doc.slices.first() {
        if let Some([px, py]) = s.pivot {
            pivot = Some([(s.bounds[0] + px) as f32 / doc.width.max(1) as f32, (s.bounds[1] + py) as f32 / doc.height.max(1) as f32]);
        }
        if let Some([cx, cy, cw, ch]) = s.center {
            nine = Some([cx.max(0) as u32, cy.max(0) as u32, (s.bounds[2] - cx - cw).max(0) as u32, (s.bounds[3] - cy - ch).max(0) as u32]);
        }
        if doc.slices.len() > 1 {
            out.issues.push(Issue::info("aseprite-slices", format!("{}: {} slices; the first (\"{}\") gives pivot / 9-slice", path.display(), doc.slices.len(), s.name)));
        }
    }

    let origin = |frame: usize, layer: Option<&str>| SourceRef { path: path.to_string_lossy().into_owned(), frame: Some(frame as u32), layer: layer.map(String::from) };
    let push = |out: &mut Loaded, name: String, img: RgbaImage, frame: usize, layer: Option<&str>| {
        out.sprites.push(SourceSprite { name, image: Arc::new(img), origin: origin(frame, layer), duration_ms: Some(doc.frames[frame].0), pivot, nine_slice: nine });
    };
    let tag_order = |t: &Tag| -> (Vec<usize>, LoopMode) {
        let fwd: Vec<usize> = (t.from..=t.to.min(doc.frames.len().saturating_sub(1))).collect();
        match t.direction {
            1 => (fwd.into_iter().rev().collect(), LoopMode::Loop),
            2 => (fwd, LoopMode::Pingpong),
            3 => (fwd.into_iter().rev().collect(), LoopMode::Pingpong),
            _ => (fwd, LoopMode::Loop),
        }
    };

    match layers {
        LayerSelection::Mode(mode) => {
            let include = |_: usize, l: &Layer| *mode == LayerMode::All || l.effective_visible;
            let frames: Vec<RgbaImage> = (0..doc.frames.len()).map(|f| composite(&doc, f, &include)).collect();
            let mut in_tag = vec![false; frames.len()];
            for t in &doc.tags {
                let (order, loop_mode) = tag_order(t);
                let tag = super::clean_name(&t.name);
                let mut names = Vec::new();
                for (i, &f) in order.iter().enumerate() {
                    in_tag[f] = true;
                    let name = format!("{base}_{tag}_{i}");
                    push(&mut out, name.clone(), frames[f].clone(), f, None);
                    names.push(name);
                }
                let durations = order.iter().map(|&f| doc.frames[f].0).collect();
                out.animations.push(anim::from_durations(tag.clone(), names, durations, loop_mode));
            }
            let untagged: Vec<usize> = (0..frames.len()).filter(|&f| !in_tag[f]).collect();
            for &f in &untagged {
                let name = if frames.len() == 1 { base.clone() } else { format!("{base}_{f}") };
                push(&mut out, name, frames[f].clone(), f, None);
            }
            if doc.tags.is_empty() && frames.len() > 1 {
                let names = untagged.iter().map(|f| format!("{base}_{f}")).collect();
                out.animations.push(anim::from_durations(base.clone(), names, untagged.iter().map(|&f| doc.frames[f].0).collect(), LoopMode::Loop));
            }
        }
        LayerSelection::Names(wanted) => {
            for w in wanted {
                let Some(idx) = doc.layers.iter().position(|l| &l.name == w) else {
                    out.issues.push(Issue::error("layer-not-found", format!("{}: no layer named \"{w}\"", path.display())).at(path));
                    continue;
                };
                let lname = super::clean_name(w);
                for f in 0..doc.frames.len() {
                    let img = composite(&doc, f, &|i, _| i == idx);
                    push(&mut out, format!("{base}_{lname}_{f}"), img, f, Some(w));
                }
                for t in &doc.tags {
                    let (order, loop_mode) = tag_order(t);
                    let names = order.iter().map(|f| format!("{base}_{lname}_{f}")).collect();
                    out.animations.push(anim::from_durations(format!("{lname}_{}", super::clean_name(&t.name)), names, order.iter().map(|&f| doc.frames[f].0).collect(), loop_mode));
                }
            }
        }
    }
    out
}

/// Writes small `.aseprite` files for tests (RGBA, compressed cels).
#[cfg(test)]
pub mod test_support {
    use std::io::Write;

    use flate2::write::ZlibEncoder;
    use image::RgbaImage;

    pub struct TestLayer<'a> {
        pub name: &'a str,
        pub visible: bool,
    }

    /// `cels[frame]` = (layer, x, y, image).
    pub fn write(w: u16, h: u16, layers: &[TestLayer], frames: &[(u16, Vec<(u16, i16, i16, RgbaImage)>)], tags: &[(&str, u16, u16, u8)], slice: Option<([i32; 4], [i32; 4], [i32; 2])>) -> Vec<u8> {
        let mut body = Vec::new();
        for (fi, (dur, cels)) in frames.iter().enumerate() {
            let mut chunks: Vec<Vec<u8>> = Vec::new();
            if fi == 0 {
                for l in layers {
                    let mut c = Vec::new();
                    c.extend((if l.visible { 1u16 } else { 0 } | 2).to_le_bytes());
                    c.extend(0u16.to_le_bytes()); // normal layer
                    c.extend(0u16.to_le_bytes()); // child level
                    c.extend([0u8; 4]);
                    c.extend(0u16.to_le_bytes()); // blend
                    c.push(255);
                    c.extend([0u8; 3]);
                    c.extend((l.name.len() as u16).to_le_bytes());
                    c.extend(l.name.as_bytes());
                    chunks.push(chunk(0x2004, c));
                }
                if !tags.is_empty() {
                    let mut c = Vec::new();
                    c.extend((tags.len() as u16).to_le_bytes());
                    c.extend([0u8; 8]);
                    for (name, from, to, dir) in tags {
                        c.extend(from.to_le_bytes());
                        c.extend(to.to_le_bytes());
                        c.push(*dir);
                        c.extend([0u8; 12]);
                        c.extend((name.len() as u16).to_le_bytes());
                        c.extend(name.as_bytes());
                    }
                    chunks.push(chunk(0x2018, c));
                }
                if let Some((bounds, center, pivot)) = slice {
                    let mut c = Vec::new();
                    c.extend(1u32.to_le_bytes());
                    c.extend(3u32.to_le_bytes());
                    c.extend(0u32.to_le_bytes());
                    c.extend(5u16.to_le_bytes());
                    c.extend(b"slice");
                    c.extend(0u32.to_le_bytes());
                    for v in bounds.iter().chain(&center).chain(&pivot) {
                        c.extend(v.to_le_bytes());
                    }
                    chunks.push(chunk(0x2022, c));
                }
            }
            for (layer, x, y, img) in cels {
                let mut c = Vec::new();
                c.extend(layer.to_le_bytes());
                c.extend(x.to_le_bytes());
                c.extend(y.to_le_bytes());
                c.push(255);
                c.extend(2u16.to_le_bytes());
                c.extend([0u8; 7]);
                c.extend((img.width() as u16).to_le_bytes());
                c.extend((img.height() as u16).to_le_bytes());
                let mut z = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                z.write_all(img.as_raw()).unwrap();
                c.extend(z.finish().unwrap());
                chunks.push(chunk(0x2005, c));
            }
            let size: usize = 16 + chunks.iter().map(Vec::len).sum::<usize>();
            body.extend((size as u32).to_le_bytes());
            body.extend(0xF1FAu16.to_le_bytes());
            body.extend((chunks.len() as u16).to_le_bytes());
            body.extend(dur.to_le_bytes());
            body.extend([0u8; 2]);
            body.extend((chunks.len() as u32).to_le_bytes());
            for c in chunks {
                body.extend(c);
            }
        }
        let mut out = Vec::new();
        out.extend(((128 + body.len()) as u32).to_le_bytes());
        out.extend(0xA5E0u16.to_le_bytes());
        out.extend((frames.len() as u16).to_le_bytes());
        out.extend(w.to_le_bytes());
        out.extend(h.to_le_bytes());
        out.extend(32u16.to_le_bytes());
        out.extend(1u32.to_le_bytes()); // layer opacity valid
        out.resize(128, 0);
        out.extend(body);
        out
    }

    fn chunk(kind: u16, data: Vec<u8>) -> Vec<u8> {
        let mut c = Vec::new();
        c.extend(((data.len() + 6) as u32).to_le_bytes());
        c.extend(kind.to_le_bytes());
        c.extend(data);
        c
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use crate::core::sources::test_support::shape;

    fn sample() -> Vec<u8> {
        let red = |_: u8| shape(4, 4, 0, 0, 4, 4, [255, 0, 0, 255]);
        let blue = shape(2, 2, 0, 0, 2, 2, [0, 0, 255, 255]);
        write(
            16,
            16,
            &[TestLayer { name: "Body", visible: true }, TestLayer { name: "FX", visible: true }, TestLayer { name: "Hidden", visible: false }],
            &[
                (100, vec![(0, 1, 1, red(0)), (1, 8, 8, blue.clone()), (2, 0, 0, blue.clone())]),
                (100, vec![(0, 2, 1, red(0))]),
                (200, vec![(0, 3, 1, red(0))]),
                (100, vec![(0, 4, 1, red(0))]),
            ],
            &[("idle", 0, 1, 0), ("attack", 2, 3, 2)],
            Some(([0, 0, 16, 16], [4, 4, 8, 8], [8, 16])),
        )
    }

    #[test]
    fn parses_layers_tags_and_slices() {
        let doc = parse(&sample()).unwrap();
        assert_eq!((doc.width, doc.height, doc.layers.len(), doc.frames.len()), (16, 16, 3, 4));
        assert_eq!(doc.tags[1].name, "attack");
        assert_eq!(doc.slices[0].pivot, Some([8, 16]));
        let f0 = composite(&doc, 0, &|_, l| l.effective_visible);
        assert_eq!(f0.get_pixel(1, 1)[0], 255, "body");
        assert_eq!(f0.get_pixel(8, 8)[2], 255, "fx");
        assert_eq!(f0.get_pixel(0, 0)[3], 0, "hidden layer left out");
    }

    #[test]
    fn names_animations_and_pivot() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("hero.aseprite");
        std::fs::write(&p, sample()).unwrap();
        let l = load(&p, &LayerSelection::default());
        assert!(l.issues.is_empty(), "{:?}", l.issues);
        let names: Vec<&str> = l.sprites.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["hero_idle_0", "hero_idle_1", "hero_attack_0", "hero_attack_1"]);
        let attack = l.animations.iter().find(|a| a.name == "attack").unwrap();
        assert_eq!(attack.loop_mode, LoopMode::Pingpong);
        assert_eq!(attack.durations_ms, Some(vec![200, 100]));
        assert_eq!(l.sprites[0].pivot, Some([0.5, 1.0]));
        assert_eq!(l.sprites[0].nine_slice, Some([4, 4, 4, 4]));

        let per_layer = load(&p, &LayerSelection::Names(vec!["FX".into(), "Nope".into()]));
        assert_eq!(per_layer.sprites.len(), 4);
        assert_eq!(per_layer.sprites[0].name, "hero_FX_0");
        assert!(per_layer.issues.iter().any(|i| i.code == "layer-not-found"));
        assert!(per_layer.animations.iter().any(|a| a.name == "FX_idle"));

        std::fs::write(&p, b"not an aseprite file at all, sorry").unwrap();
        assert_eq!(load(&p, &LayerSelection::default()).issues[0].code, "unreadable-aseprite");
    }
}

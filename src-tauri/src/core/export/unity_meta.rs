//! Unity `.png.meta` with `spriteMode: 2` (Multiple): Unity slices the atlas itself on import.
//! Target: Unity 6 (decision Q1); pixels per unit 100 by default (Q4).
//!
//! - An existing `.meta` keeps its `guid` and every other key: only `spriteMode`, the
//!   `spriteSheet.sprites` block and `spriteSheet.nameFileIdTable` are replaced (sprite ids of names
//!   already there are kept).
//! - A new `.meta`: random guid, `textureType: 8` (Sprite), point or bilinear filter, no mipmaps.
//! - Rects use Unity's bottom-left origin (`y = atlasH − y − h`); pivots are normalised in the trimmed
//!   rect; borders are `x=left, y=bottom, z=right, w=top`.
//!
//! *Verify*: written from Unity 6's TextureImporter layout without a user fixture; import once and
//! report differences (docs/PROGRESS.md).

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;

use xxhash_rust::xxh3::{xxh3_128, xxh3_64};

use super::{fmt_f, page_file, page_images, Output};
use crate::core::pipeline::{Packed, PackedSprite};
use crate::core::project::{ExportTarget, FilterMode, Project};
use crate::core::Issue;

/// Stable 32-hex sprite id from the name (same name → same id on every export).
pub fn sprite_id(name: &str) -> String {
    format!("{:032x}", xxh3_128(format!("coral.sprite:{name}").as_bytes()))
}

/// Stable non-zero int64 for `internalID` / `nameFileIdTable`.
pub fn internal_id(name: &str) -> i64 {
    let v = (xxh3_64(format!("coral.internal:{name}").as_bytes()) >> 1) as i64;
    if v == 0 {
        1
    } else {
        v
    }
}

fn new_guid(seed: &str) -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{:032x}", xxh3_128(format!("{seed}:{nanos}:{}", std::process::id()).as_bytes()))
}

/// `(x, y, w, h)` with a bottom-left origin, pivot in the trimmed rect, border L B R T.
pub struct UnityRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub pivot: [f32; 2],
    pub border: [u32; 4],
}

pub fn unity_rect(s: &PackedSprite, atlas_h: u32) -> UnityRect {
    let t = &s.trim;
    let px = s.pivot[0] * t.source_w as f32 - t.x as f32;
    let py_top = s.pivot[1] * t.source_h as f32 - t.y as f32;
    let pivot = [px / t.w.max(1) as f32, 1.0 - py_top / t.h.max(1) as f32];
    let border = match s.nine_slice {
        Some([l, top, r, b]) => {
            let right_cut = t.source_w - t.x - t.w;
            let bottom_cut = t.source_h - t.y - t.h;
            [l.saturating_sub(t.x), b.saturating_sub(bottom_cut), r.saturating_sub(right_cut), top.saturating_sub(t.y)]
        }
        None => [0; 4],
    };
    UnityRect { x: s.frame.x, y: atlas_h - s.frame.y - s.frame.h, w: s.frame.w, h: s.frame.h, pivot, border }
}

fn round4(v: f32) -> String {
    let r = (v * 10000.0).round() / 10000.0;
    if r.fract() == 0.0 {
        format!("{}", r as i64)
    } else {
        fmt_f(r)
    }
}

/// Existing `name → (spriteID, internalID)` in an old meta, to keep them.
fn old_ids(meta: &str) -> HashMap<String, (String, i64)> {
    let mut out = HashMap::new();
    let mut name: Option<String> = None;
    let mut sid: Option<String> = None;
    for line in meta.lines() {
        let l = line.trim_start();
        if let Some(n) = l.strip_prefix("name: ") {
            name = Some(n.trim().to_string());
            sid = None;
        } else if let Some(v) = l.strip_prefix("spriteID: ") {
            sid = Some(v.trim().to_string());
        } else if let Some(v) = l.strip_prefix("internalID: ") {
            if let (Some(n), Some(s), Ok(i)) = (name.take(), sid.take(), v.trim().parse::<i64>()) {
                if !s.is_empty() {
                    out.insert(n, (s, i));
                }
            }
        }
    }
    out
}

fn sprites_block(sprites: &[&PackedSprite], atlas_h: u32, ids: &HashMap<String, (String, i64)>) -> (String, String) {
    let mut s = String::from("    sprites:\n");
    let mut table = String::from("    nameFileIdTable:\n");
    if sprites.is_empty() {
        return ("    sprites: []\n".into(), "    nameFileIdTable: {}\n".into());
    }
    for sp in sprites {
        let r = unity_rect(sp, atlas_h);
        let (sid, iid) = ids.get(&sp.name).cloned().unwrap_or_else(|| (sprite_id(&sp.name), internal_id(&sp.name)));
        let _ = write!(
            s,
            "    - serializedVersion: 2\n      name: {name}\n      rect:\n        serializedVersion: 2\n        x: {x}\n        y: {y}\n        width: {w}\n        height: {h}\n      alignment: 9\n      pivot: {{x: {px}, y: {py}}}\n      border: {{x: {bl}, y: {bb}, z: {br}, w: {bt}}}\n      customData: \n      outline: []\n      physicsShape: []\n      tessellationDetail: -1\n      bones: []\n      spriteID: {sid}\n      internalID: {iid}\n      vertices: []\n      indices: \n      edges: []\n      weights: []\n",
            name = sp.name,
            x = r.x,
            y = r.y,
            w = r.w,
            h = r.h,
            px = round4(r.pivot[0]),
            py = round4(r.pivot[1]),
            bl = r.border[0],
            bb = r.border[1],
            br = r.border[2],
            bt = r.border[3],
        );
        let _ = writeln!(table, "      {}: {iid}", sp.name);
    }
    (s, table)
}

pub fn new_meta(guid: &str, sprites: &[&PackedSprite], atlas_h: u32, max_size: u32, ppu: f32, filter: FilterMode) -> String {
    let (block, table) = sprites_block(sprites, atlas_h, &HashMap::new());
    let filter_mode = if filter == FilterMode::Point { 0 } else { 1 };
    let max_tex = [32u32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384].into_iter().find(|&s| s >= max_size).unwrap_or(16384);
    format!(
        "fileFormatVersion: 2
guid: {guid}
TextureImporter:
  internalIDToNameTable: []
  externalObjects: {{}}
  serializedVersion: 13
  mipmaps:
    mipMapMode: 0
    enableMipMap: 0
    sRGBTexture: 1
    linearTexture: 0
    fadeOut: 0
    borderMipMap: 0
    mipMapsPreserveCoverage: 0
    alphaTestReferenceValue: 0.5
    mipMapFadeDistanceStart: 1
    mipMapFadeDistanceEnd: 3
  bumpmap:
    convertToNormalMap: 0
    externalNormalMap: 0
    heightScale: 0.25
    normalMapFilter: 0
    flipGreenChannel: 0
  isReadable: 0
  streamingMipmaps: 0
  streamingMipmapsPriority: 0
  vTOnly: 0
  ignoreMipmapLimit: 0
  grayScaleToAlpha: 0
  generateCubemap: 6
  cubemapConvolution: 0
  seamlessCubemap: 0
  textureFormat: 1
  maxTextureSize: 2048
  textureSettings:
    serializedVersion: 2
    filterMode: {filter_mode}
    aniso: 1
    mipBias: 0
    wrapU: 1
    wrapV: 1
    wrapW: 1
  nPOTScale: 0
  lightmap: 0
  compressionQuality: 50
  spriteMode: 2
  spriteExtrude: 1
  spriteMeshType: 1
  alignment: 0
  spritePivot: {{x: 0.5, y: 0.5}}
  spritePixelsToUnits: {ppu}
  spriteBorder: {{x: 0, y: 0, z: 0, w: 0}}
  spriteGenerateFallbackPhysicsShape: 1
  alphaUsage: 1
  alphaIsTransparency: 1
  spriteTessellationDetail: -1
  textureType: 8
  textureShape: 1
  singleChannelComponent: 0
  flipbookRows: 1
  flipbookColumns: 1
  maxTextureSizeSet: 0
  compressionQualitySet: 0
  textureFormatSet: 0
  ignorePngGamma: 0
  applyGammaDecoding: 0
  swizzle: 50462976
  cookieLightType: 0
  platformSettings:
  - serializedVersion: 4
    buildTarget: DefaultTexturePlatform
    maxTextureSize: {max_tex}
    resizeAlgorithm: 0
    textureFormat: -1
    textureCompression: 1
    compressionQuality: 50
    crunchedCompression: 0
    allowsAlphaSplitting: 0
    overridden: 0
    ignorePlatformSupport: 0
    androidETC2FallbackOverride: 0
    forceMaximumCompressionQuality_BC6H_BC7: 0
  spriteSheet:
    serializedVersion: 2
{block}    outline: []
    customData:
    physicsShape: []
    bones: []
    spriteID:
    internalID: 0
    vertices: []
    indices:
    edges: []
    weights: []
    secondaryTextures: []
    spriteCustomMetadata:
      entries: []
{table}  mipmapLimitGroupName:
  pSDRemoveMatte: 0
  userData:
  assetBundleName:
  assetBundleVariant:
",
        ppu = round4(ppu),
    )
}

/// Range of lines `[start, end)` of the block that starts with `header` at `indent` spaces: the
/// header line and every following line indented deeper (or a `- ` list item at the same indent).
fn block_range(lines: &[&str], header: &str, indent: usize, from: usize) -> Option<(usize, usize)> {
    let pad = " ".repeat(indent);
    let start = (from..lines.len()).find(|&i| lines[i].strip_prefix(&pad).is_some_and(|r| r.starts_with(header)))?;
    let mut end = start + 1;
    while end < lines.len() {
        let l = lines[end];
        let lead = l.len() - l.trim_start().len();
        let list_item = lead == indent && l.trim_start().starts_with("- ");
        if !(lead > indent || list_item || l.trim().is_empty()) {
            break;
        }
        end += 1;
    }
    Some((start, end))
}

/// Update an existing meta in place (keys other than the sprite ones untouched). None: the file has
/// no `spriteSheet:` block and is rewritten from scratch with its guid.
pub fn update_meta(old: &str, sprites: &[&PackedSprite], atlas_h: u32) -> Option<String> {
    let ids = old_ids(old);
    let (block, table) = sprites_block(sprites, atlas_h, &ids);
    let lines: Vec<&str> = old.lines().collect();
    let (sheet, _) = block_range(&lines, "spriteSheet:", 2, 0)?;
    let mut out: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
    // Replace from the bottom up so earlier indices stay valid.
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    match block_range(&lines, "sprites:", 4, sheet) {
        Some((s, e)) => edits.push((s, e, block)),
        None => edits.push((sheet + 2, sheet + 2, block)),
    }
    match block_range(&lines, "nameFileIdTable:", 4, sheet) {
        Some((s, e)) => edits.push((s, e, table)),
        None => {
            let (_, sheet_end) = block_range(&lines, "spriteSheet:", 2, 0)?;
            edits.push((sheet_end, sheet_end, table));
        }
    }
    if let Some(i) = lines.iter().position(|l| l.trim_start().starts_with("spriteMode:")) {
        let lead = lines[i].len() - lines[i].trim_start().len();
        edits.push((i, i + 1, format!("{}spriteMode: 2\n", " ".repeat(lead))));
    }
    edits.sort_by(|a, b| b.0.cmp(&a.0));
    for (s, e, text) in edits {
        let new_lines: Vec<String> = text.trim_end_matches('\n').lines().map(String::from).collect();
        out.splice(s..e.min(out.len()).max(s), new_lines);
    }
    let mut text = out.join("\n");
    text.push('\n');
    Some(text)
}

pub fn guid_of(meta: &str) -> Option<String> {
    meta.lines().find_map(|l| l.strip_prefix("guid: ")).map(|g| g.trim().to_string()).filter(|g| g.len() == 32)
}

pub fn write(target: &ExportTarget, project: &Project, packed: &Packed, out: &Path, o: &mut Output) {
    if packed.sprites.iter().any(|s| s.rotated) {
        o.issues.push(Issue::error("unity-rotated", "Unity sprite rects cannot be rotated: turn off \"Allow rotation\" for this project"));
        return;
    }
    page_images(project, packed, out, o);
    let ext = project.pack.image_format.ext();
    let n = packed.pages.len();
    let ppu = target.pixels_per_unit.unwrap_or(100.0);
    let filter = target.filter.unwrap_or_default();
    for (p, page) in packed.pages.iter().enumerate() {
        let image = out.join(page_file(project, p, n, ext));
        let meta_path = image.with_file_name(format!("{}.meta", image.file_name().unwrap().to_string_lossy()));
        let sprites: Vec<&PackedSprite> = packed.sprites.iter().filter(|s| s.page == p).collect();
        let text = match std::fs::read_to_string(&meta_path) {
            Ok(old) => update_meta(&old, &sprites, page.height).unwrap_or_else(|| {
                o.issues.push(Issue::warn("unity-meta-rewritten", format!("{}: no spriteSheet block, written anew (guid kept)", meta_path.display())).at(&meta_path));
                new_meta(&guid_of(&old).unwrap_or_else(|| new_guid(&image.to_string_lossy())), &sprites, page.height, project.pack.max_size, ppu, filter)
            }),
            Err(_) => new_meta(&new_guid(&image.to_string_lossy()), &sprites, page.height, project.pack.max_size, ppu, filter),
        };
        o.add(meta_path, "meta", text.into_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::pipeline::test_support::hero_packed;

    #[test]
    fn rects_pivots_and_borders_in_unity_space() {
        let (_, mut packed) = hero_packed();
        packed.sprites[0].nine_slice = Some([4, 6, 4, 6]);
        let r = unity_rect(&packed.sprites[0], 64);
        // Trimmed 30×44 at (1,4) of 32×48; bottom-centre pivot → (0.5, 0).
        assert_eq!((r.x, r.y, r.w, r.h), (0, 20, 30, 44));
        assert_eq!(r.pivot, [0.5, 0.0]);
        assert_eq!(r.border, [3, 6, 3, 2], "L B R T minus the trimmed margins");
    }

    #[test]
    fn new_meta_and_update_keep_the_guid() {
        let (_, packed) = hero_packed();
        let sprites: Vec<&PackedSprite> = packed.sprites.iter().collect();
        let meta = new_meta("0123456789abcdef0123456789abcdef", &sprites, 64, 2048, 100.0, FilterMode::Point);
        assert!(meta.contains("spriteMode: 2\n") && meta.contains("textureType: 8\n") && meta.contains("spritePixelsToUnits: 100\n"));
        assert!(meta.contains("      name: hero_idle_0\n") && meta.contains("      hero_idle_1: "));
        assert_eq!(meta.matches("- serializedVersion: 2").count(), 2);

        // A hand-made meta (as Unity writes it with one manual sprite and custom keys).
        let old = "fileFormatVersion: 2\nguid: ffffffffffffffffffffffffffffffff\nTextureImporter:\n  serializedVersion: 13\n  spriteMode: 1\n  spritePixelsToUnits: 32\n  spriteSheet:\n    serializedVersion: 2\n    sprites:\n    - serializedVersion: 2\n      name: hero_idle_0\n      rect:\n        x: 0\n      spriteID: aaaabbbbccccddddaaaabbbbccccdddd\n      internalID: 123456\n    outline: []\n    nameFileIdTable:\n      hero_idle_0: 123456\n  userData: keep me\n";
        let up = update_meta(old, &sprites, 64).unwrap();
        assert_eq!(guid_of(&up).as_deref(), Some("ffffffffffffffffffffffffffffffff"));
        assert!(up.contains("  spriteMode: 2\n") && up.contains("spritePixelsToUnits: 32") && up.contains("userData: keep me"));
        assert!(up.contains("spriteID: aaaabbbbccccddddaaaabbbbccccdddd") && up.contains("      hero_idle_0: 123456"), "ids of known names kept:\n{up}");
        assert!(up.contains(&format!("      hero_idle_1: {}", internal_id("hero_idle_1"))));
        assert_eq!(up.matches("
    outline: []").count(), 1);
        assert!(update_meta("fileFormatVersion: 2\nguid: x\n", &sprites, 64).is_none());
        assert_eq!(sprite_id("a"), sprite_id("a"));
        assert_ne!(internal_id("a"), internal_id("b"));
    }
}

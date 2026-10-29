# Coral — Changelog

All user-visible changes. Newest first; one section per released version.

## Unreleased (0.1.0 candidate)

- **Pack**: sources from folders (PNG / TGA / GIF), Aseprite (merged or per layer, tags, frame durations,
  slice pivot and 9-slice), PSD (per layer or flattened) and GIF; alpha trim, duplicate merging
  (aliases), MaxRects packing with 5 heuristics, padding, extrusion, rotation, power-of-two / square
  pages, multiple pages, alpha bleed, premultiply, PNG or TGA, optional normal-map atlas.
- Exports: JSON Hash / JSON Array (TexturePacker layout + `meta.coral` with fps, loops, durations,
  hitboxes, 9-slice; Paper2D extension), Godot 4 SpriteFrames (+ AtlasTexture per sprite), Unity 6
  `.png.meta` (keeps the guid), Spine / libGDX `.atlas`. Dry run first, atomic writes.
- Projects `*.coral.json` with relative paths, recent projects, Ctrl+O / Ctrl+S.
- **Watch**: re-pack and re-export on every source change (cached decoding).
- **Slice**: grid or automatic (connected shapes, merging, reading order), rect editing, name
  patterns, pivots, 9-slice; separate PNGs, JSON Hash or Unity meta for the uncut sheet.
- **Animate**: player with per-frame durations, loop / once / ping-pong, onion skin; pivot and
  hitbox / hurtbox / custom box editing per frame.
- **Flipbook**: frame sequences → even N×M sheets + JSON, frame range and step, Lanczos3 cells.
- **Pixel**: nearest upscale, palette extract / load / save (`.hex` `.gpl` `.pal`), OKLab remap, palette
  swap variants, alpha bleed (in place with `.bak`).
- **Normal 2D**: luminance or alpha-bevel height, blur, strength, OpenGL / DirectX, lit WebGL preview.
- Command line: `coral pack | slice | flipbook | bleed | palette extract|remap | upscale | normal` with
  `--report`, `--dry-run`, `--fail-on`, `--quiet`, `pack --watch`.

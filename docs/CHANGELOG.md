# Coral — Changelog

All user-visible changes. Newest first; one section per released version.

## Unreleased

## 0.1.1 — 2026-11-09

- **Help / Hướng dẫn** (`F1`, the **?** button in the header, or `Ctrl+K` → "Help / Hướng dẫn"): an
  in-app usage guide in Vietnamese and English — Getting started (layout, drag & drop, projects, how
  files are written, shortcuts, appearance, updates) and one topic per module (Pack, Slice, Animate,
  Flipbook, Pixel, Normal 2D). Opens on the current module; topic filter that ignores diacritics
  ("huong dan" finds "Hướng dẫn"); VI / EN switch, remembered (default Vietnamese). The panel is the
  shared `HelpPanel` of `@abyss/ui`; Coral's content is in `src/help/topics.ts`.
- **User guide (PDF)**: `docs/user-guide/Coral_User_Guide_vi.pdf` and `_en.pdf`, built from the same
  content as the Help panel by `npm run guide` (Abyss `scripts/user-guide.mjs`, prints with Edge / Chrome) and
  bundled with the app. Open it with the **PDF** button in the Help panel header (in the panel's language) or
  `Ctrl+K` → "User guide (PDF) / Tài liệu hướng dẫn"; it opens in the default PDF viewer.
- **Fix — Pixel → Bleed** no longer overwrites the inputs without a `.bak` when the output folder is
  empty ("next to each input") or is the inputs' own folder: outputs are now `<name>_bleed.png`. *In
  place* still overwrites, with a `.bak` first. Same for `coral bleed --out`.

## 0.1.0 — 2026-10-29

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

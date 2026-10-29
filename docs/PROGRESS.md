# Coral — Progress

## Status
- Current phase: P11 — Polish (code done; waiting on the user for engine checks and the release)
- Last session: 2026-10-29 — template upgrade (Abyss T1–T5) then Coral P0–P11 in one session
- `npm run check`: green (typecheck, 32 commands mocked, 52 Rust tests; 3 more `#[ignore]`: 2 benchmarks, sample generator)
- Release build: `npm run tauri build` OK; `coral.exe --help` and `coral.exe pack sample-assets/sample.coral.json` checked (see below)

## Phases
- [x] P0 Scaffold — 6 modules on the rail (Ctrl+1…6), crates, `sample-assets/` (generated: `gen_sample_assets`)
- [x] P1 Sources, trim, dedupe, project — folder / Aseprite / PSD / GIF, natural sort, names per spec §3.2, `duplicate-name`, round-trip + migration tests
- [x] P2 Packer — own MaxRects (5 heuristics, rotation), size search, multi-page, padding, extrude, bleed; property test (500 random rects, 5 configurations); benchmark 1000 × 64 px < 1 ms release (target < 300 ms)
- [x] P3 Exporters — JSON Hash, JSON Array, Godot `.tres`, Unity `.meta`, Spine `.atlas`, atomic writes, dry run — **engine import by the user still to do** (see next steps)
- [x] P4 CLI + watch — `coral pack` (+ `--target --dry-run --report --watch --fail-on --quiet`), watch action `packProject`, `parses_flags` / `runs_pack` / IPC watch test; re-pack of 200 cached sprites ≈ 35 ms (target < 1 s)
- [x] P5 UI Pack + mock — drop → unsaved project, atlas view 25–800 %, hover / select, settings, exports, Pack & Export (dry run → confirm), JobProgress, WatchToggle; release 0.1.0 **not published** (needs the user's go-ahead)
- [x] P6 Slice — grid / auto (6-shape ordering test), rect editing, names, pivots, 9-slice, PNGs / JSON / Unity meta
- [x] P7 Animate — player, durations, onion skin, pivot, hitboxes per frame (copy to next), FORMATS.md#hitboxes with Godot / Unity snippets
- [x] P8 Flipbook — natural sort, range / step, suggested grid, Lanczos3, JSON; 64 frames → 8×8 order test
- [x] P9 Pixel — upscale, palette extract / `.hex` `.gpl` `.pal`, OKLab remap (4-colour test), swap tables (saved in the project), bleed
- [x] P10 Normal 2D — luminance / alpha bevel, blur, Sobel, OpenGL / DirectX, direction test, normal atlas with the same rects, WebGL preview
- [ ] P11 Polish — user guide written (`docs/user-guide/`, no screenshots yet); 2000-sprite open-time check and the v0.2.0 release not done

## Decisions (answers to the spec's open questions)
- 2026-10-29 — Q1 Unity: **Unity 6**; no real `.meta` fixture provided yet (user)
- 2026-10-29 — Q2 Atlas images: **PNG and TGA** (`pack.imageFormat`) (user)
- 2026-10-29 — Q3 Unreal Paper2D: **not used** — the `.paper2dsprites` option exists, no import check (user)
- 2026-10-29 — Q4 Unity pixels per unit: **100** by default (user)
- 2026-10-29 — Q5 Godot: **one SpriteFrames file** with every animation (user)
- 2026-10-29 — Aseprite reader written in-house (`core/sources/aseprite.rs`, from the published file spec) instead of `asefile`: asefile 0.3.8 pins `image` 0.24, a second copy of the image codecs next to our 0.25. Tested with files written by our own test writer (layers, linked / compressed cels, tags, slices).
- 2026-10-29 — Release profile without `panic = "abort"`: the `psd` crate may panic on unusual files; the panic is caught per file and reported as `unreadable-psd` instead of closing the app.
- 2026-10-29 — Packer layout code is in `core/pack/mod.rs` (spec named it `pack/atlas.rs`); MaxRects in `pack/maxrects.rs`.
- 2026-10-29 — `pack_preview` returns the report only; page images come from `preview_png("atlas:<i>")` (a command cannot return JSON and bytes together). Same for flipbook (`"flipbook"`) and normal (`"normal"`) previews. Extra commands: `sources_for_paths`, `image_png` (TGA cannot be shown by the webview), `palette_load`, `palette_save`, `normal_preview`.
- 2026-10-29 — Rotated sprites: JSON `frame.w/h` and Spine `size` are the unrotated size, stored 90° clockwise (TexturePacker / libGDX convention). Godot and Unity refuse rotated atlases.
- 2026-10-29 — Default pivot 0.5, 0.5 (TexturePacker's default); Aseprite slice pivots and project overrides replace it.
- 2026-10-29 — Default PSD mode for dropped files: `layers`; default folder include: `*.png, *.tga`.

## Known issues / next steps
- **Engine checks (user)**, spec P3 "Xong khi": import `sample-assets` exports into at least one engine
  and write the result here:
  - Godot 4: open `hero.tres` (margin of trimmed frames, durations, ping-pong unrolled).
  - Unity 6: import `hero.png` + `.meta` (rects, pivots, borders); a real hand-cut `.meta` for
    `src-tauri/tests/fixtures/unity/` would let the test compare against it.
  - Spine / libGDX: load `hero.atlas` (offsets, rotated regions).
  - Phaser 3 / PixiJS: load the JSON Hash (rotated frames).
- Release `0.1.0` (spec P5): `npm run release -- -Version 0.1.0 -Notes "…"`, then `-Publish`, `git init`,
  `gh repo create HoangTrongPhi/Coral` and the catalog status — only with the user's go-ahead. Back up
  `%USERPROFILE%\.tauri\coral.key`.
- Aseprite reader: blend modes other than Normal are merged as Normal (warn); tilemap layers skipped;
  not yet tried on files from the latest Aseprite release (fixture wanted).
- PSD: the `psd` crate reads pixel layers only (no effects / smart objects); group visibility is not
  applied to child layers.
- P11: screenshots for the user guide; check opening a 2000-sprite project < 2 s (the sprite list
  renders up to 3000 rows without virtualisation — add `@tanstack/react-virtual` if that is too slow).
- Thumbnails in Pack / Animate are cut from the atlas preview; `sprite_thumbnail` exists for other uses.

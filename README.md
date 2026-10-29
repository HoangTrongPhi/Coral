# Coral

Sprite atlas & 2D animation toolkit — part of the Phineas suite, installed and updated by [Abyss](https://github.com/HoangTrongPhi/Abyss).
Tauri 2 + Rust + React, Studio Dark UI from `@abyss/ui`. What it does and how it is built:
[docs/SPEC.md](docs/SPEC.md); where it stands: [docs/PROGRESS.md](docs/PROGRESS.md); file formats:
[docs/FORMATS.md](docs/FORMATS.md); how to use each module: [docs/user-guide/](docs/user-guide/README.md).

| Module | |
|---|---|
| **Pack** (Ctrl+1) | Folders of PNG / TGA, `.aseprite`, `.psd`, `.gif` → trimmed, deduplicated atlas pages (MaxRects, padding, extrusion, alpha bleed) → JSON Hash / Array (Phaser, PixiJS, Paper2D), Godot SpriteFrames, Unity `.meta`, Spine `.atlas`. Watch re-packs on save. |
| **Slice** (Ctrl+2) | Cut an existing sheet by grid or automatically, edit rects, names, pivots, 9-slice; write PNGs or metadata for the uncut sheet. |
| **Animate** (Ctrl+3) | Play animations (fps, loop / ping-pong, per-frame durations, onion skin), set pivots, draw hitboxes / hurtboxes per frame. |
| **Flipbook** (Ctrl+4) | Numbered frames → even N×M SubUV sheet + JSON for Niagara / Unity particles. |
| **Pixel** (Ctrl+5) | Nearest upscale, palettes (`.hex` `.gpl` `.pal`), OKLab remap, palette-swap variants, alpha bleed. |
| **Normal 2D** (Ctrl+6) | Normal maps from luminance or an alpha bevel, OpenGL / DirectX, lit WebGL preview; normal-map atlas with the same rects. |

## Commands

```bash
npm install
npm run check          # typecheck + mock coverage + cargo test: green before every hand-off
npm run dev            # UI only; open http://localhost:1452/?mock for the fake backend
npm run tauri dev      # desktop app with hot reload
npm run tauri build    # release build (unsigned updater artifacts fail; use npm run release)
npm run release -- -Version 0.1.0 -Notes "First release" [-Publish]
cargo test --manifest-path src-tauri/Cargo.toml gen_sample_assets -- --ignored   # sample-assets/
cargo test --manifest-path src-tauri/Cargo.toml --release -- --ignored bench --nocapture
```

`@abyss/ui` is linked from `file:../Abyss/packages/abyss-ui`; keep the Abyss folder next to this one (`npm install` in
Abyss first).

## Command line

The same exe runs headless when its first argument is a subcommand (anything else opens the window,
e.g. a `.coral.json` from Explorer's "Open with"):

```powershell
coral.exe pack hero.coral.json --report pack.json             # pack + every export of the project
coral.exe pack hero.coral.json --target godot --dry-run       # one target, list the files only
coral.exe pack hero.coral.json --watch                        # again on every source change; Ctrl+C stops
coral.exe slice sheet.png --grid 32x32 --offset 0,0 --spacing 0,0 --out sprites
coral.exe slice sheet.png --auto --merge 2 --meta unity       # .png.meta next to the sheet
coral.exe flipbook fx\explosion --cols 8 --rows 8 --cell 256 --from 1 --to 64 --out T_Explosion_SubUV.png --json
coral.exe bleed atlas.png --in-place                          # .bak first
coral.exe palette extract art --out art.hex
coral.exe palette remap art --palette endesga-32.hex --out art_remapped --dry-run
coral.exe upscale icons --factor 4 --out icons_x4
coral.exe normal sprites --strength 2 --convention directx --source alpha-bevel --out normals
coral.exe --help
```

Every subcommand takes `--report <file.json>` (the same structure the app shows), `--fail-on warn|error`
and `--quiet`; the writing ones take `--dry-run`.

| Exit code | Meaning |
|---|---|
| 0 | Done; no issue at the `--fail-on` level (default `error`) |
| 1 | Ran, but found issues at that level: an atlas that does not fit, an unreadable source, duplicate sprite names, a failed write |
| 2 | Bad arguments, a broken project file or missing input |

In `cmd.exe` use `start /wait coral.exe …` to wait for the exit code; PowerShell and CI call it
directly. In GitHub Actions:

```yaml
- name: Pack sprites
  run: '& "$env:LOCALAPPDATA\Coral\coral.exe" pack art/hero.coral.json --report pack.json --fail-on warn'
  shell: pwsh
```

## Mock mode

`npm run dev`, then `http://localhost:1452/?mock`: `src/dev/mockBackend.ts` answers every command in
`src/lib/ipc.ts` with canvas-drawn sprites and deliberate problems (an unreadable PSD, a blend-mode
warning, a stale override), emits progress so Cancel works and fakes the dialog / updater plugins.
Only in dev builds with `?mock` in the URL, never inside Tauri. `npm run check:mock` fails when a
command has no mock case.

## Watch

The Pack module's **Watch** (and `coral pack --watch`) re-packs and re-exports when a source or the
project file changes: debounced, files must stop changing first, the export folders and editor temp
files are ignored, changes during a run cause one more run. Decoded sources are cached by
`(path, mtime, size)`, so only changed files are read again (200 cached 64×64 sprites re-pack in
~35 ms). Watches stop when the app closes; `"watch": { "autoStart": true }` in a project starts one
when it is opened.

## Project layout

```
src/                     React UI
  components/tabs/       PackTab, SliceTab, AnimateTab, FlipbookTab, PixelTab, NormalTab
  components/common/     ZoomCanvas (pixel-perfect zoom), Fields (inputs, issue / file lists)
  stores/                appStore, projectStore (project, preview, export, watch), toolStores
  lib/ipc.ts             every invoke() and event name
  types/coral.ts         TypeScript mirrors of the Rust structs
src-tauri/src/
  main.rs / cli.rs       window or command line, same exe
  commands/              project_cmd, tools_cmd, watch_cmd, app_cmd (+ contract tests in mod.rs)
  core/                  project, sources/{folder,aseprite,psd,gif}, trim, pack/{mod,maxrects},
                         bleed, pipeline, export/{json,godot,unity_meta,spine}, slice, anim,
                         flipbook, palette, normal2d (no tauri imports)
  utils/                 logbus, jobs, watch, paths
docs/                    SPEC, PROGRESS, CHANGELOG, FORMATS, user-guide/
```

## Releasing

`npm run release` follows the suite's release standard (Abyss `docs/RELEASE_STANDARD.md`): signed NSIS
installer, `latest.json`, `SHA256SUMS.txt`, GitHub release `v<version>` on `HoangTrongPhi/Coral`. The signing key is
`%USERPROFILE%\.tauri\coral.key` — back it up. In GitHub Actions, `.github/workflows/release.yml`
does the same with the key in the `TAURI_SIGNING_PRIVATE_KEY` secret.

After the first release, set `"status": "available"` for `coral` in Abyss `catalog/catalog.json` and
bump its `revision`.

# Coral — File formats

Every file Coral reads or writes. Conventions (Abyss `docs/app-specs/00_SHARED_FRAMEWORK.md` §6):
a `"format"` and `"version"` field, paths relative to the file, 2-space indent, stable key order,
unknown fields ignored, missing fields take their defaults.

## Project — `*.coral.json` (`format: "coral.project"`, `version: 1`)

```json
{
  "format": "coral.project",
  "version": 1,
  "name": "hero",
  "sources": [
    { "kind": "folder", "path": "art/hero", "recursive": true, "include": ["*.png", "*.tga"] },
    { "kind": "aseprite", "path": "art/hero_fx.aseprite", "layers": "visible" },
    { "kind": "aseprite", "path": "art/fx2.aseprite", "layers": ["Body", "FX"] },
    { "kind": "psd", "path": "art/ui.psd", "mode": "layers" },
    { "kind": "gif", "path": "art/blink.gif" }
  ],
  "sprites": { "hero_idle_0": { "pivot": [0.5, 1.0], "nineSlice": [4, 4, 4, 4] } },
  "animations": [
    { "name": "idle", "frames": ["hero_idle_0", "hero_idle_1"], "fps": 8, "loop": "loop", "durationsMs": null }
  ],
  "hitboxes": { "hero_attack_3": [{ "kind": "hitbox", "rect": [12, 4, 20, 10], "name": "sword" }] },
  "pack": {
    "maxSize": 2048, "powerOfTwo": true, "square": false, "allowRotation": false,
    "padding": 2, "extrude": 1, "trim": "alpha", "trimThreshold": 0, "alphaBleed": true,
    "dedupe": true, "premultiply": false, "heuristic": "bestShortSideFit", "multiPage": true,
    "imageFormat": "png", "normalMaps": false,
    "normal": { "source": "luminance", "strength": 2, "blur": 0, "bevel": 4, "convention": "opengl", "invert": false }
  },
  "exports": [
    { "target": "jsonHash", "out": "build/hero", "paper2d": false },
    { "target": "jsonArray", "out": "build/hero-array" },
    { "target": "godot", "out": "../game/assets/hero", "godotProject": "../game", "atlasTextures": false },
    { "target": "unity", "out": "../UnityGame/Assets/Art/hero", "pixelsPerUnit": 100, "filter": "point" },
    { "target": "spine", "out": "build/spine/hero", "filter": "bilinear" }
  ],
  "swaps": [{ "variant": "red", "map": [["#4f8cff", "#e43b44"]] }],
  "watch": { "autoStart": false }
}
```

| Field | Meaning |
|---|---|
| `sources[].kind` | `folder` (PNG/TGA/GIF files, `include` globs on the file name), `aseprite`, `psd`, `gif` |
| `aseprite.layers` | `"visible"` (visible layers merged), `"all"` (every layer merged), or a list of names (one sprite per layer) |
| `psd.mode` | `"layers"`: one sprite per visible pixel layer, named after it; `"composite"`: the flattened image |
| `sprites` | per-sprite overrides: `pivot` normalised in the **untrimmed** sprite, top-left origin; `nineSlice` = `[left, top, right, bottom]` px |
| `animations` | replace automatic animations of the same name; `loop` = `loop` \| `once` \| `pingpong`; `durationsMs` per frame or `null` |
| `hitboxes` | per sprite; see [Hitboxes](#hitboxes) |
| `pack.imageFormat` | `png` or `tga` (decision Q2) |
| `pack.normalMaps` | also pack `<name>_n.<ext>` with the same rects; settings in `pack.normal` |
| `exports[].godotProject` | folder of `project.godot`; when missing, searched upward from `out` |
| `exports[].pixelsPerUnit` | Unity, default 100 (decision Q4) |
| `swaps` | palette swap tables saved from the Pixel module |

Older files: a missing `version` is read as 0 and migrated (plain folder strings in `sources`
become folder sources). A newer `version` is refused with a message, never silently.

### Sprite names

- Folder: path below the folder without the extension, sub-folders as `/` (`fx/spark_0`).
- Aseprite: `<file>_<tag>_<i>` (i = index inside the tag), `<file>_<frame>` outside tags, `<file>` for a
  single frame; per layer: `<file>_<layer>_<frame>`.
- PSD layers: the layer name (characters other than letters, digits, `_ - . /` become `_`).
- GIF: `<file>_<i>`.
- Two sprites with the same name: error `duplicate-name` (no silent renaming).

### Automatic animations

Aseprite tags (direction: forward, reverse, ping-pong; frame durations) and numbered names
(`run_0…run_n`, `walk0001…`, natural order). The most common frame time gives `fps`; the others are
kept in `durationsMs`.

## Exports

### JSON Hash / JSON Array (`<name>.json`)

TexturePacker's layout (Phaser 3, PixiJS). `frame` = pixels in the atlas, `spriteSourceSize` = where
they sit in the untrimmed sprite, `sourceSize` = untrimmed size, `pivot` normalised. A rotated sprite
(`rotated: true`) is stored 90° clockwise; `frame.w/h` are its unrotated size. Several pages:
`<name>-0.json` … with `meta.related_multi_packs`. `meta.coral` carries what TexturePacker has no place
for:

```json
"meta": {
  "app": "https://github.com/HoangTrongPhi/Coral", "version": "1.0", "image": "hero.png",
  "format": "RGBA8888", "size": { "w": 256, "h": 128 }, "scale": "1",
  "coral": {
    "fps": { "idle": 8 },
    "loop": { "idle": "loop" },
    "durationsMs": { "attack": [100, 100, 300] },
    "hitboxes": { "hero_attack_3": [{ "kind": "hitbox", "x": 12, "y": 4, "w": 20, "h": 10, "name": "sword" }] },
    "nineSlice": { "button": { "left": 4, "top": 4, "right": 4, "bottom": 4 } }
  }
}
```

With `paper2d: true` the file is `<name>.paper2dsprites` for Unreal's Paper2D importer (not verified:
decision Q3).

### Godot 4 (`<name>.tres`, SpriteFrames)

One `AtlasTexture` sub-resource per frame (`region` = frame, `margin` = `Rect2(trim.x, trim.y,
sourceW − w, sourceH − h)` for trimmed sprites), one entry per animation. Frame `duration` is relative:
`durationMs / (1000 / fps)`. Ping-pong is unrolled (0 1 2 3 2 1); `once` → `"loop": false`. Without
animations, all sprites go into `default` at 5 fps. Rotated atlases are refused (`godot-rotated`).
`atlasTextures: true` also writes `<sprite>.tres` per sprite (`/` in names becomes `_`).

### Unity (`<name>.png.meta`)

Sprite Mode Multiple for Unity 6 (decision Q1). Rects use the bottom-left origin
(`y = atlasH − y − h`), pivots are normalised in the trimmed rect (`alignment: 9`), borders are
`x = left, y = bottom, z = right, w = top`. `spriteID` (32 hex) and `internalID` are hashes of the sprite
name, so they stay the same across exports. An existing `.meta` keeps its `guid` and every other key;
only `spriteMode`, `spriteSheet.sprites` and `spriteSheet.nameFileIdTable` are replaced, and ids of
sprites already there are kept.

### Spine / libGDX (`<name>.atlas`)

The classic text format: per page the image name, `size`, `format`, `filter`, `repeat`; per region
`rotate`, `xy`, `size`, `orig`, `offset` (bottom-left origin of the untrimmed sprite:
`offset.y = sourceH − trim.y − h`) and `index: -1`.

## Hitboxes

Stored in the project (`hitboxes`) and exported in `meta.coral.hitboxes` of the JSON exports:
`kind` (`hitbox`, `hurtbox` or `custom:<name>`), `x, y, w, h` in **untrimmed sprite pixels, top-left
origin** (independent of trimming and of where the sprite sits in the atlas), optional `name`.

Reading them in Godot (GDScript):

```gdscript
var meta = JSON.parse_string(FileAccess.get_file_as_string("res://assets/hero/hero.json"))["meta"]["coral"]
func boxes_for(frame_name: String) -> Array:
    var out := []
    for b in meta["hitboxes"].get(frame_name, []):
        # Sprite2D centred: shift by half the untrimmed size.
        out.append({ "kind": b["kind"], "rect": Rect2(b["x"] - 16, b["y"] - 24, b["w"], b["h"]) })
    return out
```

Reading them in Unity (C#, with Newtonsoft JSON):

```csharp
var meta = JObject.Parse(File.ReadAllText(path))["meta"]["coral"];
foreach (var b in meta["hitboxes"]?[frameName] ?? new JArray()) {
    // Unity is y-up: flip inside the untrimmed sprite (sourceH from frames[frameName].sourceSize.h).
    var rect = new Rect((float)b["x"], sourceH - (float)b["y"] - (float)b["h"], (float)b["w"], (float)b["h"]);
    var worldRect = new Rect(rect.position / pixelsPerUnit, rect.size / pixelsPerUnit);
}
```

## Flipbook (`<out>.json`)

```json
{ "image": "T_Explosion_SubUV.png", "cols": 8, "rows": 8, "frames": 64, "fps": 30, "cell": { "w": 256, "h": 256 } }
```

Cells are filled left to right, top to bottom, frames in natural order. Niagara SubUV: Sub Image Size =
cols × rows. Unity Texture Sheet Animation: Tiles X = cols, Y = rows.

## Normal maps

`<name>_n.png`, RGB = n·0.5 + 0.5. **OpenGL** (default; Godot, Unity): green up — a height rising toward
+x gives R < 128, one rising toward the bottom of the image gives G > 128. **DirectX** (Unreal) flips
green. Transparent pixels are flat `(128, 128, 255)` with alpha kept. Height: luminance, or an alpha
bevel (distance from the transparent edge, `bevel` px to full height), optional Gaussian blur.

## Palettes

`.hex` (Lospec: one `rrggbb` per line), `.gpl` (GIMP: `GIMP Palette` header, `R G B name` lines),
`.pal` (JASC-PAL: header, `0100`, count, `R G B` lines). Remapping uses the nearest colour in OKLab and
keeps alpha.

## Reports (`--report`)

The command line writes the same structure the app shows. Every report has `elapsedMs`, `ok`,
`cancelled` and `issues[]` of `{ severity: "info" | "warn" | "error", code, message, path? }`.
`coral pack` writes a `PackReport` (`pages`, `sprites` with frame / trim / pivot / alias, `animations`,
`files` with `planned` / `written` status); the tools write a `WriteReport` (`files`, `notes`).

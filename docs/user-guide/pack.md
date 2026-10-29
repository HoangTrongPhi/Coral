# Pack

1. **Add sources**: drop a folder of PNG / TGA files, `.aseprite`, `.psd` or `.gif` files onto the window
   (a loose PNG brings its folder), or use the folder button of *Sources*. Click a source to edit it:
   sub-folders and include globs for folders, merged or per-layer for Aseprite, layers or flattened
   for PSD.
2. The **atlas preview** updates as you edit. Zoom 25–800 %, hover a sprite to see its name, click it
   to see its frame, trim and pivot; pages appear as tabs when one page is not enough. The left list
   shows every sprite (thumbnails cut from the atlas); `alias` marks sprites with the same pixels as
   another one — they share one rect.
3. **Pack settings** (right): max size, PNG or TGA, padding, extrude (repeats edge pixels; stops
   bleeding with filtering), trim threshold, power of two, square, multiple pages, rotation (not for
   Godot or Unity), alpha bleed, premultiply, normal-map atlas, placement heuristic.
4. **Exports**: + adds a target and asks for its folder.
   - *JSON Hash / Array*: Phaser 3, PixiJS; tick *Paper2D* for Unreal's `.paper2dsprites`.
   - *Godot*: `hero.tres` (SpriteFrames) next to the atlas; the folder must be inside a Godot project
     (`project.godot` is searched upward, or set it). Optional: one AtlasTexture per sprite.
   - *Unity*: the atlas + `hero.png.meta` (Sprite Mode Multiple, pixels per unit, point / bilinear).
     Exporting again keeps the meta's guid, so references in scenes survive.
   - *Spine*: `hero.atlas` (libGDX format).
5. **Pack & Export** lists the files it will write; *Write files* writes them. Errors (an unreadable
   source, duplicate names, a sprite larger than the atlas) block writing — fix them first; the
   *Issues* card says what and where.
6. **Save** the project (`hero.coral.json`, paths relative to it, so it can live in the game repo) and
   run the same pack from a build script: `coral pack hero.coral.json`.
7. **Watch**: after the first export, *Watch* packs and exports again whenever a source file or the
   saved project changes (save in Aseprite → the engine hot-reloads). The pill shows the last run; it
   turns red on an error — click it for the log.

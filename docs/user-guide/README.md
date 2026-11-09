# Coral user guide

The full guide as a PDF: [Coral_User_Guide_vi.pdf](Coral_User_Guide_vi.pdf) / [Coral_User_Guide_en.pdf](Coral_User_Guide_en.pdf)
(built from the in-app Help by `npm run guide`; also the **PDF** button in the Help panel).

| Page | Module |
|---|---|
| [pack.md](pack.md) | Pack — sources, atlas, exports, watch |
| [slice.md](slice.md) | Slice — cut a sheet |
| [animate.md](animate.md) | Animate — playback, pivots, hitboxes |
| [flipbook.md](flipbook.md) | Flipbook — VFX SubUV sheets |
| [pixel.md](pixel.md) | Pixel — upscale, palettes, swaps, bleed |
| [normal.md](normal.md) | Normal 2D — normal maps |

Everywhere: **F1** opens the in-app Help / Hướng dẫn (Vietnamese / English, on the current module's topic),
**Ctrl+1…6** switch modules, **Ctrl+K** opens the command palette (modules, actions,
recent projects), **Ctrl+O / Ctrl+S / Ctrl+Shift+S** open / save / save the project as, **F5** runs the
current module again (never writes without asking), **Ctrl+`** shows the log. Drop files anywhere
on the window: they go to the module you are in.

Every tool that writes files shows the list first (a dry run) and writes only after you confirm.
Files are written atomically (`.tmp`, then renamed); "in place" tools copy each input to `.bak` first.
Coral never deletes your files.

# Pixel

Drop images or folders (folders include sub-folders). Before / after shows the first input and its
result. *Dry run* lists the files; the main button writes them. Output: a folder (default: next to each
input, with a suffix) or *In place* (each input copied to `.bak` first).

- **Upscale**: nearest neighbour ×2…×8 (`<name>_x4.png`).
- **Remap**: every colour moves to the nearest palette colour (OKLab, alpha kept). Build the palette
  with *Extract from inputs* (by use or by hue), load a Lospec `.hex`, GIMP `.gpl` or JASC `.pal`, add
  colours with +, click a swatch to remove it, save it with ⤓. The result lists the pixels changed and
  which colours were merged into which.
- **Swap**: tables of `from → to` colours; each table makes variants `<name>_<variant>.png` and never
  touches the source. Click a palette swatch to add a pair, then pick the target colour. *Save to
  project* stores the tables in the open project; *From project* loads them.
- **Bleed**: transparent pixels take their neighbours' colour, alpha stays 0 — no dark fringes with
  filtering or mipmaps.

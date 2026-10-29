# Slice

1. Drop a sheet (PNG, TGA, GIF) or *Choose a sheet…*.
2. **Automatic** finds every separate shape (8-connected alpha); shapes closer than *Merge within* px
   become one sprite, specks under *Ignore under* px are skipped. Rects come in reading order (rows top
   to bottom, left to right). **Grid** cuts cells of a fixed size with an offset and gaps; empty cells
   are dropped unless *Keep empty cells* is on. Press **Detect**.
3. Edit: drag a rect to move it, its corner to resize it, *+ Rect* then drag to add one, the bin to
   delete the selected one. Set its name, pivot (9 presets or exact values; *Use this pivot for every
   rect*) and 9-slice borders.
4. **Names**: a pattern such as `{base}_{i}` (`{base}` = the sheet's name, `{i}` index, `{i:03}` padded,
   `{i1}` from 1) → *Rename all*.
5. **Write**: *Separate PNGs* into a folder, or metadata for the uncut sheet — *JSON Hash* or *Unity
   .meta* (next to the sheet; Unity slices it on import). The file list comes first; *Write files*
   writes them.

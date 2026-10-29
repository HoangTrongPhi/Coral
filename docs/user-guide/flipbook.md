# Flipbook

1. Drop a folder of numbered frames (`explosion_0001.png … explosion_0064.png`); frames are sorted
   naturally. The output defaults to `T_Explosion_SubUV.png` next to the folder.
2. Choose the frames (*From* / *To*, 1-based, and *Step*), the grid (*Columns* / *Rows*; 0 = the closest
   to square) and the *Cell* size (longer side, resized with Lanczos3; 0 = the frames' size). Cells
   shrink to keep the sheet within *Max size*. No trim, no padding: every cell is the same size.
3. The preview shows the sheet and plays it cell by cell at the chosen FPS, with the engine settings:
   Niagara SubUV *Sub Image Size* = cols × rows; Unity Texture Sheet Animation *Tiles X / Y*.
4. *Write flipbook* writes the PNG (or TGA, by the file extension) and, if ticked, a `.json` with
   `cols`, `rows`, `frames`, `fps` and `cell`.

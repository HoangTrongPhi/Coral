# Normal 2D

1. Drop sprites (or folders). The preview shows the first one lit by a light that follows the pointer,
   next to its normal map; switch images with the buttons under the preview.
2. **Height from** *Luminance* (bright = raised) or *Alpha bevel* (raised from the sprite's edge inward,
   *Bevel* px wide — good for flat-coloured sprites). *Strength* steepens the slopes, *Blur* softens
   them, *Invert* swaps high and low.
3. **Green channel**: *OpenGL* for Godot and Unity, *DirectX* for Unreal.
4. *Write normal maps* writes `<name>_n.png` for every input. Transparent pixels are flat.
5. *Use for the project's normal atlas* turns on the normal-map atlas in Pack with these settings:
   `hero_n.png` gets the same rects as `hero.png`, so one atlas description serves both.

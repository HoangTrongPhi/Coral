# Animate

Works on the project open in Pack (its atlas preview). Animations come from Aseprite tags and numbered
names (`run_0…`, `walk0001…`); editing one stores it in the project (marked *edited*).

- **Player**: Play / Pause, ◀ ▶ step, FPS, *Loop* / *Once* / *Ping-pong*, onion skin (1–3 frames
  before and after at 30 %). The timeline under the frame shows every frame with its duration in ms;
  type a number to change one frame. *Same duration for every frame* drops the per-frame values.
- **Pivot**: pick the crosshair tool and click or drag on the frame, or use the 9 presets.
  *Apply to the whole animation* sets it on every frame (else only the current one). Pivots are stored
  normalised in the untrimmed sprite and exported to every target.
- **Boxes**: choose *Hitbox*, *Hurtbox* or *Custom* (with a name), pick the box tool and drag on the
  frame. Select a box with the arrow tool to name or delete it; the copy button copies the frame's
  boxes to the next frame and moves there. Boxes are exported in `meta.coral.hitboxes` of the JSON
  exports — untrimmed sprite pixels, top-left origin; see [FORMATS.md](../FORMATS.md#hitboxes) for
  reading them in Godot and Unity.

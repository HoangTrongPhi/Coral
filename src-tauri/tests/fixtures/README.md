# Test fixtures

None are checked in: every test builds its data in code (images with `image::RgbaImage`, Aseprite
files with `core::sources::aseprite::test_support::write`, GIFs with `image`'s encoder, projects as
strings).

Wanted from the user, then added here (under 200 KB each, with their origin):

- `unity/sheet.png.meta` — a real Unity 6 meta of a 2-sprite sheet cut by hand (Q1), to compare the
  structure written by `core/export/unity_meta.rs`.
- `aseprite/*.aseprite` — files saved by the current Aseprite release (layers, tags, slices), to check
  the reader against real chunks.
- `psd/*.psd` — a PSD with pixel, group and adjustment layers.

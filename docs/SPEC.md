# Coral — Sprite atlas & 2D animation toolkit

> **Ưu tiên**: 1 · **Dùng cho**: game 2D, UI, VFX flipbook cho game 3D · **MVP**: hết P5 (v0.1.0)
> **Đọc trước**: `00_SHARED_FRAMEWORK.md`. Tiền điều kiện: `00_TEMPLATE_UPGRADE.md` đã xong.
> **Tạo app**:
> `npm run new-app -- "Coral" --tagline "Sprite atlas & 2D animation toolkit" --category "2D Pipeline" --spec docs/app-specs/01_CORAL.md`

## 1. Mục tiêu

Một app thay được TexturePacker cộng phần lớn việc vặt 2D hằng ngày:

1. Pack sprite thành atlas.
2. Cắt sheet có sẵn.
3. Xem trước animation, chỉnh pivot và hitbox.
4. Ghép flipbook cho VFX.
5. Công cụ pixel art.
6. Sinh normal map 2D.

Mọi thứ xuất thẳng ra định dạng engine đọc được, chạy được từ CLI, và tự pack lại khi file nguồn đổi.

### Ngoài phạm vi v1
- Import video (cần ffmpeg).
- Sinh file `.anim` / AnimatorController cho Unity.
- Nén GPU (DDS/KTX2) — atlas xuất PNG hoặc TGA.
- Vẽ hoặc sửa pixel (Coral không phải trình vẽ).

## 2. Người dùng và luồng chính

| # | Luồng | Kết quả |
|---|---|---|
| U1 | Kéo thư mục PNG (hoặc `.aseprite`, `.psd`) vào → Pack → Export Godot | `hero.png` + `hero.tres` (SpriteFrames) trong project Godot |
| U2 | Mở project `hero.coral.json`, bật Watch → sửa sprite trong Aseprite và lưu | Atlas tự pack lại trong ≤ 2 giây, engine hot-reload |
| U3 | Kéo một sheet PNG cũ vào → Slice tự động → chỉnh pivot → Export | Danh sách sprite có tên và pivot, xuất JSON/Unity |
| U4 | Tab Animate: chọn tag `attack`, vẽ hitbox ở frame 3–5 | Hitbox nằm trong JSON xuất ra |
| U5 | Kéo thư mục `explosion_0001..0064.png` → Flipbook 8×8 | `T_Explosion_SubUV.png` + JSON (cols, rows, fps) |
| U6 | Chọn palette `endesga-32.hex` → Remap thư mục sprite | Sprite đúng palette, kèm báo cáo số màu đã đổi |
| U7 | CI: `coral pack hero.coral.json --report r.json` | Mã thoát 0/1, atlas được tạo lại |

## 3. Khái niệm và mô hình dữ liệu

### 3.1 Project `*.coral.json` (`format: "coral.project"`, `version: 1`)

```jsonc
{
  "format": "coral.project", "version": 1,
  "name": "hero",
  "sources": [
    { "kind": "folder", "path": "art/hero", "recursive": true, "include": ["*.png"] },
    { "kind": "aseprite", "path": "art/hero_fx.aseprite", "layers": "visible" },   // or ["Body","FX"]
    { "kind": "psd", "path": "art/ui.psd", "mode": "layers" }                       // or "composite"
  ],
  "sprites": {                       // per-sprite overrides, key = sprite name
    "hero_idle_0": { "pivot": [0.5, 1.0], "nineSlice": null }
  },
  "animations": [
    { "name": "idle", "frames": ["hero_idle_0", "hero_idle_1"], "fps": 8, "loop": "loop",   // loop | once | pingpong
      "durationsMs": null }          // per-frame ms overrides (from Aseprite)
  ],
  "hitboxes": { "hero_attack_3": [ { "kind": "hitbox", "rect": [12, 4, 20, 10], "name": "sword" } ] },
  "pack": {
    "maxSize": 2048, "powerOfTwo": true, "square": false, "allowRotation": false,
    "padding": 2, "extrude": 1, "trim": "alpha", "trimThreshold": 0, "alphaBleed": true,
    "dedupe": true, "premultiply": false, "heuristic": "bestShortSideFit", "multiPage": true
  },
  "exports": [
    { "target": "jsonHash", "out": "build/hero" },
    { "target": "godot", "out": "../game/assets/hero", "godotProject": "../game" },
    { "target": "unity", "out": "../UnityGame/Assets/Art/hero" },
    { "target": "spine", "out": "build/spine/hero" }
  ],
  "watch": { "autoStart": false }
}
```

Mọi đường dẫn tương đối so với file project (§6 của khung chung).

### 3.2 Tên sprite
- Thư mục: tên file bỏ đuôi; thư mục con thêm tiền tố bằng `/` (ví dụ `fx/spark_0`).
- Aseprite: `<file>_<tag>_<frameIndexInTag>`. Frame không thuộc tag: `<file>_<frame>`. Nếu
  `layers` là một mảng tên lớp: mỗi lớp thành một sprite riêng `<file>_<layer>_<frame>`.
- PSD ở mode `layers`: tên lớp (làm sạch ký tự lạ). Mode `composite`: tên file.
- Trùng tên thì lỗi `duplicate-name` (mức error), không tự đổi tên.

### 3.3 Animation tự động
- Tag của Aseprite → animation; lấy hướng (forward / reverse / pingpong) và thời lượng từng frame.
- Thư mục: nhóm theo hậu tố số `name_0..name_n` hoặc `name0001`. Sắp xếp theo **natural sort**.
- Animation tự động có thể ghi đè bằng `animations` trong project.

## 4. Kiến trúc

### 4.1 Rust `src-tauri/src/core/`

| File | Nội dung |
|---|---|
| `image_io.rs` | đọc/ghi PNG, TGA; `RgbaImage`; hash xxh3 của bytes RGBA |
| `sources/folder.rs`, `sources/aseprite.rs`, `sources/psd.rs`, `sources/gif.rs` | biến nguồn thành `Vec<SourceSprite { name, image, origin: SourceRef, durationMs? }>` |
| `trim.rs` | bbox alpha > ngưỡng → `TrimInfo { source_size, trimmed_rect }` |
| `pack/maxrects.rs` | MaxRects (Jukka Jylänki), heuristic BSSF/BLSF/BAF/BL/CP, xoay 90° tùy chọn |
| `pack/atlas.rs` | multi-page, chọn kích thước nhỏ nhất vừa đủ, padding, extrude, dedupe (alias), kết quả xác định |
| `bleed.rs` | alpha bleed: loang màu RGB vào pixel alpha = 0 (BFS nhiều vòng), alpha giữ nguyên |
| `export/json_hash.rs`, `export/json_array.rs`, `export/godot.rs`, `export/unity_meta.rs`, `export/spine.rs` | ghi metadata |
| `slice.rs` | cắt theo lưới; tự nhận "đảo" bằng connected components trên alpha, gộp đảo gần nhau |
| `anim.rs` | xây animation từ tag/tên, ghép per-frame duration |
| `flipbook.rs` | chuỗi frame → lưới N×M |
| `palette.rs` | trích palette, đọc/ghi `.gpl` `.hex` `.pal`, remap nearest trong OKLab, palette swap |
| `normal2d.rs` | height (luminance hoặc bevel từ alpha) → Sobel → normal, tùy chọn OpenGL/DirectX |
| `project.rs` | serde Project, resolve/relativize path, migrate version |
| `pipeline.rs` | `run_pack(project, &RunCtx) -> PackReport`: nguồn → trim → dedupe → pack → bleed → ghi ảnh → export |

Crate: `image` (png, tga, gif), `asefile`, `psd`, `xxhash-rust`, `rayon`. **Tự viết MaxRects** (khoảng 300
dòng) để output xác định và kiểm soát được multi-page; crate `texture_packer` chỉ để tham khảo.

### 4.2 Commands

| Command | Input | Output | Event |
|---|---|---|---|
| `load_project` | `path` | `Project` + `ResolvedSources` (danh sách sprite, kích thước, lỗi nguồn) | — |
| `save_project` | `path, project` | `()` | — |
| `scan_sources` | `project` (chưa lưu) | `SpriteInfo[]` (tên, kích thước, trim, hash, nguồn) | `coral://progress` |
| `pack_preview` | `project` | `PackReport` + atlas PNG trả dạng bytes (`ipc::Response`) theo `page` | `coral://progress` |
| `pack_and_export` | `project, { dryRun }` | `PackReport` (file đã/sẽ ghi) | `coral://progress` |
| `sprite_thumbnail` | `project, name, maxSize` | PNG bytes | — |
| `slice_sheet` | `path, SliceOptions` | `SliceResult { rects[], suggestedNames[] }` | — |
| `apply_slice` | `path, rects, names, outDir, dryRun` | `WriteReport` | — |
| `build_flipbook` | `FlipbookOptions` | `FlipbookReport` + preview bytes | `coral://progress` |
| `palette_extract` | `paths[]` | `PaletteReport { colors[{rgba,count}] }` | — |
| `palette_remap` | `RemapOptions` | `WriteReport` | `coral://progress` |
| `palette_swap` | `SwapOptions` | `WriteReport` | `coral://progress` |
| `upscale_nearest` | `paths[], factor, outDir, dryRun` | `WriteReport` | — |
| `alpha_bleed` | `paths[], outDir \| inPlace, dryRun` | `WriteReport` | — |
| `normal_map` | `NormalOptions` | `WriteReport` + preview bytes | — |
| `watch_start` | `{ action: { kind: "packProject", project } }` | `watchId` | `watch://status` |

### 4.3 UI: module trên rail

| Hotkey | Module | Nội dung chính |
|---|---|---|
| 1 | **Pack** | cột trái: nguồn và danh sách sprite (lọc, thumbnail); giữa: xem atlas (zoom pixel-perfect, hover hiện tên sprite, tab theo page); phải: cài đặt pack, export target; footer: thống kê (số sprite, page, % lấp đầy, alias) |
| 2 | **Slice** | canvas sheet + lưới hoặc rect tự nhận, kéo sửa rect, đặt tên hàng loạt (pattern `{base}_{i}`), pivot preset (9 điểm + custom), viền 9-slice |
| 3 | **Animate** | danh sách animation, player (fps, loop/pingpong, bước frame, onion skin trước/sau), kéo pivot, vẽ hitbox/hurtbox theo frame, timeline frame có duration |
| 4 | **Flipbook** | chọn thư mục chuỗi frame, cols/rows (tự đề xuất), cell size, khoảng frame, bước nhảy, xem trước chạy thử |
| 5 | **Pixel** | Upscale ×N, Palette (trích, nạp, remap, swap theo bảng màu), so sánh trước/sau |
| 6 | **Normal 2D** | nguồn height, cường độ, blur, bevel, quy ước Y, xem trước có đèn kéo được (WebGL) |

Canvas: dùng `<canvas>` 2D với `imageSmoothingEnabled = false`. Xem trước normal map dùng WebGL fragment
shader đơn giản (Lambert với đèn theo vị trí chuột).

## 5. Đặc tả định dạng xuất (phải đúng từng byte có ý nghĩa)

### 5.1 JSON Hash (tương thích TexturePacker; Phaser 3, PixiJS đọc được)

```json
{ "frames": { "hero_idle_0": {
    "frame": {"x":2,"y":2,"w":30,"h":44}, "rotated": false, "trimmed": true,
    "spriteSourceSize": {"x":1,"y":4,"w":30,"h":44}, "sourceSize": {"w":32,"h":48},
    "pivot": {"x":0.5,"y":1.0} } },
  "animations": { "idle": ["hero_idle_0","hero_idle_1"] },
  "meta": { "app": "https://github.com/HoangTrongPhi/Coral", "version": "1.0", "image": "hero.png",
            "format": "RGBA8888", "size": {"w":256,"h":128}, "scale": "1",
            "coral": { "fps": {"idle":8}, "loop": {"idle":"loop"}, "durationsMs": {}, "hitboxes": {}, "nineSlice": {} } } }
```
- Multi-page: mỗi page một file `hero-0.json`/`hero-0.png` … và `meta.related_multi_packs`, giống
  TexturePacker.
- `JSON Array`: cùng nội dung, `frames` là mảng có `filename`.
- **Unreal Paper2D**: cùng JSON Hash, có tùy chọn đuôi `.paper2dsprites`. *Kiểm chứng bắt buộc*: import
  vào UE 5 bằng Paper2D Sprite Sheet importer (người dùng làm, ghi kết quả vào PROGRESS.md).

### 5.2 Godot 4: `SpriteFrames` `.tres` (`format=3`)

```
[gd_resource type="SpriteFrames" load_steps=4 format=3]

[ext_resource type="Texture2D" path="res://assets/hero/hero.png" id="1_atlas"]

[sub_resource type="AtlasTexture" id="AtlasTexture_idle0"]
atlas = ExtResource("1_atlas")
region = Rect2(2, 2, 30, 44)
margin = Rect2(1, 4, 2, 4)

[resource]
animations = [{
"frames": [{ "duration": 1.0, "texture": SubResource("AtlasTexture_idle0") }],
"loop": true,
"name": &"idle",
"speed": 8.0
}]
```

- `res://` = đường dẫn từ thư mục chứa `project.godot`. Tìm `godotProject` trong project, nếu không có thì
  đi ngược lên từ `out`. Không tìm thấy → lỗi `godot-project-not-found`.
- `margin`: `Rect2(spriteSourceSize.x, spriteSourceSize.y, sourceSize.w − w, sourceSize.h − h)`.
  Godot vẽ region lệch theo `margin.position`, và kích thước texture = `region.size + margin.size`, nên
  sprite đã trim giữ đúng khung gốc. Ví dụ ở trên: nguồn 32×48, phần trim 30×44 tại (1,4) →
  `Rect2(1, 4, 2, 4)`. Kiểm chứng bằng một test dựng tay và một lần mở trong Godot.
- `duration` của frame là **hệ số tương đối**: `durationMs / (1000 / fps)`.
- Pingpong: Godot không có, nên nhân bản frame (0,1,2,3,2,1). `once` → `"loop": false`.
- `load_steps` = số ext_resource + số sub_resource + 1.
- Atlas có xoay (`rotated`) → **không hỗ trợ cho Godot**: lỗi mức error, gợi ý tắt `allowRotation`.
- Thêm tùy chọn xuất `AtlasTexture` riêng lẻ (`<name>.tres`) cho sprite UI dùng ngoài animation.

### 5.3 Unity: ghi `.png.meta` (spriteMode Multiple)

- Nếu đã có `.meta`: **giữ nguyên `guid`** và mọi khóa khác; chỉ thay `spriteMode: 2`, khối
  `spriteSheet.sprites`, `nameFileIdTable` và các `internalID`.
- Nếu chưa có: tạo meta mới với guid ngẫu nhiên (32 hex), `textureType: 8` (Sprite), `spriteMode: 2`,
  `spritePixelsToUnits` lấy từ export option (mặc định 100), `filterMode` (Point cho pixel art),
  `textureCompression`.
- Mỗi sprite gồm: `name`, `rect` (**gốc tọa độ dưới-trái**: `y_unity = atlasH − y − h`), `alignment: 9`
  (custom), `pivot` (chuẩn hóa 0–1 theo rect **đã trim**, tính lại từ pivot của sprite gốc), `border`
  (9-slice: L,B,R,T), `spriteID` (32 hex, giữ ổn định theo tên giữa các lần xuất), `internalID` (int64 ổn
  định, từ hash của tên).
- *Câu hỏi mở Q1* (§9): Unity bản nào. Phải có **fixture `.meta` thật** do người dùng tạo bằng bản Unity của
  họ (sheet 2 sprite cắt tay) đặt ở `tests/fixtures/unity/`. Test so sánh cấu trúc YAML với fixture.
- Phương án dự phòng (P3b, chỉ làm khi Q1 cho thấy ghi meta trực tiếp không ổn định): xuất
  `CoralImporter.cs` (Editor script dùng `ISpriteEditorDataProvider`) đọc JSON Hash rồi cắt sprite.

### 5.4 Spine / libGDX `.atlas`

```
hero.png
size: 256,128
format: RGBA8888
filter: Linear,Linear
repeat: none
hero_idle_0
  rotate: false
  xy: 2, 2
  size: 30, 44
  orig: 32, 48
  offset: 1, 0
  index: -1
```
`offset` theo gốc dưới-trái của ảnh gốc: `offset.y = sourceH − y − h`. *Kiểm chứng*: nạp bằng
spine-runtimes hoặc libGDX TexturePacker reader (người dùng xác nhận).

## 6. CLI

```
coral pack <project.coral.json> [--target jsonHash|jsonArray|godot|unity|spine|all] [--dry-run] [--report r.json] [--watch]
coral slice <sheet.png> (--grid 32x32 [--offset 0,0] [--spacing 0,0] | --auto [--merge 2]) --out <dir> [--dry-run]
coral flipbook <folder> [--cols N --rows M] [--cell 256] [--from 1 --to 64 --step 1] --out fx.png [--json]
coral bleed <file|folder> (--out <dir> | --in-place) [--dry-run]
coral palette extract <file|folder> --out palette.hex
coral palette remap <file|folder> --palette p.gpl --out <dir> [--dry-run]
coral upscale <file|folder> --factor 4 --out <dir>
coral normal <file|folder> --strength 2.0 [--convention opengl|directx] [--source luminance|alpha-bevel] --out <dir>
```
Mã thoát 1 khi: sprite không vừa `maxSize` mà `multiPage` tắt, có lỗi nguồn mức error, hoặc trùng tên.

## 7. Watch

- Action `packProject`: theo dõi mọi `sources[].path` của project, cộng chính file project.
- Bỏ qua mọi thư mục `exports[].out`, và file tạm của Aseprite/Photoshop (`*.tmp`, `~*`).
- Tăng dần: cache ảnh đã decode và trim theo `(path, mtime, size)`. Nguồn không đổi thì không decode lại.
  Pack luôn chạy lại toàn bộ, vì nhanh.
- Mục tiêu: 200 sprite 64×64 pack lại trong dưới 1 giây sau khi file ổn định.

## 8. Các phase

### P0 — Scaffold
1. Tạo app bằng lệnh ở đầu file. Mở `docs/PROGRESS.md` và ghi các phase P0–P11.
2. Đổi module mẫu Folder Stats thành 6 module rỗng theo §4.3. Mỗi module có `EmptyState` và hotkey.
3. Thêm crate `image` (features: png, tga, gif, rayon), `asefile`, `psd`, `xxhash-rust`.
4. `sample-assets/`: `scripts/gen-samples.mjs`, hoặc một test Rust có `#[ignore]`, sinh 3 animation
   (idle 4, run 8, attack 6 frame) dạng hình tròn/vuông màu kích thước 32×48, có viền trong suốt để thử trim.
**Xong khi**: `npm run check` xanh; `?mock` hiện 6 module.

### P1 — Nguồn, trim, dedupe, project
1. `project.rs`: struct theo §3.1, serde default, resolve/relativize, `migrate()`. Test round-trip.
2. `sources/folder.rs`: walk + include glob + natural sort + đặt tên theo §3.2.
3. `sources/aseprite.rs`: đọc frame qua `asefile`, ghép lớp nhìn thấy hoặc từng lớp, tag → animation,
   frame duration, slice (9-slice center và pivot của slice nếu có).
   Fixture: một `.aseprite` nhỏ do người dùng cung cấp, hoặc tự tạo bằng Aseprite CLI nếu máy có.
4. `sources/psd.rs`: layer mode và composite mode, bỏ qua layer ẩn.
5. `sources/gif.rs`: mỗi frame là một sprite, delay → durationMs.
6. `trim.rs`, kèm test: ảnh 32×48 có hình 10×10 ở (5,7) → `trimmed_rect = (5,7,10,10)`.
7. Dedupe theo hash RGBA đã trim cộng kích thước. Alias giữ tên riêng nhưng chung rect.
8. Command `load_project`, `save_project`, `scan_sources` + contract test + mock.
**Xong khi**: test xanh; mock của `scan_sources` trả danh sách sprite có thumbnail.

### P2 — Packer
1. `maxrects.rs`: free rectangles, split, prune; 5 heuristic. Kết quả **xác định**: sắp xếp đầu vào theo
   (diện tích giảm dần, tên tăng dần).
2. `atlas.rs`:
   - thử kích thước từ nhỏ đến `maxSize` (power-of-two / square theo cài đặt);
   - padding giữa các sprite;
   - extrude N px (nhân bản pixel viền ra ngoài rect);
   - hết chỗ → page mới (nếu `multiPage`);
   - báo % lấp đầy.
3. `bleed.rs`, kèm test: pixel trong suốt cạnh pixel đỏ phải có RGB đỏ và alpha 0.
4. **Test tính chất**: 500 rect ngẫu nhiên (seed cố định) → không rect nào chồng nhau (tính cả padding),
   tất cả nằm trong biên, mọi sprite đều được đặt.
5. Benchmark (test `#[ignore]`): 1000 sprite 64×64 pack dưới 300 ms bản release.
**Xong khi**: test tính chất xanh; `pack_preview` trả PNG đúng.

### P3 — Exporter
1. JSON Hash, JSON Array (§5.1). Test so sánh với JSON mẫu viết tay.
2. Godot (§5.2). Test: chuỗi `.tres` khớp mẫu; margin đúng với sprite trim lệch.
3. Spine (§5.4).
4. Unity meta (§5.3): **hỏi Q1 trước**. Test với fixture.
5. Ghi file nguyên tử (§7 khung chung); `dryRun` trả danh sách file.
**Xong khi**: 4 target có test; người dùng đã thử import vào ít nhất một engine và ghi kết quả.

### P4 — CLI + watch cho pack
1. `coral pack` theo §6, kèm `parses_flags` và `runs_pack`.
2. Watch action `packProject` (§7), test watch có thư mục tạm.
3. README: mục Command line.
**Xong khi**: `coral pack sample.coral.json --report r.json` → mã 0; sửa một PNG khi đang `--watch` → in
dòng chạy lại.

### P5 — UI Pack + mock (MVP, phát hành v0.1.0)
1. Module Pack theo §4.3: kéo thả thư mục hoặc file nguồn → tạo project chưa lưu; nút Save project.
2. Xem atlas: zoom 25–800%, pixel-perfect, hover hiện tên, click chọn sprite và hiện thông tin trim/alias.
3. Panel cài đặt pack (`Segmented`, `Checkbox`, số), danh sách export target (thêm/xóa, chọn thư mục).
4. Nút **Pack & Export** (dry-run trước, hiện danh sách file sẽ ghi, rồi xác nhận), `JobProgress`,
   `WatchToggle`.
5. Mock: project mẫu với 3 animation sinh bằng canvas; atlas giả ghép bằng canvas.
6. Phát hành `0.1.0` theo §14 khung chung (khi người dùng đồng ý).
**Xong khi**: luồng U1, U2, U7 chạy được trên app thật; DoD chung đạt.

### P6 — Slice
1. Lưới: kích thước ô, offset, spacing; bỏ ô hoàn toàn trong suốt.
2. Tự động: gán nhãn connected components (8-neighbour, alpha > ngưỡng) → bbox → gộp bbox cách nhau ≤
   `merge` px → sắp xếp theo hàng (y rồi x, có dung sai theo chiều cao trung bình).
3. UI: kéo sửa rect, thêm/xóa rect, đặt tên hàng loạt, pivot preset, border 9-slice.
4. Xuất: thành sprite rời (PNG) **hoặc** ghi trực tiếp metadata (JSON Hash/Unity meta) cho sheet gốc mà
   không cắt ảnh.
**Xong khi**: luồng U3 chạy được; test auto-slice trên ảnh sinh có 6 hình rời → 6 rect đúng thứ tự.

### P7 — Animate
1. Player trên canvas: fps, loop mode, bước frame, onion skin (±1–3 frame, alpha 30%).
2. Durations theo frame (lấy từ Aseprite hoặc sửa tay).
3. Pivot: kéo trên canvas, áp cho cả animation hoặc một frame.
4. Hitbox: vẽ rect theo frame, loại `hitbox` / `hurtbox` / `custom:<name>`, sao chép sang frame kế, lưu vào
   `project.hitboxes`, xuất trong `meta.coral.hitboxes` (tọa độ theo sprite gốc chưa trim, gốc trên-trái).
5. Viết `docs/FORMATS.md#hitboxes`, kèm ví dụ đọc trong Godot (GDScript ngắn) và Unity (C# ngắn).
**Xong khi**: luồng U4 chạy được; JSON xuất ra có hitbox đúng tọa độ.

### P8 — Flipbook
1. Chọn thư mục, natural sort, khoảng frame và bước nhảy.
2. Tự đề xuất cols×rows gần vuông nhất và cell size để tổng ≤ `maxSize`; resize cell bằng Lanczos3
   (`image::imageops`).
3. Không trim, padding 0 (lưới đều). Xuất PNG/TGA và JSON `{cols, rows, frames, fps, cell}`.
4. UI: chạy thử flipbook trong canvas; hiện ghi chú "Niagara SubUV: Sub Image Size = cols × rows",
   "Unity Texture Sheet Animation: Tiles X = cols, Y = rows".
**Xong khi**: luồng U5 chạy được; test 64 frame → 8×8 với thứ tự đúng.

### P9 — Pixel tools
1. Upscale nearest ×2..×8.
2. Trích palette (đếm màu, sắp theo tần suất hoặc hue); đọc/ghi `.hex` (Lospec), `.gpl` (GIMP), `.pal` (JASC).
3. Remap: màu nearest trong OKLab, giữ alpha; báo số pixel đổi và màu nào bị gộp.
4. Palette swap: bảng `from → to`, áp cho nhiều file, tạo biến thể `<name>_<variant>.png`. Có thể lưu bảng
   swap vào project.
**Xong khi**: luồng U6 chạy được; test remap trên ảnh 4 màu đã biết.

### P10 — Normal map 2D
1. Height từ luminance, hoặc **bevel từ alpha** (distance transform từ biên alpha, chuẩn hóa theo `bevel` px).
2. Blur Gaussian tùy chọn → Sobel → `n = normalize(-dx*s, -dy*s, 1)` → RGB = n*0.5+0.5. DirectX thì lật G.
3. Pixel alpha 0 → (128,128,255). Xuất `<name>_n.png`. Tùy chọn pack atlas normal song song với atlas màu,
   **dùng cùng rect** (trong project: `pack.normalMaps: true`).
4. Preview WebGL có đèn theo chuột.
**Xong khi**: test hướng: dốc tăng theo x → kênh R < 128 (OpenGL), theo quy ước đã ghi trong FORMATS.md.

### P11 — Hoàn thiện, v0.2.0
- User guide `docs/user-guide/` (một trang mỗi module), ảnh chụp từ app.
- Kiểm tra hiệu năng: 2000 sprite, mở project dưới 2 giây.
- Phát hành.

## 9. Câu hỏi mở (hỏi người dùng khi tới phase liên quan)

| # | Câu hỏi | Phase |
|---|---|---|
| Q1 | Dùng Unity bản nào (2022.3 LTS, Unity 6…)? Cung cấp một `.png.meta` thật đã cắt 2 sprite | P3 |
| Q2 | Có cần TGA hay chỉ PNG? | P3 |
| Q3 | Có dùng Unreal Paper2D thật không (nếu không thì bỏ qua bước kiểm chứng)? | P3 |
| Q4 | Pixel-per-unit mặc định cho Unity (100 hay 16/32 cho pixel art)? | P3 |
| Q5 | Có cần Godot `SpriteFrames` cho mọi animation hay tách theo nhóm? | P3 |

## 10. Rủi ro

- **Unity meta thay đổi giữa các phiên bản** → test với fixture thật, và có phương án C# importer.
- **`asefile` không đọc được bản Aseprite mới** (chunk lạ) → bắt lỗi theo file, gợi ý xuất PNG sequence;
  ghi phiên bản Aseprite vào issue.
- **PSD có layer effect hoặc smart object** → crate `psd` chỉ đọc pixel layer. Báo warn
  `psd-layer-unsupported`, gợi ý rasterize trong Photoshop.

import type { HelpBlock, HelpSection, HelpTopic } from "@abyss/ui";

/*
 * Coral's Help content (F1), Vietnamese + English. One topic per module (topic id = TabId, so the
 * panel opens on the current module) plus "Getting started". Labels in **bold** match the UI
 * exactly (the UI is in English): keep them in sync when a label changes in the code.
 * Source of truth is the code (components/tabs, stores, src-tauri/src/core), then docs/user-guide.
 */

type Pair = [vi: string, en: string];

const p = (vi: string, en: string): HelpBlock => ({ kind: "p", text: { vi, en } });
const steps = (...items: Pair[]): HelpBlock => ({ kind: "steps", items: items.map(([vi, en]) => ({ vi, en })) });
const tips = (...items: Pair[]): HelpBlock => ({ kind: "tips", items: items.map(([vi, en]) => ({ vi, en })) });
const keys = (...items: [keys: string, vi: string, en: string][]): HelpBlock => ({ kind: "keys", items: items.map(([k, vi, en]) => ({ keys: k, label: { vi, en } })) });
const section = (vi: string, en: string, ...blocks: HelpBlock[]): HelpSection => ({ heading: { vi, en }, blocks });

/** Id of the overview topic (not a module). */
export const START_TOPIC_ID = "getting-started";

// ---------------------------------------------------------------- getting started

const gettingStarted: HelpTopic = {
  id: START_TOPIC_ID,
  title: { vi: "Bắt đầu", en: "Getting started" },
  summary: {
    vi: "Coral là gì, bố cục giao diện, kéo & thả, project, cách Coral ghi file an toàn, phím tắt, log và giao diện.",
    en: "What Coral is, the layout, drag & drop, projects, how Coral writes files safely, shortcuts, the log and appearance.",
  },
  sections: [
    section(
      "Coral là gì?",
      "What is Coral?",
      p(
        "Coral là bộ công cụ desktop cho sprite 2D và animation: **đóng gói sprite thành atlas** rồi xuất cho Godot, Unity, Spine, Phaser / PixiJS (Pack), **cắt sprite sheet** có sẵn (Slice), **xem animation, đặt pivot, vẽ hitbox** (Animate), **ghép chuỗi frame VFX thành sheet SubUV** (Flipbook), **công cụ pixel art** — phóng to, palette, đổi màu (Pixel) — và **tạo normal map** cho ánh sáng 2D (Normal 2D).",
        "Coral is a desktop toolkit for 2D sprites and animation: **pack sprites into an atlas** and export it for Godot, Unity, Spine and Phaser / PixiJS (Pack), **cut an existing sprite sheet** (Slice), **play animations, set pivots and draw hitboxes** (Animate), **lay out VFX frame sequences as a SubUV sheet** (Flipbook), **pixel-art tools** — upscale, palettes, colour swaps (Pixel) — and **make normal maps** for 2D lighting (Normal 2D).",
      ),
      tips(
        ["Giao diện ứng dụng bằng tiếng Anh; hướng dẫn này in **đậm** đúng tên nút / tuỳ chọn như trên màn hình.", "The app's interface is in English; this guide shows button and option names in **bold** exactly as on screen."],
        ["Mở hướng dẫn bất cứ lúc nào bằng `F1` (mở đúng chủ đề của module đang dùng), nút **?** trên header, hoặc Ctrl+K → **Help / Hướng dẫn**.", "Open this guide any time with `F1` (it opens on the module you are in), the **?** button in the header, or Ctrl+K → **Help / Hướng dẫn**."],
      ),
    ),
    section(
      "Bố cục giao diện",
      "Layout",
      tips(
        ["**Thanh điều hướng (cột trái)**: 6 module theo thứ tự Ctrl+1 … Ctrl+6 — Pack, Slice, Animate, Flipbook, Pixel, Normal 2D. Phía dưới: **Settings** (Ctrl+,), **About & contact**, **Command palette** (Ctrl+K) và nút **Show log / Hide log** (có huy hiệu đếm lỗi khi log đang đóng).", "**Navigation rail (left)**: the 6 modules, in the order of Ctrl+1 … Ctrl+6 — Pack, Slice, Animate, Flipbook, Pixel, Normal 2D. At the bottom: **Settings** (Ctrl+,), **About & contact**, **Command palette** (Ctrl+K) and **Show log / Hide log** (with an error count while the log is closed)."],
        ["**Header**: `Coral / <module>`, tên project (`(unsaved)` khi chưa lưu, dấu `•` khi có thay đổi chưa lưu), ô **Search actions…** (mở Command Palette), nút **?** (hướng dẫn này) và nhãn phiên bản (`browser preview` khi chạy ngoài app desktop; `via Abyss` khi được Abyss khởi chạy).", "**Header**: `Coral / <module>`, the project name (`(unsaved)` before the first save, `•` when there are unsaved changes), the **Search actions…** box (opens the command palette), the **?** button (this guide) and the version pill (`browser preview` outside the desktop app; `via Abyss` when Abyss launched it)."],
        ["**Log (dưới đáy)**: nhật ký của giao diện và phần xử lý Rust. Bật / tắt bằng Ctrl+`; Ctrl+K → **Clear log** để xoá.", "**Log (bottom)**: messages from the UI and the Rust backend. Toggle it with Ctrl+`; Ctrl+K → **Clear log** empties it."],
      ),
    ),
    section(
      "Mở file và thư mục",
      "Opening files and folders",
      tips(
        ["**Kéo & thả** file hoặc thư mục vào bất kỳ đâu trên cửa sổ: chúng được nạp vào **module đang mở**. Mỗi module nhận một loại đầu vào khác nhau (xem chủ đề của từng module).", "**Drag & drop** files or folders anywhere on the window: they load into **the module you are in**. Each module takes different inputs (see its topic)."],
        ["Pack và Animate: thả thư mục / `.aseprite` / `.psd` / `.gif` để thêm nguồn; thả một file `*.coral.json` để mở project đó.", "Pack and Animate: drop folders / `.aseprite` / `.psd` / `.gif` to add sources; drop a `*.coral.json` file to open that project."],
        ["Slice: một sheet PNG / TGA / GIF. Flipbook: một thư mục frame. Pixel và Normal 2D: ảnh hoặc thư mục (được thêm vào danh sách đầu vào).", "Slice: one PNG / TGA / GIF sheet. Flipbook: a folder of frames. Pixel and Normal 2D: images or folders (added to the input list)."],
        ["Ctrl+K → **Add a folder…** chọn một thư mục và gửi nó tới module đang mở, như khi thả vào cửa sổ.", "Ctrl+K → **Add a folder…** picks a folder and sends it to the current module, as if you had dropped it."],
      ),
    ),
    section(
      "Project (*.coral.json)",
      "Projects (*.coral.json)",
      p(
        "Project lưu những gì Pack và Animate dùng: nguồn sprite, thiết lập pack, các đích xuất (exports), pivot đã sửa, animation đã sửa, hitbox và bảng swap màu (nếu bạn bấm **Save to project** trong Pixel). Đường dẫn trong file là **tương đối** so với file project, nên có thể để project trong repo game.",
        "A project holds what Pack and Animate use: sprite sources, pack settings, export targets, edited pivots, edited animations, hitboxes, and colour-swap tables (when you click **Save to project** in Pixel). Paths inside it are **relative** to the project file, so it can live in the game repo.",
      ),
      tips(
        ["**Open…** / **Save** ở thanh trên cùng của Pack, hoặc Ctrl+O / Ctrl+S / Ctrl+Shift+S (lưu thành file khác). Lần lưu đầu tiên hỏi nơi lưu, tên mặc định `<atlas>.coral.json`.", "**Open…** / **Save** in Pack's top bar, or Ctrl+O / Ctrl+S / Ctrl+Shift+S (save as). The first save asks where, defaulting to `<atlas>.coral.json`."],
        ["Ctrl+K → **New project** bắt đầu project trống (dừng Watch nếu đang chạy; không hỏi lưu — hãy lưu trước). Mục **Recent projects** trong Command Palette mở lại 10 project gần nhất.", "Ctrl+K → **New project** starts an empty project (stops Watch; it does not ask to save — save first). **Recent projects** in the command palette reopens the last 10."],
        ["Thiết lập của Slice, Pixel và Normal 2D **không** nằm trong project. Tuỳ chọn của Flipbook (trừ danh sách frame) được ứng dụng nhớ giữa các lần mở.", "Slice, Pixel and Normal 2D settings are **not** stored in the project. Flipbook's options (except the frame list) are remembered by the app between sessions."],
      ),
    ),
    section(
      "Coral ghi file thế nào",
      "How Coral writes files",
      tips(
        ["**Pack** và **Slice** luôn hiện danh sách file sẽ ghi trước (hộp thoại **Write N file(s)?**, mỗi dòng gắn nhãn `will write`); chỉ khi bấm **Write files** mới ghi.", "**Pack** and **Slice** always list the files first (the **Write N file(s)?** dialog, each line tagged `will write`); nothing is written until you click **Write files**."],
        ["**Flipbook**, **Pixel** và **Normal 2D** ghi ngay khi bấm nút chính. Pixel và Normal 2D có nút **Dry run** để xem danh sách trước; Flipbook tự lập kế hoạch lại mỗi khi bạn đổi tuỳ chọn.", "**Flipbook**, **Pixel** and **Normal 2D** write as soon as you click the main button. Pixel and Normal 2D have a **Dry run** button to see the list first; Flipbook re-plans by itself whenever an option changes."],
        ["Mọi file được ghi **nguyên tử**: ghi ra `<file>.tmp` rồi đổi tên. File đã có cùng đường dẫn sẽ bị **thay thế**.", "Every file is written **atomically**: to `<file>.tmp`, then renamed. An existing file at the same path is **replaced**."],
        ["Chế độ **In place (.bak)** (Pixel) sao chép từng file gốc sang `<file>.bak` trước khi ghi đè. Nếu `.bak` đã tồn tại thì giữ nguyên `.bak` cũ (bản gốc đầu tiên được bảo toàn).", "**In place (.bak)** (Pixel) copies each input to `<file>.bak` before overwriting it. An existing `.bak` is kept as is (the first original survives)."],
        ["Coral không bao giờ xoá file của bạn. Bấm vào đường dẫn của file đã ghi trong danh sách kết quả để mở nó trong Explorer.", "Coral never deletes your files. Click a written file's path in a result list to show it in Explorer."],
      ),
    ),
    section(
      "Phím tắt",
      "Shortcuts",
      keys(
        ["F1", "Mở / đóng hướng dẫn này (đúng chủ đề của module đang mở)", "Open / close this guide (on the current module's topic)"],
        ["Ctrl+K", "Command Palette: chuyển module, thao tác project, project gần đây, cập nhật…", "Command palette: modules, project actions, recent projects, updates…"],
        ["Ctrl+1 … Ctrl+6", "Chuyển module (Pack, Slice, Animate, Flipbook, Pixel, Normal 2D)", "Switch module (Pack, Slice, Animate, Flipbook, Pixel, Normal 2D)"],
        ["Ctrl+O", "Mở project", "Open a project"],
        ["Ctrl+S / Ctrl+Shift+S", "Lưu project / lưu thành file khác", "Save the project / save it as"],
        ["F5", "Chạy lại module hiện tại (không bao giờ ghi file): Pack / Animate pack lại bản xem trước, Slice **Detect** lại, Flipbook lập kế hoạch lại, Normal 2D làm mới bản xem trước; Pixel không làm gì", "Run the current module again (never writes): Pack / Animate re-pack the preview, Slice runs **Detect** again, Flipbook re-plans, Normal 2D refreshes its preview; nothing in Pixel"],
        ["Ctrl+`", "Hiện / ẩn log", "Show / hide the log"],
        ["Ctrl+,", "Settings (giao diện)", "Settings (appearance)"],
        ["Esc", "Đóng hộp thoại / Command Palette / hướng dẫn", "Close a dialog / the command palette / this guide"],
      ),
      tips(["Trong Slice, `F5` chạy lại **Detect** và **thay toàn bộ** các rect — mọi chỉnh sửa tay (vị trí, tên, pivot, 9-slice) sẽ mất.", "In Slice, `F5` runs **Detect** again and **replaces every rect** — hand edits (positions, names, pivots, 9-slice) are lost."]),
    ),
    section(
      "Giao diện và cập nhật",
      "Appearance and updates",
      tips(
        ["**Settings** (Ctrl+,): chủ đề sáng / tối / theo Windows, font giao diện và font code, cỡ giao diện, mức hiệu ứng.", "**Settings** (Ctrl+,): light / dark / Windows theme, UI and code fonts, UI scale, effects level."],
        ["Ctrl+K → **Check for updates…** (hoặc trong **About & contact**). Khi có bản mới, header hiện nút **Update to vX.Y.Z** để tải, cài và khởi động lại.", "Ctrl+K → **Check for updates…** (or in **About & contact**). When a new version exists, the header shows **Update to vX.Y.Z** to download, install and restart."],
        ["Khi được Abyss khởi chạy, Coral không tự kiểm tra cập nhật: Abyss báo và cài bản mới khi ứng dụng đóng.", "When Abyss launches Coral, Coral does not check by itself: Abyss shows the update and installs it when the app closes."],
      ),
    ),
  ],
};

// ---------------------------------------------------------------- Pack

const pack: HelpTopic = {
  id: "pack",
  title: { vi: "Pack — đóng gói atlas", en: "Pack — atlas packing" },
  summary: {
    vi: "Gom PNG / TGA, Aseprite, PSD, GIF thành atlas (trim, gộp trùng, MaxRects) rồi xuất JSON (Phaser, PixiJS, Paper2D), Godot, Unity, Spine. Có Watch để tự xuất lại khi lưu.",
    en: "Turn PNG / TGA, Aseprite, PSD and GIF sources into an atlas (trim, duplicate merging, MaxRects) and export JSON (Phaser, PixiJS, Paper2D), Godot, Unity and Spine. Watch re-exports on save.",
  },
  sections: [
    section(
      "Cách dùng",
      "How to use",
      steps(
        ["**Thêm nguồn**: thả thư mục PNG / TGA, file `.aseprite` / `.ase`, `.psd` hoặc `.gif` vào cửa sổ, hoặc bấm nút thư mục (**Add a folder**) trên thẻ **Sources**. Thả một PNG lẻ sẽ thêm **thư mục chứa nó** (không gồm thư mục con, chỉ loại file đó).", "**Add sources**: drop a folder of PNG / TGA files, `.aseprite` / `.ase`, `.psd` or `.gif` files onto the window, or use the folder button (**Add a folder**) of the **Sources** card. A loose PNG adds **its folder** (without sub-folders, that file type only)."],
        ["Đặt **tên atlas** ở ô bên trái thanh trên cùng (dùng cho tên file xuất). Project chưa lưu tự lấy tên của nguồn đầu tiên.", "Set the **atlas name** in the box at the left of the top bar (it names the exported files). An unsaved project takes the first source's name."],
        ["Xem **atlas** ở giữa: nó được pack lại sau mỗi chỉnh sửa. Chỉnh **Pack** settings bên phải.", "Watch the **atlas** in the middle: it re-packs after every edit. Adjust the **Pack** settings on the right."],
        ["Trong thẻ **Exports**, bấm **+**, chọn đích (**JSON Hash**, **JSON Array**, **Godot**, **Unity**, **Spine**) rồi chọn thư mục xuất.", "In the **Exports** card, click **+**, choose a target (**JSON Hash**, **JSON Array**, **Godot**, **Unity**, **Spine**) and pick its output folder."],
        ["Bấm **Pack & Export**: Coral liệt kê các file sẽ ghi; bấm **Write files** để ghi.", "Click **Pack & Export**: Coral lists the files it will write; click **Write files** to write them."],
        ["**Save** (Ctrl+S) để lưu project, rồi bật **Watch** nếu muốn atlas tự xuất lại mỗi khi bạn lưu file nguồn.", "**Save** (Ctrl+S) the project, then turn on **Watch** to re-export whenever you save a source file."],
      ),
    ),
    section(
      "Nguồn (Sources)",
      "Sources",
      p("Bấm vào một nguồn để sửa nó; nút **×** bỏ nguồn khỏi project (không xoá file).", "Click a source to edit it; **×** removes it from the project (no file is deleted)."),
      tips(
        ["**Thư mục**: **Include subfolders** (bật sẵn khi thả thư mục) và danh sách mẫu tên, cách nhau bằng dấu phẩy (mặc định `*.png, *.tga`).", "**Folder**: **Include subfolders** (on for a dropped folder) and a comma-separated list of include patterns (default `*.png, *.tga`)."],
        ["**Aseprite**: **Visible merged** (gộp các layer đang hiện), **All merged** (gộp mọi layer) hoặc **Per layer** — gõ tên các layer cần tách, cách nhau bằng dấu phẩy (ví dụ `Body, FX`); mỗi layer thành một bộ sprite riêng. Tag thành animation; thời lượng frame, pivot và 9-slice từ slice được giữ.", "**Aseprite**: **Visible merged** (visible layers merged), **All merged** (every layer merged) or **Per layer** — type the layer names to split, comma separated (e.g. `Body, FX`); each layer becomes its own set of sprites. Tags become animations; frame durations, slice pivots and 9-slice are kept."],
        ["**PSD**: **One sprite per layer** (mặc định) hoặc **Flattened** (cả file thành một sprite). Chỉ đọc layer pixel (không đọc effect / smart object).", "**PSD**: **One sprite per layer** (default) or **Flattened** (the whole file as one sprite). Only pixel layers are read (no effects / smart objects)."],
        ["**GIF**: mỗi frame là một sprite `<file>_<i>`.", "**GIF**: each frame becomes a sprite `<file>_<i>`."],
        ["Tên sprite: đường dẫn trong thư mục, bỏ đuôi, thư mục con nối bằng `/` (ví dụ `fx/spark_0`); Aseprite: `<file>_<tag>_<i>`. Hai sprite trùng tên là lỗi `duplicate-name` — Coral không tự đổi tên.", "Sprite names: the path inside the folder without the extension, sub-folders joined with `/` (e.g. `fx/spark_0`); Aseprite: `<file>_<tag>_<i>`. Two sprites with the same name are a `duplicate-name` error — Coral never renames silently."],
        ["Animation tự nhận từ tag Aseprite và tên đánh số (`run_0, run_1…`, `walk0001…`). Xem và sửa chúng trong module **Animate**.", "Animations come from Aseprite tags and numbered names (`run_0, run_1…`, `walk0001…`). Play and edit them in the **Animate** module."],
      ),
    ),
    section(
      "Xem atlas",
      "Atlas preview",
      tips(
        ["Zoom 25 %–800 % bằng các nút phía trên. Rê chuột lên một sprite để thấy tên; bấm để chọn — thanh dưới hiện frame trong atlas, kích thước gốc, vùng đã trim và pivot.", "Zoom 25 %–800 % with the buttons above. Hover a sprite to see its name; click it to select it — the bar below shows its frame in the atlas, source size, trimmed area and pivot."],
        ["Khi một trang không đủ chỗ, các trang hiện dưới dạng **Page 1**, **Page 2**… Nếu bật **Normal-map atlas**, ô **Normal map** cho xem atlas normal.", "When one page is not enough, pages appear as **Page 1**, **Page 2**… With **Normal-map atlas** on, the **Normal map** box shows the normal atlas."],
        ["Danh sách **Sprites** bên trái có ô **Filter…**. Nhãn `alias` = sprite có pixel giống hệt sprite khác: chúng dùng chung một ô trong atlas.", "The **Sprites** list on the left has a **Filter…** box. `alias` marks a sprite with the same pixels as another one: they share one rect in the atlas."],
        ["Thanh dưới cùng: số sprite, trang, alias, % lấp đầy, số animation và thời gian pack bản xem trước.", "The footer: sprites, pages, aliases, fill %, animations and how long the preview took to pack."],
      ),
    ),
    section(
      "Thiết lập Pack",
      "Pack settings",
      tips(
        ["**Max size** (512–8192, mặc định 2048): cạnh tối đa của một trang. **Image**: **PNG** hoặc **TGA**.", "**Max size** (512–8192, default 2048): the largest page side. **Image**: **PNG** or **TGA**."],
        ["**Padding** (mặc định 2 px): khoảng trống giữa các sprite. **Extrude** (mặc định 1 px): lặp lại pixel viền ra ngoài để lọc texture không kéo màu của sprite bên cạnh.", "**Padding** (default 2 px): space between sprites. **Extrude** (default 1 px): repeats the edge pixels outward so texture filtering does not pull in neighbours."],
        ["**Trim** cắt viền trong suốt; **Trim alpha ≤** coi pixel có alpha ≤ giá trị này là trống. Kích thước gốc và offset vẫn được xuất, nên sprite không bị lệch trong engine.", "**Trim** cuts away transparent borders; **Trim alpha ≤** treats pixels at or below that alpha as empty. The original size and offset are exported, so sprites stay in place in the engine."],
        ["**Merge duplicates**: sprite giống hệt nhau dùng chung một ô (alias). **Power of two** / **Square**: kích thước trang là luỹ thừa của 2 / hình vuông. **Multiple pages**: thêm trang khi một trang không đủ (tắt thì sprite không vừa là lỗi).", "**Merge duplicates**: identical sprites share one rect (alias). **Power of two** / **Square**: page sizes are powers of two / square. **Multiple pages**: add pages when one is not enough (off: sprites that do not fit are an error)."],
        ["**Allow rotation**: xoay sprite 90° để xếp chặt hơn — **Godot và Unity không hỗ trợ**: xuất cho chúng sẽ bị chặn.", "**Allow rotation**: rotates sprites 90° for a tighter fit — **not supported by Godot or Unity**: exports for them are blocked."],
        ["**Alpha bleed**: tô pixel trong suốt bằng màu lân cận (alpha vẫn 0) để không có viền tối khi lọc / mipmap. **Premultiply**: nhân trước màu với alpha (chỉ bật nếu engine của bạn cần).", "**Alpha bleed**: fills transparent pixels with neighbouring colours (alpha stays 0) so filtering / mipmaps show no dark fringes. **Premultiply**: premultiplies colour by alpha (only if your engine expects it)."],
        ["**Normal-map atlas**: thêm atlas thứ hai `<name>_n` với cùng các ô; thiết lập lấy từ module **Normal 2D** (nút **Use for the project's normal atlas**).", "**Normal-map atlas**: a second atlas `<name>_n` with the same rects; its settings come from the **Normal 2D** module (**Use for the project's normal atlas**)."],
        ["**Placement**: cách MaxRects chọn chỗ — **Best short side** (mặc định), **Best long side**, **Best area**, **Bottom left**, **Contact point**. Thử vài cách nếu muốn atlas nhỏ hơn.", "**Placement**: how MaxRects picks a spot — **Best short side** (default), **Best long side**, **Best area**, **Bottom left**, **Contact point**. Try a few for a smaller atlas."],
      ),
    ),
    section(
      "Đích xuất (Exports) và file tạo ra",
      "Export targets and their files",
      p(
        "Mỗi đích ghi **ảnh atlas riêng của nó** vào thư mục của nó (`<name>.png`, hoặc `<name>-0.png`, `<name>-1.png`… khi có nhiều trang; `<name>_n.png` cho normal atlas) cùng file dữ liệu bên dưới. `<name>` là tên atlas.",
        "Each target writes **its own copy of the atlas image(s)** into its folder (`<name>.png`, or `<name>-0.png`, `<name>-1.png`… with several pages; `<name>_n.png` for the normal atlas) plus the data file below. `<name>` is the atlas name.",
      ),
      tips(
        ["**JSON Hash** (định dạng TexturePacker — Phaser 3, PixiJS): `<name>.json`. Tick **Unreal Paper2D (.paper2dsprites)** để file dữ liệu có đuôi `.paper2dsprites` cho Paper2D. **JSON Array**: cùng nội dung, frames là mảng. FPS, kiểu lặp, thời lượng frame, hitbox và 9-slice nằm trong `meta.coral`.", "**JSON Hash** (TexturePacker format — Phaser 3, PixiJS): `<name>.json`. Tick **Unreal Paper2D (.paper2dsprites)** to give the data file the `.paper2dsprites` extension for Paper2D. **JSON Array**: same content, frames as an array. FPS, loop modes, frame durations, hitboxes and 9-slice are in `meta.coral`."],
        ["**Godot** (Godot 4): `<name>.tres` (SpriteFrames chứa mọi animation; ping-pong được trải ra thành frame). Thư mục xuất phải nằm trong một project Godot — `project.godot` được tìm ngược lên các thư mục cha, hoặc bấm dòng **Godot project:** để chỉ định. Tuỳ chọn **Also one AtlasTexture .tres per sprite** cho sprite UI.", "**Godot** (Godot 4): `<name>.tres` (one SpriteFrames with every animation; ping-pong is unrolled into frames). The output folder must be inside a Godot project — `project.godot` is searched upward, or click the **Godot project:** line to set it. **Also one AtlasTexture .tres per sprite** for UI sprites."],
        ["**Unity** (Unity 6): `<name>.png.meta` (Sprite Mode Multiple) với **Pixels per unit** (mặc định 100) và **Filter** **Point** / **Bilinear**. Xuất lại **giữ nguyên guid** của `.meta`, nên tham chiếu trong scene không bị mất.", "**Unity** (Unity 6): `<name>.png.meta` (Sprite Mode Multiple) with **Pixels per unit** (default 100) and **Filter** **Point** / **Bilinear**. Exporting again **keeps the meta's guid**, so references in scenes survive."],
        ["**Spine** (Spine / libGDX): `<name>.atlas`, filter **Linear** hoặc **Nearest**.", "**Spine** (Spine / libGDX): `<name>.atlas`, filter **Linear** or **Nearest**."],
        ["Bấm vào đường dẫn của một đích để đổi thư mục; biểu tượng thùng rác bỏ đích đó. Đích chưa có thư mục hiện **Choose a folder…** màu đỏ và sẽ chặn việc xuất.", "Click a target's path to change its folder; the bin removes the target. A target without a folder shows a red **Choose a folder…** and blocks the export."],
      ),
    ),
    section(
      "Pack & Export",
      "Pack & Export",
      steps(
        ["Bấm **Pack & Export** (cần ít nhất một đích xuất).", "Click **Pack & Export** (at least one export target is needed)."],
        ["Hộp thoại **Write N file(s)?** liệt kê mọi file (`will write`). File cùng đường dẫn sẽ bị thay thế; `.meta` Unity có sẵn giữ guid.", "The **Write N file(s)?** dialog lists every file (`will write`). Files at those paths are replaced; an existing Unity `.meta` keeps its guid."],
        ["Bấm **Write files**. Thẻ **Last export** hiện kết quả và thời gian; bấm một đường dẫn để mở trong Explorer.", "Click **Write files**. The **Last export** card shows the result and time; click a path to show it in Explorer."],
      ),
      tips(
        ["**Export blocked**: có lỗi (nguồn không đọc được, trùng tên sprite, sprite lớn hơn atlas, xoay sprite khi xuất Godot / Unity, thiếu thư mục…). Không file nào được ghi; thẻ **Issues** nói rõ lỗi gì và ở đâu.", "**Export blocked**: there are errors (an unreadable source, duplicate sprite names, a sprite larger than the atlas, rotation with a Godot / Unity target, a missing folder…). Nothing is written; the **Issues** card says what and where."],
      ),
    ),
    section(
      "Watch",
      "Watch",
      tips(
        ["**Watch** bật được khi project có nguồn và ít nhất một đích xuất. Nó pack và xuất lại mỗi khi một file nguồn (hoặc file project đã lưu) thay đổi — lưu trong Aseprite là engine tự nạp lại.", "**Watch** is available once the project has sources and at least one export target. It packs and exports again whenever a source file (or the saved project file) changes — save in Aseprite and the engine hot-reloads."],
        ["Watch **ghi file mà không hỏi lại** mỗi lần chạy, vào đúng các thư mục xuất đã đặt.", "Watch **writes without asking** on every run, into the export folders you set."],
        ["Nhãn trạng thái hiện lần chạy gần nhất; chuyển đỏ khi lỗi — bấm vào để mở log. **Stop** để dừng. Watch dừng khi đóng ứng dụng; `\"watch\": { \"autoStart\": true }` trong project sẽ tự bật khi mở project.", "The status pill shows the last run; it turns red on an error — click it for the log. **Stop** ends it. Watches stop when the app closes; `\"watch\": { \"autoStart\": true }` in a project starts one when it opens."],
      ),
    ),
    section(
      "Dòng lệnh",
      "Command line",
      p(
        "Cùng file exe chạy không giao diện khi tham số đầu là một lệnh, ví dụ `coral.exe pack hero.coral.json` (mọi đích xuất của project), thêm `--target godot --dry-run` để chỉ liệt kê, `--watch` để theo dõi, `--report pack.json` để ghi báo cáo. Xem `coral.exe --help` và README của Coral.",
        "The same exe runs headless when its first argument is a subcommand, e.g. `coral.exe pack hero.coral.json` (every export of the project); add `--target godot --dry-run` to list the files only, `--watch` to keep watching, `--report pack.json` for a report. See `coral.exe --help` and Coral's README.",
      ),
    ),
  ],
};

// ---------------------------------------------------------------- Slice

const slice: HelpTopic = {
  id: "slice",
  title: { vi: "Slice — cắt sprite sheet", en: "Slice — cut a sprite sheet" },
  summary: {
    vi: "Cắt một sheet có sẵn theo lưới hoặc tự động, sửa rect, tên, pivot, 9-slice; ghi PNG riêng lẻ hoặc metadata (JSON Hash, Unity .meta) cho sheet gốc.",
    en: "Cut an existing sheet by grid or automatically, edit rects, names, pivots and 9-slice; write separate PNGs or metadata (JSON Hash, Unity .meta) for the uncut sheet.",
  },
  sections: [
    section(
      "Cách dùng",
      "How to use",
      steps(
        ["Thả một sheet PNG / TGA / GIF vào cửa sổ, hoặc bấm **Choose a sheet…**. Coral chạy **Detect** ngay (chế độ **Automatic**).", "Drop a PNG / TGA / GIF sheet onto the window, or click **Choose a sheet…**. Coral runs **Detect** right away (**Automatic** mode)."],
        ["Chọn **Mode**: **Automatic** hoặc **Grid**, chỉnh thông số rồi bấm **Detect**.", "Choose the **Mode**: **Automatic** or **Grid**, adjust the numbers, then click **Detect**."],
        ["Sửa rect trên sheet (xem bên dưới), đặt tên, pivot và 9-slice trong thẻ **Rect N**.", "Edit the rects on the sheet (see below); set names, pivots and 9-slice in the **Rect N** card."],
        ["Trong thẻ **Write**, chọn kiểu đầu ra rồi bấm **Slice N sprite(s)…**: Coral liệt kê file sẽ ghi; bấm **Write files** để ghi.", "In the **Write** card, choose the output, then click **Slice N sprite(s)…**: Coral lists the files; click **Write files** to write them."],
      ),
    ),
    section(
      "Automatic và Grid",
      "Automatic and Grid",
      tips(
        ["**Automatic**: tìm từng hình tách rời (các pixel có alpha > **Alpha >**, nối 8 hướng). Hình cách nhau ≤ **Merge within** px (mặc định 2) gộp thành một sprite; hình nhỏ hơn **Ignore under** pixel (mặc định 4) bị bỏ qua. Rect được xếp theo thứ tự đọc: hàng trên xuống dưới, trái sang phải.", "**Automatic**: finds every separate shape (pixels with alpha above **Alpha >**, 8-connected). Shapes within **Merge within** px (default 2) become one sprite; specks under **Ignore under** pixels (default 4) are skipped. Rects come in reading order: rows top to bottom, left to right."],
        ["**Grid**: cắt ô cố định **Cell W** × **Cell H** (mặc định 32 × 32), bắt đầu từ **Offset X / Y**, với khoảng cách **Gap X / Y** giữa các ô. Ô trống bị bỏ trừ khi bật **Keep empty cells**.", "**Grid**: cuts fixed **Cell W** × **Cell H** cells (default 32 × 32), starting at **Offset X / Y**, with **Gap X / Y** between cells. Empty cells are dropped unless **Keep empty cells** is on."],
        ["**Detect** (và `F5`) thay **toàn bộ** rect hiện có — chỉnh sửa tay sẽ mất. Hãy chọn chế độ và thông số trước, rồi mới sửa tay.", "**Detect** (and `F5`) replaces **every** rect — hand edits are lost. Settle the mode and numbers first, then edit by hand."],
      ),
    ),
    section(
      "Sửa rect",
      "Editing rects",
      tips(
        ["Kéo một rect để di chuyển; kéo góc dưới-phải của rect đang chọn để đổi kích thước. Số thứ tự hiện trên rect khi zoom từ 200 %.", "Drag a rect to move it; drag the selected rect's bottom-right corner to resize it. Index numbers show on the rects from 200 % zoom."],
        ["**+ Rect** rồi kéo trên sheet để thêm rect mới. Biểu tượng thùng rác trên thẻ **Rect N** xoá rect đang chọn.", "**+ Rect**, then drag on the sheet to add a rect. The bin on the **Rect N** card deletes the selected rect."],
        ["Thẻ **Rect N**: tên, **X / Y / W / H** chính xác, **Pivot** (9 vị trí sẵn hoặc giá trị X / Y từ 0 đến 1; **Use this pivot for every rect** áp cho mọi rect) và **9-slice borders** (**L / T / R / B**, hiện bằng nét đứt xanh).", "The **Rect N** card: name, exact **X / Y / W / H**, **Pivot** (9 presets or X / Y values 0–1; **Use this pivot for every rect** applies it to all) and **9-slice borders** (**L / T / R / B**, shown as green dashed lines)."],
      ),
    ),
    section(
      "Đặt tên hàng loạt",
      "Names",
      p(
        "Thẻ **Names**: mẫu tên như `{base}_{i}` — `{base}` = tên sheet, `{i}` = số thứ tự từ 0, `{i:03}` = số có đệm 0 (`007`), `{i1}` = đánh số từ 1. Bấm **Rename all** để áp dụng cho mọi rect (Detect cũng dùng mẫu này).",
        "The **Names** card: a pattern such as `{base}_{i}` — `{base}` = the sheet's name, `{i}` = index from 0, `{i:03}` = zero-padded (`007`), `{i1}` = counting from 1. **Rename all** applies it to every rect (Detect uses it too).",
      ),
    ),
    section(
      "Đầu ra",
      "Output",
      tips(
        ["**Separate PNGs**: mỗi rect thành một PNG trong **Output folder** (mặc định `<sheet>_sprites` cạnh sheet).", "**Separate PNGs**: one PNG per rect in the **Output folder** (default `<sheet>_sprites` next to the sheet)."],
        ["**JSON Hash**: `<sheet>.json` mô tả các rect của sheet gốc (không cắt ảnh), ghi vào **Output folder**.", "**JSON Hash**: `<sheet>.json` describing the rects of the uncut sheet (no image is cut), written to the **Output folder**."],
        ["**Unity .meta**: `<sheet>.png.meta` **cạnh sheet** — Unity tự cắt khi import.", "**Unity .meta**: `<sheet>.png.meta` **next to the sheet** — Unity slices it on import."],
        ["**Cannot slice**: hai rect trùng tên hoặc rect nằm ngoài sheet. Sửa rồi thử lại.", "**Cannot slice**: two rects share a name or a rect lies outside the sheet. Fix it and try again."],
        ["Rect, tên và pivot của Slice không được lưu vào project và mất khi mở sheet khác — hãy ghi xong trước khi chuyển sheet.", "Slice's rects, names and pivots are not saved in the project and are lost when you open another sheet — write them before switching."],
      ),
    ),
  ],
};

// ---------------------------------------------------------------- Animate

const animate: HelpTopic = {
  id: "animate",
  title: { vi: "Animate — animation, pivot, hitbox", en: "Animate — playback, pivots, hitboxes" },
  summary: {
    vi: "Xem animation của project đang mở trong Pack, chỉnh FPS / kiểu lặp / thời lượng từng frame, onion skin, đặt pivot và vẽ hitbox / hurtbox theo từng frame.",
    en: "Play the animations of the project open in Pack, edit FPS / loop mode / per-frame durations, onion skin, set pivots and draw hitboxes / hurtboxes per frame.",
  },
  sections: [
    section(
      "Animation đến từ đâu",
      "Where animations come from",
      p(
        "Animate dùng project đang mở trong **Pack** (atlas xem trước của nó). Animation tự nhận từ tag Aseprite và tên đánh số (`run_0…`, `walk0001…`). Chưa có nguồn thì thêm ở Pack trước — thả nguồn vào Animate cũng thêm vào project.",
        "Animate works on the project open in **Pack** (its atlas preview). Animations come from Aseprite tags and numbered names (`run_0…`, `walk0001…`). Add sources in Pack first — dropping sources on Animate adds them to the project too.",
      ),
      tips(["Sửa một animation sẽ lưu nó vào project (nhãn **edited** trong danh sách). Nhớ **Save** project (Ctrl+S) để giữ thay đổi.", "Editing an animation stores it in the project (the **edited** badge in the list). **Save** the project (Ctrl+S) to keep the changes."]),
    ),
    section(
      "Trình phát",
      "Player",
      tips(
        ["**Play** / **Pause**, ◀ ▶ để lùi / tiến một frame. Chọn animation ở danh sách **Animations** bên trái (số frame · FPS).", "**Play** / **Pause**, ◀ ▶ to step one frame. Pick an animation in the **Animations** list on the left (frames · FPS)."],
        ["Thẻ **Playback**: **FPS**, **Loop** (**Loop**, **Once**, **Ping-pong**) và **Onion skin** (0–3 frame trước và sau, mờ dần từ 30 %).", "The **Playback** card: **FPS**, **Loop** (**Loop**, **Once**, **Ping-pong**) and **Onion skin** (0–3 frames before and after, fading from 30 %)."],
        ["Dải frame dưới khung hình: bấm để nhảy tới frame; ô số dưới mỗi frame là thời lượng (ms) — gõ số để đổi riêng frame đó. **Same duration for every frame** bỏ thời lượng riêng, quay về FPS.", "The strip under the frame: click to jump to a frame; the number under each frame is its duration (ms) — type a value to change that frame. **Same duration for every frame** drops the per-frame values and goes back to FPS."],
        ["Chấm đỏ dưới một frame nghĩa là frame đó có box.", "A red dot under a frame means it has boxes."],
      ),
    ),
    section(
      "Pivot",
      "Pivot",
      steps(
        ["Chọn công cụ hình tâm ngắm (**Click to set the pivot**) rồi bấm hoặc kéo trên khung hình — hoặc dùng 9 vị trí sẵn trong thẻ **Pivot**.", "Pick the crosshair tool (**Click to set the pivot**) and click or drag on the frame — or use the 9 presets in the **Pivot** card."],
        ["**Apply to the whole animation** (bật sẵn) đặt pivot cho mọi frame; tắt để chỉ đổi frame hiện tại.", "**Apply to the whole animation** (on by default) sets it on every frame; turn it off to change only the current frame."],
      ),
      tips(["Pivot được lưu dạng chuẩn hoá 0–1 theo sprite **chưa trim** và được xuất sang mọi đích (JSON, Godot, Unity, Spine).", "Pivots are stored normalised 0–1 in the **untrimmed** sprite and exported to every target (JSON, Godot, Unity, Spine)."]),
    ),
    section(
      "Hitbox, hurtbox, box tuỳ chỉnh",
      "Hitboxes, hurtboxes and custom boxes",
      steps(
        ["Trong thẻ **Boxes · frame N**, chọn **New box type**: **Hitbox** (đỏ), **Hurtbox** (xanh lá) hoặc **Custom** (vàng, gõ tên).", "In the **Boxes · frame N** card, choose the **New box type**: **Hitbox** (red), **Hurtbox** (green) or **Custom** (amber, with a name)."],
        ["Chọn công cụ hình vuông (**Drag to draw a box**) rồi kéo trên khung hình.", "Pick the square tool (**Drag to draw a box**) and drag on the frame."],
        ["Dùng công cụ mũi tên (**Select a box**) để chọn box; đặt tên hoặc xoá nó trong danh sách.", "Use the arrow tool (**Select a box**) to select a box; name or delete it in the list."],
        ["Nút sao chép (**Copy boxes to the next frame**) chép mọi box của frame này sang frame kế tiếp và chuyển tới đó — tiện cho chuỗi đòn đánh.", "The copy button (**Copy boxes to the next frame**) copies this frame's boxes to the next frame and moves there — handy for attack sequences."],
      ),
      tips(["Box được xuất trong `meta.coral.hitboxes` của các file JSON (JSON Hash / Array): toạ độ pixel của sprite **chưa trim**, gốc ở góc trên-trái. `docs/FORMATS.md` có ví dụ đọc chúng trong Godot và Unity.", "Boxes are exported in `meta.coral.hitboxes` of the JSON exports (JSON Hash / Array): **untrimmed** sprite pixels, top-left origin. `docs/FORMATS.md` shows how to read them in Godot and Unity."]),
    ),
  ],
};

// ---------------------------------------------------------------- Flipbook

const flipbook: HelpTopic = {
  id: "flipbook",
  title: { vi: "Flipbook — sheet SubUV cho VFX", en: "Flipbook — SubUV sheets for VFX" },
  summary: {
    vi: "Ghép chuỗi frame đánh số thành một sheet N×M đều nhau (SubUV) cho Niagara / Unity particle, kèm JSON tuỳ chọn.",
    en: "Lay out a numbered frame sequence as an even N×M sheet (SubUV) for Niagara / Unity particles, with an optional JSON.",
  },
  sections: [
    section(
      "Cách dùng",
      "How to use",
      steps(
        ["Thả một thư mục frame đánh số (`explosion_0001.png … explosion_0064.png`) hoặc dùng **Add folder…** trong thẻ **Frames**. Frame được sắp theo thứ tự tự nhiên (2 trước 10).", "Drop a folder of numbered frames (`explosion_0001.png … explosion_0064.png`) or use **Add folder…** in the **Frames** card. Frames are sorted naturally (2 before 10)."],
        ["Chọn frame: **From** / **To** (đánh số từ 1; 0 = đầu / cuối) và **Step** (lấy 1 frame mỗi N frame).", "Choose the frames: **From** / **To** (1-based; 0 = first / last) and **Step** (take every Nth frame)."],
        ["Thẻ **Grid**: **Columns** / **Rows** (0 = Coral gợi ý lưới gần vuông nhất), **Cell** (cạnh dài của ô, px; 0 = giữ kích thước frame; thu phóng bằng Lanczos3) và **Max size** (1024–8192).", "The **Grid** card: **Columns** / **Rows** (0 = Coral suggests the grid closest to square), **Cell** (longer side, px; 0 = the frames' size; resized with Lanczos3) and **Max size** (1024–8192)."],
        ["Xem sheet và trình phát (chạy từng ô theo **FPS**). Bên dưới là thông số cho engine: Niagara SubUV **Sub Image Size** = cột × hàng; Unity Texture Sheet Animation **Tiles X / Y**.", "Check the sheet and the player (cell by cell at the **FPS**). Below it are the engine settings: Niagara SubUV **Sub Image Size** = cols × rows; Unity Texture Sheet Animation **Tiles X / Y**."],
        ["Kiểm tra **Output image** rồi bấm **Write flipbook**.", "Check the **Output image** and click **Write flipbook**."],
      ),
    ),
    section(
      "Đầu ra",
      "Output",
      tips(
        ["**Output image** mặc định là `T_<Thư mục>_SubUV.png` cạnh thư mục frame (ví dụ `T_Explosion_SubUV.png`). Đổi đuôi thành `.tga` để ghi TGA.", "The **Output image** defaults to `T_<Folder>_SubUV.png` next to the frame folder (e.g. `T_Explosion_SubUV.png`). Use a `.tga` extension to write TGA."],
        ["**Also write a .json (cols, rows, frames, fps, cell)**: ghi thêm `<tên>.json` cạnh ảnh.", "**Also write a .json (cols, rows, frames, fps, cell)**: also writes `<name>.json` next to the image."],
        ["**Write flipbook** ghi ngay, không có hộp thoại xác nhận, và thay thế file cùng tên nếu đã có.", "**Write flipbook** writes at once, without a confirmation dialog, and replaces files of the same name."],
      ),
    ),
    section(
      "Lưu ý",
      "Notes",
      tips(
        ["Không trim, không padding: mọi ô cùng kích thước. Frame khác kích thước được co về kích thước của frame đầu tiên (có cảnh báo).", "No trim, no padding: every cell is the same size. Frames of other sizes are resized to the first frame's size (with a warning)."],
        ["Nếu sheet vượt **Max size**, các ô tự thu nhỏ cho vừa. Lưới quá nhỏ so với số frame thì các frame cuối bị bỏ (có cảnh báo).", "If the sheet would exceed **Max size**, cells shrink to fit. A grid too small for the frames leaves the last ones out (with a warning)."],
        ["Ô được điền trái sang phải, trên xuống dưới. Tuỳ chọn được nhớ giữa các lần mở ứng dụng (trừ danh sách frame).", "Cells fill left to right, top to bottom. Options are remembered between sessions (except the frame list)."],
      ),
    ),
  ],
};

// ---------------------------------------------------------------- Pixel

const pixel: HelpTopic = {
  id: "pixel",
  title: { vi: "Pixel — phóng to, palette, đổi màu, bleed", en: "Pixel — upscale, palettes, swaps, bleed" },
  summary: {
    vi: "Bốn công cụ hàng loạt cho pixel art: Upscale (nearest neighbour), Remap về palette (OKLab), Swap tạo biến thể màu, Bleed tô màu pixel trong suốt.",
    en: "Four batch tools for pixel art: Upscale (nearest neighbour), Remap to a palette (OKLab), Swap to make colour variants, and Bleed for transparent pixels.",
  },
  sections: [
    section(
      "Cách dùng chung",
      "Common workflow",
      steps(
        ["Chọn công cụ trong thẻ **Tool**: **Upscale**, **Remap**, **Swap** hoặc **Bleed**.", "Pick the tool in the **Tool** card: **Upscale**, **Remap**, **Swap** or **Bleed**."],
        ["Thêm đầu vào: thả ảnh / thư mục (thư mục gồm cả thư mục con) hoặc **Add folder…** / **Add file…**. **Before / after** hiện ảnh đầu tiên và kết quả của nó.", "Add inputs: drop images / folders (folders include sub-folders) or use **Add folder…** / **Add file…**. **Before / after** shows the first input and its result."],
        ["Chọn **Output**: **Folder** (một thư mục; để trống = cạnh từng file gốc) hoặc **In place (.bak)** (ghi đè file gốc sau khi sao lưu sang `<file>.bak`).", "Choose the **Output**: **Folder** (a folder; empty = next to each input) or **In place (.bak)** (overwrite the inputs after copying each to `<file>.bak`)."],
        ["Bấm **Dry run** để xem danh sách file, rồi bấm nút chính (ví dụ **Upscale ×4**) để ghi. Nút chính ghi ngay, không hỏi lại.", "Click **Dry run** to see the file list, then the main button (e.g. **Upscale ×4**) to write. The main button writes at once, without asking."],
      ),
    ),
    section(
      "Upscale",
      "Upscale",
      p(
        "Phóng to theo nearest neighbour, **Factor** ×2, ×3, ×4, ×5, ×6 hoặc ×8 — pixel giữ sắc nét. File ra: `<tên>_x4.png` (theo hệ số, giữ định dạng PNG / TGA của file gốc).",
        "Nearest-neighbour upscale, **Factor** ×2, ×3, ×4, ×5, ×6 or ×8 — pixels stay crisp. Output: `<name>_x4.png` (per factor, same PNG / TGA format as the input).",
      ),
    ),
    section(
      "Remap (đưa về palette)",
      "Remap (to a palette)",
      p(
        "Mỗi màu được thay bằng màu gần nhất trong palette (đo trong không gian OKLab, alpha giữ nguyên). File ra: `<tên>_remap.png`. Kết quả liệt kê số pixel đã đổi và màu nào bị gộp vào màu nào. Nút **Remap to palette** cần palette có ít nhất một màu.",
        "Every colour moves to the nearest palette colour (measured in OKLab, alpha kept). Output: `<name>_remap.png`. The result lists the pixels changed and which colours were merged into which. **Remap to palette** needs a palette with at least one colour.",
      ),
      tips(
        ["**Extract from inputs**: lấy palette từ chính các ảnh đầu vào, sắp **By use** (dùng nhiều nhất trước) hoặc **By hue**. Tối đa 4096 màu.", "**Extract from inputs**: builds the palette from the inputs, sorted **By use** (most used first) or **By hue**. Up to 4096 colours."],
        ["Nút tải lên (**Load palette**) mở Lospec `.hex`, GIMP `.gpl` hoặc JASC `.pal`; nút tải xuống (**Save palette**) lưu palette.", "The upload button (**Load palette**) opens a Lospec `.hex`, GIMP `.gpl` or JASC `.pal`; the download button (**Save palette**) saves the palette."],
        ["**+** thêm màu; bấm một ô màu để bỏ nó khỏi palette.", "**+** adds a colour; click a swatch to remove it from the palette."],
      ),
    ),
    section(
      "Swap (biến thể màu)",
      "Swap (colour variants)",
      p(
        "Mỗi bảng swap là một biến thể: danh sách cặp màu `từ → đến`. Mỗi bảng tạo file `<tên>_<biến thể>.png` và **không bao giờ đụng vào file gốc**: biến thể được ghi vào **Output folder**, hoặc cạnh file gốc khi để trống.",
        "Each swap table is one variant: a list of `from → to` colour pairs. Each table makes `<name>_<variant>.png` and **never touches the source**: variants go to the **Output folder**, or next to the source when it is empty.",
      ),
      steps(
        ["Lấy palette (**Extract from inputs** hoặc tải file) — các ô màu của palette hiện trong mỗi bảng.", "Get a palette (**Extract from inputs** or load one) — its swatches appear in every table."],
        ["Đặt tên biến thể (ô bên trái, ví dụ `alt`), bấm một ô màu để thêm cặp, rồi chọn màu đích. **+ pair** thêm cặp trống; **×** bỏ cặp.", "Name the variant (the box on the left, e.g. `alt`), click a swatch to add a pair, then pick the target colour. **+ pair** adds an empty pair; **×** removes one."],
        ["**+** trên thẻ **Swap tables** thêm biến thể; thùng rác bỏ biến thể. Bấm **Make variants** để ghi.", "**+** on the **Swap tables** card adds a variant; the bin removes one. Click **Make variants** to write."],
      ),
      tips(["**Save to project** lưu các bảng vào project đang mở (nhớ Ctrl+S); **From project** nạp lại chúng.", "**Save to project** stores the tables in the open project (then Ctrl+S); **From project** loads them back."]),
    ),
    section(
      "Bleed",
      "Bleed",
      p(
        "Pixel trong suốt lấy màu của pixel lân cận, alpha vẫn bằng 0 — hết viền tối khi engine lọc texture hoặc tạo mipmap. Ảnh trông không đổi.",
        "Transparent pixels take their neighbours' colour while alpha stays 0 — no dark fringes when the engine filters textures or builds mipmaps. The image looks the same.",
      ),
      tips([
        "Khi ghi ra thư mục (kể cả để trống = cạnh file gốc), file ra có hậu tố `_bleed` (ví dụ `hero_bleed.png`), file gốc không bị đụng tới. Muốn sửa trực tiếp file gốc thì chọn **In place (.bak)**: Coral tạo bản `.bak` trước khi ghi.",
        "When writing to a folder (including an empty one = next to each input), outputs get a `_bleed` suffix (e.g. `hero_bleed.png`) and the inputs are left alone. To change the inputs themselves, choose **In place (.bak)**: Coral makes a `.bak` copy first.",
      ]),
    ),
  ],
};

// ---------------------------------------------------------------- Normal 2D

const normal: HelpTopic = {
  id: "normal",
  title: { vi: "Normal 2D — normal map cho ánh sáng 2D", en: "Normal 2D — normal maps for 2D lighting" },
  summary: {
    vi: "Tạo normal map từ độ sáng hoặc vát viền alpha, chuẩn OpenGL / DirectX, xem trước có ánh sáng theo con trỏ; dùng cùng thiết lập cho normal atlas của Pack.",
    en: "Make normal maps from luminance or an alpha bevel, OpenGL / DirectX convention, with a lit preview that follows the pointer; reuse the settings for Pack's normal atlas.",
  },
  sections: [
    section(
      "Cách dùng",
      "How to use",
      steps(
        ["Thả sprite (hoặc thư mục) vào cửa sổ, hoặc **Add folder…** / **Add file…**. Bản xem trước hiện sprite đầu tiên được chiếu sáng bởi một nguồn sáng theo con trỏ, cạnh normal map của nó; khi có nhiều ảnh, các nút tên file bên dưới dùng để chuyển ảnh.", "Drop sprites (or folders) onto the window, or use **Add folder…** / **Add file…**. The preview shows the first one lit by a light that follows the pointer, next to its normal map; with several images, the file-name buttons below switch between them."],
        ["Chọn **Height from**: **Luminance** (sáng = nổi) hoặc **Alpha bevel** (nổi dần từ mép sprite vào trong, rộng **Bevel** px — hợp với sprite tô màu phẳng).", "Choose **Height from**: **Luminance** (bright = raised) or **Alpha bevel** (raised from the sprite's edge inward over **Bevel** px — good for flat-coloured sprites)."],
        ["Chỉnh **Strength** (độ dốc), **Blur** (làm mềm, px) và **Invert (dark = raised)** nếu cần.", "Adjust **Strength** (steeper slopes), **Blur** (softer, px) and **Invert (dark = raised)** if needed."],
        ["Chọn **Green channel**: **OpenGL (Godot, Unity)** hoặc **DirectX (Unreal)**.", "Choose the **Green channel**: **OpenGL (Godot, Unity)** or **DirectX (Unreal)**."],
        ["Trong thẻ **Write**, đặt **Output folder** (để trống = cạnh từng file gốc), bấm **Dry run** để xem danh sách, rồi **Write normal maps**.", "In the **Write** card, set the **Output folder** (empty = next to each input), click **Dry run** to see the list, then **Write normal maps**."],
      ),
    ),
    section(
      "Đầu ra",
      "Output",
      tips(
        ["Mỗi ảnh đầu vào cho một `<tên>_n.png`. Pixel trong suốt là normal phẳng `(128, 128, 255)`, alpha giữ nguyên.", "Each input gets `<name>_n.png`. Transparent pixels are flat `(128, 128, 255)`, alpha kept."],
        ["**Write normal maps** ghi ngay, không hỏi lại; file `_n.png` đã có sẽ bị thay thế. File gốc không bao giờ bị ghi đè (không có chế độ In place).", "**Write normal maps** writes at once, without asking; existing `_n.png` files are replaced. Inputs are never overwritten (there is no In place mode)."],
      ),
    ),
    section(
      "Normal atlas cho Pack",
      "Normal atlas for Pack",
      p(
        "**Use for the project's normal atlas** chép thiết lập hiện tại vào project và bật **Normal-map atlas** trong Pack (nút hiện `(on)` khi đã bật): `hero_n.png` có đúng các ô như `hero.png`, nên một file mô tả atlas dùng được cho cả hai.",
        "**Use for the project's normal atlas** copies the current settings into the project and turns on **Normal-map atlas** in Pack (the button shows `(on)` once it is on): `hero_n.png` gets the same rects as `hero.png`, so one atlas description serves both.",
      ),
      tips(["Rê chuột trên bản xem trước để di chuyển nguồn sáng và kiểm tra hướng nổi / lõm trước khi ghi.", "Move the pointer over the preview to move the light and check what is raised or sunken before writing."]),
    ),
  ],
};

/** Every topic, in panel order: Getting started, then the modules in rail order. */
export const HELP_TOPICS: HelpTopic[] = [gettingStarted, pack, slice, animate, flipbook, pixel, normal];

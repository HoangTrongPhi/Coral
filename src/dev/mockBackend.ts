/**
 * Dev-only fake backend so the UI can be exercised in a plain browser: `npm run dev`, then open
 * `http://localhost:1452/?mock`. Never active in production builds or inside Tauri.
 * It answers every command in `src/lib/ipc.ts` (checked by `npm run check:mock`) and the plugin
 * commands the UI uses. Sprites are drawn on canvases; packing is a simple shelf packer in JS.
 * Deliberate problems: `broken.psd` cannot be read (error), the Aseprite file uses a blend mode (warn),
 * one sprite override points at a missing sprite (info).
 */
import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Animation, FileWrite, Issue, PackReport, PackedSprite, Project, Source, SpriteInfo, WatchStatus, WriteReport } from "../types/coral";

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const ART = "D:\\Art";
const HERO = `${ART}\\hero`;
const SPARK = `${ART}\\fx\\spark.aseprite`;
const BROKEN = `${ART}\\ui\\broken.psd`;
const PROJECT_FILE = `${ART}\\hero.coral.json`;
const SHEET = `${ART}\\sheets\\hero_sheet.png`;
const EXPLOSION = `${ART}\\fx\\explosion`;

// ---------------------------------------------------------------- drawing

function canvas(w: number, h: number) {
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  return c;
}

const toPng = (c: HTMLCanvasElement): Promise<ArrayBuffer> => new Promise((res) => c.toBlob((b) => void b!.arrayBuffer().then(res), "image/png"));

/** A 32×48 character frame with transparent margins (so trimming shows). */
function heroFrame(anim: "idle" | "run" | "attack", i: number): HTMLCanvasElement {
  const c = canvas(32, 48);
  const g = c.getContext("2d")!;
  const bob = anim === "idle" ? Math.round(Math.sin((i / 4) * Math.PI * 2) * 1.5) : anim === "run" ? (i % 2) * 2 : 0;
  const body = { idle: "#4f8cff", run: "#22c55e", attack: "#f97316" }[anim];
  g.fillStyle = "#1e293b";
  g.fillRect(10, 44, 12, 2); // shadow
  g.fillStyle = body;
  g.fillRect(10, 18 + bob, 12, 18); // torso
  g.fillStyle = "#fcd9b6";
  g.beginPath();
  g.arc(16, 12 + bob, 6, 0, Math.PI * 2);
  g.fill(); // head
  g.fillStyle = "#334155";
  const leg = anim === "run" ? Math.round(Math.sin((i / 8) * Math.PI * 2) * 4) : 0;
  g.fillRect(11 + leg, 36 + bob, 4, 8 - bob);
  g.fillRect(17 - leg, 36 + bob, 4, 8 - bob);
  if (anim === "attack") {
    g.fillStyle = "#e2e8f0";
    const reach = [2, 6, 12, 14, 10, 4][i] ?? 0;
    g.fillRect(22, 24, reach, 3); // sword
  }
  return c;
}

function sparkFrame(i: number): HTMLCanvasElement {
  const c = canvas(24, 24);
  const g = c.getContext("2d")!;
  const r = 3 + i * 2.5;
  const grad = g.createRadialGradient(12, 12, 0, 12, 12, r);
  grad.addColorStop(0, "#fff7ae");
  grad.addColorStop(0.5, "#facc15");
  grad.addColorStop(1, "rgba(249,115,22,0)");
  g.fillStyle = grad;
  g.beginPath();
  g.arc(12, 12, r, 0, Math.PI * 2);
  g.fill();
  return c;
}

function genericFrame(i: number, w = 32, h = 32): HTMLCanvasElement {
  const c = canvas(w, h);
  const g = c.getContext("2d")!;
  g.fillStyle = `hsl(${(i * 47) % 360} 70% 55%)`;
  if (i % 2) g.fillRect(4 + (i % 4), 6, w - 12, h - 12);
  else {
    g.beginPath();
    g.arc(w / 2, h / 2, w / 2 - 4, 0, Math.PI * 2);
    g.fill();
  }
  return c;
}

/** The sheet the Slice module opens: six shapes in two uneven rows on 32×32 cells. */
function sheetCanvas(): HTMLCanvasElement {
  const c = canvas(128, 64);
  const g = c.getContext("2d")!;
  const shapes: [number, number][] = [[4, 3], [36, 5], [68, 4], [6, 36], [38, 35], [70, 38]];
  shapes.forEach(([x, y], i) => {
    g.fillStyle = `hsl(${i * 55} 70% 55%)`;
    g.fillRect(x, y, 22, 24);
  });
  return c;
}

// ---------------------------------------------------------------- sources

type MockSprite = { name: string; c: HTMLCanvasElement; origin: string; frame?: number; durationMs?: number };
const spriteCache = new Map<string, MockSprite[]>();

function spritesOf(src: Source): { sprites: MockSprite[]; anims: Animation[]; issues: Issue[] } {
  const anim = (name: string, frames: string[], fps: number, loop: Animation["loop"] = "loop", durationsMs: number[] | null = null): Animation => ({ name, frames, fps, loop, durationsMs });
  if (src.path === BROKEN) return { sprites: [], anims: [], issues: [{ severity: "error", code: "unreadable-psd", message: `${BROKEN}: invalid PSD signature (saved with Large Document Format?)`, path: BROKEN }] };
  let sprites = spriteCache.get(src.path);
  if (src.path === SPARK) {
    sprites ??= [0, 1, 2, 3].map((i) => ({ name: `spark_burst_${i}`, c: sparkFrame(i), origin: SPARK, frame: i, durationMs: i === 3 ? 200 : 80 }));
    spriteCache.set(src.path, sprites);
    return {
      sprites,
      anims: [anim("burst", sprites.map((s) => s.name), 12.5, "once", [80, 80, 80, 200])],
      issues: [{ severity: "warn", code: "aseprite-blend-mode", message: `${SPARK}: layers with blend modes other than Normal are merged as Normal`, path: SPARK }],
    };
  }
  if (src.path === HERO || src.path.toLowerCase().endsWith("hero")) {
    if (!sprites) {
      sprites = [];
      for (const [a, n] of [["idle", 4], ["run", 8], ["attack", 6]] as const) for (let i = 0; i < n; i++) sprites.push({ name: `hero_${a}_${i}`, c: heroFrame(a, i), origin: `${src.path}\\hero_${a}_${i}.png` });
      // hero_idle_3 repeats hero_idle_1: merged as an alias.
      sprites[3] = { ...sprites[3], c: sprites[1].c };
      spriteCache.set(src.path, sprites);
    }
    return { sprites, anims: [anim("hero_attack", sprites.filter((s) => s.name.includes("attack")).map((s) => s.name), 12), anim("hero_idle", sprites.filter((s) => s.name.includes("idle")).map((s) => s.name), 8), anim("hero_run", sprites.filter((s) => s.name.includes("run")).map((s) => s.name), 12)], issues: [] };
  }
  const base = src.path.split(/[\\/]/).pop()!.replace(/\.\w+$/, "").replace(/\W/g, "_");
  sprites ??= [...Array(6).keys()].map((i) => ({ name: `${base}_${i}`, c: genericFrame(i), origin: `${src.path}\\${base}_${i}.png` }));
  spriteCache.set(src.path, sprites);
  return { sprites, anims: [anim(base, sprites.map((s) => s.name), 10)], issues: [] };
}

function trimOf(c: HTMLCanvasElement, threshold: number) {
  const d = c.getContext("2d")!.getImageData(0, 0, c.width, c.height).data;
  let x0 = c.width, y0 = c.height, x1 = -1, y1 = -1;
  for (let y = 0; y < c.height; y++)
    for (let x = 0; x < c.width; x++)
      if (d[(y * c.width + x) * 4 + 3] > threshold) {
        x0 = Math.min(x0, x); y0 = Math.min(y0, y); x1 = Math.max(x1, x); y1 = Math.max(y1, y);
      }
  if (x1 < 0) return { sourceW: c.width, sourceH: c.height, x: 0, y: 0, w: 1, h: 1 };
  return { sourceW: c.width, sourceH: c.height, x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1 };
}

function scanProject(project: Project) {
  const issues: Issue[] = [];
  const sprites: (MockSprite & { trim: SpriteInfo["trim"]; key: string })[] = [];
  const anims = new Map<string, Animation>();
  for (const src of project.sources) {
    const r = spritesOf(src);
    issues.push(...r.issues);
    for (const s of r.sprites) {
      if (sprites.some((o) => o.name === s.name)) {
        issues.push({ severity: "error", code: "duplicate-name", message: `Sprite name "${s.name}" comes from two sources`, path: s.origin });
        continue;
      }
      const trim = project.pack.trim === "alpha" ? trimOf(s.c, project.pack.trimThreshold) : { sourceW: s.c.width, sourceH: s.c.height, x: 0, y: 0, w: s.c.width, h: s.c.height };
      const key = s.c.toDataURL();
      sprites.push({ ...s, trim, key });
    }
    for (const a of r.anims) anims.set(a.name, a);
  }
  for (const a of project.animations) anims.set(a.name, a);
  for (const name of Object.keys(project.sprites)) if (!sprites.some((s) => s.name === name)) issues.push({ severity: "info", code: "stale-override", message: `Settings for sprite "${name}" but no such sprite (renamed or removed?)` });
  const first = new Map<string, string>();
  const infos: SpriteInfo[] = sprites.map((s) => {
    const alias = project.pack.dedupe ? first.get(s.key) : undefined;
    if (!alias) first.set(s.key, s.name);
    const o = project.sprites[s.name];
    return { name: s.name, trim: s.trim, hash: String(s.key.length), origin: { path: s.origin, frame: s.frame }, aliasOf: alias, durationMs: s.durationMs, pivot: o?.pivot ?? [0.5, 0.5], nineSlice: o?.nineSlice ?? null };
  });
  return { sprites, infos, animations: [...anims.values()].sort((a, b) => a.name.localeCompare(b.name)), issues };
}

// ---------------------------------------------------------------- pack

const previews = new Map<string, ArrayBuffer>();
let nextJob = 1;
const cancelled = new Set<number>();

async function progress(jobId: number, done: number, total: number, message: string) {
  await emit("coral://progress", { jobId, done, total, message });
}

async function pack(project: Project, dryRun: boolean): Promise<PackReport> {
  const started = performance.now();
  const jobId = nextJob++;
  await progress(jobId, 0, 0, "Starting…");
  const scan = scanProject(project);
  await sleep(150);
  await progress(jobId, 1, 3, "Packing");
  if (cancelled.has(jobId)) return { name: project.name, pages: [], sprites: [], animations: [], aliases: 0, files: [], issues: scan.issues, dryRun, elapsedMs: 150, ok: false, cancelled: true };
  const { padding: pad, extrude: ext, maxSize, powerOfTwo } = project.pack;
  const unique = scan.infos.filter((i) => !i.aliasOf);
  const order = [...unique].sort((a, b) => b.trim.h - a.trim.h || a.name.localeCompare(b.name));
  // Shelf packer: rows of decreasing height, the narrowest power-of-two width that fits.
  let width = 64;
  let place: Map<string, { x: number; y: number }> = new Map();
  let height = 0;
  for (;;) {
    place = new Map();
    let x = 0, y = 0, row = 0, fits = true;
    for (const s of order) {
      const w = s.trim.w + 2 * ext, h = s.trim.h + 2 * ext;
      if (w > width) { fits = false; break; }
      if (x + w > width) { x = 0; y += row + pad; row = 0; }
      place.set(s.name, { x: x + ext, y: y + ext });
      x += w + pad;
      row = Math.max(row, h);
    }
    height = y + row;
    if (fits && (height <= width * 2 || width >= maxSize)) break;
    width *= 2;
  }
  const pageH = powerOfTwo ? 2 ** Math.ceil(Math.log2(Math.max(1, height))) : height;
  const pageW = width;
  const atlas = canvas(pageW, pageH);
  const g = atlas.getContext("2d")!;
  for (const s of scan.sprites) {
    const p = place.get(s.name);
    if (!p) continue;
    g.drawImage(s.c, s.trim.x, s.trim.y, s.trim.w, s.trim.h, p.x, p.y, s.trim.w, s.trim.h);
  }
  await sleep(150);
  await progress(jobId, 2, 3, "Composing");
  previews.set("atlas:0", await toPng(atlas));
  if (project.pack.normalMaps) previews.set("normal:0", await toPng(normalCanvas(atlas)));
  const sprites: PackedSprite[] = scan.infos.map((i) => {
    const p = place.get(i.aliasOf ?? i.name)!;
    return { name: i.name, page: 0, frame: { x: p.x, y: p.y, w: i.trim.w, h: i.trim.h }, rotated: false, trim: i.trim, pivot: i.pivot, nineSlice: i.nineSlice, aliasOf: i.aliasOf };
  });
  const area = unique.reduce((a, s) => a + s.trim.w * s.trim.h, 0);
  const issues = [...scan.issues];
  const blocked = issues.some((i) => i.severity === "error");
  const files: FileWrite[] = [];
  if (!project.exports.length) issues.push({ severity: "warn", code: "no-exports", message: "No export target: add one (JSON, Godot, Unity, Spine) to write files" });
  if (!blocked) {
    const status = dryRun ? "planned" : "written";
    const ext2 = project.pack.imageFormat;
    const seen = new Set<string>();
    const add = (path: string, kind: string, bytes: number) => {
      if (seen.has(path)) return;
      seen.add(path);
      files.push({ path, kind, status, bytes });
    };
    for (const e of project.exports) {
      if (!e.out) { issues.push({ severity: "error", code: "export-no-folder", message: `${e.target}: no output folder` }); continue; }
      add(`${e.out}\\${project.name}.${ext2}`, "atlas", 48_210);
      if (project.pack.normalMaps) add(`${e.out}\\${project.name}_n.${ext2}`, "normal-atlas", 31_004);
      if (e.target === "jsonHash" || e.target === "jsonArray") add(`${e.out}\\${project.name}.${e.paper2d ? "paper2dsprites" : "json"}`, "json", 9_412);
      if (e.target === "godot") add(`${e.out}\\${project.name}.tres`, "tres", 6_120);
      if (e.target === "unity") add(`${e.out}\\${project.name}.${ext2}.meta`, "meta", 14_880);
      if (e.target === "spine") add(`${e.out}\\${project.name}.atlas`, "atlas-text", 3_302);
    }
  } else if (project.exports.length) {
    issues.push({ severity: "info", code: "export-skipped", message: "Nothing was written because of the errors above" });
  }
  await progress(jobId, 3, 3, "Packed");
  const ok = !issues.some((i) => i.severity === "error");
  if (!dryRun) await emit("log://entry", { tsMs: Date.now(), level: ok ? "info" : "error", source: "pack", message: `${project.name}: ${sprites.length} sprite(s) on 1 page(s) ${pageW}×${pageH}; wrote ${files.length} file(s)` });
  return { name: project.name, pages: [{ index: 0, width: pageW, height: pageH, fill: area / (pageW * pageH), sprites: sprites.length }], sprites, animations: scan.animations, aliases: sprites.filter((s) => s.aliasOf).length, files, issues, dryRun, elapsedMs: Math.round(performance.now() - started), ok, cancelled: false };
}

/** A rough normal map from the alpha edge (enough to see the lit preview work). */
function normalCanvas(src: HTMLCanvasElement | HTMLImageElement): HTMLCanvasElement {
  const w = src.width, h = src.height;
  const c = canvas(w, h);
  const g = c.getContext("2d")!;
  g.drawImage(src, 0, 0);
  const img = g.getImageData(0, 0, w, h);
  const d = img.data;
  const lum = (x: number, y: number) => {
    x = Math.max(0, Math.min(w - 1, x)); y = Math.max(0, Math.min(h - 1, y));
    const i = (y * w + x) * 4;
    return ((d[i] + d[i + 1] + d[i + 2]) / 765) * (d[i + 3] / 255);
  };
  const out = g.createImageData(w, h);
  for (let y = 0; y < h; y++)
    for (let x = 0; x < w; x++) {
      const i = (y * w + x) * 4;
      if (!d[i + 3]) { out.data.set([128, 128, 255, 0], i); continue; }
      const dx = lum(x + 1, y) - lum(x - 1, y), dy = lum(x, y + 1) - lum(x, y - 1);
      const nx = -dx * 4, ny = dy * 4, len = Math.hypot(nx, ny, 1);
      out.data.set([((nx / len) * 0.5 + 0.5) * 255, ((ny / len) * 0.5 + 0.5) * 255, ((1 / len) * 0.5 + 0.5) * 255, d[i + 3]], i);
    }
  g.putImageData(out, 0, 0);
  return c;
}

function sampleProject(): Project {
  return {
    format: "coral.project", version: 1, name: "hero",
    sources: [
      { kind: "folder", path: HERO, recursive: true, include: ["*.png"] },
      { kind: "aseprite", path: SPARK, layers: "visible" },
      { kind: "psd", path: BROKEN, mode: "layers" },
    ],
    sprites: { hero_idle_0: { pivot: [0.5, 1], nineSlice: null }, hero_old_pose: { pivot: [0.5, 1], nineSlice: null } },
    animations: [],
    hitboxes: { hero_attack_3: [{ kind: "hitbox", rect: [22, 22, 10, 6], name: "sword" }, { kind: "hurtbox", rect: [10, 6, 12, 30] }] },
    pack: { maxSize: 2048, powerOfTwo: true, square: false, allowRotation: false, padding: 2, extrude: 1, trim: "alpha", trimThreshold: 0, alphaBleed: true, dedupe: true, premultiply: false, heuristic: "bestShortSideFit", multiPage: true, imageFormat: "png", normalMaps: false },
    exports: [
      { target: "godot", out: "D:\\Game\\assets\\hero", godotProject: "D:\\Game" },
      { target: "jsonHash", out: `${ART}\\build` },
    ],
    watch: { autoStart: false },
  };
}

function writeReport(paths: string[], dryRun: boolean, suffix: string, notes: string[] = []): WriteReport {
  const files: FileWrite[] = paths.flatMap((p) => (/\.\w+$/.test(p) ? [p] : [`${p}\\a.png`, `${p}\\b.png`, `${p}\\c.png`])).map((p) => ({
    path: p.replace(/(\.\w+)$/, `${suffix}$1`), kind: "image", status: dryRun ? "planned" : "written", bytes: 2_048, source: p,
  }));
  const issues: Issue[] = paths.some((p) => /\.txt$/i.test(p)) ? [{ severity: "warn", code: "not-an-image", message: "Not a PNG/TGA/GIF image: notes.txt" }] : [];
  return { files, issues, dryRun, elapsedMs: 42, ok: true, cancelled: false, notes };
}

/** The picture a path stands for: the sample sheet, or one of the hero frames. */
function imageFor(path: string): HTMLCanvasElement {
  if (/sheet/i.test(path)) return sheetCanvas();
  const hero = spritesOf({ kind: "folder", path: HERO, recursive: true, include: [] }).sprites;
  const i = Math.abs([...path].reduce((h, ch) => (h * 31 + ch.charCodeAt(0)) | 0, 7)) % hero.length;
  return hero[i].c;
}

// ---------------------------------------------------------------- watches

const watches = new Map<number, { timer: ReturnType<typeof setInterval>; status: WatchStatus }>();
let nextWatch = 1;

if (import.meta.env.DEV && new URLSearchParams(location.search).has("mock")) {
  console.info("mockBackend: active (?mock)");
  // `isTauri()` reads this flag; set it before lib/ipc.ts evaluates `inTauri`.
  (globalThis as { isTauri?: boolean }).isTauri = true;
  mockWindows("main");
  mockIPC(
    async (cmd, args) => {
      const a = (args ?? {}) as Record<string, any>; // eslint-disable-line @typescript-eslint/no-explicit-any
      switch (cmd) {
        case "app_info":
          return { version: "0.1.0-mock", debug: true, abyss: { managed: true, launcherVersion: "0.1.0" } };
        case "log_history":
          return [
            { tsMs: Date.now() - 2000, level: "info", source: "app", message: "Coral started (mock backend)" },
            { tsMs: Date.now() - 1000, level: "debug", source: "app", message: "Debug lines are hidden until you click the debug pill" },
          ];
        case "ui_log":
          return null;
        case "cancel_job":
          cancelled.add(a.jobId);
          return true;
        case "open_url":
          window.open(a.url, "_blank", "noopener");
          return null;
        case "reveal_in_explorer":
          console.info("mockBackend: reveal", a.path);
          return null;
        case "get_settings":
          return JSON.parse(localStorage.getItem("coral.mock-settings") ?? "{}");
        case "set_settings":
          localStorage.setItem("coral.mock-settings", JSON.stringify(a.settings));
          return null;

        // Projects
        case "load_project": {
          if (String(a.path).includes("missing")) throw `${a.path}: The system cannot find the file specified. (os error 2)`;
          const project = sampleProject();
          const s = scanProject(project);
          return { project, path: a.path, sprites: s.infos, animations: s.animations, issues: s.issues };
        }
        case "save_project":
          await sleep(80);
          return String(a.path).endsWith(".coral.json") ? a.path : `${String(a.path).replace(/\.json$/, "")}.coral.json`;
        case "sources_for_paths": {
          const sources: Source[] = [];
          const ignored: string[] = [];
          for (const p of a.paths as string[]) {
            if (/\.aseprite$|\.ase$/i.test(p)) sources.push({ kind: "aseprite", path: p, layers: "visible" });
            else if (/\.psd$/i.test(p)) sources.push({ kind: "psd", path: p, mode: "layers" });
            else if (/\.gif$/i.test(p)) sources.push({ kind: "gif", path: p });
            else if (/\.(png|tga)$/i.test(p)) sources.push({ kind: "folder", path: p.replace(/[\\/][^\\/]+$/, ""), recursive: false, include: ["*.png"] });
            else if (!/\.\w+$/.test(p)) sources.push({ kind: "folder", path: p, recursive: true, include: ["*.png", "*.tga"] });
            else ignored.push(p);
          }
          return { sources, ignored };
        }
        case "scan_sources": {
          const s = scanProject(a.project);
          return { sprites: s.infos, animations: s.animations, issues: s.issues };
        }
        case "pack_preview":
          return pack(a.project, true);
        case "pack_and_export":
          return pack(a.project, a.options?.dryRun ?? false);
        case "sprite_thumbnail": {
          for (const src of (a.project as Project).sources) {
            const s = spritesOf(src).sprites.find((x) => x.name === a.name);
            if (s) return toPng(s.c);
          }
          throw `No sprite named ${a.name}`;
        }
        case "preview_png": {
          const b = previews.get(a.key);
          if (!b) throw `No preview "${a.key}" yet`;
          return b.slice(0);
        }
        case "image_png":
          return toPng(imageFor(String(a.path)));

        // Slice
        case "slice_sheet": {
          const o = a.options;
          const rects = o.kind === "grid"
            ? [...Array(Math.floor(64 / o.cellH)).keys()].flatMap((r) => [...Array(Math.floor(128 / o.cellW)).keys()].map((c) => ({ x: o.offset[0] + c * (o.cellW + o.spacing[0]), y: o.offset[1] + r * (o.cellH + o.spacing[1]), w: o.cellW, h: o.cellH }))).filter((r) => o.keepEmpty || r.x < 96)
            : [[4, 3], [36, 5], [68, 4], [6, 36], [38, 35], [70, 38]].map(([x, y]) => ({ x, y, w: 22, h: 24 }));
          const base = String(a.path).split(/[\\/]/).pop()!.replace(/\.\w+$/, "");
          return { width: 128, height: 64, rects, suggestedNames: rects.map((_, i) => `${base}_${i}`) };
        }
        case "apply_slice": {
          const r = a.request;
          await sleep(200);
          const names = new Set<string>();
          const issues: Issue[] = [];
          for (const x of r.rects) { if (names.has(x.name)) issues.push({ severity: "error", code: "duplicate-name", message: `Two rects are named "${x.name}"` }); names.add(x.name); }
          const status = r.dryRun ? "planned" : "written";
          const dir = r.outDir || r.path.replace(/[\\/][^\\/]+$/, "");
          const files: FileWrite[] = issues.length ? [] : r.output === "sprites" ? r.rects.map((x: { name: string }) => ({ path: `${dir}\\${x.name}.png`, kind: "sprite", status, bytes: 640, source: r.path })) : [{ path: r.output === "unityMeta" ? `${r.path}.meta` : `${dir}\\${r.path.split(/[\\/]/).pop().replace(/\.\w+$/, "")}.json`, kind: r.output === "unityMeta" ? "meta" : "json", status, bytes: 4_096, source: r.path }];
          return { files, issues, dryRun: r.dryRun, elapsedMs: 31, ok: !issues.length, cancelled: false, notes: [`${r.rects.length} sprite(s)`] };
        }

        // Flipbook
        case "build_flipbook": {
          const o = a.options;
          const jobId = nextJob++;
          await progress(jobId, 0, 0, "Starting…");
          const frames = 64;
          const cols = o.cols || 8;
          const rows = o.rows || Math.ceil(frames / cols);
          const cell = o.cell || 64;
          const c = canvas(cols * cell, rows * cell);
          const g = c.getContext("2d")!;
          for (let i = 0; i < Math.min(frames, cols * rows); i++) {
            const x = (i % cols) * cell + cell / 2, y = Math.floor(i / cols) * cell + cell / 2;
            const t = i / frames;
            const grad = g.createRadialGradient(x, y, 0, x, y, cell * 0.48 * Math.sqrt(t + 0.05));
            grad.addColorStop(0, `rgba(255,250,200,${1 - t})`);
            grad.addColorStop(0.5, `rgba(250,140,30,${0.9 - t * 0.8})`);
            grad.addColorStop(1, "rgba(60,20,10,0)");
            g.fillStyle = grad;
            g.fillRect(x - cell / 2, y - cell / 2, cell, cell);
            if (i % 8 === 7) await progress(jobId, i + 1, frames, `explosion_${String(i + 1).padStart(4, "0")}.png`);
          }
          previews.set("flipbook", await toPng(c));
          const status = o.dryRun ? "planned" : "written";
          const files: FileWrite[] = o.out ? [{ path: o.out, kind: "image", status, bytes: 812_000 }, ...(o.json ? [{ path: o.out.replace(/\.\w+$/, ".json"), kind: "json", status, bytes: 120 } as FileWrite] : [])] : [];
          const issues: Issue[] = o.out ? [] : [{ severity: "error", code: "no-output", message: "Choose where to save the flipbook" }];
          if (cols * rows < frames) issues.push({ severity: "warn", code: "grid-too-small", message: `${cols}×${rows} holds ${cols * rows} frames; the last ${frames - cols * rows} are left out` });
          return { frames: Math.min(frames, cols * rows), cols, rows, cell: [cell, cell], width: cols * cell, height: rows * cell, sources: [...Array(frames).keys()].map((i) => `${EXPLOSION}\\explosion_${String(i + 1).padStart(4, "0")}.png`), write: { files, issues, dryRun: o.dryRun, elapsedMs: 260, ok: !issues.some((i) => i.severity === "error"), cancelled: false, notes: [`Niagara SubUV: Sub Image Size = ${cols} × ${rows}`, `Unity Texture Sheet Animation: Tiles X = ${cols}, Y = ${rows}`] } };
        }

        // Pixel
        case "palette_extract":
          await sleep(120);
          return { colors: ["#4f8cff", "#fcd9b6", "#334155", "#22c55e", "#f97316", "#1e293b", "#e2e8f0", "#facc15"].map((hex, i) => ({ hex, count: 2_400 - i * 260 })), pixels: 12_000, files: (a.paths as string[]).length, issues: [] };
        case "palette_save":
          console.info("mockBackend: palette_save", a.path, a.colors.length);
          return null;
        case "palette_load":
          return ["#be4a2f", "#d77643", "#ead4aa", "#e4a672", "#b86f50", "#733e39", "#3e2731", "#a22633", "#e43b44", "#f77622", "#feae34", "#fee761", "#63c74d", "#3e8948", "#265c42", "#193c3e", "#124e89", "#0099db", "#2ce8f5", "#ffffff", "#c0cbdc", "#8b9bb4", "#5a6988", "#3a4466", "#262b44", "#181425", "#ff0044", "#68386c", "#b55088", "#f6757a", "#e8b796", "#c28569"];
        case "palette_remap":
          await sleep(300);
          return writeReport(a.options.paths, a.options.dryRun, "_remap", ["3 412 pixel(s) changed in 3 file(s), 9 colour(s) mapped onto the 32-colour palette", "#4f8cff → #0099db", "#22c55e → #63c74d"]);
        case "palette_swap":
          await sleep(300);
          return writeReport(a.options.paths, a.options.dryRun, `_${a.options.tables[0]?.variant ?? "alt"}`, a.options.tables.map((t: { variant: string }) => `${t.variant}: 880 pixel(s) swapped`));
        case "upscale_nearest":
          await sleep(250);
          return writeReport(a.options.paths, a.options.dryRun, `_x${a.options.factor}`, [`×${a.options.factor} nearest neighbour`]);
        case "alpha_bleed": {
          const r = writeReport(a.options.paths, a.options.dryRun, a.options.output.kind === "inPlace" ? "" : "");
          if (a.options.output.kind === "inPlace") r.files = r.files.flatMap((f) => [{ ...f, path: `${f.path}.bak`, kind: "backup" }, f]);
          return r;
        }

        // Normal 2D
        case "normal_map":
          await sleep(400);
          return writeReport(a.options.paths, a.options.dryRun, "_n", [`${a.options.paths.length} normal map(s)`]);
        case "normal_preview":
          return toPng(normalCanvas(imageFor(String(a.path))));

        // Watch
        case "watch_start": {
          const watchId = nextWatch++;
          let runs = 0;
          const status: WatchStatus = { watchId, state: "idle", lastRunAt: Date.now(), lastDurationMs: 380, lastSummary: "22 sprite(s) (1 alias) on 1 page(s) 256×256 (41%); wrote 3 file(s)", lastError: null, lastIssues: 1, changedPaths: [] };
          const tick = async () => {
            runs++;
            const failed = runs % 3 === 0;
            Object.assign(status, {
              state: failed ? "error" : "idle",
              lastRunAt: Date.now(),
              lastDurationMs: 250 + Math.round(Math.random() * 300),
              lastError: failed ? `${BROKEN}: invalid PSD signature` : null,
              changedPaths: [`${HERO}\\hero_run_${runs % 8}.png`],
            });
            await emit("watch://status", { ...status });
            await emit("log://entry", { tsMs: Date.now(), level: failed ? "error" : "info", source: "watch", message: failed ? String(status.lastError) : `Packed (1 change(s)): ${status.lastSummary}` });
          };
          watches.set(watchId, { timer: setInterval(() => void tick(), 5000), status });
          setTimeout(() => void emit("watch://status", { ...status }), 300);
          return watchId;
        }
        case "watch_stop": {
          const w = watches.get(a.watchId);
          if (w) clearInterval(w.timer);
          return watches.delete(a.watchId);
        }
        case "watch_list":
          return [...watches.values()].map((w) => w.status);

        // Plugins
        case "plugin:dialog|open": {
          const o = a.options ?? {};
          if (o.directory) return HERO;
          const ext: string[] = o.filters?.[0]?.extensions ?? [];
          if (ext.includes("json")) return PROJECT_FILE;
          if (ext.includes("hex")) return "D:\\Palettes\\endesga-32.hex";
          return o.multiple ? [SHEET] : SHEET;
        }
        case "plugin:dialog|save":
          return a.options?.defaultPath?.endsWith(".hex") ? "D:\\Palettes\\hero.hex" : PROJECT_FILE;
        case "plugin:updater|check":
          return { rid: 1, currentVersion: "0.1.0-mock", version: "0.2.0", date: "2026-11-01T00:00:00Z", body: "- Animate: onion skin", rawJson: {} };
        case "plugin:updater|download_and_install":
          await sleep(1500);
          return null;
        case "plugin:window|set_theme":
        case "plugin:webview|set_webview_zoom":
          return null;
        case "plugin:process|restart":
          location.reload();
          return null;
        default:
          if (cmd.startsWith("plugin:event|")) return 0;
          console.warn("mockBackend: unhandled", cmd, args);
          return null;
      }
    },
    { shouldMockEvents: true },
  );
}

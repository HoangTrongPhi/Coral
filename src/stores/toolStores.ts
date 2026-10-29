// Stores of the Slice, Flipbook, Pixel and Normal 2D modules.
import { create } from "zustand";
import { persist } from "zustand/middleware";
import { registerDropHandler } from "../lib/dropRouter";
import { inTauri, ipc, loadImage, pngUrl } from "../lib/ipc";
import { cancelJob, dirName, withJob, type RunningJob } from "../lib/jobs";
import type { FlipbookOptions, FlipbookReport, NamedRect, NormalSettings, OutputMode, PaletteColor, SliceOptions, SliceOutput, SwapTable, WriteReport } from "../types/coral";
import { errorText, uiLog } from "./appStore";

const IMAGE = /\.(png|tga|gif)$/i;

// ---------------------------------------------------------------- Slice

interface SliceState {
  path: string | null;
  image: HTMLImageElement | null;
  options: SliceOptions;
  rects: NamedRect[];
  selected: number | null;
  pattern: string;
  output: SliceOutput;
  outDir: string;
  result: WriteReport | null;
  plan: WriteReport | null;
  job: RunningJob | null;
  load: (path: string) => Promise<void>;
  setOptions: (o: SliceOptions) => void;
  detect: () => Promise<void>;
  setRects: (r: NamedRect[]) => void;
  updateRect: (i: number, r: Partial<NamedRect>) => void;
  select: (i: number | null) => void;
  rename: () => void;
  setOutput: (o: Partial<{ output: SliceOutput; outDir: string; pattern: string }>) => void;
  apply: (dryRun: boolean) => Promise<void>;
}

const sliceBase = (p: string | null) => (p ?? "sheet").split(/[\\/]/).pop()!.replace(/\.[^.]+$/, "");

export function patternNames(pattern: string, base: string, n: number): string[] {
  return [...Array(n).keys()].map((i) => {
    let s = pattern.replaceAll("{base}", base).replaceAll("{i1}", String(i + 1));
    s = s.replace(/\{i:0(\d+)\}/, (_, w) => String(i).padStart(Number(w), "0"));
    return s.replaceAll("{i}", String(i));
  });
}

export const useSliceStore = create<SliceState>()((set, get) => ({
  path: null,
  image: null,
  options: { kind: "auto", merge: 2, alphaThreshold: 0, minPixels: 4 },
  rects: [],
  selected: null,
  pattern: "{base}_{i}",
  output: "sprites",
  outDir: "",
  result: null,
  plan: null,
  job: null,

  async load(path) {
    try {
      const image = await loadImage(pngUrl(await ipc.imagePng(path)));
      set({ path, image, rects: [], selected: null, result: null, plan: null, outDir: get().outDir || `${dirName(path)}\\${sliceBase(path)}_sprites` });
      await get().detect();
    } catch (e) {
      uiLog("error", `${path}: ${errorText(e)}`, "slice");
    }
  },
  setOptions: (options) => set({ options }),
  async detect() {
    const { path, options, pattern } = get();
    if (!path) return;
    try {
      const r = await ipc.sliceSheet(path, options);
      const names = patternNames(pattern, sliceBase(path), r.rects.length);
      set({ rects: r.rects.map((rect, i) => ({ name: names[i], rect, pivot: null, nineSlice: null })), selected: null, result: null });
      uiLog("info", `${r.rects.length} sprite(s) found in ${r.width}×${r.height}`, "slice");
    } catch (e) {
      uiLog("error", errorText(e), "slice");
    }
  },
  setRects: (rects) => set({ rects }),
  updateRect: (i, r) => set((s) => ({ rects: s.rects.map((o, j) => (j === i ? { ...o, ...r } : o)) })),
  select: (selected) => set({ selected }),
  rename() {
    const { rects, pattern, path } = get();
    const names = patternNames(pattern, sliceBase(path), rects.length);
    set({ rects: rects.map((r, i) => ({ ...r, name: names[i] })) });
  },
  setOutput: (o) => set(o),
  async apply(dryRun) {
    const { path, rects, output, outDir } = get();
    if (!path || !rects.length) return;
    try {
      const r = await withJob("Slicing", (job) => set({ job }), () => ipc.applySlice({ path, rects, output, outDir, dryRun }));
      set(dryRun ? { plan: r } : { result: r, plan: null });
    } catch (e) {
      uiLog("error", errorText(e), "slice");
    }
  },
}));

registerDropHandler("slice", (paths) => {
  const img = paths.find((p) => IMAGE.test(p));
  if (img) void useSliceStore.getState().load(img);
  else uiLog("warn", "Slice takes one PNG, TGA or GIF sheet", "slice");
});

// ---------------------------------------------------------------- Flipbook

interface FlipbookState {
  options: FlipbookOptions;
  report: FlipbookReport | null;
  preview: HTMLImageElement | null;
  job: RunningJob | null;
  set: (o: Partial<FlipbookOptions>) => void;
  run: (dryRun: boolean) => Promise<void>;
  cancel: () => Promise<void>;
}

export const useFlipbookStore = create<FlipbookState>()(
  persist(
    (set, get) => ({
      options: { paths: [], from: 0, to: 0, step: 1, cols: 0, rows: 0, cell: 0, maxSize: 4096, fps: 30, format: "png", out: "", json: true, dryRun: true },
      report: null,
      preview: null,
      job: null,
      set: (o) => set((s) => ({ options: { ...s.options, ...o } })),
      async run(dryRun) {
        const { options } = get();
        if (!options.paths.length || !inTauri) return;
        try {
          const report = await withJob("Flipbook", (job) => set({ job }), () => ipc.buildFlipbook({ ...options, dryRun }));
          let preview: HTMLImageElement | null = null;
          if (report.frames) preview = await loadImage(pngUrl(await ipc.previewPng("flipbook")));
          set({ report, preview });
        } catch (e) {
          uiLog("error", errorText(e), "flipbook");
        }
      },
      cancel: () => cancelJob(get().job, (job) => set({ job })),
    }),
    { name: "coral.flipbook", partialize: (s) => ({ options: { ...s.options, paths: [] } }) },
  ),
);

/** Frames from `paths`; an empty output becomes `T_<Folder>_SubUV.png` next to the folder. */
export function setFlipbookFrames(paths: string[]) {
  const st = useFlipbookStore.getState();
  const folder = paths.find((p) => !/\.\w+$/.test(p)) ?? (paths[0] ? dirName(paths[0]) : "");
  const name = folder.split(/[\\/]/).pop() ?? "fx";
  const nice = name.replace(/[_\-\s]*\d+$/, "").replace(/(^|[_\-\s])(\w)/g, (_, __, c: string) => c.toUpperCase()) || "Fx";
  st.set({ paths, out: st.options.out || (folder ? `${dirName(folder)}\\T_${nice}_SubUV.png` : "") });
}

registerDropHandler("flipbook", (paths) => {
  setFlipbookFrames([paths.find((p) => !/\.\w+$/.test(p)) ?? dirName(paths[0])]);
  void useFlipbookStore.getState().run(true);
});

// ---------------------------------------------------------------- Pixel

export type PixelTool = "upscale" | "palette" | "swap" | "bleed";

interface PixelState {
  tool: PixelTool;
  paths: string[];
  output: OutputMode;
  factor: number;
  palette: string[];
  extracted: PaletteColor[];
  tables: SwapTable[];
  result: WriteReport | null;
  before: string | null;
  after: string | null;
  job: RunningJob | null;
  set: (s: Partial<Pick<PixelState, "tool" | "paths" | "output" | "factor" | "palette" | "tables">>) => void;
  extract: (sort: "frequency" | "hue") => Promise<void>;
  loadPalette: (path: string) => Promise<void>;
  savePalette: (path: string) => Promise<void>;
  run: (dryRun: boolean) => Promise<void>;
}

export const usePixelStore = create<PixelState>()((set, get) => ({
  tool: "upscale",
  paths: [],
  output: { kind: "folder", path: "" },
  factor: 4,
  palette: [],
  extracted: [],
  tables: [{ variant: "alt", map: [] }],
  result: null,
  before: null,
  after: null,
  job: null,
  set(s) {
    set(s);
    if (s.paths) void showBefore(s.paths);
  },
  async extract(sort) {
    const { paths } = get();
    if (!paths.length) return;
    try {
      const r = await ipc.paletteExtract(paths, sort);
      set({ extracted: r.colors, palette: r.colors.map((c) => c.hex) });
      for (const i of r.issues) uiLog(i.severity === "error" ? "error" : "warn", i.message, "pixel");
    } catch (e) {
      uiLog("error", errorText(e), "pixel");
    }
  },
  async loadPalette(path) {
    try {
      set({ palette: await ipc.paletteLoad(path), extracted: [] });
    } catch (e) {
      uiLog("error", `${path}: ${errorText(e)}`, "pixel");
    }
  },
  async savePalette(path) {
    await ipc.paletteSave(path, get().palette).catch((e) => uiLog("error", errorText(e), "pixel"));
  },
  async run(dryRun) {
    const { tool, paths, output, factor, palette, tables } = get();
    if (!paths.length) return;
    const setJob = (job: RunningJob | null) => set({ job });
    try {
      const r = await withJob(tool, setJob, () => {
        if (tool === "upscale") return ipc.upscaleNearest({ paths, factor, output, dryRun });
        if (tool === "palette") return ipc.paletteRemap({ paths, palette: { kind: "colors", colors: palette }, output, dryRun });
        if (tool === "swap") return ipc.paletteSwap({ paths, tables: tables.filter((t) => t.map.length), output, dryRun });
        return ipc.alphaBleed({ paths, output, dryRun });
      });
      set({ result: r });
      const first = r.files.find((f) => f.kind === "image" && f.status === "written");
      if (first) set({ after: pngUrl(await ipc.imagePng(first.path)) });
    } catch (e) {
      uiLog("error", errorText(e), "pixel");
    }
  },
}));

/** First input image (a file, or the first image in a folder per the backend) for before / after. */
async function showBefore(paths: string[]) {
  const file = paths.find((p) => IMAGE.test(p));
  usePixelStore.setState({ before: null, after: null, result: null });
  if (!file) return;
  try {
    usePixelStore.setState({ before: pngUrl(await ipc.imagePng(file)) });
  } catch {
    // Not decodable: the run will report it.
  }
}

registerDropHandler("pixel", (paths) => usePixelStore.getState().set({ paths: [...new Set([...usePixelStore.getState().paths, ...paths])] }));

// ---------------------------------------------------------------- Normal 2D

interface NormalState {
  paths: string[];
  settings: NormalSettings;
  output: OutputMode;
  /** The image the preview shows. */
  current: string | null;
  diffuse: HTMLImageElement | null;
  normal: HTMLImageElement | null;
  result: WriteReport | null;
  job: RunningJob | null;
  set: (s: Partial<Pick<NormalState, "paths" | "output" | "current">>) => void;
  setSettings: (s: Partial<NormalSettings>) => void;
  preview: () => Promise<void>;
  run: (dryRun: boolean) => Promise<void>;
}

export const DEFAULT_NORMAL: NormalSettings = { source: "luminance", strength: 2, blur: 0, bevel: 4, convention: "opengl", invert: false };

let previewTimer: ReturnType<typeof setTimeout> | null = null;

export const useNormalStore = create<NormalState>()((set, get) => ({
  paths: [],
  settings: DEFAULT_NORMAL,
  output: { kind: "folder", path: "" },
  current: null,
  diffuse: null,
  normal: null,
  result: null,
  job: null,
  set(s) {
    set(s);
    if (s.paths && !s.current) set({ current: s.paths.find((p) => IMAGE.test(p)) ?? null });
    void get().preview();
  },
  setSettings(s) {
    set((st) => ({ settings: { ...st.settings, ...s } }));
    if (previewTimer) clearTimeout(previewTimer);
    previewTimer = setTimeout(() => void get().preview(), 200);
  },
  async preview() {
    const { current, settings } = get();
    if (!current || !inTauri) return set({ diffuse: null, normal: null });
    try {
      const [d, n] = await Promise.all([ipc.imagePng(current), ipc.normalPreview(current, settings)]);
      set({ diffuse: await loadImage(pngUrl(d)), normal: await loadImage(pngUrl(n)) });
    } catch (e) {
      uiLog("error", errorText(e), "normal");
    }
  },
  async run(dryRun) {
    const { paths, settings, output } = get();
    if (!paths.length) return;
    try {
      const r = await withJob("Normal maps", (job) => set({ job }), () => ipc.normalMap({ paths, settings, output, dryRun }));
      set({ result: r });
    } catch (e) {
      uiLog("error", errorText(e), "normal");
    }
  },
}));

registerDropHandler("normal", (paths) => {
  const s = useNormalStore.getState();
  const all = [...new Set([...s.paths, ...paths])];
  s.set({ paths: all, current: paths.find((p) => IMAGE.test(p)) ?? s.current });
});

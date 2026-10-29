import { create } from "zustand";
import { pickOne, pickSavePath } from "../lib/dialogs";
import { registerDropHandler } from "../lib/dropRouter";
import { inTauri, ipc, loadImage, on, pngUrl } from "../lib/ipc";
import { cancelJob, withJob, type RunningJob } from "../lib/jobs";
import type { Animation, ExportTarget, Hitbox, PackReport, PackSettings, Project, Source, SpriteOverride, Target, WatchStatus } from "../types/coral";
import { errorText, uiLog } from "./appStore";

const RECENT_KEY = "coral.recent";
const PROJECT_FILTER = [{ name: "Coral project", extensions: ["json"] }];

export function newProject(name = "atlas"): Project {
  return {
    format: "coral.project",
    version: 1,
    name,
    sources: [],
    sprites: {},
    animations: [],
    hitboxes: {},
    pack: {
      maxSize: 2048,
      powerOfTwo: true,
      square: false,
      allowRotation: false,
      padding: 2,
      extrude: 1,
      trim: "alpha",
      trimThreshold: 0,
      alphaBleed: true,
      dedupe: true,
      premultiply: false,
      heuristic: "bestShortSideFit",
      multiPage: true,
      imageFormat: "png",
      normalMaps: false,
    },
    exports: [],
    watch: { autoStart: false },
  };
}

function readRecent(): string[] {
  try {
    return JSON.parse(localStorage.getItem(RECENT_KEY) ?? "[]");
  } catch {
    return [];
  }
}

function pushRecent(path: string) {
  const list = [path, ...readRecent().filter((p) => p !== path)].slice(0, 10);
  try {
    localStorage.setItem(RECENT_KEY, JSON.stringify(list));
  } catch {
    // Private mode: recent projects are a convenience.
  }
  useProjectStore.setState({ recent: list });
}

interface ProjectState {
  project: Project;
  /** Saved file, or null for an unsaved project. */
  path: string | null;
  dirty: boolean;
  recent: string[];

  /** Last pack preview (dry run) and its page images (object URLs). */
  report: PackReport | null;
  pages: string[];
  pageImages: HTMLImageElement[];
  normalPages: string[];
  /** Export plan waiting for confirmation (dry run of Pack & Export). */
  plan: PackReport | null;
  /** Result of the last real export. */
  exported: PackReport | null;
  selected: string | null;
  job: RunningJob | null;
  error: string | null;
  watch: WatchStatus | null;

  setJob: (job: RunningJob | null) => void;
  update: (fn: (p: Project) => Project, opts?: { repack?: boolean }) => void;
  newProject: () => void;
  open: (path?: string) => Promise<void>;
  save: () => Promise<boolean>;
  saveAs: () => Promise<boolean>;
  addPaths: (paths: string[]) => Promise<void>;
  removeSource: (i: number) => void;
  updateSource: (i: number, s: Source) => void;
  setPack: (p: Partial<PackSettings>) => void;
  addExport: (target: Target) => Promise<void>;
  updateExport: (i: number, e: Partial<ExportTarget>) => void;
  removeExport: (i: number) => void;
  setSpriteOverride: (name: string, o: Partial<SpriteOverride>) => void;
  setAnimation: (a: Animation) => void;
  setHitboxes: (name: string, boxes: Hitbox[]) => void;
  select: (name: string | null) => void;
  refresh: () => Promise<void>;
  planExport: () => Promise<void>;
  confirmExport: () => Promise<void>;
  dismissPlan: () => void;
  cancel: () => Promise<void>;
  startWatch: () => Promise<void>;
  stopWatch: () => Promise<void>;
}

let refreshTimer: ReturnType<typeof setTimeout> | null = null;
/** Re-pack a moment after the last edit (sources, pack settings). */
function scheduleRefresh() {
  if (refreshTimer) clearTimeout(refreshTimer);
  refreshTimer = setTimeout(() => void useProjectStore.getState().refresh(), 350);
}

function revoke(urls: string[]) {
  for (const u of urls) URL.revokeObjectURL(u);
}

export const useProjectStore = create<ProjectState>()((set, get) => ({
  project: newProject(),
  path: null,
  dirty: false,
  recent: readRecent(),
  report: null,
  pages: [],
  pageImages: [],
  normalPages: [],
  plan: null,
  exported: null,
  selected: null,
  job: null,
  error: null,
  watch: null,

  setJob: (job) => set({ job }),

  update(fn, opts = {}) {
    set((s) => ({ project: fn(s.project), dirty: true }));
    if (opts.repack ?? true) scheduleRefresh();
  },

  newProject() {
    void get().stopWatch();
    revoke(get().pages);
    set({ project: newProject(), path: null, dirty: false, report: null, pages: [], pageImages: [], normalPages: [], selected: null, error: null, exported: null });
  },

  async open(path) {
    const file = path ?? (await pickOne("file", PROJECT_FILTER));
    if (!file) return;
    try {
      const loaded = await ipc.loadProject(file);
      await get().stopWatch();
      set({ project: loaded.project, path: loaded.path, dirty: false, selected: null, error: null, exported: null });
      pushRecent(loaded.path);
      for (const i of loaded.issues.filter((i) => i.severity !== "info").slice(0, 10)) uiLog(i.severity === "error" ? "error" : "warn", i.message, "project");
      await get().refresh();
      if (loaded.project.watch.autoStart) await get().startWatch();
    } catch (e) {
      uiLog("error", `Cannot open ${file}: ${errorText(e)}`, "project");
    }
  },

  async save() {
    const { path } = get();
    if (!path) return get().saveAs();
    try {
      const saved = await ipc.saveProject(path, get().project);
      set({ path: saved, dirty: false });
      pushRecent(saved);
      return true;
    } catch (e) {
      uiLog("error", `Save failed: ${errorText(e)}`, "project");
      return false;
    }
  },

  async saveAs() {
    const target = await pickSavePath(`${get().project.name}.coral.json`, PROJECT_FILTER);
    if (!target) return false;
    set({ path: target });
    return get().save();
  },

  async addPaths(paths) {
    if (!paths.length) return;
    // A dropped project file opens it.
    const projectFile = paths.find((p) => p.toLowerCase().endsWith(".coral.json"));
    if (projectFile) return get().open(projectFile);
    try {
      const { sources, ignored } = await ipc.sourcesForPaths(paths);
      for (const p of ignored) uiLog("warn", `Not a sprite source: ${p}`, "pack");
      if (!sources.length) return;
      get().update((p) => {
        const known = new Set(p.sources.map((s) => s.path));
        const added = sources.filter((s) => !known.has(s.path));
        // An unsaved project takes its name from the first source.
        const name = p.sources.length === 0 && p.name === "atlas" && added[0] ? added[0].path.split(/[\\/]/).pop()!.replace(/\.[^.]+$/, "").replace(/[^\w-]/g, "_") || p.name : p.name;
        return { ...p, name, sources: [...p.sources, ...added] };
      });
    } catch (e) {
      uiLog("error", errorText(e), "pack");
    }
  },

  removeSource: (i) => get().update((p) => ({ ...p, sources: p.sources.filter((_, j) => j !== i) })),
  updateSource: (i, s) => get().update((p) => ({ ...p, sources: p.sources.map((o, j) => (j === i ? s : o)) })),
  setPack: (pack) => get().update((p) => ({ ...p, pack: { ...p.pack, ...pack } })),

  async addExport(target) {
    const out = await pickOne("folder");
    if (inTauri && !out) return;
    const e: ExportTarget = { target, out: out ?? "" };
    if (target === "unity") Object.assign(e, { pixelsPerUnit: 100, filter: "point" });
    get().update((p) => ({ ...p, exports: [...p.exports, e] }));
  },
  updateExport: (i, e) => get().update((p) => ({ ...p, exports: p.exports.map((o, j) => (j === i ? { ...o, ...e } : o)) })),
  removeExport: (i) => get().update((p) => ({ ...p, exports: p.exports.filter((_, j) => j !== i) })),

  setSpriteOverride: (name, o) =>
    get().update((p) => {
      const next = { ...p.sprites[name], ...o };
      const sprites = { ...p.sprites };
      if (next.pivot == null && next.nineSlice == null) delete sprites[name];
      else sprites[name] = next;
      return { ...p, sprites };
    }),
  setAnimation: (a) =>
    get().update((p) => ({ ...p, animations: [...p.animations.filter((o) => o.name !== a.name), a] })),
  setHitboxes: (name, boxes) =>
    get().update(
      (p) => {
        const hitboxes = { ...p.hitboxes };
        if (boxes.length) hitboxes[name] = boxes;
        else delete hitboxes[name];
        return { ...p, hitboxes };
      },
      { repack: false },
    ),
  select: (selected) => set({ selected }),

  async refresh() {
    const { project, job } = get();
    if (!inTauri || job) return;
    if (!project.sources.length) {
      revoke(get().pages);
      set({ report: null, pages: [], pageImages: [], normalPages: [], error: null });
      return;
    }
    try {
      const report = await withJob("Packing preview", get().setJob, () => ipc.packPreview(project));
      const pages = await Promise.all(report.pages.map(async (p) => pngUrl(await ipc.previewPng(`atlas:${p.index}`))));
      const normalPages = project.pack.normalMaps ? await Promise.all(report.pages.map(async (p) => pngUrl(await ipc.previewPng(`normal:${p.index}`)))) : [];
      const pageImages = await Promise.all(pages.map(loadImage));
      revoke([...get().pages, ...get().normalPages]);
      set({ report, pages, pageImages, normalPages, error: null });
      // The project changed while packing: pack again.
      if (get().project !== project) scheduleRefresh();
    } catch (e) {
      set({ error: errorText(e) });
    }
  },

  async planExport() {
    const { project } = get();
    if (!project.exports.length) {
      uiLog("warn", "Add an export target first (JSON, Godot, Unity or Spine)", "pack");
      return;
    }
    try {
      const plan = await withJob("Planning export", get().setJob, () => ipc.packAndExport(project, { dryRun: true }));
      set({ plan });
    } catch (e) {
      uiLog("error", errorText(e), "pack");
    }
  },
  async confirmExport() {
    const { project } = get();
    set({ plan: null });
    try {
      const exported = await withJob("Packing & exporting", get().setJob, () => ipc.packAndExport(project, { dryRun: false }));
      set({ exported });
    } catch (e) {
      uiLog("error", errorText(e), "pack");
    }
  },
  dismissPlan: () => set({ plan: null }),
  cancel: () => cancelJob(get().job, get().setJob),

  async startWatch() {
    const { project, path, dirty } = get();
    if (!inTauri || !project.sources.length) return;
    if (!project.exports.length) {
      uiLog("warn", "Watch packs and exports on every change: add an export target first", "watch");
      return;
    }
    try {
      // The watch reloads a saved project when its file changes; unsaved edits go with the request.
      const watchId = await ipc.watchStart({ action: { kind: "packProject", project, projectPath: dirty ? null : path } });
      set({ watch: { watchId, state: "running", lastRunAt: null, lastDurationMs: null, lastSummary: null, lastError: null, lastIssues: null, changedPaths: [] } });
    } catch (e) {
      uiLog("error", `Watch: ${errorText(e)}`, "watch");
    }
  },
  async stopWatch() {
    const w = get().watch;
    if (!w) return;
    set({ watch: null });
    await ipc.watchStop(w.watchId).catch((e) => uiLog("error", errorText(e), "watch"));
  },
}));

if (inTauri) {
  void on.watch((s) => {
    const w = useProjectStore.getState().watch;
    if (w && w.watchId === s.watchId) useProjectStore.setState({ watch: s });
  });
}

// Drops on Pack and Animate add sources (or open a dropped project).
registerDropHandler("pack", (paths) => void useProjectStore.getState().addPaths(paths));
registerDropHandler("animate", (paths) => void useProjectStore.getState().addPaths(paths));

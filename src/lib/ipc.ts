// Every invoke() and event name of the app lives here (and in dev/mockBackend.ts, which answers them in ?mock).
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppInfo,
  AppSettings,
  ApplySlice,
  BleedOptions,
  DropSources,
  FlipbookOptions,
  FlipbookReport,
  JobProgressEvent,
  LoadedProject,
  LogEntry,
  NormalOptions,
  NormalSettings,
  PackOptions,
  PackReport,
  PaletteReport,
  Project,
  RemapOptions,
  ScanResult,
  SliceOptions,
  SliceResult,
  SwapOptions,
  UpscaleOptions,
  WatchRequest,
  WatchStatus,
  WriteReport,
} from "../types/coral";

/** False in a plain browser (`npm run dev` without `?mock`): backend calls are skipped. */
export const inTauri = isTauri();

export const EVENTS = {
  log: "log://entry",
  progress: "coral://progress",
  watch: "watch://status",
} as const;

export const ipc = {
  appInfo: () => invoke<AppInfo>("app_info"),
  logHistory: () => invoke<LogEntry[]>("log_history"),
  uiLog: (level: string, message: string) => invoke<void>("ui_log", { level, message }),
  cancelJob: (jobId: number) => invoke<boolean>("cancel_job", { jobId }),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  revealInExplorer: (path: string) => invoke<void>("reveal_in_explorer", { path }),
  getSettings: () => invoke<AppSettings>("get_settings"),
  setSettings: (settings: AppSettings) => invoke<void>("set_settings", { settings }),

  // Projects and packing
  loadProject: (path: string) => invoke<LoadedProject>("load_project", { path }),
  saveProject: (path: string, project: Project) => invoke<string>("save_project", { path, project }),
  sourcesForPaths: (paths: string[]) => invoke<DropSources>("sources_for_paths", { paths }),
  scanSources: (project: Project) => invoke<ScanResult>("scan_sources", { project }),
  packPreview: (project: Project) => invoke<PackReport>("pack_preview", { project }),
  packAndExport: (project: Project, options: PackOptions) => invoke<PackReport>("pack_and_export", { project, options }),
  spriteThumbnail: (project: Project, name: string, maxSize: number) => invoke<ArrayBuffer>("sprite_thumbnail", { project, name, maxSize }),
  previewPng: (key: string) => invoke<ArrayBuffer>("preview_png", { key }),
  imagePng: (path: string) => invoke<ArrayBuffer>("image_png", { path }),

  // Slice
  sliceSheet: (path: string, options: SliceOptions) => invoke<SliceResult>("slice_sheet", { path, options }),
  applySlice: (request: ApplySlice) => invoke<WriteReport>("apply_slice", { request }),

  // Flipbook
  buildFlipbook: (options: FlipbookOptions) => invoke<FlipbookReport>("build_flipbook", { options }),

  // Pixel
  paletteExtract: (paths: string[], sort: "frequency" | "hue") => invoke<PaletteReport>("palette_extract", { paths, sort }),
  paletteSave: (path: string, colors: string[]) => invoke<void>("palette_save", { path, colors }),
  paletteLoad: (path: string) => invoke<string[]>("palette_load", { path }),
  paletteRemap: (options: RemapOptions) => invoke<WriteReport>("palette_remap", { options }),
  paletteSwap: (options: SwapOptions) => invoke<WriteReport>("palette_swap", { options }),
  upscaleNearest: (options: UpscaleOptions) => invoke<WriteReport>("upscale_nearest", { options }),
  alphaBleed: (options: BleedOptions) => invoke<WriteReport>("alpha_bleed", { options }),

  // Normal 2D
  normalMap: (options: NormalOptions) => invoke<WriteReport>("normal_map", { options }),
  normalPreview: (path: string, settings: NormalSettings) => invoke<ArrayBuffer>("normal_preview", { path, settings }),

  watchStart: (request: WatchRequest) => invoke<number>("watch_start", { request }),
  watchStop: (watchId: number) => invoke<boolean>("watch_stop", { watchId }),
  watchList: () => invoke<WatchStatus[]>("watch_list"),
};

export const on = {
  log: (cb: (e: LogEntry) => void): Promise<UnlistenFn> => listen<LogEntry>(EVENTS.log, (e) => cb(e.payload)),
  progress: (cb: (e: JobProgressEvent) => void): Promise<UnlistenFn> => listen<JobProgressEvent>(EVENTS.progress, (e) => cb(e.payload)),
  watch: (cb: (e: WatchStatus) => void): Promise<UnlistenFn> => listen<WatchStatus>(EVENTS.watch, (e) => cb(e.payload)),
};

/** PNG bytes from a raw-response command → an object URL for <img> / canvas. */
export function pngUrl(buf: ArrayBuffer | Uint8Array | number[]): string {
  const bytes = buf instanceof ArrayBuffer ? new Uint8Array(buf) : buf instanceof Uint8Array ? buf : new Uint8Array(buf);
  return URL.createObjectURL(new Blob([bytes as BlobPart], { type: "image/png" }));
}

/** Decode an object URL into an HTMLImageElement (for canvas drawing). */
export function loadImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error("Image failed to decode"));
    img.src = url;
  });
}

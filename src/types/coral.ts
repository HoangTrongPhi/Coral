// Mirrors of the Rust structs (serde camelCase). Keep every field in sync with src-tauri/src.
import type { AbyssContext } from "@abyss/ui";

export type { LogEntry, LogLevel, WatchStatus } from "@abyss/ui";

export type TabId = "pack" | "slice" | "animate" | "flipbook" | "pixel" | "normal";

export interface AppInfo {
  version: string;
  debug: boolean;
  abyss: AbyssContext;
}

export type Severity = "info" | "warn" | "error";

export interface Issue {
  severity: Severity;
  code: string;
  message: string;
  path?: string;
}

export interface JobProgressEvent {
  jobId: number;
  done: number;
  total: number;
  message: string;
}

// ---------------------------------------------------------------- project (core/project.rs)

export type LayerSelection = "visible" | "all" | string[];

export type Source =
  | { kind: "folder"; path: string; recursive: boolean; include: string[] }
  | { kind: "aseprite"; path: string; layers: LayerSelection }
  | { kind: "psd"; path: string; mode: "layers" | "composite" }
  | { kind: "gif"; path: string };

export interface SpriteOverride {
  /** Normalised in the untrimmed sprite, origin top-left. */
  pivot?: [number, number] | null;
  /** [left, top, right, bottom] px. */
  nineSlice?: [number, number, number, number] | null;
}

export type LoopMode = "loop" | "once" | "pingpong";

export interface Animation {
  name: string;
  frames: string[];
  fps: number;
  loop: LoopMode;
  durationsMs: number[] | null;
}

export interface Hitbox {
  /** `hitbox`, `hurtbox` or `custom:<name>`. */
  kind: string;
  /** [x, y, w, h] in the untrimmed sprite. */
  rect: [number, number, number, number];
  name?: string;
}

export type Heuristic = "bestShortSideFit" | "bestLongSideFit" | "bestAreaFit" | "bottomLeft" | "contactPoint";
export type ImageKind = "png" | "tga";

export interface NormalSettings {
  source: "luminance" | "alphaBevel";
  strength: number;
  blur: number;
  bevel: number;
  convention: "opengl" | "directx";
  invert: boolean;
}

export interface PackSettings {
  maxSize: number;
  powerOfTwo: boolean;
  square: boolean;
  allowRotation: boolean;
  padding: number;
  extrude: number;
  trim: "none" | "alpha";
  trimThreshold: number;
  alphaBleed: boolean;
  dedupe: boolean;
  premultiply: boolean;
  heuristic: Heuristic;
  multiPage: boolean;
  imageFormat: ImageKind;
  normalMaps: boolean;
  normal?: NormalSettings | null;
}

export type Target = "jsonHash" | "jsonArray" | "godot" | "unity" | "spine";

export interface ExportTarget {
  target: Target;
  out: string;
  godotProject?: string | null;
  atlasTextures?: boolean;
  pixelsPerUnit?: number | null;
  filter?: "point" | "bilinear" | null;
  paper2d?: boolean;
}

export interface SwapTable {
  variant: string;
  map: [string, string][];
}

export interface Project {
  format: "coral.project";
  version: number;
  name: string;
  sources: Source[];
  sprites: Record<string, SpriteOverride>;
  animations: Animation[];
  hitboxes: Record<string, Hitbox[]>;
  pack: PackSettings;
  exports: ExportTarget[];
  swaps?: SwapTable[];
  watch: { autoStart: boolean };
}

// ---------------------------------------------------------------- pipeline (core/pipeline.rs)

export interface TrimInfo {
  sourceW: number;
  sourceH: number;
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface SourceRef {
  path: string;
  frame?: number;
  layer?: string;
}

export interface SpriteInfo {
  name: string;
  trim: TrimInfo;
  hash: string;
  origin: SourceRef;
  aliasOf?: string;
  durationMs?: number;
  pivot: [number, number];
  nineSlice: [number, number, number, number] | null;
}

export interface ScanResult {
  sprites: SpriteInfo[];
  animations: Animation[];
  issues: Issue[];
}

export interface LoadedProject extends ScanResult {
  project: Project;
  path: string;
}

export interface FrameRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface PackedSprite {
  name: string;
  page: number;
  frame: FrameRect;
  rotated: boolean;
  trim: TrimInfo;
  pivot: [number, number];
  nineSlice: [number, number, number, number] | null;
  aliasOf?: string;
}

export interface PageInfo {
  index: number;
  width: number;
  height: number;
  fill: number;
  sprites: number;
}

export type WriteStatus = "planned" | "written" | "skipped" | "failed";

export interface FileWrite {
  path: string;
  kind: string;
  status: WriteStatus;
  bytes?: number;
  source?: string;
  note?: string;
}

export interface PackReport {
  name: string;
  pages: PageInfo[];
  sprites: PackedSprite[];
  animations: Animation[];
  aliases: number;
  files: FileWrite[];
  issues: Issue[];
  dryRun: boolean;
  elapsedMs: number;
  ok: boolean;
  cancelled: boolean;
}

export interface PackOptions {
  dryRun: boolean;
  targets?: Target[];
}

export interface DropSources {
  sources: Source[];
  ignored: string[];
}

export interface WriteReport {
  files: FileWrite[];
  issues: Issue[];
  dryRun: boolean;
  elapsedMs: number;
  ok: boolean;
  cancelled: boolean;
  notes: string[];
}

export type OutputMode = { kind: "folder"; path: string } | { kind: "inPlace" };

// ---------------------------------------------------------------- slice (core/slice.rs)

export type SliceOptions =
  | { kind: "grid"; cellW: number; cellH: number; offset: [number, number]; spacing: [number, number]; keepEmpty: boolean }
  | { kind: "auto"; merge: number; alphaThreshold: number; minPixels: number };

export interface SliceRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface SliceResult {
  width: number;
  height: number;
  rects: SliceRect[];
  suggestedNames: string[];
}

export interface NamedRect {
  name: string;
  rect: SliceRect;
  pivot: [number, number] | null;
  nineSlice: [number, number, number, number] | null;
}

export type SliceOutput = "sprites" | "jsonHash" | "unityMeta";

export interface ApplySlice {
  path: string;
  rects: NamedRect[];
  output: SliceOutput;
  outDir: string;
  dryRun: boolean;
}

// ---------------------------------------------------------------- flipbook / pixel / normal

export interface FlipbookOptions {
  paths: string[];
  from: number;
  to: number;
  step: number;
  cols: number;
  rows: number;
  cell: number;
  maxSize: number;
  fps: number;
  format: ImageKind;
  out: string;
  json: boolean;
  dryRun: boolean;
}

export interface FlipbookReport {
  frames: number;
  cols: number;
  rows: number;
  cell: [number, number];
  width: number;
  height: number;
  sources: string[];
  write: WriteReport;
}

export interface PaletteColor {
  hex: string;
  count: number;
}

export interface PaletteReport {
  colors: PaletteColor[];
  pixels: number;
  files: number;
  issues: Issue[];
}

export type PaletteSource = { kind: "file"; path: string } | { kind: "colors"; colors: string[] };

export interface RemapOptions {
  paths: string[];
  palette: PaletteSource;
  output: OutputMode;
  dryRun: boolean;
}

export interface SwapOptions {
  paths: string[];
  tables: SwapTable[];
  output: OutputMode;
  dryRun: boolean;
}

export interface UpscaleOptions {
  paths: string[];
  factor: number;
  output: OutputMode;
  dryRun: boolean;
}

export interface BleedOptions {
  paths: string[];
  output: OutputMode;
  dryRun: boolean;
}

export interface NormalOptions {
  paths: string[];
  settings: NormalSettings;
  output: OutputMode;
  dryRun: boolean;
}

// ---------------------------------------------------------------- watch

export interface WatchConfig {
  paths: string[];
  recursive: boolean;
  include: string[];
  exclude: string[];
  ignore: string[];
  debounceMs: number;
  settleMs: number;
}

export type WatchAction = { kind: "packProject"; project: Project; projectPath?: string | null };

export interface WatchRequest {
  action: WatchAction;
  config?: Partial<WatchConfig> | null;
}

export type AppSettings = Record<string, unknown>;

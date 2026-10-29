import { Checkbox, EmptyState, FieldLabel, GlassCard, JobProgress, Modal, PathText, PillBadge, Segmented, TextField, WatchToggle } from "@abyss/ui";
import { Boxes, FileJson, FolderOpen, FolderPlus, Layers, Package, Plus, Save, Trash2, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { pickOne } from "../../lib/dialogs";
import { baseName } from "../../lib/jobs";
import { useAppStore } from "../../stores/appStore";
import { useProjectStore } from "../../stores/projectStore";
import type { ExportTarget, Heuristic, PackedSprite, Source, Target } from "../../types/coral";
import { FileList, IssueList, NumField } from "../common/Fields";
import { drawPacked, ZoomCanvas, ZoomControl } from "../common/ZoomCanvas";

const TARGETS: { value: Target; label: string; hint: string }[] = [
  { value: "jsonHash", label: "JSON Hash", hint: "TexturePacker format: Phaser 3, PixiJS, Unreal Paper2D" },
  { value: "jsonArray", label: "JSON Array", hint: "Same content, frames as an array" },
  { value: "godot", label: "Godot", hint: "SpriteFrames .tres (Godot 4)" },
  { value: "unity", label: "Unity", hint: ".png.meta, Sprite Mode Multiple (Unity 6)" },
  { value: "spine", label: "Spine", hint: "Spine / libGDX .atlas" },
];
const targetLabel = (t: Target) => TARGETS.find((x) => x.value === t)?.label ?? t;

const SOURCE_ICON = { folder: FolderOpen, aseprite: Layers, psd: Layers, gif: Layers } as const;

/** One sprite cut from the atlas preview, untrimmed box, fit in `size` px. */
export function SpriteThumb({ sprite, size = 32 }: { sprite: PackedSprite; size?: number }) {
  const pages = useProjectStore((s) => s.pageImages);
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const c = ref.current;
    const ctx = c?.getContext("2d");
    if (!c || !ctx) return;
    ctx.clearRect(0, 0, c.width, c.height);
    ctx.imageSmoothingEnabled = false;
    const s = Math.min(size / sprite.trim.sourceW, size / sprite.trim.sourceH, 4);
    ctx.setTransform(s, 0, 0, s, (size - sprite.trim.sourceW * s) / 2, (size - sprite.trim.sourceH * s) / 2);
    drawPacked(ctx, pages[sprite.page], sprite, 0, 0);
  }, [sprite, pages, size]);
  return <canvas ref={ref} width={size} height={size} className="checker shrink-0 rounded" style={{ width: size, height: size, imageRendering: "pixelated" }} />;
}

function sourceLabel(s: Source) {
  if (s.kind === "folder") return `${s.recursive ? "folder + subfolders" : "folder"} · ${s.include.join(", ")}`;
  if (s.kind === "aseprite") return `Aseprite · ${Array.isArray(s.layers) ? `layers ${s.layers.join(", ")}` : `${s.layers} layers`}`;
  if (s.kind === "psd") return `PSD · ${s.mode}`;
  return "GIF frames";
}

function SourcesPanel() {
  const { project, addPaths, removeSource, updateSource } = useProjectStore();
  const [edit, setEdit] = useState<number | null>(null);
  return (
    <GlassCard
      title={`Sources (${project.sources.length})`}
      actions={
        <button type="button" className="icon-btn size-6" title="Add a folder" aria-label="Add a folder" onClick={async () => { const p = await pickOne("folder"); if (p) void addPaths([p]); }}>
          <FolderPlus className="size-3.5" />
        </button>
      }
    >
      <ul className="divide-y divide-edge">
        {project.sources.map((s, i) => {
          const Icon = SOURCE_ICON[s.kind];
          return (
            <li key={`${s.path}-${i}`} className="px-3 py-1.5">
              <div className="flex items-center gap-2">
                <Icon className="size-3.5 shrink-0 text-ink-faint" />
                <button type="button" className="min-w-0 flex-1 text-left" onClick={() => setEdit(edit === i ? null : i)} title="Edit">
                  <PathText path={s.path} className="block text-ink" />
                  <span className="block text-[10.5px] text-ink-faint">{sourceLabel(s)}</span>
                </button>
                <button type="button" className="icon-btn size-6" aria-label="Remove source" onClick={() => removeSource(i)}>
                  <X className="size-3.5" />
                </button>
              </div>
              {edit === i && (
                <div className="mt-1.5 flex flex-col gap-1.5 rounded-md bg-canvas/50 p-2">
                  {s.kind === "folder" && (
                    <>
                      <Checkbox checked={s.recursive} onChange={(recursive) => updateSource(i, { ...s, recursive })} label="Include subfolders" />
                      <TextField mono value={s.include.join(", ")} onChange={(e) => updateSource(i, { ...s, include: e.target.value.split(",").map((v) => v.trim()).filter(Boolean) })} placeholder="*.png, *.tga" />
                    </>
                  )}
                  {s.kind === "aseprite" && (
                    <>
                      <Segmented size="sm" value={Array.isArray(s.layers) ? "names" : s.layers} options={[{ value: "visible", label: "Visible merged" }, { value: "all", label: "All merged" }, { value: "names", label: "Per layer" }]} onChange={(v) => updateSource(i, { ...s, layers: v === "names" ? [] : (v as "visible" | "all") })} />
                      {Array.isArray(s.layers) && <TextField mono value={s.layers.join(", ")} placeholder="Body, FX" onChange={(e) => updateSource(i, { ...s, layers: e.target.value.split(",").map((v) => v.trim()).filter(Boolean) })} />}
                    </>
                  )}
                  {s.kind === "psd" && <Segmented size="sm" value={s.mode} options={[{ value: "layers", label: "One sprite per layer" }, { value: "composite", label: "Flattened" }]} onChange={(mode) => updateSource(i, { ...s, mode })} />}
                </div>
              )}
            </li>
          );
        })}
      </ul>
      {project.sources.length === 0 && <div className="px-4 py-3 text-[12px] text-ink-faint">Drop folders, .aseprite, .psd or .gif files.</div>}
    </GlassCard>
  );
}

function SpriteList() {
  const report = useProjectStore((s) => s.report);
  const selected = useProjectStore((s) => s.selected);
  const select = useProjectStore((s) => s.select);
  const [filter, setFilter] = useState("");
  const sprites = useMemo(() => (report?.sprites ?? []).filter((s) => !filter || s.name.toLowerCase().includes(filter.toLowerCase())), [report, filter]);
  const listRef = useRef<HTMLUListElement>(null);
  useEffect(() => {
    listRef.current?.querySelector(`[data-name="${CSS.escape(selected ?? "")}"]`)?.scrollIntoView({ block: "nearest" });
  }, [selected]);
  return (
    <GlassCard title={`Sprites (${report?.sprites.length ?? 0})`} className="flex min-h-0 flex-1 flex-col">
      <div className="border-b border-edge p-2">
        <TextField className="w-full" placeholder="Filter…" value={filter} onChange={(e) => setFilter(e.target.value)} />
      </div>
      <ul ref={listRef} className="min-h-0 flex-1 overflow-y-auto py-1">
        {/* Up to a few thousand rows: plain list; thumbnails are cut from the atlas preview. */}
        {sprites.slice(0, 3000).map((s) => (
          <li key={s.name} data-name={s.name}>
            <button type="button" onClick={() => select(s.name)} className={`flex w-full items-center gap-2 px-2 py-1 text-left text-[12px] ${selected === s.name ? "bg-accent/14 text-ink" : "text-ink-muted hover:bg-tint/5"}`}>
              <SpriteThumb sprite={s} size={28} />
              <span className="min-w-0 flex-1 truncate font-mono text-[11.5px]">{s.name}</span>
              {s.aliasOf && <PillBadge>alias</PillBadge>}
            </button>
          </li>
        ))}
      </ul>
    </GlassCard>
  );
}

function AtlasView() {
  const { report, pageImages, normalPages, selected, select, job } = useProjectStore();
  const [page, setPage] = useState(0);
  const [zoom, setZoom] = useState(1);
  const [hover, setHover] = useState<string | null>(null);
  const [showNormal, setShowNormal] = useState(false);
  const [normalImg, setNormalImg] = useState<HTMLImageElement | null>(null);
  const p = report?.pages[Math.min(page, (report?.pages.length ?? 1) - 1)];
  const sprites = useMemo(() => report?.sprites.filter((s) => s.page === (p?.index ?? 0)) ?? [], [report, p]);
  const sel = report?.sprites.find((s) => s.name === selected);

  useEffect(() => {
    if (sel && sel.page !== page) setPage(sel.page);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sel?.name]);
  useEffect(() => {
    const url = normalPages[p?.index ?? 0];
    if (!showNormal || !url) return setNormalImg(null);
    const img = new Image();
    img.onload = () => setNormalImg(img);
    img.src = url;
  }, [showNormal, normalPages, p]);

  const at = (x: number, y: number) => sprites.find((s) => {
    const [w, h] = s.rotated ? [s.frame.h, s.frame.w] : [s.frame.w, s.frame.h];
    return !s.aliasOf && x >= s.frame.x && y >= s.frame.y && x < s.frame.x + w && y < s.frame.y + h;
  });

  if (!report || !p) {
    return (
      <div className="grid h-full place-items-center">
        {job ? <div className="text-[12px] text-ink-faint">Packing…</div> : <EmptyState icon={<Boxes className="size-5" />} title="No atlas yet">Drop a folder of PNGs, or .aseprite / .psd / .gif files, onto the window. The atlas appears here as you go.</EmptyState>}
      </div>
    );
  }
  const img = showNormal && normalImg ? normalImg : pageImages[p.index];
  const rectOf = (s: PackedSprite) => (s.rotated ? [s.frame.x, s.frame.y, s.frame.h, s.frame.w] : [s.frame.x, s.frame.y, s.frame.w, s.frame.h]);
  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex flex-wrap items-center gap-2 border-b border-edge px-3 py-2">
        {report.pages.length > 1 && (
          <Segmented size="sm" value={String(p.index)} options={report.pages.map((pg) => ({ value: String(pg.index), label: `Page ${pg.index + 1}` }))} onChange={(v) => setPage(Number(v))} />
        )}
        <span className="text-[12px] text-ink-muted tabular-nums">
          {p.width}×{p.height} · {(p.fill * 100).toFixed(0)}% filled
        </span>
        {normalPages.length > 0 && <Checkbox checked={showNormal} onChange={setShowNormal} label="Normal map" />}
        <div className="flex-1" />
        <span className="max-w-64 truncate font-mono text-[11.5px] text-ink-faint">{hover ?? ""}</span>
        <ZoomControl zoom={zoom} setZoom={setZoom} />
      </div>
      <div className="min-h-0 flex-1 overflow-auto p-4">
        <ZoomCanvas
          width={p.width}
          height={p.height}
          zoom={zoom}
          deps={[img, sprites, selected, hover]}
          cursor="crosshair"
          draw={(ctx) => {
            if (img) ctx.drawImage(img, 0, 0);
            ctx.lineWidth = 1 / zoom;
            for (const s of sprites) {
              if (s.aliasOf) continue;
              const [x, y, w, h] = rectOf(s);
              const isSel = s.name === selected || (sel?.aliasOf === s.name);
              if (isSel || s.name === hover) {
                ctx.strokeStyle = isSel ? "#a855f7" : "rgba(255,255,255,0.7)";
                ctx.lineWidth = (isSel ? 2 : 1) / zoom;
                ctx.strokeRect(x, y, w, h);
              } else if (zoom >= 1) {
                ctx.strokeStyle = "rgba(168,85,247,0.25)";
                ctx.lineWidth = 1 / zoom;
                ctx.strokeRect(x + 0.5 / zoom, y + 0.5 / zoom, w - 1 / zoom, h - 1 / zoom);
              }
            }
          }}
          onMove={(pt) => setHover(at(pt.x, pt.y)?.name ?? null)}
          onLeave={() => setHover(null)}
          onDown={(pt) => select(at(pt.x, pt.y)?.name ?? null)}
        />
      </div>
      {sel && <SpriteInfoBar sprite={sel} />}
    </div>
  );
}

function SpriteInfoBar({ sprite }: { sprite: PackedSprite }) {
  const t = sprite.trim;
  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-1 border-t border-edge bg-panel/60 px-3 py-2 text-[12px]">
      <SpriteThumb sprite={sprite} size={40} />
      <span className="font-mono text-ink select-text">{sprite.name}</span>
      <span className="text-ink-muted tabular-nums">frame {sprite.frame.x},{sprite.frame.y} {sprite.frame.w}×{sprite.frame.h}{sprite.rotated ? " (rotated)" : ""}</span>
      <span className="text-ink-muted tabular-nums">source {t.sourceW}×{t.sourceH}</span>
      <span className="text-ink-muted tabular-nums">{t.w !== t.sourceW || t.h !== t.sourceH ? `trimmed to ${t.w}×${t.h} at ${t.x},${t.y}` : "not trimmed"}</span>
      <span className="text-ink-muted tabular-nums">pivot {sprite.pivot[0].toFixed(2)}, {sprite.pivot[1].toFixed(2)}</span>
      {sprite.aliasOf && <PillBadge tone="amber">same pixels as {sprite.aliasOf}</PillBadge>}
    </div>
  );
}

function ExportRow({ e, i }: { e: ExportTarget; i: number }) {
  const { updateExport, removeExport } = useProjectStore();
  return (
    <li className="flex flex-col gap-1.5 px-3 py-2">
      <div className="flex items-center gap-2">
        <PillBadge tone="accent">{targetLabel(e.target)}</PillBadge>
        <button type="button" className="min-w-0 flex-1 text-left" title="Choose the folder" onClick={async () => { const p = await pickOne("folder"); if (p) updateExport(i, { out: p }); }}>
          {e.out ? <PathText path={e.out} className="block text-ink" /> : <span className="text-[12px] text-rose">Choose a folder…</span>}
        </button>
        <button type="button" className="icon-btn size-6" aria-label="Remove export" onClick={() => removeExport(i)}>
          <Trash2 className="size-3.5" />
        </button>
      </div>
      {e.target === "godot" && (
        <div className="flex flex-col gap-1 pl-1">
          <button type="button" className="text-left text-[11.5px] text-ink-faint hover:text-ink" onClick={async () => { const p = await pickOne("folder"); if (p) updateExport(i, { godotProject: p }); }}>
            Godot project: {e.godotProject ? <PathText path={e.godotProject} /> : "found from the output folder (click to set)"}
          </button>
          <Checkbox checked={!!e.atlasTextures} onChange={(atlasTextures) => updateExport(i, { atlasTextures })} label="Also one AtlasTexture .tres per sprite" />
        </div>
      )}
      {e.target === "unity" && (
        <div className="flex items-end gap-3 pl-1">
          <NumField label="Pixels per unit" value={e.pixelsPerUnit ?? 100} min={1} onChange={(v) => updateExport(i, { pixelsPerUnit: v })} />
          <div>
            <FieldLabel>Filter</FieldLabel>
            <Segmented size="sm" value={e.filter ?? "point"} options={[{ value: "point", label: "Point" }, { value: "bilinear", label: "Bilinear" }]} onChange={(filter) => updateExport(i, { filter })} />
          </div>
        </div>
      )}
      {e.target === "spine" && (
        <div className="pl-1">
          <Segmented size="sm" value={e.filter ?? "bilinear"} options={[{ value: "bilinear", label: "Linear" }, { value: "point", label: "Nearest" }]} onChange={(filter) => updateExport(i, { filter })} />
        </div>
      )}
      {e.target === "jsonHash" && <Checkbox checked={!!e.paper2d} onChange={(paper2d) => updateExport(i, { paper2d })} label="Unreal Paper2D (.paper2dsprites)" />}
    </li>
  );
}

const HEURISTICS: { value: Heuristic; label: string }[] = [
  { value: "bestShortSideFit", label: "Best short side" },
  { value: "bestLongSideFit", label: "Best long side" },
  { value: "bestAreaFit", label: "Best area" },
  { value: "bottomLeft", label: "Bottom left" },
  { value: "contactPoint", label: "Contact point" },
];

function SettingsPanel() {
  const { project, setPack, addExport } = useProjectStore();
  const s = project.pack;
  const [adding, setAdding] = useState(false);
  return (
    <div className="flex flex-col gap-3">
      <GlassCard title="Pack">
        <div className="flex flex-col gap-3 p-3">
          <div className="flex flex-wrap gap-3">
            <div>
              <FieldLabel>Max size</FieldLabel>
              <Segmented size="sm" value={String(s.maxSize)} options={[512, 1024, 2048, 4096, 8192].map((v) => ({ value: String(v), label: String(v) }))} onChange={(v) => setPack({ maxSize: Number(v) })} />
            </div>
            <div>
              <FieldLabel>Image</FieldLabel>
              <Segmented size="sm" value={s.imageFormat} options={[{ value: "png", label: "PNG" }, { value: "tga", label: "TGA" }]} onChange={(imageFormat) => setPack({ imageFormat })} />
            </div>
          </div>
          <div className="flex flex-wrap gap-3">
            <NumField label="Padding" value={s.padding} min={0} max={64} suffix="px" onChange={(padding) => setPack({ padding })} />
            <NumField label="Extrude" value={s.extrude} min={0} max={16} suffix="px" onChange={(extrude) => setPack({ extrude })} />
            <NumField label="Trim alpha ≤" value={s.trimThreshold} min={0} max={254} onChange={(trimThreshold) => setPack({ trimThreshold })} hint="Pixels with alpha at or below this count as empty" />
          </div>
          <div className="grid grid-cols-2 gap-x-3 gap-y-1.5">
            <Checkbox checked={s.trim === "alpha"} onChange={(v) => setPack({ trim: v ? "alpha" : "none" })} label="Trim" />
            <Checkbox checked={s.dedupe} onChange={(dedupe) => setPack({ dedupe })} label="Merge duplicates" />
            <Checkbox checked={s.powerOfTwo} onChange={(powerOfTwo) => setPack({ powerOfTwo })} label="Power of two" />
            <Checkbox checked={s.square} onChange={(square) => setPack({ square })} label="Square" />
            <Checkbox checked={s.multiPage} onChange={(multiPage) => setPack({ multiPage })} label="Multiple pages" />
            <Checkbox checked={s.allowRotation} onChange={(allowRotation) => setPack({ allowRotation })} label="Allow rotation" hint="Not for Godot or Unity" />
            <Checkbox checked={s.alphaBleed} onChange={(alphaBleed) => setPack({ alphaBleed })} label="Alpha bleed" />
            <Checkbox checked={s.premultiply} onChange={(premultiply) => setPack({ premultiply })} label="Premultiply" />
            <Checkbox checked={s.normalMaps} onChange={(normalMaps) => setPack({ normalMaps })} label="Normal-map atlas" hint="A second atlas (<name>_n) with the same rects; settings from the Normal 2D module" />
          </div>
          <div>
            <FieldLabel>Placement</FieldLabel>
            <select className="h-7 w-full rounded-md border border-edge bg-canvas/70 px-2 text-[12px] text-ink" value={s.heuristic} onChange={(e) => setPack({ heuristic: e.target.value as Heuristic })}>
              {HEURISTICS.map((h) => (
                <option key={h.value} value={h.value}>{h.label}</option>
              ))}
            </select>
          </div>
        </div>
      </GlassCard>
      <GlassCard
        title={`Exports (${project.exports.length})`}
        actions={
          <button type="button" className="icon-btn size-6" aria-label="Add export" onClick={() => setAdding(!adding)}>
            <Plus className="size-3.5" />
          </button>
        }
      >
        {adding && (
          <div className="flex flex-wrap gap-1.5 border-b border-edge p-2">
            {TARGETS.map((t) => (
              <button key={t.value} type="button" className="btn h-6 text-[11.5px]" title={t.hint} onClick={() => { setAdding(false); void addExport(t.value); }}>
                {t.label}
              </button>
            ))}
          </div>
        )}
        <ul className="divide-y divide-edge">
          {project.exports.map((e, i) => <ExportRow key={i} e={e} i={i} />)}
        </ul>
        {project.exports.length === 0 && !adding && <div className="px-4 py-3 text-[12px] text-ink-faint">Add where the atlas goes: + above.</div>}
      </GlassCard>
    </div>
  );
}

function PlanDialog() {
  const { plan, dismissPlan, confirmExport } = useProjectStore();
  if (!plan) return null;
  const blocked = !plan.ok;
  return (
    <Modal
      open
      title={blocked ? "Export blocked" : `Write ${plan.files.length} file(s)?`}
      onClose={dismissPlan}
      className="max-w-2xl"
      footer={
        <>
          <button type="button" className="btn" onClick={dismissPlan}>Cancel</button>
          <button type="button" className="btn btn-primary" disabled={blocked || plan.files.length === 0} onClick={() => void confirmExport()}>
            <Package className="size-3.5" /> Write files
          </button>
        </>
      }
    >
      <div className="flex flex-col gap-3">
        <p className="text-[12.5px] text-ink-muted">{blocked ? "Fix the errors below first; nothing was written." : "Existing files at these paths are replaced (written to a .tmp file, then renamed). An existing Unity .meta keeps its guid."}</p>
        <div className="rounded-lg border border-edge"><FileList files={plan.files} /></div>
        <IssueList issues={plan.issues} />
      </div>
    </Modal>
  );
}

export function PackTab() {
  const { project, path, dirty, report, job, error, watch, exported, update, open, save, planExport, cancel, startWatch, stopWatch } = useProjectStore();
  const openLog = () => useAppStore.getState().toggleLogDrawer(true);
  const issues = report?.issues ?? [];
  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex flex-wrap items-center gap-2 border-b border-edge px-4 py-2">
        <TextField className="w-40 font-medium" value={project.name} onChange={(e) => update((p) => ({ ...p, name: e.target.value.replace(/[^\w.-]/g, "_") || "atlas" }), { repack: false })} title="Atlas name (file names)" />
        <span className="min-w-0 truncate text-[12px] text-ink-faint" title={path ?? undefined}>
          {path ? baseName(path) : "unsaved project"}{dirty ? " •" : ""}
        </span>
        <button type="button" className="btn" onClick={() => void open()} title="Ctrl+O"><FileJson className="size-3.5" /> Open…</button>
        <button type="button" className="btn" onClick={() => void save()} title="Ctrl+S"><Save className="size-3.5" /> Save</button>
        <div className="flex-1" />
        <WatchToggle status={watch} onStart={() => void startWatch()} onStop={() => void stopWatch()} onErrorClick={openLog} disabled={!project.sources.length || !project.exports.length} />
        <button type="button" className="btn btn-primary" disabled={!report || !!job} onClick={() => void planExport()} title="Dry run first, then confirm">
          <Package className="size-3.5" /> Pack & Export
        </button>
      </div>
      {job && (
        <div className="px-4 pt-2">
          <JobProgress done={job.done} total={job.total} message={`${job.label}: ${job.message}`} startedAt={job.startedAt} onCancel={() => void cancel()} cancelling={job.cancelling} />
        </div>
      )}
      {error && <div className="mx-4 mt-2 rounded-lg border border-rose/30 bg-rose/10 px-3 py-2 text-[12px] text-rose">{error}</div>}
      <div className="grid min-h-0 flex-1 grid-cols-[260px_1fr_300px] gap-3 p-3">
        <div className="flex min-h-0 flex-col gap-3">
          <SourcesPanel />
          <SpriteList />
        </div>
        <div className="min-h-0 overflow-hidden rounded-xl border border-edge bg-panel/50">
          <AtlasView />
        </div>
        <div className="flex min-h-0 flex-col gap-3 overflow-y-auto pr-1">
          <SettingsPanel />
          {issues.length > 0 && <IssueList issues={issues} />}
          {exported && (
            <GlassCard title="Last export" actions={<PillBadge tone={exported.ok ? "emerald" : "rose"}>{exported.ok ? `${exported.files.length} file(s)` : "failed"} · {exported.elapsedMs} ms</PillBadge>}>
              <FileList files={exported.files} />
            </GlassCard>
          )}
        </div>
      </div>
      <footer className="flex flex-wrap items-center gap-4 border-t border-edge px-4 py-1.5 text-[11.5px] text-ink-faint tabular-nums">
        <span>{report?.sprites.length ?? 0} sprite(s)</span>
        <span>{report?.pages.length ?? 0} page(s)</span>
        <span>{report?.aliases ?? 0} alias(es)</span>
        <span>{report?.pages.length ? `${((report.pages.reduce((a, p) => a + p.fill * p.width * p.height, 0) / report.pages.reduce((a, p) => a + p.width * p.height, 0)) * 100).toFixed(0)}% filled` : ""}</span>
        <span>{report?.animations.length ?? 0} animation(s)</span>
        <div className="flex-1" />
        {report && <span>preview packed in {report.elapsedMs} ms</span>}
      </footer>
      <PlanDialog />
    </div>
  );
}

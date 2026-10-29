import { Checkbox, EmptyState, FieldLabel, GlassCard, JobProgress, Modal, Segmented, TextField } from "@abyss/ui";
import { FileImage, Plus, ScanSearch, Scissors, Trash2 } from "lucide-react";
import { useRef, useState } from "react";
import { pickOne } from "../../lib/dialogs";
import { useSliceStore } from "../../stores/toolStores";
import type { NamedRect, SliceRect } from "../../types/coral";
import { FileList, IssueList, NumField, PathField, WriteResult } from "../common/Fields";
import { ZoomCanvas, ZoomControl } from "../common/ZoomCanvas";

type Drag = { mode: "move" | "resize" | "new"; i: number; ox: number; oy: number; start: SliceRect };

const PIVOTS: [number, number][] = [0, 0.5, 1].flatMap((y) => [0, 0.5, 1].map((x) => [x, y] as [number, number]));

export function SliceTab() {
  const s = useSliceStore();
  const [zoom, setZoom] = useState(2);
  const [drawing, setDrawing] = useState(false);
  const drag = useRef<Drag | null>(null);
  const [, force] = useState(0);
  const sel = s.selected != null ? s.rects[s.selected] : null;
  const opts = s.options;

  if (!s.path || !s.image) {
    return (
      <div className="grid h-full place-items-center">
        <div className="flex flex-col items-center gap-3">
          <EmptyState icon={<Scissors className="size-5" />} title="Drop a sprite sheet">A PNG, TGA or GIF. Cut it by grid or find the sprites automatically, then write separate PNGs or metadata for the sheet.</EmptyState>
          <button type="button" className="btn" onClick={async () => { const p = await pickOne("file", [{ name: "Images", extensions: ["png", "tga", "gif"] }]); if (p) void s.load(p); }}>
            <FileImage className="size-3.5" /> Choose a sheet…
          </button>
        </div>
      </div>
    );
  }

  const img = s.image;
  const hit = (x: number, y: number) => {
    for (let i = s.rects.length - 1; i >= 0; i--) {
      const r = s.rects[i].rect;
      if (x >= r.x && y >= r.y && x < r.x + r.w && y < r.y + r.h) return i;
    }
    return -1;
  };
  const clampRect = (r: SliceRect): SliceRect => {
    const w = Math.max(1, Math.min(r.w, img.width));
    const h = Math.max(1, Math.min(r.h, img.height));
    return { x: Math.max(0, Math.min(img.width - w, r.x)), y: Math.max(0, Math.min(img.height - h, r.y)), w, h };
  };

  return (
    <div className="grid h-full min-h-0 grid-cols-[1fr_300px] gap-3 p-3">
      <div className="flex min-h-0 flex-col overflow-hidden rounded-xl border border-edge bg-panel/50">
        <div className="flex flex-wrap items-end gap-3 border-b border-edge px-3 py-2">
          <div>
            <FieldLabel>Mode</FieldLabel>
            <Segmented
              size="sm"
              value={opts.kind}
              options={[{ value: "grid", label: "Grid" }, { value: "auto", label: "Automatic" }]}
              onChange={(k) => s.setOptions(k === "grid" ? { kind: "grid", cellW: 32, cellH: 32, offset: [0, 0], spacing: [0, 0], keepEmpty: false } : { kind: "auto", merge: 2, alphaThreshold: 0, minPixels: 4 })}
            />
          </div>
          {opts.kind === "grid" ? (
            <>
              <NumField label="Cell W" value={opts.cellW} min={1} onChange={(cellW) => s.setOptions({ ...opts, cellW })} />
              <NumField label="Cell H" value={opts.cellH} min={1} onChange={(cellH) => s.setOptions({ ...opts, cellH })} />
              <NumField label="Offset X" value={opts.offset[0]} min={0} onChange={(v) => s.setOptions({ ...opts, offset: [v, opts.offset[1]] })} />
              <NumField label="Offset Y" value={opts.offset[1]} min={0} onChange={(v) => s.setOptions({ ...opts, offset: [opts.offset[0], v] })} />
              <NumField label="Gap X" value={opts.spacing[0]} min={0} onChange={(v) => s.setOptions({ ...opts, spacing: [v, opts.spacing[1]] })} />
              <NumField label="Gap Y" value={opts.spacing[1]} min={0} onChange={(v) => s.setOptions({ ...opts, spacing: [opts.spacing[0], v] })} />
              <Checkbox checked={opts.keepEmpty} onChange={(keepEmpty) => s.setOptions({ ...opts, keepEmpty })} label="Keep empty cells" />
            </>
          ) : (
            <>
              <NumField label="Merge within" value={opts.merge} min={0} max={64} suffix="px" onChange={(merge) => s.setOptions({ ...opts, merge })} />
              <NumField label="Alpha >" value={opts.alphaThreshold} min={0} max={254} onChange={(alphaThreshold) => s.setOptions({ ...opts, alphaThreshold })} />
              <NumField label="Ignore under" value={opts.minPixels} min={1} suffix="px" onChange={(minPixels) => s.setOptions({ ...opts, minPixels })} />
            </>
          )}
          <button type="button" className="btn btn-primary" onClick={() => void s.detect()}>
            <ScanSearch className="size-3.5" /> Detect
          </button>
          <div className="flex-1" />
          <ZoomControl
            zoom={zoom}
            setZoom={setZoom}
            extra={
              <button type="button" className={`btn ml-2 ${drawing ? "border-accent text-accent" : ""}`} onClick={() => setDrawing(!drawing)} title="Drag on the sheet to add a rect">
                <Plus className="size-3.5" /> Rect
              </button>
            }
          />
        </div>
        <div className="min-h-0 flex-1 overflow-auto p-4">
          <ZoomCanvas
            width={img.width}
            height={img.height}
            zoom={zoom}
            cursor={drawing ? "crosshair" : "default"}
            deps={[img, s.rects, s.selected]}
            draw={(ctx) => {
              ctx.drawImage(img, 0, 0);
              s.rects.forEach((r, i) => {
                const { x, y, w, h } = r.rect;
                const on = i === s.selected;
                ctx.strokeStyle = on ? "#a855f7" : "rgba(168,85,247,0.6)";
                ctx.lineWidth = (on ? 2 : 1) / zoom;
                ctx.strokeRect(x, y, w, h);
                if (on) {
                  ctx.fillStyle = "#a855f7";
                  ctx.fillRect(x + w - 4 / zoom, y + h - 4 / zoom, 6 / zoom, 6 / zoom);
                  if (r.nineSlice) {
                    const [l, t, rr, b] = r.nineSlice;
                    ctx.strokeStyle = "rgba(16,185,129,0.8)";
                    ctx.setLineDash([2 / zoom, 2 / zoom]);
                    ctx.beginPath();
                    ctx.moveTo(x + l, y); ctx.lineTo(x + l, y + h);
                    ctx.moveTo(x + w - rr, y); ctx.lineTo(x + w - rr, y + h);
                    ctx.moveTo(x, y + t); ctx.lineTo(x + w, y + t);
                    ctx.moveTo(x, y + h - b); ctx.lineTo(x + w, y + h - b);
                    ctx.stroke();
                    ctx.setLineDash([]);
                  }
                }
                if (zoom >= 2) {
                  ctx.fillStyle = "rgba(255,255,255,0.75)";
                  ctx.font = `${10 / zoom}px sans-serif`;
                  ctx.fillText(String(i), x + 1.5 / zoom, y + 10 / zoom);
                }
              });
            }}
            onDown={(p) => {
              const x = Math.floor(p.x);
              const y = Math.floor(p.y);
              if (drawing) {
                const r = { x, y, w: 1, h: 1 };
                const n: NamedRect = { name: `${s.path!.split(/[\\/]/).pop()!.replace(/\.[^.]+$/, "")}_${s.rects.length}`, rect: r, pivot: null, nineSlice: null };
                s.setRects([...s.rects, n]);
                s.select(s.rects.length);
                drag.current = { mode: "new", i: s.rects.length, ox: x, oy: y, start: r };
                return;
              }
              const cur = s.selected != null ? s.rects[s.selected].rect : null;
              if (cur && Math.abs(p.x - (cur.x + cur.w)) * zoom < 8 && Math.abs(p.y - (cur.y + cur.h)) * zoom < 8) {
                drag.current = { mode: "resize", i: s.selected!, ox: x, oy: y, start: cur };
                return;
              }
              const i = hit(p.x, p.y);
              s.select(i >= 0 ? i : null);
              if (i >= 0) drag.current = { mode: "move", i, ox: x, oy: y, start: s.rects[i].rect };
            }}
            onMove={(p) => {
              const d = drag.current;
              if (!d || !p.buttons) return;
              const dx = Math.floor(p.x) - d.ox;
              const dy = Math.floor(p.y) - d.oy;
              const r =
                d.mode === "move"
                  ? { ...d.start, x: d.start.x + dx, y: d.start.y + dy }
                  : d.mode === "resize"
                    ? { ...d.start, w: d.start.w + dx, h: d.start.h + dy }
                    : { x: Math.min(d.ox, d.ox + dx), y: Math.min(d.oy, d.oy + dy), w: Math.abs(dx) + 1, h: Math.abs(dy) + 1 };
              s.updateRect(d.i, { rect: clampRect(r) });
              force((n) => n + 1);
            }}
            onUp={() => {
              if (drag.current?.mode === "new") setDrawing(false);
              drag.current = null;
            }}
          />
        </div>
        <div className="border-t border-edge px-3 py-1.5 text-[11.5px] text-ink-faint">
          {s.image.width}×{s.image.height} · {s.rects.length} rect(s) · drag a rect to move it, its corner to resize it
        </div>
      </div>

      <div className="flex min-h-0 flex-col gap-3 overflow-y-auto pr-1">
        <GlassCard title="Names">
          <div className="flex flex-col gap-2 p-3">
            <TextField mono value={s.pattern} onChange={(e) => s.setOutput({ pattern: e.target.value })} />
            <div className="flex items-center gap-2">
              <span className="flex-1 text-[11px] text-ink-faint">{"{base}"} sheet name · {"{i}"} index · {"{i:03}"} padded · {"{i1}"} from 1</span>
              <button type="button" className="btn" onClick={s.rename}>Rename all</button>
            </div>
          </div>
        </GlassCard>
        {sel && s.selected != null && (
          <GlassCard
            title={`Rect ${s.selected}`}
            actions={
              <button type="button" className="icon-btn size-6" aria-label="Delete rect" onClick={() => { s.setRects(s.rects.filter((_, j) => j !== s.selected)); s.select(null); }}>
                <Trash2 className="size-3.5" />
              </button>
            }
          >
            <div className="flex flex-col gap-3 p-3">
              <TextField mono value={sel.name} onChange={(e) => s.updateRect(s.selected!, { name: e.target.value })} />
              <div className="flex flex-wrap gap-2">
                {(["x", "y", "w", "h"] as const).map((k) => (
                  <NumField key={k} label={k.toUpperCase()} value={sel.rect[k]} min={k === "w" || k === "h" ? 1 : 0} onChange={(v) => s.updateRect(s.selected!, { rect: clampRect({ ...sel.rect, [k]: v }) })} />
                ))}
              </div>
              <div>
                <FieldLabel>Pivot</FieldLabel>
                <div className="flex items-center gap-3">
                  <div className="grid w-24 grid-cols-3 gap-1">
                    {PIVOTS.map(([x, y]) => {
                      const on = sel.pivot?.[0] === x && sel.pivot?.[1] === y;
                      return (
                        <button key={`${x}${y}`} type="button" title={`${x}, ${y}`} className={`grid size-7 place-items-center rounded border ${on ? "border-accent bg-accent/15" : "border-edge hover:border-edge-strong"}`} onClick={() => s.updateRect(s.selected!, { pivot: [x, y] })}>
                          <span className={`size-1.5 rounded-full ${on ? "bg-accent" : "bg-ink-faint"}`} />
                        </button>
                      );
                    })}
                  </div>
                  <div className="flex flex-col gap-1">
                    <NumField label="X" value={sel.pivot?.[0] ?? 0.5} step={0.05} min={0} max={1} onChange={(v) => s.updateRect(s.selected!, { pivot: [v, sel.pivot?.[1] ?? 0.5] })} />
                    <NumField label="Y" value={sel.pivot?.[1] ?? 0.5} step={0.05} min={0} max={1} onChange={(v) => s.updateRect(s.selected!, { pivot: [sel.pivot?.[0] ?? 0.5, v] })} />
                  </div>
                </div>
                <button type="button" className="mt-1.5 text-[11px] text-accent hover:underline" onClick={() => s.setRects(s.rects.map((r) => ({ ...r, pivot: sel.pivot })))}>Use this pivot for every rect</button>
              </div>
              <div>
                <Checkbox checked={!!sel.nineSlice} onChange={(v) => s.updateRect(s.selected!, { nineSlice: v ? [4, 4, 4, 4] : null })} label="9-slice borders" />
                {sel.nineSlice && (
                  <div className="mt-1.5 flex flex-wrap gap-2">
                    {(["L", "T", "R", "B"] as const).map((k, j) => (
                      <NumField key={k} label={k} value={sel.nineSlice![j]} min={0} onChange={(v) => { const n = [...sel.nineSlice!] as [number, number, number, number]; n[j] = v; s.updateRect(s.selected!, { nineSlice: n }); }} />
                    ))}
                  </div>
                )}
              </div>
            </div>
          </GlassCard>
        )}
        <GlassCard title="Write">
          <div className="flex flex-col gap-3 p-3">
            <Segmented
              size="sm"
              value={s.output}
              options={[{ value: "sprites", label: "Separate PNGs" }, { value: "jsonHash", label: "JSON Hash", hint: "Metadata for the uncut sheet" }, { value: "unityMeta", label: "Unity .meta", hint: "Next to the sheet; Unity slices it on import" }]}
              onChange={(output) => s.setOutput({ output })}
            />
            {s.output !== "unityMeta" && <PathField label="Output folder" value={s.outDir} onChange={(outDir) => s.setOutput({ outDir })} />}
            {s.job && <JobProgress done={s.job.done} total={s.job.total} message={s.job.message} startedAt={s.job.startedAt} />}
            <button type="button" className="btn btn-primary self-start" disabled={!s.rects.length || !!s.job} onClick={() => void s.apply(true)}>
              <Scissors className="size-3.5" /> Slice {s.rects.length} sprite(s)…
            </button>
          </div>
        </GlassCard>
        {s.result && <WriteResult report={s.result} title="Slice result" />}
      </div>

      {s.plan && (
        <Modal
          open
          title={s.plan.ok ? `Write ${s.plan.files.length} file(s)?` : "Cannot slice"}
          onClose={() => useSliceStore.setState({ plan: null })}
          className="max-w-2xl"
          footer={
            <>
              <button type="button" className="btn" onClick={() => useSliceStore.setState({ plan: null })}>Cancel</button>
              <button type="button" className="btn btn-primary" disabled={!s.plan.ok} onClick={() => void s.apply(false)}>Write files</button>
            </>
          }
        >
          <div className="flex flex-col gap-3">
            <div className="rounded-lg border border-edge"><FileList files={s.plan.files} /></div>
            <IssueList issues={s.plan.issues} />
          </div>
        </Modal>
      )}
    </div>
  );
}

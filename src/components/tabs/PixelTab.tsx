import { FieldLabel, GlassCard, JobProgress, Segmented, TextField } from "@abyss/ui";
import { ArrowRight, Download, Palette, Pipette, Plus, Trash2, Upload, Wand2 } from "lucide-react";
import { useState } from "react";
import { pickOne, pickSavePath } from "../../lib/dialogs";
import { usePixelStore, type PixelTool } from "../../stores/toolStores";
import { useProjectStore } from "../../stores/projectStore";
import { InputList, OutputField, WriteResult } from "../common/Fields";

const PALETTE_FILTER = [{ name: "Palette", extensions: ["hex", "gpl", "pal"] }];
const toHex6 = (c: string) => (c.length >= 7 ? c.slice(0, 7) : c);

function Swatch({ hex, onClick, title, size = 22 }: { hex: string; onClick?: () => void; title?: string; size?: number }) {
  return <button type="button" onClick={onClick} title={title ?? hex} className="shrink-0 rounded ring-1 ring-edge-strong" style={{ background: hex, width: size, height: size }} />;
}

function PaletteEditor() {
  const { palette, extracted, set, extract, loadPalette, savePalette, paths } = usePixelStore();
  const [sort, setSort] = useState<"frequency" | "hue">("frequency");
  return (
    <GlassCard
      title={`Palette (${palette.length})`}
      actions={
        <>
          <button type="button" className="icon-btn size-6" title="Load .hex / .gpl / .pal" aria-label="Load palette" onClick={async () => { const p = await pickOne("file", PALETTE_FILTER); if (p) void loadPalette(p); }}><Upload className="size-3.5" /></button>
          <button type="button" className="icon-btn size-6" title="Save palette" aria-label="Save palette" disabled={!palette.length} onClick={async () => { const p = await pickSavePath("palette.hex", PALETTE_FILTER); if (p) void savePalette(p); }}><Download className="size-3.5" /></button>
        </>
      }
    >
      <div className="flex flex-col gap-3 p-3">
        <div className="flex items-center gap-2">
          <Segmented size="sm" value={sort} options={[{ value: "frequency", label: "By use" }, { value: "hue", label: "By hue" }]} onChange={setSort} />
          <button type="button" className="btn" disabled={!paths.length} onClick={() => void extract(sort)}><Pipette className="size-3.5" /> Extract from inputs</button>
        </div>
        <div className="flex flex-wrap gap-1">
          {palette.map((c, i) => (
            <Swatch key={`${c}-${i}`} hex={c} title={`${c}${extracted[i] ? ` · ${extracted[i].count.toLocaleString()} px` : ""} — click to remove`} onClick={() => set({ palette: palette.filter((_, j) => j !== i) })} />
          ))}
          <label className="grid size-[22px] cursor-pointer place-items-center rounded border border-dashed border-edge-strong text-ink-faint hover:text-ink" title="Add a colour">
            <Plus className="size-3" />
            <input type="color" className="sr-only" onChange={(e) => set({ palette: [...palette, e.target.value] })} />
          </label>
        </div>
        {!palette.length && <span className="text-[11.5px] text-ink-faint">Load a Lospec .hex, a GIMP .gpl or a JASC .pal, or extract one from the inputs.</span>}
      </div>
    </GlassCard>
  );
}

function SwapEditor() {
  const { tables, set, palette } = usePixelStore();
  const project = useProjectStore((s) => s.project);
  const update = (i: number, t: (typeof tables)[number]) => set({ tables: tables.map((o, j) => (j === i ? t : o)) });
  return (
    <GlassCard
      title="Swap tables"
      actions={
        <>
          {project.swaps?.length ? <button type="button" className="btn h-6 text-[11.5px]" onClick={() => set({ tables: project.swaps! })}>From project</button> : null}
          <button type="button" className="btn h-6 text-[11.5px]" title="Store these tables in the open project" onClick={() => useProjectStore.getState().update((p) => ({ ...p, swaps: tables }), { repack: false })}>Save to project</button>
          <button type="button" className="icon-btn size-6" aria-label="Add variant" onClick={() => set({ tables: [...tables, { variant: `v${tables.length + 1}`, map: [] }] })}><Plus className="size-3.5" /></button>
        </>
      }
    >
      <div className="flex flex-col gap-3 p-3">
        {tables.map((t, i) => (
          <div key={i} className="flex flex-col gap-1.5 rounded-lg border border-edge p-2">
            <div className="flex items-center gap-2">
              <TextField className="w-28 font-mono" value={t.variant} onChange={(e) => update(i, { ...t, variant: e.target.value.replace(/[^\w-]/g, "_") })} />
              <span className="flex-1 text-[11px] text-ink-faint">→ &lt;name&gt;_{t.variant}.png</span>
              <button type="button" className="icon-btn size-6" aria-label="Remove variant" onClick={() => set({ tables: tables.filter((_, j) => j !== i) })}><Trash2 className="size-3.5" /></button>
            </div>
            {t.map.map(([from, to], k) => (
              <div key={k} className="flex items-center gap-2">
                <input type="color" value={toHex6(from)} onChange={(e) => update(i, { ...t, map: t.map.map((m, j) => (j === k ? [e.target.value, m[1]] : m)) })} />
                <ArrowRight className="size-3.5 text-ink-faint" />
                <input type="color" value={toHex6(to)} onChange={(e) => update(i, { ...t, map: t.map.map((m, j) => (j === k ? [m[0], e.target.value] : m)) })} />
                <code className="flex-1 font-mono text-[11px] text-ink-faint">{from} → {to}</code>
                <button type="button" className="text-ink-faint hover:text-rose" aria-label="Remove pair" onClick={() => update(i, { ...t, map: t.map.filter((_, j) => j !== k) })}>×</button>
              </div>
            ))}
            <div className="flex flex-wrap gap-1">
              {palette.slice(0, 32).map((c) => (
                <Swatch key={c} hex={c} size={16} title={`Swap ${c}`} onClick={() => update(i, { ...t, map: [...t.map, [c, c]] })} />
              ))}
              <button type="button" className="btn h-6 text-[11px]" onClick={() => update(i, { ...t, map: [...t.map, ["#000000", "#ffffff"]] })}>+ pair</button>
            </div>
          </div>
        ))}
        <span className="text-[11px] text-ink-faint">Swatches come from the palette: extract it from the inputs first.</span>
      </div>
    </GlassCard>
  );
}

export function PixelTab() {
  const s = usePixelStore();
  const label: Record<PixelTool, string> = { upscale: `Upscale ×${s.factor}`, palette: "Remap to palette", swap: "Make variants", bleed: "Alpha bleed" };
  const canRun = s.paths.length > 0 && (s.tool !== "palette" || s.palette.length > 0) && (s.tool !== "swap" || s.tables.some((t) => t.map.length));
  return (
    <div className="grid h-full min-h-0 grid-cols-[380px_1fr] gap-3 p-3">
      <div className="flex min-h-0 flex-col gap-3 overflow-y-auto pr-1">
        <GlassCard title="Tool">
          <div className="flex flex-col gap-3 p-3">
            <Segmented
              size="sm"
              value={s.tool}
              options={[{ value: "upscale", label: "Upscale" }, { value: "palette", label: "Remap" }, { value: "swap", label: "Swap" }, { value: "bleed", label: "Bleed" }]}
              onChange={(tool) => s.set({ tool })}
            />
            <InputList paths={s.paths} onChange={(paths) => s.set({ paths })} />
            {s.tool === "upscale" && (
              <div>
                <FieldLabel>Factor (nearest neighbour)</FieldLabel>
                <Segmented size="sm" value={String(s.factor)} options={[2, 3, 4, 5, 6, 8].map((f) => ({ value: String(f), label: `×${f}` }))} onChange={(v) => s.set({ factor: Number(v) })} />
              </div>
            )}
            {s.tool === "bleed" && <span className="text-[11.5px] text-ink-faint">Transparent pixels take the colour of their neighbours (alpha stays 0), so filtering and mipmaps stop pulling dark fringes into the edges.</span>}
            <OutputField value={s.output} onChange={(output) => s.set({ output })} allowInPlace={s.tool !== "swap"} />
            {s.job && <JobProgress done={s.job.done} total={s.job.total} message={s.job.message} startedAt={s.job.startedAt} />}
            <div className="flex gap-2">
              <button type="button" className="btn" disabled={!canRun || !!s.job} onClick={() => void s.run(true)}>Dry run</button>
              <button type="button" className="btn btn-primary" disabled={!canRun || !!s.job} onClick={() => void s.run(false)}>
                <Wand2 className="size-3.5" /> {label[s.tool]}
              </button>
            </div>
          </div>
        </GlassCard>
        {(s.tool === "palette" || s.tool === "swap") && <PaletteEditor />}
        {s.tool === "swap" && <SwapEditor />}
      </div>
      <div className="flex min-h-0 flex-col gap-3 overflow-y-auto">
        <GlassCard title="Before / after">
          <div className="grid grid-cols-2 gap-3 p-3">
            {[["Before", s.before], ["After", s.after]].map(([t, url]) => (
              <div key={t} className="flex flex-col gap-1">
                <FieldLabel>{t}</FieldLabel>
                <div className="checker grid h-64 place-items-center overflow-hidden rounded">
                  {url ? <img src={url} alt={t ?? ""} className="max-h-full max-w-full object-contain" style={{ imageRendering: "pixelated", minHeight: 64 }} /> : <Palette className="size-6 text-ink-faint" />}
                </div>
              </div>
            ))}
          </div>
        </GlassCard>
        {s.result && <WriteResult report={s.result} title={label[s.tool]} />}
      </div>
    </div>
  );
}

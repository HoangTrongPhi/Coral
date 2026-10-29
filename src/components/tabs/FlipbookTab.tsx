import { Checkbox, EmptyState, FieldLabel, GlassCard, JobProgress, Segmented } from "@abyss/ui";
import { Grid3x3, Pause, Play } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useFlipbookStore, setFlipbookFrames } from "../../stores/toolStores";
import { InputList, NumField, PathField, WriteResult } from "../common/Fields";

/** Plays the laid-out sheet cell by cell, the way an engine would. */
function Player({ img, cols, rows, frames, fps }: { img: HTMLImageElement; cols: number; rows: number; frames: number; fps: number }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const [playing, setPlaying] = useState(true);
  const [i, setI] = useState(0);
  const cw = img.width / cols;
  const ch = img.height / rows;
  const scale = Math.min(256 / cw, 256 / ch, 8);
  useEffect(() => {
    if (!playing) return;
    const t = setInterval(() => setI((n) => (n + 1) % Math.max(1, frames)), 1000 / Math.max(1, fps));
    return () => clearInterval(t);
  }, [playing, frames, fps]);
  useEffect(() => {
    const ctx = ref.current?.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);
    ctx.imageSmoothingEnabled = scale < 1;
    ctx.drawImage(img, (i % cols) * cw, Math.floor(i / cols) * ch, cw, ch, 0, 0, cw * scale, ch * scale);
  }, [i, img, cols, cw, ch, scale]);
  return (
    <div className="flex flex-col items-center gap-2">
      <canvas ref={ref} width={Math.round(cw * scale)} height={Math.round(ch * scale)} className="checker rounded" />
      <div className="flex items-center gap-2">
        <button type="button" className="btn" onClick={() => setPlaying(!playing)}>{playing ? <Pause className="size-3.5" /> : <Play className="size-3.5" />}</button>
        <span className="text-[12px] text-ink-muted tabular-nums">frame {i + 1}/{frames}</span>
      </div>
    </div>
  );
}

export function FlipbookTab() {
  const { options: o, report, preview, job, set, run, cancel } = useFlipbookStore();
  const dirty = useRef(false);
  // Re-plan (dry run) when an option changes.
  useEffect(() => {
    if (!dirty.current) {
      dirty.current = true;
      return;
    }
    const t = setTimeout(() => void run(true), 300);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [o.paths, o.from, o.to, o.step, o.cols, o.rows, o.cell, o.maxSize]);

  return (
    <div className="grid h-full min-h-0 grid-cols-[340px_1fr] gap-3 p-3">
      <div className="flex min-h-0 flex-col gap-3 overflow-y-auto pr-1">
        <GlassCard title="Frames">
          <div className="flex flex-col gap-3 p-3">
            <InputList paths={o.paths} onChange={setFlipbookFrames} accept="a folder of numbered frames" />
            <div className="flex flex-wrap gap-2">
              <NumField label="From" value={o.from} min={0} onChange={(from) => set({ from })} hint="1-based; 0 = first" />
              <NumField label="To" value={o.to} min={0} onChange={(to) => set({ to })} hint="1-based; 0 = last" />
              <NumField label="Step" value={o.step} min={1} onChange={(step) => set({ step })} />
              <NumField label="FPS" value={o.fps} min={1} max={120} onChange={(fps) => set({ fps })} />
            </div>
          </div>
        </GlassCard>
        <GlassCard title="Grid">
          <div className="flex flex-col gap-3 p-3">
            <div className="flex flex-wrap gap-2">
              <NumField label="Columns" value={o.cols} min={0} onChange={(cols) => set({ cols })} hint="0 = suggested" />
              <NumField label="Rows" value={o.rows} min={0} onChange={(rows) => set({ rows })} hint="0 = suggested" />
              <NumField label="Cell" value={o.cell} min={0} suffix="px" onChange={(cell) => set({ cell })} hint="Longer side; 0 = the frames' size (Lanczos3 resize)" />
            </div>
            <div>
              <FieldLabel>Max size</FieldLabel>
              <Segmented size="sm" value={String(o.maxSize)} options={[1024, 2048, 4096, 8192].map((v) => ({ value: String(v), label: String(v) }))} onChange={(v) => set({ maxSize: Number(v) })} />
            </div>
          </div>
        </GlassCard>
        <GlassCard title="Write">
          <div className="flex flex-col gap-3 p-3">
            <PathField label="Output image" kind="file" value={o.out} onChange={(out) => set({ out, format: out.toLowerCase().endsWith(".tga") ? "tga" : "png" })} placeholder="T_Explosion_SubUV.png" />
            <Checkbox checked={o.json} onChange={(json) => set({ json })} label="Also write a .json (cols, rows, frames, fps, cell)" />
            {job && <JobProgress done={job.done} total={job.total} message={job.message} startedAt={job.startedAt} onCancel={() => void cancel()} cancelling={job.cancelling} />}
            <button type="button" className="btn btn-primary self-start" disabled={!o.paths.length || !o.out || !!job} onClick={() => void run(false)}>
              <Grid3x3 className="size-3.5" /> Write flipbook
            </button>
          </div>
        </GlassCard>
      </div>

      <div className="flex min-h-0 flex-col gap-3 overflow-y-auto">
        {!report ? (
          <div className="grid flex-1 place-items-center rounded-xl border border-dashed border-edge">
            <EmptyState icon={<Grid3x3 className="size-5" />} title="Drop a folder of frames">explosion_0001.png … explosion_0064.png become an 8×8 sheet; frames are sorted naturally.</EmptyState>
          </div>
        ) : (
          <>
            <GlassCard title={`${report.frames} frame(s) · ${report.cols} × ${report.rows} · cell ${report.cell[0]}×${report.cell[1]} · ${report.width}×${report.height}`}>
              <div className="grid gap-4 p-4 lg:grid-cols-[1fr_auto]">
                {preview && <img src={preview.src} alt="Flipbook sheet" className="checker max-h-[50vh] w-full rounded object-contain" style={{ imageRendering: "pixelated" }} />}
                {preview && <Player img={preview} cols={report.cols} rows={report.rows} frames={report.frames} fps={o.fps} />}
              </div>
              <div className="flex flex-wrap gap-x-6 gap-y-1 border-t border-edge px-4 py-2 font-mono text-[11.5px] text-ink-muted select-text">
                <span>Niagara SubUV: Sub Image Size = {report.cols} × {report.rows}</span>
                <span>Unity Texture Sheet Animation: Tiles X = {report.cols}, Y = {report.rows}</span>
              </div>
            </GlassCard>
            {!report.write.dryRun && <WriteResult report={report.write} title="Flipbook written" />}
            {report.write.dryRun && report.write.issues.length > 0 && <WriteResult report={report.write} title="Plan" />}
          </>
        )}
      </div>
    </div>
  );
}

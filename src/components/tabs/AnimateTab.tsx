import { Checkbox, EmptyState, FieldLabel, GlassCard, PillBadge, Segmented, TextField } from "@abyss/ui";
import { ChevronLeft, ChevronRight, Copy, Crosshair, Film, MousePointer2, Pause, Play, Square, Trash2 } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useProjectStore } from "../../stores/projectStore";
import type { Animation, Hitbox, LoopMode, PackedSprite } from "../../types/coral";
import { NumField } from "../common/Fields";
import { drawPacked, ZoomCanvas, ZoomControl } from "../common/ZoomCanvas";
import { SpriteThumb } from "./PackTab";

type Tool = "select" | "pivot" | "box";
const KIND_COLOR: Record<string, string> = { hitbox: "#ef4444", hurtbox: "#10b981" };
const colorOf = (kind: string) => KIND_COLOR[kind] ?? "#f59e0b";

/** Play order (ping-pong unrolled) as frame indices. */
function playOrder(a: Animation): number[] {
  const n = a.frames.length;
  const fwd = [...Array(n).keys()];
  return a.loop === "pingpong" && n > 2 ? [...fwd, ...fwd.slice(1, -1).reverse()] : fwd;
}

const durationOf = (a: Animation, i: number) => a.durationsMs?.[i] ?? 1000 / Math.max(0.01, a.fps);

export function AnimateTab() {
  const { project, report, pageImages, setAnimation, setSpriteOverride, setHitboxes } = useProjectStore();
  const anims = report?.animations ?? [];
  const [animName, setAnimName] = useState<string | null>(null);
  const anim = anims.find((a) => a.name === animName) ?? anims[0];
  const [frame, setFrame] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [zoom, setZoom] = useState(4);
  const [onion, setOnion] = useState(0);
  const [tool, setTool] = useState<Tool>("select");
  const [pivotAll, setPivotAll] = useState(true);
  const [boxKind, setBoxKind] = useState("hitbox");
  const [selBox, setSelBox] = useState<number | null>(null);
  const [drag, setDrag] = useState<{ x0: number; y0: number; x1: number; y1: number } | null>(null);

  const byName = useMemo(() => new Map((report?.sprites ?? []).map((s) => [s.name, s])), [report]);
  const sprites: PackedSprite[] = useMemo(() => (anim?.frames ?? []).map((f) => byName.get(f)).filter((s): s is PackedSprite => !!s), [anim, byName]);
  const box = useMemo(() => ({ w: Math.max(1, ...sprites.map((s) => s.trim.sourceW)), h: Math.max(1, ...sprites.map((s) => s.trim.sourceH)) }), [sprites]);
  const cur = sprites[Math.min(frame, sprites.length - 1)];
  const boxes: Hitbox[] = (cur && project.hitboxes[cur.name]) || [];

  useEffect(() => {
    setFrame(0);
    setSelBox(null);
  }, [anim?.name]);

  // Playback with per-frame durations.
  const orderPos = useRef(0);
  useEffect(() => {
    if (!playing || !anim || sprites.length < 2) return;
    const order = playOrder(anim);
    let t: ReturnType<typeof setTimeout>;
    const tick = () => {
      const i = order[orderPos.current % order.length];
      setFrame(i);
      t = setTimeout(() => {
        orderPos.current++;
        if (anim.loop === "once" && orderPos.current >= order.length) {
          setPlaying(false);
          orderPos.current = 0;
          return;
        }
        tick();
      }, durationOf(anim, i));
    };
    tick();
    return () => clearTimeout(t);
  }, [playing, anim, sprites.length]);

  if (!report || !anim) {
    return (
      <div className="grid h-full place-items-center">
        <EmptyState icon={<Film className="size-5" />} title="No animations">
          Animations come from Aseprite tags and numbered file names (run_0, run_1… or walk0001…). Add sources in Pack first.
        </EmptyState>
      </div>
    );
  }

  const edit = (a: Partial<Animation>) => setAnimation({ ...anim, ...a });
  const setDuration = (i: number, ms: number) => {
    const d = anim.durationsMs ? [...anim.durationsMs] : anim.frames.map(() => Math.round(1000 / anim.fps));
    d[i] = Math.max(1, Math.round(ms));
    edit({ durationsMs: d });
  };
  const pivotOf = (s: PackedSprite) => project.sprites[s.name]?.pivot ?? s.pivot;
  const setPivot = (px: number, py: number) => {
    if (!cur) return;
    const p: [number, number] = [Math.round((px / cur.trim.sourceW) * 1000) / 1000, Math.round((py / cur.trim.sourceH) * 1000) / 1000];
    for (const s of pivotAll ? sprites : [cur]) setSpriteOverride(s.name, { pivot: p });
  };
  const hitAt = (x: number, y: number) => boxes.findIndex((b) => x >= b.rect[0] && y >= b.rect[1] && x < b.rect[0] + b.rect[2] && y < b.rect[1] + b.rect[3]);
  const copyToNext = () => {
    const next = sprites[frame + 1];
    if (!cur || !next) return;
    setHitboxes(next.name, boxes.map((b) => ({ ...b })));
    setFrame(frame + 1);
  };
  const pageOf = (s: PackedSprite) => pageImages[s.page];

  return (
    <div className="grid h-full min-h-0 grid-cols-[220px_1fr_280px] gap-3 p-3">
      <GlassCard title={`Animations (${anims.length})`} className="flex min-h-0 flex-col">
        <ul className="min-h-0 flex-1 overflow-y-auto py-1">
          {anims.map((a) => (
            <li key={a.name}>
              <button type="button" onClick={() => setAnimName(a.name)} className={`flex w-full items-center gap-2 px-3 py-1.5 text-left text-[12px] ${a.name === anim.name ? "bg-accent/14 text-ink" : "text-ink-muted hover:bg-tint/5"}`}>
                <span className="min-w-0 flex-1 truncate font-mono">{a.name}</span>
                <span className="text-[11px] text-ink-faint tabular-nums">{a.frames.length}f · {a.fps}</span>
                {project.animations.some((p) => p.name === a.name) && <PillBadge tone="accent">edited</PillBadge>}
              </button>
            </li>
          ))}
        </ul>
      </GlassCard>

      <div className="flex min-h-0 flex-col overflow-hidden rounded-xl border border-edge bg-panel/50">
        <div className="flex flex-wrap items-center gap-2 border-b border-edge px-3 py-2">
          <button type="button" className="icon-btn" aria-label="Previous frame" onClick={() => { setPlaying(false); setFrame((frame - 1 + sprites.length) % sprites.length); }}><ChevronLeft className="size-4" /></button>
          <button type="button" className="btn btn-primary h-7" onClick={() => setPlaying(!playing)}>{playing ? <Pause className="size-3.5" /> : <Play className="size-3.5" />}{playing ? "Pause" : "Play"}</button>
          <button type="button" className="icon-btn" aria-label="Next frame" onClick={() => { setPlaying(false); setFrame((frame + 1) % sprites.length); }}><ChevronRight className="size-4" /></button>
          <span className="text-[12px] text-ink-muted tabular-nums">frame {frame + 1}/{sprites.length}</span>
          <div className="mx-2 h-5 w-px bg-edge" />
          <Segmented
            size="sm"
            value={tool}
            options={[
              { value: "select", label: <MousePointer2 className="size-3.5" />, hint: "Select a box" },
              { value: "pivot", label: <Crosshair className="size-3.5" />, hint: "Click to set the pivot" },
              { value: "box", label: <Square className="size-3.5" />, hint: "Drag to draw a box" },
            ]}
            onChange={setTool}
          />
          <div className="flex-1" />
          <ZoomControl zoom={zoom} setZoom={setZoom} />
        </div>
        <div className="grid min-h-0 flex-1 place-items-center overflow-auto p-6">
          <ZoomCanvas
            width={box.w}
            height={box.h}
            zoom={zoom}
            cursor={tool === "select" ? "default" : "crosshair"}
            deps={[cur, onion, sprites, boxes, drag, selBox, project.sprites, pageImages]}
            draw={(ctx) => {
              for (let k = onion; k >= 1; k--) {
                const prev = sprites[frame - k];
                const next = sprites[frame + k];
                if (prev) drawPacked(ctx, pageOf(prev), prev, 0, 0, 0.3 / k);
                if (next) drawPacked(ctx, pageOf(next), next, 0, 0, 0.3 / k);
              }
              if (cur) drawPacked(ctx, pageOf(cur), cur, 0, 0);
              ctx.lineWidth = 1 / zoom;
              boxes.forEach((b, i) => {
                ctx.strokeStyle = colorOf(b.kind);
                ctx.fillStyle = `${colorOf(b.kind)}33`;
                ctx.fillRect(b.rect[0], b.rect[1], b.rect[2], b.rect[3]);
                ctx.lineWidth = (i === selBox ? 2.5 : 1.5) / zoom;
                ctx.strokeRect(b.rect[0], b.rect[1], b.rect[2], b.rect[3]);
              });
              if (drag) {
                ctx.strokeStyle = colorOf(boxKind);
                ctx.setLineDash([3 / zoom, 2 / zoom]);
                ctx.strokeRect(Math.min(drag.x0, drag.x1), Math.min(drag.y0, drag.y1), Math.abs(drag.x1 - drag.x0), Math.abs(drag.y1 - drag.y0));
                ctx.setLineDash([]);
              }
              if (cur) {
                const [px, py] = pivotOf(cur);
                const x = px * cur.trim.sourceW;
                const y = py * cur.trim.sourceH;
                ctx.strokeStyle = "#a855f7";
                ctx.lineWidth = 1.5 / zoom;
                ctx.beginPath();
                ctx.moveTo(x - 6 / zoom, y);
                ctx.lineTo(x + 6 / zoom, y);
                ctx.moveTo(x, y - 6 / zoom);
                ctx.lineTo(x, y + 6 / zoom);
                ctx.stroke();
              }
              ctx.strokeStyle = "rgba(255,255,255,0.15)";
              ctx.lineWidth = 1 / zoom;
              ctx.strokeRect(0, 0, box.w, box.h);
            }}
            onDown={(p) => {
              setPlaying(false);
              if (tool === "pivot") setPivot(p.x, p.y);
              else if (tool === "box") setDrag({ x0: Math.round(p.x), y0: Math.round(p.y), x1: Math.round(p.x), y1: Math.round(p.y) });
              else setSelBox(hitAt(p.x, p.y) >= 0 ? hitAt(p.x, p.y) : null);
            }}
            onMove={(p) => {
              if (tool === "pivot" && p.buttons) setPivot(p.x, p.y);
              if (drag && p.buttons) setDrag({ ...drag, x1: Math.round(Math.min(box.w, Math.max(0, p.x))), y1: Math.round(Math.min(box.h, Math.max(0, p.y))) });
            }}
            onUp={() => {
              if (!drag || !cur) return;
              const r: [number, number, number, number] = [Math.min(drag.x0, drag.x1), Math.min(drag.y0, drag.y1), Math.abs(drag.x1 - drag.x0), Math.abs(drag.y1 - drag.y0)];
              setDrag(null);
              if (r[2] < 1 || r[3] < 1) return;
              setHitboxes(cur.name, [...boxes, { kind: boxKind, rect: r }]);
              setSelBox(boxes.length);
            }}
          />
        </div>
        <div className="flex gap-1 overflow-x-auto border-t border-edge p-2">
          {sprites.map((s, i) => (
            <div key={`${s.name}-${i}`} className={`flex shrink-0 flex-col items-center gap-1 rounded-md p-1 ${i === frame ? "bg-accent/14 ring-1 ring-accent/40" : "hover:bg-tint/5"}`}>
              <button type="button" onClick={() => { setPlaying(false); setFrame(i); }} title={s.name}>
                <SpriteThumb sprite={s} size={40} />
              </button>
              <input
                type="number"
                className="h-5 w-14 rounded border border-edge bg-canvas/70 px-1 text-center text-[10.5px] text-ink tabular-nums"
                value={Math.round(durationOf(anim, i))}
                min={1}
                title="Duration (ms)"
                onChange={(e) => Number(e.target.value) > 0 && setDuration(i, Number(e.target.value))}
              />
              {(project.hitboxes[s.name]?.length ?? 0) > 0 && <span className="size-1.5 rounded-full bg-rose" title="Has boxes" />}
            </div>
          ))}
        </div>
      </div>

      <div className="flex min-h-0 flex-col gap-3 overflow-y-auto pr-1">
        <GlassCard title="Playback">
          <div className="flex flex-col gap-3 p-3">
            <div className="flex items-end gap-3">
              <NumField label="FPS" value={anim.fps} min={0.5} max={120} step={0.5} onChange={(fps) => edit({ fps })} />
              <NumField label="Onion skin" value={onion} min={0} max={3} onChange={setOnion} hint="Frames before and after, 30% alpha" />
            </div>
            <div>
              <FieldLabel>Loop</FieldLabel>
              <Segmented size="sm" value={anim.loop} options={[{ value: "loop", label: "Loop" }, { value: "once", label: "Once" }, { value: "pingpong", label: "Ping-pong" }]} onChange={(loop: LoopMode) => edit({ loop })} />
            </div>
            {anim.durationsMs && (
              <button type="button" className="btn self-start" onClick={() => edit({ durationsMs: null })}>Same duration for every frame</button>
            )}
          </div>
        </GlassCard>
        <GlassCard title="Pivot">
          <div className="flex flex-col gap-2 p-3 text-[12px]">
            {cur && <div className="text-ink-muted tabular-nums">{pivotOf(cur)[0].toFixed(3)}, {pivotOf(cur)[1].toFixed(3)} (0–1 of {cur.trim.sourceW}×{cur.trim.sourceH})</div>}
            <Checkbox checked={pivotAll} onChange={setPivotAll} label="Apply to the whole animation" />
            <div className="grid w-24 grid-cols-3 gap-1">
              {[0, 0.5, 1].flatMap((y) => [0, 0.5, 1].map((x) => (
                <button key={`${x}${y}`} type="button" className="grid size-7 place-items-center rounded border border-edge hover:border-accent" title={`${x}, ${y}`} onClick={() => cur && setPivot(x * cur.trim.sourceW, y * cur.trim.sourceH)}>
                  <span className="size-1.5 rounded-full bg-ink-muted" />
                </button>
              )))}
            </div>
            <span className="text-[11px] text-ink-faint">Or pick the crosshair tool and click / drag on the frame.</span>
          </div>
        </GlassCard>
        <GlassCard title={`Boxes · frame ${frame + 1}`} actions={<button type="button" className="icon-btn size-6" title="Copy to the next frame" aria-label="Copy boxes to the next frame" disabled={!sprites[frame + 1]} onClick={copyToNext}><Copy className="size-3.5" /></button>}>
          <div className="flex flex-col gap-2 p-3">
            <div>
              <FieldLabel>New box type</FieldLabel>
              <Segmented size="sm" value={["hitbox", "hurtbox"].includes(boxKind) ? boxKind : "custom"} options={[{ value: "hitbox", label: "Hitbox" }, { value: "hurtbox", label: "Hurtbox" }, { value: "custom", label: "Custom" }]} onChange={(v) => setBoxKind(v === "custom" ? "custom:box" : v)} />
              {boxKind.startsWith("custom:") && <TextField className="mt-1.5 w-full" value={boxKind.slice(7)} onChange={(e) => setBoxKind(`custom:${e.target.value.replace(/\s/g, "_") || "box"}`)} placeholder="name" />}
            </div>
            <ul className="flex flex-col gap-1">
              {boxes.map((b, i) => (
                <li key={i} className={`flex items-center gap-2 rounded-md px-2 py-1 text-[11.5px] ${i === selBox ? "bg-tint/8" : ""}`} onClick={() => setSelBox(i)}>
                  <span className="size-2.5 rounded-sm" style={{ background: colorOf(b.kind) }} />
                  <span className="font-mono text-ink">{b.kind}</span>
                  <input className="h-5 min-w-0 flex-1 rounded border border-edge bg-canvas/70 px-1 text-[11px] text-ink" placeholder="name" value={b.name ?? ""} onChange={(e) => setHitboxes(cur!.name, boxes.map((o, j) => (j === i ? { ...o, name: e.target.value || undefined } : o)))} />
                  <span className="text-ink-faint tabular-nums">{b.rect.join(",")}</span>
                  <button type="button" className="text-ink-faint hover:text-rose" aria-label="Delete box" onClick={(e) => { e.stopPropagation(); setHitboxes(cur!.name, boxes.filter((_, j) => j !== i)); setSelBox(null); }}>
                    <Trash2 className="size-3" />
                  </button>
                </li>
              ))}
            </ul>
            {boxes.length === 0 && <span className="text-[11.5px] text-ink-faint">Pick the box tool and drag on the frame. Boxes are exported in meta.coral.hitboxes (untrimmed sprite pixels, top-left origin).</span>}
          </div>
        </GlassCard>
      </div>
    </div>
  );
}

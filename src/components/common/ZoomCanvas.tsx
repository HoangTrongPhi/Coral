import { Segmented } from "@abyss/ui";
import { Minus, Plus } from "lucide-react";
import { useEffect, useRef, type PointerEvent as RPointerEvent, type ReactNode } from "react";

export const ZOOMS = [0.25, 0.5, 1, 2, 4, 8] as const;

export interface CanvasPoint {
  /** Content pixels (fractional). */
  x: number;
  y: number;
  buttons: number;
  shift: boolean;
}

/**
 * A pixel-perfect canvas (no smoothing) of `width`×`height` content pixels at `zoom`, scrollable, with
 * pointer events in content coordinates. `draw` gets a context already scaled by `zoom`.
 */
export function ZoomCanvas({ width, height, zoom, draw, deps, onDown, onMove, onUp, onLeave, cursor, checker = true }: {
  width: number;
  height: number;
  zoom: number;
  draw: (ctx: CanvasRenderingContext2D) => void;
  deps: unknown[];
  onDown?: (p: CanvasPoint) => void;
  onMove?: (p: CanvasPoint) => void;
  onUp?: (p: CanvasPoint) => void;
  onLeave?: () => void;
  cursor?: string;
  checker?: boolean;
}) {
  const ref = useRef<HTMLCanvasElement>(null);
  const dpr = typeof window === "undefined" ? 1 : window.devicePixelRatio || 1;
  const cw = Math.max(1, Math.round(width * zoom));
  const ch = Math.max(1, Math.round(height * zoom));

  useEffect(() => {
    const c = ref.current;
    if (!c) return;
    const ctx = c.getContext("2d");
    if (!ctx) return;
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, c.width, c.height);
    ctx.imageSmoothingEnabled = false;
    ctx.setTransform(zoom * dpr, 0, 0, zoom * dpr, 0, 0);
    draw(ctx);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [zoom, width, height, dpr, ...deps]);

  const point = (e: RPointerEvent<HTMLCanvasElement>): CanvasPoint => {
    const r = e.currentTarget.getBoundingClientRect();
    return { x: (e.clientX - r.left) / zoom, y: (e.clientY - r.top) / zoom, buttons: e.buttons, shift: e.shiftKey };
  };

  return (
    <canvas
      ref={ref}
      width={cw * dpr}
      height={ch * dpr}
      style={{ width: cw, height: ch, cursor, imageRendering: "pixelated" }}
      className={checker ? "checker shrink-0" : "shrink-0"}
      onPointerDown={(e) => {
        e.currentTarget.setPointerCapture(e.pointerId);
        onDown?.(point(e));
      }}
      onPointerMove={(e) => onMove?.(point(e))}
      onPointerUp={(e) => onUp?.(point(e))}
      onPointerLeave={() => onLeave?.()}
    />
  );
}

/** Zoom buttons: 25 % – 800 %. */
export function ZoomControl({ zoom, setZoom, extra }: { zoom: number; setZoom: (z: number) => void; extra?: ReactNode }) {
  const i = ZOOMS.indexOf(zoom as (typeof ZOOMS)[number]);
  return (
    <div className="flex items-center gap-1">
      <button type="button" className="icon-btn" aria-label="Zoom out" disabled={i <= 0} onClick={() => setZoom(ZOOMS[Math.max(0, i - 1)])}>
        <Minus className="size-3.5" />
      </button>
      <Segmented size="sm" value={String(zoom)} options={ZOOMS.map((z) => ({ value: String(z), label: `${z * 100}%` }))} onChange={(v) => setZoom(Number(v))} />
      <button type="button" className="icon-btn" aria-label="Zoom in" disabled={i >= ZOOMS.length - 1} onClick={() => setZoom(ZOOMS[Math.min(ZOOMS.length - 1, i + 1)])}>
        <Plus className="size-3.5" />
      </button>
      {extra}
    </div>
  );
}

/** Draw a packed sprite (frame of an atlas page, rotated ones turned back) at `dx, dy` in its untrimmed box. */
export function drawPacked(
  ctx: CanvasRenderingContext2D,
  page: HTMLImageElement | undefined,
  s: { frame: { x: number; y: number; w: number; h: number }; rotated: boolean; trim: { x: number; y: number } },
  dx: number,
  dy: number,
  alpha = 1,
) {
  if (!page) return;
  ctx.save();
  ctx.globalAlpha = alpha;
  const { x, y, w, h } = s.frame;
  if (s.rotated) {
    // Stored 90° clockwise (h × w in the page): rotate back counter-clockwise.
    ctx.translate(dx + s.trim.x, dy + s.trim.y + h);
    ctx.rotate(-Math.PI / 2);
    ctx.drawImage(page, x, y, h, w, 0, 0, h, w);
  } else {
    ctx.drawImage(page, x, y, w, h, dx + s.trim.x, dy + s.trim.y, w, h);
  }
  ctx.restore();
}

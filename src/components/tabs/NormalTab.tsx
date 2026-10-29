import { Checkbox, EmptyState, FieldLabel, GlassCard, JobProgress, Segmented } from "@abyss/ui";
import { Mountain, Wand2 } from "lucide-react";
import { useEffect, useRef } from "react";
import { baseName } from "../../lib/jobs";
import { useProjectStore } from "../../stores/projectStore";
import { useNormalStore } from "../../stores/toolStores";
import { InputList, NumField, OutputField, WriteResult } from "../common/Fields";

const VERT = `attribute vec2 p; varying vec2 uv; void main(){ uv = vec2(p.x*0.5+0.5, 0.5-p.y*0.5); gl_Position = vec4(p,0.,1.); }`;
// Lambert with a point light at the pointer; green flipped for DirectX maps so both light the same.
const FRAG = `precision mediump float; varying vec2 uv; uniform sampler2D diffuse; uniform sampler2D normalMap;
uniform vec3 light; uniform float flipG; uniform float aspect;
void main(){
  vec4 d = texture2D(diffuse, uv); vec4 n4 = texture2D(normalMap, uv);
  vec3 n = normalize(n4.rgb*2.0-1.0); n.y *= flipG;
  vec3 pos = vec3(uv.x*aspect, 1.0-uv.y, 0.0);
  vec3 l = normalize(vec3(light.x*aspect, light.y, light.z) - pos);
  float lambert = max(dot(n, l), 0.0);
  gl_FragColor = vec4(d.rgb*(0.18+0.95*lambert), d.a);
}`;

function LitPreview({ diffuse, normal, directx }: { diffuse: HTMLImageElement; normal: HTMLImageElement; directx: boolean }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const light = useRef<[number, number]>([0.3, 0.7]);
  const draw = useRef<() => void>(() => {});
  useEffect(() => {
    const c = ref.current;
    const gl = c?.getContext("webgl", { premultipliedAlpha: false });
    if (!c || !gl) return;
    const sh = (type: number, src: string) => {
      const s = gl.createShader(type)!;
      gl.shaderSource(s, src);
      gl.compileShader(s);
      return s;
    };
    const prog = gl.createProgram()!;
    gl.attachShader(prog, sh(gl.VERTEX_SHADER, VERT));
    gl.attachShader(prog, sh(gl.FRAGMENT_SHADER, FRAG));
    gl.linkProgram(prog);
    gl.useProgram(prog);
    const buf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
    const loc = gl.getAttribLocation(prog, "p");
    gl.enableVertexAttribArray(loc);
    gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
    const tex = (img: HTMLImageElement, unit: number) => {
      const t = gl.createTexture();
      gl.activeTexture(gl.TEXTURE0 + unit);
      gl.bindTexture(gl.TEXTURE_2D, t);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);
      for (const p of [gl.TEXTURE_WRAP_S, gl.TEXTURE_WRAP_T]) gl.texParameteri(gl.TEXTURE_2D, p, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    };
    tex(diffuse, 0);
    tex(normal, 1);
    gl.uniform1i(gl.getUniformLocation(prog, "diffuse"), 0);
    gl.uniform1i(gl.getUniformLocation(prog, "normalMap"), 1);
    gl.uniform1f(gl.getUniformLocation(prog, "flipG"), directx ? -1 : 1);
    gl.uniform1f(gl.getUniformLocation(prog, "aspect"), diffuse.width / diffuse.height);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    const lightLoc = gl.getUniformLocation(prog, "light");
    draw.current = () => {
      gl.viewport(0, 0, c.width, c.height);
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
      gl.uniform3f(lightLoc, light.current[0], light.current[1], 0.35);
      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    };
    draw.current();
  }, [diffuse, normal, directx]);
  const scale = Math.max(1, Math.floor(Math.min(384 / diffuse.width, 384 / diffuse.height)));
  return (
    <canvas
      ref={ref}
      width={diffuse.width}
      height={diffuse.height}
      className="checker rounded"
      style={{ width: diffuse.width * scale, height: diffuse.height * scale, imageRendering: "pixelated", cursor: "none" }}
      onPointerMove={(e) => {
        const r = e.currentTarget.getBoundingClientRect();
        light.current = [(e.clientX - r.left) / r.width, 1 - (e.clientY - r.top) / r.height];
        draw.current();
      }}
    />
  );
}

export function NormalTab() {
  const n = useNormalStore();
  const s = n.settings;
  const projectNormals = useProjectStore((p) => p.project.pack.normalMaps);
  useEffect(() => {
    if (n.current && !n.diffuse) void n.preview();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return (
    <div className="grid h-full min-h-0 grid-cols-[340px_1fr] gap-3 p-3">
      <div className="flex min-h-0 flex-col gap-3 overflow-y-auto pr-1">
        <GlassCard title="Height">
          <div className="flex flex-col gap-3 p-3">
            <InputList paths={n.paths} onChange={(paths) => n.set({ paths, current: paths[0] ?? null })} />
            <div>
              <FieldLabel>Height from</FieldLabel>
              <Segmented size="sm" value={s.source} options={[{ value: "luminance", label: "Luminance" }, { value: "alphaBevel", label: "Alpha bevel", hint: "Raised from the sprite's edge inward" }]} onChange={(source) => n.setSettings({ source })} />
            </div>
            <div className="flex flex-wrap gap-2">
              <NumField label="Strength" value={s.strength} min={0} max={20} step={0.25} onChange={(strength) => n.setSettings({ strength })} />
              <NumField label="Blur" value={s.blur} min={0} max={16} step={0.5} suffix="px" onChange={(blur) => n.setSettings({ blur })} />
              {s.source === "alphaBevel" && <NumField label="Bevel" value={s.bevel} min={0.5} max={64} step={0.5} suffix="px" onChange={(bevel) => n.setSettings({ bevel })} />}
            </div>
            <Checkbox checked={s.invert} onChange={(invert) => n.setSettings({ invert })} label="Invert (dark = raised)" />
            <div>
              <FieldLabel>Green channel</FieldLabel>
              <Segmented size="sm" value={s.convention} options={[{ value: "opengl", label: "OpenGL (Godot, Unity)" }, { value: "directx", label: "DirectX (Unreal)" }]} onChange={(convention) => n.setSettings({ convention })} />
            </div>
            <button type="button" className="btn self-start" onClick={() => useProjectStore.getState().setPack({ normal: s, normalMaps: true })} title="The Pack module then builds a normal-map atlas with the same rects">
              Use for the project's normal atlas{projectNormals ? " (on)" : ""}
            </button>
          </div>
        </GlassCard>
        <GlassCard title="Write">
          <div className="flex flex-col gap-3 p-3">
            <OutputField value={n.output} onChange={(output) => n.set({ output })} allowInPlace={false} />
            <span className="text-[11px] text-ink-faint">Each input gets &lt;name&gt;_n.png. Transparent pixels are flat (128, 128, 255).</span>
            {n.job && <JobProgress done={n.job.done} total={n.job.total} message={n.job.message} startedAt={n.job.startedAt} />}
            <div className="flex gap-2">
              <button type="button" className="btn" disabled={!n.paths.length || !!n.job} onClick={() => void n.run(true)}>Dry run</button>
              <button type="button" className="btn btn-primary" disabled={!n.paths.length || !!n.job} onClick={() => void n.run(false)}><Wand2 className="size-3.5" /> Write normal maps</button>
            </div>
          </div>
        </GlassCard>
      </div>
      <div className="flex min-h-0 flex-col gap-3 overflow-y-auto">
        {!n.diffuse || !n.normal ? (
          <div className="grid flex-1 place-items-center rounded-xl border border-dashed border-edge">
            <EmptyState icon={<Mountain className="size-5" />} title="Drop a sprite">Its normal map shows here, lit by a light that follows the pointer.</EmptyState>
          </div>
        ) : (
          <GlassCard title={n.current ? baseName(n.current) : "Preview"}>
            <div className="flex flex-wrap items-start justify-center gap-6 p-4">
              <div className="flex flex-col items-center gap-1">
                <LitPreview diffuse={n.diffuse} normal={n.normal} directx={s.convention === "directx"} />
                <span className="text-[11px] text-ink-faint">Lit — move the pointer over it</span>
              </div>
              <div className="flex flex-col items-center gap-1">
                <img src={n.normal.src} alt="Normal map" className="checker rounded" style={{ width: Math.min(384, n.normal.width * Math.max(1, Math.floor(384 / Math.max(n.normal.width, n.normal.height)))), imageRendering: "pixelated" }} />
                <span className="text-[11px] text-ink-faint">Normal map ({s.convention === "opengl" ? "OpenGL" : "DirectX"})</span>
              </div>
            </div>
            {n.paths.length > 1 && (
              <div className="flex flex-wrap gap-1 border-t border-edge px-3 py-2">
                {n.paths.filter((p) => /\.(png|tga|gif)$/i.test(p)).map((p) => (
                  <button key={p} type="button" className={`btn h-6 text-[11px] ${p === n.current ? "border-accent text-accent" : ""}`} onClick={() => n.set({ current: p })}>{baseName(p)}</button>
                ))}
              </div>
            )}
          </GlassCard>
        )}
        {n.result && <WriteResult report={n.result} title="Normal maps" />}
      </div>
    </div>
  );
}

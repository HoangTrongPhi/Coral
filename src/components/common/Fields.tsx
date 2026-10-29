import { FieldLabel, GlassCard, PathText, PillBadge, Segmented, TextField } from "@abyss/ui";
import { AlertTriangle, CheckCircle2, FileOutput, FolderOpen, Info, XCircle } from "lucide-react";
import type { ReactNode } from "react";
import { pickOne } from "../../lib/dialogs";
import { inTauri, ipc } from "../../lib/ipc";
import { fmtBytes } from "../../lib/jobs";
import type { FileWrite, Issue, OutputMode, WriteReport } from "../../types/coral";

/** A labelled number input; empty or invalid input keeps the last value. */
export function NumField({ label, value, onChange, min, max, step = 1, hint, suffix }: {
  label: string;
  value: number;
  onChange: (v: number) => void;
  min?: number;
  max?: number;
  step?: number;
  hint?: string;
  suffix?: string;
}) {
  return (
    <label className="flex flex-col" title={hint}>
      <FieldLabel>{label}</FieldLabel>
      <span className="flex items-center gap-1">
        <TextField
          type="number"
          className="num-input"
          value={Number.isFinite(value) ? value : 0}
          min={min}
          max={max}
          step={step}
          onChange={(e) => {
            const v = Number(e.target.value);
            if (e.target.value !== "" && Number.isFinite(v)) onChange(min != null || max != null ? Math.min(max ?? v, Math.max(min ?? v, v)) : v);
          }}
        />
        {suffix && <span className="text-[11px] text-ink-faint">{suffix}</span>}
      </span>
    </label>
  );
}

export function Row({ children, className = "" }: { children: ReactNode; className?: string }) {
  return <div className={`flex flex-wrap items-end gap-3 ${className}`}>{children}</div>;
}

const SEV_ICON = {
  error: <XCircle className="mt-0.5 size-3.5 shrink-0 text-rose" />,
  warn: <AlertTriangle className="mt-0.5 size-3.5 shrink-0 text-amber" />,
  info: <Info className="mt-0.5 size-3.5 shrink-0 text-accent" />,
};

export function IssueList({ issues, title = "Issues", max = 200 }: { issues: Issue[]; title?: string; max?: number }) {
  if (!issues.length) return null;
  const order = { error: 0, warn: 1, info: 2 };
  const sorted = [...issues].sort((a, b) => order[a.severity] - order[b.severity]);
  const errors = issues.filter((i) => i.severity === "error").length;
  return (
    <GlassCard title={`${title} (${issues.length})`} actions={errors ? <PillBadge tone="rose">{errors} error(s)</PillBadge> : undefined}>
      <ul className="max-h-64 divide-y divide-edge overflow-y-auto">
        {sorted.slice(0, max).map((i, n) => (
          <li key={n} className="flex items-start gap-2 px-4 py-1.5 text-[12px]">
            {SEV_ICON[i.severity]}
            <div className="min-w-0 flex-1">
              <div className="break-words text-ink select-text">{i.message}</div>
              {i.path && <PathText path={i.path} className="block text-ink-faint" />}
            </div>
            <code className="shrink-0 font-mono text-[10.5px] text-ink-faint">{i.code}</code>
          </li>
        ))}
      </ul>
    </GlassCard>
  );
}

const STATUS_TONE = { planned: "accent", written: "emerald", skipped: "neutral", failed: "rose" } as const;

export function FileList({ files }: { files: FileWrite[] }) {
  if (!files.length) return <div className="px-4 py-3 text-[12px] text-ink-faint">No files.</div>;
  return (
    <ul className="max-h-72 divide-y divide-edge overflow-y-auto">
      {files.map((f, i) => (
        <li key={i} className="flex items-center gap-2 px-4 py-1.5 text-[12px]">
          <PillBadge tone={STATUS_TONE[f.status]}>{f.status === "planned" ? "will write" : f.status}</PillBadge>
          <code className="w-20 shrink-0 font-mono text-[10.5px] text-ink-faint">{f.kind}</code>
          <button type="button" className="min-w-0 flex-1 text-left hover:text-accent disabled:hover:text-ink" disabled={!inTauri || f.status !== "written"} onClick={() => void ipc.revealInExplorer(f.path)} title={f.status === "written" ? "Show in Explorer" : undefined}>
            <PathText path={f.path} className="block text-ink" />
          </button>
          {f.bytes != null && <span className="shrink-0 text-ink-faint tabular-nums">{fmtBytes(f.bytes)}</span>}
          {f.note && <span className="shrink-0 text-ink-faint">{f.note}</span>}
        </li>
      ))}
    </ul>
  );
}

/** Result card of a batch tool (files + notes + issues). */
export function WriteResult({ report, title }: { report: WriteReport; title: string }) {
  const written = report.files.filter((f) => f.kind !== "backup").length;
  return (
    <div className="flex flex-col gap-3">
      <GlassCard
        title={title}
        actions={
          <PillBadge tone={report.ok ? (report.dryRun ? "accent" : "emerald") : "rose"}>
            {report.ok ? <CheckCircle2 className="size-3" /> : <XCircle className="size-3" />}
            {report.dryRun ? `Dry run: ${written} file(s)` : `${written} file(s) written`} · {report.elapsedMs} ms
          </PillBadge>
        }
      >
        {report.notes.length > 0 && (
          <ul className="max-h-32 overflow-y-auto border-b border-edge px-4 py-2 font-mono text-[11.5px] text-ink-muted select-text">
            {report.notes.map((n, i) => (
              <li key={i}>{n}</li>
            ))}
          </ul>
        )}
        <FileList files={report.files} />
      </GlassCard>
      <IssueList issues={report.issues} />
    </div>
  );
}

/** A path with a Choose… button. */
export function PathField({ label, value, onChange, kind = "folder", placeholder }: { label: string; value: string; onChange: (v: string) => void; kind?: "folder" | "file"; placeholder?: string }) {
  return (
    <div className="flex min-w-64 flex-1 flex-col">
      <FieldLabel>{label}</FieldLabel>
      <div className="flex gap-1.5">
        <TextField mono className="flex-1" value={value} placeholder={placeholder} onChange={(e) => onChange(e.target.value)} />
        <button
          type="button"
          className="btn"
          onClick={async () => {
            const p = await pickOne(kind);
            if (p) onChange(p);
          }}
        >
          <FolderOpen className="size-3.5" /> Choose…
        </button>
      </div>
    </div>
  );
}

/** Output folder, or over the inputs (with `.bak`). */
export function OutputField({ value, onChange, allowInPlace = true }: { value: OutputMode; onChange: (v: OutputMode) => void; allowInPlace?: boolean }) {
  return (
    <div className="flex flex-1 flex-wrap items-end gap-3">
      {allowInPlace && (
        <div>
          <FieldLabel>Output</FieldLabel>
          <Segmented
            value={value.kind}
            options={[
              { value: "folder", label: <span className="flex items-center gap-1"><FileOutput className="size-3" /> Folder</span> },
              { value: "inPlace", label: "In place (.bak)", hint: "Overwrite the inputs after copying each to <file>.bak" },
            ]}
            onChange={(k) => onChange(k === "folder" ? { kind: "folder", path: value.kind === "folder" ? value.path : "" } : { kind: "inPlace" })}
          />
        </div>
      )}
      {value.kind === "folder" && <PathField label="Output folder" value={value.path} onChange={(path) => onChange({ kind: "folder", path })} placeholder="Next to each input" />}
    </div>
  );
}

/** The inputs of a batch tool: dropped or chosen files / folders. */
export function InputList({ paths, onChange, accept = "images" }: { paths: string[]; onChange: (p: string[]) => void; accept?: string }) {
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex items-center gap-2">
        <FieldLabel>Inputs ({accept})</FieldLabel>
        <div className="flex-1" />
        <button type="button" className="btn h-6 text-[11.5px]" onClick={async () => { const p = await pickOne("folder"); if (p) onChange([...paths, p]); }}>
          <FolderOpen className="size-3" /> Add folder…
        </button>
        <button type="button" className="btn h-6 text-[11.5px]" onClick={async () => { const p = await pickOne("file", [{ name: "Images", extensions: ["png", "tga", "gif"] }]); if (p) onChange([...paths, p]); }}>
          Add file…
        </button>
        {paths.length > 0 && <button type="button" className="btn h-6 text-[11.5px]" onClick={() => onChange([])}>Clear</button>}
      </div>
      {paths.length === 0 ? (
        <div className="rounded-lg border border-dashed border-edge px-3 py-3 text-center text-[12px] text-ink-faint">Drop images or folders anywhere on the window</div>
      ) : (
        <ul className="flex max-h-28 flex-col gap-0.5 overflow-y-auto rounded-lg border border-edge bg-canvas/40 px-2 py-1">
          {paths.map((p, i) => (
            <li key={p} className="flex items-center gap-2 text-[12px]">
              <PathText path={p} className="min-w-0 flex-1 text-ink-muted" />
              <button type="button" className="text-ink-faint hover:text-rose" aria-label="Remove" onClick={() => onChange(paths.filter((_, j) => j !== i))}>×</button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

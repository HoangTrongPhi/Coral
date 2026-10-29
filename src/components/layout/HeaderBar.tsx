import { PillBadge } from "@abyss/ui";
import { Search } from "lucide-react";
import type { ReactNode } from "react";
import { moduleById } from "../../lib/modules";
import { useAppStore } from "../../stores/appStore";
import { useProjectStore } from "../../stores/projectStore";
import { useUpdateStore } from "../../stores/updateStore";

/** App name, module, an optional project / module slot (`children`), search, version, `via Abyss`, Update. */
export function HeaderBar({ children }: { children?: ReactNode }) {
  const activeTab = useAppStore((s) => s.activeTab);
  const setPaletteOpen = useAppStore((s) => s.setPaletteOpen);
  const info = useAppStore((s) => s.appInfo);
  const update = useUpdateStore();
  const mod = moduleById(activeTab);
  const project = useProjectStore((s) => s.project.name);
  const path = useProjectStore((s) => s.path);
  const dirty = useProjectStore((s) => s.dirty);

  return (
    <header className="flex h-12 shrink-0 items-center gap-3 border-b border-edge bg-panel/70 px-4">
      <div className="flex min-w-0 items-baseline gap-2">
        <h1 className="text-[14px] font-semibold whitespace-nowrap text-ink">Coral</h1>
        <span className="text-ink-faint">/</span>
        <span className="truncate text-[13px] text-ink-muted">{mod.label}</span>
        <span className="truncate text-[12px] text-ink-faint" title={path ?? "Not saved yet"}>
          · {project}
          {path ? "" : " (unsaved)"}
          {dirty ? " •" : ""}
        </span>
      </div>
      {children}
      <div className="flex-1" />
      <button
        type="button"
        onClick={() => setPaletteOpen(true)}
        className="flex h-8 w-56 items-center gap-2 rounded-lg border border-edge bg-canvas/70 px-3 text-left text-[12px] text-ink-faint transition-colors hover:border-edge-strong hover:text-ink-muted"
      >
        <Search className="size-3.5" />
        <span className="flex-1">Search actions…</span>
        <kbd>Ctrl</kbd>
        <kbd>K</kbd>
      </button>
      {update.phase === "available" && (
        <button type="button" className="btn btn-primary" onClick={() => void update.installAndRestart()}>
          Update to v{update.available?.version}
        </button>
      )}
      {(update.phase === "downloading" || update.phase === "installing") && <PillBadge tone="accent">Updating…</PillBadge>}
      <PillBadge tone={info ? "neutral" : "amber"}>{info ? `v${info.version}` : "browser preview"}</PillBadge>
      {info?.abyss.managed && <PillBadge tone="accent">via Abyss</PillBadge>}
    </header>
  );
}

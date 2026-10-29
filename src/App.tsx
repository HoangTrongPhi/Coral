import { CommandPalette, LogDrawer } from "@abyss/ui";
import { Loader2 } from "lucide-react";
import { lazy, Suspense, useMemo, type ComponentType } from "react";
import { PackTab } from "./components/tabs/PackTab";
import { AppAbout } from "./components/layout/AppAbout";
import { HeaderBar } from "./components/layout/HeaderBar";
import { NavigationRail } from "./components/layout/NavigationRail";
import { SettingsDialog } from "./components/layout/SettingsDialog";
import { useBackendBridge } from "./hooks/useBackendBridge";
import { useGlobalHotkeys } from "./hooks/useGlobalHotkeys";
import { useWindowDragDrop } from "./hooks/useWindowDragDrop";
import { buildCommands } from "./lib/commands";
import { errorText, uiLog, useAppStore } from "./stores/appStore";
import type { TabId } from "./types/coral";
// Drop handlers of every module, registered before any tab is opened.
import "./stores/projectStore";
import "./stores/toolStores";

// Pack opens first; the other modules load when first visited, keeping startup light.
const named = <K extends string>(load: () => Promise<Record<K, ComponentType>>, key: K) => lazy(() => load().then((m) => ({ default: m[key] })));
const TABS: Record<TabId, ComponentType> = {
  pack: PackTab,
  slice: named(() => import("./components/tabs/SliceTab"), "SliceTab"),
  animate: named(() => import("./components/tabs/AnimateTab"), "AnimateTab"),
  flipbook: named(() => import("./components/tabs/FlipbookTab"), "FlipbookTab"),
  pixel: named(() => import("./components/tabs/PixelTab"), "PixelTab"),
  normal: named(() => import("./components/tabs/NormalTab"), "NormalTab"),
};

export default function App() {
  useBackendBridge();
  useGlobalHotkeys();
  useWindowDragDrop();

  const activeTab = useAppStore((s) => s.activeTab);
  const logs = useAppStore((s) => s.logs);
  const logOpen = useAppStore((s) => s.logDrawerOpen);
  const paletteOpen = useAppStore((s) => s.paletteOpen);
  const { toggleLogDrawer, clearLogs, setPaletteOpen } = useAppStore.getState();
  const commands = useMemo(() => (paletteOpen ? buildCommands() : []), [paletteOpen]);
  const ActiveTab = TABS[activeTab];

  return (
    <div className="flex h-full">
      <NavigationRail />
      <div className="flex min-w-0 flex-1 flex-col">
        <HeaderBar />
        <main className="min-h-0 flex-1 overflow-y-auto">
          <Suspense
            fallback={
              <div className="grid h-full place-items-center text-ink-faint">
                <Loader2 className="size-5 animate-spin" />
              </div>
            }
          >
            <ActiveTab />
          </Suspense>
        </main>
        <LogDrawer entries={logs} open={logOpen} onToggle={() => toggleLogDrawer()} onClear={clearLogs} />
      </div>
      <CommandPalette open={paletteOpen} onClose={() => setPaletteOpen(false)} items={commands} onError={(item, err) => uiLog("error", `${item.label} failed: ${errorText(err)}`)} />
      <AppAbout />
      <SettingsDialog />
    </div>
  );
}

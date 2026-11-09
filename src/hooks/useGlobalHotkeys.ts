import { useEffect } from "react";
import { MODULES } from "../lib/modules";
import { runActiveModule } from "../lib/rerun";
import { useAppStore } from "../stores/appStore";
import { useProjectStore } from "../stores/projectStore";

/** F1 help, Ctrl+K palette, Ctrl+` log drawer, Ctrl+, settings, Ctrl+1…9 module switch, F5 run again,
 * Ctrl+O / Ctrl+S / Ctrl+Shift+S project. */
export function useGlobalHotkeys() {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const s = useAppStore.getState();
      if (e.key === "F1" && !e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey) {
        // Opens on the current module's topic (AppHelp); F1 again closes.
        e.preventDefault();
        s.setPaletteOpen(false);
        s.setHelpOpen(!s.helpOpen);
        return;
      }
      if (e.key === "F5" && !e.ctrlKey && !e.altKey) {
        // Also stops the webview from reloading the page.
        e.preventDefault();
        runActiveModule();
        return;
      }
      if (!(e.ctrlKey || e.metaKey) || e.altKey) return;
      const key = e.key.toLowerCase();
      const p = useProjectStore.getState();
      if (key === "o") {
        e.preventDefault();
        void p.open();
      } else if (key === "s") {
        e.preventDefault();
        void (e.shiftKey ? p.saveAs() : p.save());
      } else if (key === "k") {
        e.preventDefault();
        s.setPaletteOpen(!s.paletteOpen);
      } else if (key === ",") {
        e.preventDefault();
        s.setSettingsOpen(!s.settingsOpen);
      } else if (e.code === "Backquote") {
        e.preventDefault();
        s.toggleLogDrawer();
      } else {
        const mod = MODULES.find((m) => m.hotkey === e.key);
        if (mod) {
          e.preventDefault();
          s.setPaletteOpen(false);
          s.setActiveTab(mod.id);
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}

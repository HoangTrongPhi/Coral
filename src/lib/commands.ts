import type { PaletteItem } from "@abyss/ui";
import { createElement } from "react";
import { FileJson, FilePlus, FolderOpen, History, Package, PanelBottom, RefreshCw, Save, Settings2, Trash2, UserRound, type LucideIcon } from "lucide-react";
import { useAppStore } from "../stores/appStore";
import { useProjectStore } from "../stores/projectStore";
import { useUpdateStore } from "../stores/updateStore";
import { pickPaths } from "./dialogs";
import { MODULES } from "./modules";
import { runActiveModule } from "./rerun";

const icon = (I: LucideIcon) => createElement(I, { strokeWidth: 1.75 });

/** Everything reachable from the command palette (Ctrl+K): modules, actions, recent projects. */
export function buildCommands(): PaletteItem[] {
  const s = useAppStore.getState();
  const p = useProjectStore.getState();
  return [
    ...MODULES.map<PaletteItem>((m) => ({
      id: `nav.${m.id}`,
      label: `Go to ${m.label}`,
      group: "Navigate",
      icon: icon(m.icon),
      hint: `Ctrl+${m.hotkey}`,
      keywords: m.description,
      run: () => s.setActiveTab(m.id),
    })),
    { id: "proj.new", label: "New project", group: "Project", icon: icon(FilePlus), keywords: "clear empty", run: () => p.newProject() },
    { id: "proj.open", label: "Open project…", group: "Project", icon: icon(FileJson), hint: "Ctrl+O", keywords: "coral.json load", run: () => p.open() },
    { id: "proj.save", label: "Save project", group: "Project", icon: icon(Save), hint: "Ctrl+S", run: () => p.save() },
    { id: "proj.saveAs", label: "Save project as…", group: "Project", icon: icon(Save), hint: "Ctrl+Shift+S", run: () => p.saveAs() },
    { id: "proj.export", label: "Pack & export…", group: "Project", icon: icon(Package), keywords: "write atlas godot unity spine json", run: () => p.planExport() },
    { id: "act.addFolder", label: "Add a folder…", group: "Actions", icon: icon(FolderOpen), keywords: "browse directory source", run: () => pickPaths("folder") },
    { id: "act.rerun", label: "Run the current module again", group: "Actions", icon: icon(RefreshCw), hint: "F5", keywords: "refresh repack", run: runActiveModule },
    ...p.recent.map<PaletteItem>((path, i) => ({ id: `recent.${i}`, label: path.split(/[\\/]/).pop() ?? path, hint: path, group: "Recent projects", icon: icon(History), keywords: path, run: () => p.open(path) })),
    { id: "app.checkUpdates", label: "Check for updates…", group: "Actions", icon: icon(RefreshCw), keywords: "update upgrade version", run: () => useUpdateStore.getState().check() },
    { id: "view.toggleLog", label: "Toggle log drawer", group: "View", icon: icon(PanelBottom), hint: "Ctrl+`", keywords: "console terminal output", run: () => s.toggleLogDrawer() },
    { id: "view.settings", label: "Settings…", group: "View", icon: icon(Settings2), hint: "Ctrl+,", keywords: "theme dark light font size appearance preferences", run: () => s.setSettingsOpen(true) },
    { id: "view.about", label: "About & contact", group: "View", icon: icon(UserRound), keywords: "author contact version help", run: () => s.setAboutOpen(true) },
    { id: "view.clearLog", label: "Clear log", group: "View", icon: icon(Trash2), keywords: "console reset", run: () => s.clearLogs() },
  ];
}

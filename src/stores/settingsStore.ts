import type { HelpLang } from "@abyss/ui";
import { create } from "zustand";
import { persist } from "zustand/middleware";

/**
 * Coral's own preferences, kept in the webview's localStorage like the appearance (`coral.appearance`)
 * and the module stores (`coral.flipbook`, …). Appearance itself lives in `stores/appearance.ts`.
 */
interface SettingsState {
  /** Language of the Help panel (F1); Vietnamese by default. */
  helpLang: HelpLang;
  setHelpLang: (lang: HelpLang) => void;
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      helpLang: "vi",
      setHelpLang: (helpLang) => set({ helpLang }),
    }),
    { name: "coral.settings", version: 1 },
  ),
);

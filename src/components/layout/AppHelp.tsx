import { HelpPanel } from "@abyss/ui";
import { useCallback } from "react";
import { HELP_TOPICS } from "../../help/topics";
import { openUserGuide } from "../../lib/userGuide";
import { useAppStore } from "../../stores/appStore";
import { useSettingsStore } from "../../stores/settingsStore";

/** The shared Help panel, wired to Coral: F1 / header "?" / command palette, opens on the current module. */
export function AppHelp() {
  const open = useAppStore((s) => s.helpOpen);
  const setOpen = useAppStore((s) => s.setHelpOpen);
  const activeTab = useAppStore((s) => s.activeTab);
  const lang = useSettingsStore((s) => s.helpLang);
  const setLang = useSettingsStore((s) => s.setHelpLang);
  const close = useCallback(() => setOpen(false), [setOpen]);

  return <HelpPanel open={open} onClose={close} topics={HELP_TOPICS} initialTopicId={activeTab} lang={lang} onLangChange={setLang} onOpenGuide={openUserGuide} appName="Coral" />;
}

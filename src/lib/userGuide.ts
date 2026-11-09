import type { HelpLang } from "@abyss/ui";
import { errorText, uiLog } from "../stores/appStore";
import { useSettingsStore } from "../stores/settingsStore";
import { ipc } from "./ipc";

/** Open the user guide PDF bundled with the app (`docs/user-guide`, built from the Help content by `npm run guide`). */
export function openUserGuide(lang: HelpLang = useSettingsStore.getState().helpLang) {
  ipc.openUserGuide(lang).catch((e) => uiLog("error", `User guide: ${errorText(e)}`));
}

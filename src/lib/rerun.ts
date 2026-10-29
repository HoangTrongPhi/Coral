import { useAppStore } from "../stores/appStore";
import { useProjectStore } from "../stores/projectStore";
import { useFlipbookStore, useNormalStore, useSliceStore } from "../stores/toolStores";
import type { TabId } from "../types/coral";

/** F5 / "Run again": what each module re-runs (never a write without confirmation). */
const RERUN: Record<TabId, () => unknown> = {
  pack: () => useProjectStore.getState().refresh(),
  animate: () => useProjectStore.getState().refresh(),
  slice: () => useSliceStore.getState().detect(),
  flipbook: () => useFlipbookStore.getState().run(true),
  pixel: () => {},
  normal: () => useNormalStore.getState().preview(),
};

export function runActiveModule() {
  void RERUN[useAppStore.getState().activeTab]?.();
}

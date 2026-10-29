import { Boxes, Film, Grid3x3, Mountain, Palette, Scissors, type LucideIcon } from "lucide-react";
import type { TabId } from "../types/coral";

export interface ModuleDef {
  id: TabId;
  label: string;
  description: string;
  icon: LucideIcon;
  /** Ctrl+<digit> shortcut. */
  hotkey: string;
  /** What a drop on this module loads (dropRouter / useWindowDragDrop). */
  accepts: "folder" | "files" | "any";
}

/** Coral's modules, in navigation order (spec §4.3). */
export const MODULES: ModuleDef[] = [
  { id: "pack", label: "Pack", description: "Sprites → atlas, exported for Godot, Unity, Spine, Phaser…", icon: Boxes, hotkey: "1", accepts: "any" },
  { id: "slice", label: "Slice", description: "Cut an existing sheet by grid or automatically", icon: Scissors, hotkey: "2", accepts: "files" },
  { id: "animate", label: "Animate", description: "Play animations, set pivots, draw hitboxes", icon: Film, hotkey: "3", accepts: "any" },
  { id: "flipbook", label: "Flipbook", description: "Frame sequence → SubUV sheet for VFX", icon: Grid3x3, hotkey: "4", accepts: "folder" },
  { id: "pixel", label: "Pixel", description: "Upscale, palettes, remap and palette swaps", icon: Palette, hotkey: "5", accepts: "any" },
  { id: "normal", label: "Normal 2D", description: "Normal maps for 2D lighting", icon: Mountain, hotkey: "6", accepts: "any" },
];

export const moduleById = (id: TabId) => MODULES.find((m) => m.id === id) ?? MODULES[0];

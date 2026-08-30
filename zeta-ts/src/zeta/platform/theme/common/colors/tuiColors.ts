import { registerColor } from "../colorRegistry.js";
import { accentForeground } from "./baseColors.js";

const owner = "tui.presentation";
const color = (id: string, dark: string, light: string, description: string): string => registerColor(id, { dark, light }, { description, owner });
const alias = (id: string, value: string, description: string): string => registerColor(id, { dark: value, light: value }, { description, owner });

export const tuiHighlightForeground = alias("tui.highlightForeground", accentForeground, "Foreground for selected items, borders, and other TUI highlights.");
export const tuiActiveSelectionForeground = color("tui.activeSelectionForeground", "#000000", "#000000", "Foreground for the active TUI list item.");
export const tuiActiveSelectionBackground = color("tui.activeSelectionBackground", "#c0c0c0", "#c0c0c0", "Background for the active TUI list item.");

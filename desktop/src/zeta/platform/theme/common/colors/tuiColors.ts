import { registerColor } from "../colorRegistry.js";
import { accentForeground } from "./baseColors.js";

const owner = "tui.presentation";
const alias = (id: string, value: string, description: string): string => registerColor(id, { dark: value, light: value }, { description, owner });

export const tuiHighlightForeground = alias("tui.highlightForeground", accentForeground, "Foreground for selected items, borders, and other TUI highlights.");

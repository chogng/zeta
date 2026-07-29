import { registerColor } from "../colorRegistry.js";
import { descriptionForeground, foreground } from "./baseColors.js";

const owner = "workbench.shell";
const color = (id: string, dark: string, light: string, description: string): string => registerColor(id, { dark, light }, { description, owner });
const alias = (id: string, value: string, description: string): string => registerColor(id, { dark: value, light: value }, { description, owner });

export const sectionHeaderForeground = alias("sectionHeader.foreground", descriptionForeground, "Section header foreground.");
export const workbenchBackground = color("workbench.background", "#1e1e1e", "#ffffff", "Workbench root background.");
export const editorBackground = alias("editor.background", workbenchBackground, "Editor background.");
export const editorForeground = color("editor.foreground", "#d4d4d4", "#333333", "Editor foreground.");
export const titleBarBackground = color("titleBar.background", "#181818", "#f3f3f3", "Title bar background.");
export const titleBarForeground = color("titleBar.foreground", "#f0f0f0", "#1f1f1f", "Title bar foreground.");
export const titleBarActionForeground = color("titleBar.actionForeground", "#d6d6d6", "#424242", "Title bar action foreground.");
export const titleBarHoverBackground = color("titleBar.hoverBackground", "#2a2d2e", "#e5e5e5", "Hovered title bar item background.");
export const sideBarBackground = color("sideBar.background", "#181818", "#f8f8f8", "Primary side bar background.");
export const auxiliaryBarBackground = alias("auxiliaryBar.background", sideBarBackground, "Auxiliary side bar background.");
export const panelBackground = alias("panel.background", sideBarBackground, "Panel background.");
export const compositeBarForeground = color("compositeBar.foreground", "#ffffff", "#1f1f1f", "Active composite bar foreground.");
export const compositeBarInactiveForeground = color("compositeBar.inactiveForeground", "#858585", "#616161", "Inactive composite bar foreground.");
export const statusBarForeground = color("statusBar.foreground", "#ffffff", "#ffffff", "Status bar foreground.");
export const statusBarBackground = color("statusBar.background", "#007acc", "#007acc", "Status bar background.");
export const sashHoverBackground = alias("sash.hoverBackground", statusBarBackground, "Hovered sash background.");

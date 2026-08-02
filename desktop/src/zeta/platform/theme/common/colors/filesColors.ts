import { registerColor } from "../colorRegistry.js";

const owner = "files.presentation";
const color = (id: string, dark: string, light: string, description: string): string => registerColor(id, { dark, light }, { description, owner });
const alias = (id: string, value: string, description: string): string => registerColor(id, { dark: value, light: value }, { description, owner });

export const emptyExplorerOpenFolderBackground = color("files.emptyExplorerOpenFolderBackground", "#328eb9", "#328eb9", "Background for the Empty Explorer Open Folder action.");
export const emptyExplorerOpenFolderHoverBackground = alias("files.emptyExplorerOpenFolderHoverBackground", emptyExplorerOpenFolderBackground, "Hovered background for the Empty Explorer Open Folder action.");

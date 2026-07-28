import { ColorScheme } from "./theme.js";

/** Stable identifiers consumed by workbench and base UI styles. */
export const ColorId = {
  foreground: "foreground",
  descriptionForeground: "description.foreground",
  mutedForeground: "muted.foreground",
  sectionHeaderForeground: "sectionHeader.foreground",
  workbenchBackground: "workbench.background",
  editorBackground: "editor.background",
  titleBarBackground: "titleBar.background",
  titleBarForeground: "titleBar.foreground",
  titleBarActionForeground: "titleBar.actionForeground",
  sideBarBackground: "sideBar.background",
  auxiliaryBarBackground: "auxiliaryBar.background",
  panelBackground: "panel.background",
  border: "border",
  selectionForeground: "selection.foreground",
  selectionBackground: "selection.background",
  buttonForeground: "button.foreground",
  buttonHoverBackground: "button.hoverBackground",
  buttonActiveBackground: "button.activeBackground",
  focusBorder: "focusBorder",
  primaryButtonForeground: "primaryButton.foreground",
  primaryButtonBackground: "primaryButton.background",
  primaryButtonHoverBackground: "primaryButton.hoverBackground",
  widgetShadow: "widget.shadow",
  statusBarForeground: "statusBar.foreground",
  statusBarBackground: "statusBar.background",
  sashHoverBackground: "sash.hoverBackground",
} as const;

export type ColorIdentifier = typeof ColorId[keyof typeof ColorId];

export const colorIdentifiers: readonly ColorIdentifier[] =
  Object.freeze(Object.values(ColorId));

export type ThemeColors = Readonly<Record<ColorIdentifier, string>>;

/** Immutable colors and metadata selected for one workbench window. */
export interface IColorTheme {
  readonly id: string;
  readonly label: string;
  readonly colorScheme: ColorScheme;
  readonly colors: ThemeColors;
}

export interface IColorThemeOptions {
  readonly id: string;
  readonly label: string;
  readonly colorScheme: ColorScheme;
  readonly colors: ThemeColors;
}

/** Creates a color theme whose metadata and color table cannot be mutated. */
export function createColorTheme(
  options: IColorThemeOptions,
): IColorTheme {
  if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(options.id)) {
    throw new TypeError(`Invalid color theme ID '${options.id}'`);
  }
  return Object.freeze({
    id: options.id,
    label: options.label,
    colorScheme: options.colorScheme,
    colors: Object.freeze({ ...options.colors }),
  });
}

/** Returns the CSS custom property corresponding to a color identifier. */
export function colorCssVariable(id: ColorIdentifier): string {
  const kebabId = id
    .replaceAll(".", "-")
    .replace(/[A-Z]/g, (character) => `-${character.toLowerCase()}`);
  return `--zeta-${kebabId}`;
}

export const darkColorTheme = createColorTheme({
  id: "zeta-dark",
  label: "Zeta Dark",
  colorScheme: ColorScheme.Dark,
  colors: {
    [ColorId.foreground]: "#cccccc",
    [ColorId.descriptionForeground]: "#b8b8b8",
    [ColorId.mutedForeground]: "#8f8f8f",
    [ColorId.sectionHeaderForeground]: "#bbbbbb",
    [ColorId.workbenchBackground]: "#1e1e1e",
    [ColorId.editorBackground]: "#1e1e1e",
    [ColorId.titleBarBackground]: "#181818",
    [ColorId.titleBarForeground]: "#f0f0f0",
    [ColorId.titleBarActionForeground]: "#d6d6d6",
    [ColorId.sideBarBackground]: "#181818",
    [ColorId.auxiliaryBarBackground]: "#181818",
    [ColorId.panelBackground]: "#181818",
    [ColorId.border]: "#2b2b2b",
    [ColorId.selectionForeground]: "#ffffff",
    [ColorId.selectionBackground]: "#264f78",
    [ColorId.buttonForeground]: "#cccccc",
    [ColorId.buttonHoverBackground]: "#2a2d2e",
    [ColorId.buttonActiveBackground]: "#37373d",
    [ColorId.focusBorder]: "#007fd4",
    [ColorId.primaryButtonForeground]: "#ffffff",
    [ColorId.primaryButtonBackground]: "#0078d4",
    [ColorId.primaryButtonHoverBackground]: "#0086ed",
    [ColorId.widgetShadow]: "#00000033",
    [ColorId.statusBarForeground]: "#ffffff",
    [ColorId.statusBarBackground]: "#007acc",
    [ColorId.sashHoverBackground]: "#007acc",
  },
});

export const lightColorTheme = createColorTheme({
  id: "zeta-light",
  label: "Zeta Light",
  colorScheme: ColorScheme.Light,
  colors: {
    [ColorId.foreground]: "#3b3b3b",
    [ColorId.descriptionForeground]: "#616161",
    [ColorId.mutedForeground]: "#767676",
    [ColorId.sectionHeaderForeground]: "#616161",
    [ColorId.workbenchBackground]: "#ffffff",
    [ColorId.editorBackground]: "#ffffff",
    [ColorId.titleBarBackground]: "#f3f3f3",
    [ColorId.titleBarForeground]: "#1f1f1f",
    [ColorId.titleBarActionForeground]: "#424242",
    [ColorId.sideBarBackground]: "#f8f8f8",
    [ColorId.auxiliaryBarBackground]: "#f8f8f8",
    [ColorId.panelBackground]: "#f8f8f8",
    [ColorId.border]: "#e5e5e5",
    [ColorId.selectionForeground]: "#000000",
    [ColorId.selectionBackground]: "#add6ff",
    [ColorId.buttonForeground]: "#3b3b3b",
    [ColorId.buttonHoverBackground]: "#e8e8e8",
    [ColorId.buttonActiveBackground]: "#dcdcdc",
    [ColorId.focusBorder]: "#0078d4",
    [ColorId.primaryButtonForeground]: "#ffffff",
    [ColorId.primaryButtonBackground]: "#0078d4",
    [ColorId.primaryButtonHoverBackground]: "#006cbe",
    [ColorId.widgetShadow]: "#00000022",
    [ColorId.statusBarForeground]: "#ffffff",
    [ColorId.statusBarBackground]: "#007acc",
    [ColorId.sashHoverBackground]: "#007acc",
  },
});

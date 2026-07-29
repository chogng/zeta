import type { Color } from "../../../base/common/color.js";
import { Colors, colorCssVariable, type ColorIdentifier, type ColorValue, type ResolvedColorContribution } from "./colorRegistry.js";
import * as baseColors from "./colors/baseColors.js";
import * as componentColors from "./colors/componentColors.js";
import * as workbenchColors from "./colors/workbenchColors.js";
import { Sizes, sizeCssVariable, sizeToCss, type SizeContribution, type SizeValue } from "./sizeRegistry.js";
import "./sizes/baseSizes.js";
import { ColorScheme } from "./theme.js";

/** Named compatibility facade. New token domains should export their registered identifiers directly. */
export const ColorId = Object.freeze({
  foreground: baseColors.foreground,
  descriptionForeground: baseColors.descriptionForeground,
  mutedForeground: baseColors.mutedForeground,
  accentForeground: baseColors.accentForeground,
  errorForeground: baseColors.errorForeground,
  warningForeground: baseColors.warningForeground,
  focusBorder: baseColors.focusBorder,
  border: baseColors.border,
  widgetBorder: baseColors.widgetBorder,
  widgetShadow: baseColors.widgetShadow,
  inputForeground: componentColors.inputForeground,
  inputBackground: componentColors.inputBackground,
  inputBorder: componentColors.inputBorder,
  inputPlaceholderForeground: componentColors.inputPlaceholderForeground,
  selectionForeground: componentColors.selectionForeground,
  selectionBackground: componentColors.selectionBackground,
  listHoverBackground: componentColors.listHoverBackground,
  listActiveSelectionForeground: componentColors.listActiveSelectionForeground,
  listActiveSelectionBackground: componentColors.listActiveSelectionBackground,
  menuSelectionForeground: componentColors.menuSelectionForeground,
  menuSelectionBackground: componentColors.menuSelectionBackground,
  buttonForeground: componentColors.buttonForeground,
  buttonBackground: componentColors.buttonBackground,
  buttonHoverBackground: componentColors.buttonHoverBackground,
  buttonActiveBackground: componentColors.buttonActiveBackground,
  buttonSecondaryBackground: componentColors.buttonSecondaryBackground,
  primaryButtonForeground: componentColors.primaryButtonForeground,
  primaryButtonBackground: componentColors.primaryButtonBackground,
  primaryButtonHoverBackground: componentColors.primaryButtonHoverBackground,
  toolbarHoverBackground: componentColors.toolbarHoverBackground,
  keybindingLabelForeground: componentColors.keybindingLabelForeground,
  keybindingLabelBackground: componentColors.keybindingLabelBackground,
  keybindingLabelBorder: componentColors.keybindingLabelBorder,
  keybindingLabelBottomBorder: componentColors.keybindingLabelBottomBorder,
  scrollbarSliderBackground: componentColors.scrollbarSliderBackground,
  scrollbarSliderHoverBackground: componentColors.scrollbarSliderHoverBackground,
  scrollbarSliderActiveBackground: componentColors.scrollbarSliderActiveBackground,
  dialogBackground: componentColors.dialogBackground,
  dialogBorder: componentColors.dialogBorder,
  dialogBackdropBackground: componentColors.dialogBackdropBackground,
  dialogShadow: componentColors.dialogShadow,
  quickInputBackground: componentColors.quickInputBackground,
  quickInputBackdropBackground: componentColors.quickInputBackdropBackground,
  textCodeBlockBackground: componentColors.textCodeBlockBackground,
  searchMatchBackground: componentColors.searchMatchBackground,
  sectionHeaderForeground: workbenchColors.sectionHeaderForeground,
  workbenchBackground: workbenchColors.workbenchBackground,
  editorBackground: workbenchColors.editorBackground,
  editorForeground: workbenchColors.editorForeground,
  titleBarBackground: workbenchColors.titleBarBackground,
  titleBarForeground: workbenchColors.titleBarForeground,
  titleBarActionForeground: workbenchColors.titleBarActionForeground,
  titleBarHoverBackground: workbenchColors.titleBarHoverBackground,
  sideBarBackground: workbenchColors.sideBarBackground,
  auxiliaryBarBackground: workbenchColors.auxiliaryBarBackground,
  panelBackground: workbenchColors.panelBackground,
  compositeBarForeground: workbenchColors.compositeBarForeground,
  compositeBarInactiveForeground: workbenchColors.compositeBarInactiveForeground,
  statusBarForeground: workbenchColors.statusBarForeground,
  statusBarBackground: workbenchColors.statusBarBackground,
  sashHoverBackground: workbenchColors.sashHoverBackground,
});

export { colorCssVariable, sizeCssVariable };
export type { ColorIdentifier };

export const colorIdentifiers: readonly ColorIdentifier[] = Object.freeze(Colors.getColors().map(({ id }) => id));
export const sizeIdentifiers: readonly string[] = Object.freeze(Sizes.getSizes().map(({ id }) => id));
export type ThemeColors = Readonly<Record<ColorIdentifier, string>>;

/** Immutable, fully resolved theme snapshot selected for one workbench window. */
export interface IColorTheme {
  readonly id: string;
  readonly label: string;
  readonly colorScheme: ColorScheme;
  readonly colors: ThemeColors;
  readonly colorEntries: readonly ResolvedColorContribution[];
  readonly sizeEntries: readonly SizeContribution[];
  getColor(id: ColorIdentifier): Color | undefined;
  getColorCss(id: ColorIdentifier): string | undefined;
  getSize(id: string): SizeValue | undefined;
}

export interface IColorThemeOptions {
  readonly id: string;
  readonly label: string;
  readonly colorScheme: ColorScheme;
  readonly colorOverrides?: Readonly<Record<string, ColorValue>>;
}

/** Compiles registry contributions and overrides into an immutable snapshot. */
export function createColorTheme(options: IColorThemeOptions): IColorTheme {
  if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(options.id)) throw new TypeError(`Invalid color theme ID '${options.id}'`);
  Colors.seal();
  Sizes.seal();
  const colorEntries = Colors.resolve(options.colorScheme, options.colorOverrides);
  const colorMap = new Map(colorEntries.map(({ id, value }) => [id, value] as const));
  const colors = Object.freeze(Object.fromEntries(colorEntries.filter(({ value }) => value !== null).map(({ id, value }) => [id, value!.toString()])));
  const sizeEntries = Object.freeze(Sizes.getSizes().map((entry) => Object.freeze({ ...entry, value: Object.freeze({ ...entry.value }) })));
  const sizeMap = new Map(sizeEntries.map(({ id, value }) => [id, value] as const));
  return Object.freeze({
    id: options.id,
    label: options.label,
    colorScheme: options.colorScheme,
    colors,
    colorEntries,
    sizeEntries,
    getColor: (id: ColorIdentifier) => colorMap.get(id) ?? undefined,
    getColorCss: (id: ColorIdentifier) => colorMap.get(id)?.toString(),
    getSize: (id: string) => sizeMap.get(id),
  });
}

export const darkColorTheme = createColorTheme({ id: "zeta-dark", label: "Zeta Dark", colorScheme: ColorScheme.Dark });
export const lightColorTheme = createColorTheme({ id: "zeta-light", label: "Zeta Light", colorScheme: ColorScheme.Light });

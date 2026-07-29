import type { ITheme } from "@xterm/xterm";
import { ColorId, type IColorTheme } from "../../../../../platform/theme/common/colorTheme.js";

/** Projects the workbench theme into xterm's renderer theme contract. */
export function terminalTheme(theme: IColorTheme): ITheme {
  return {
    background: theme.getColorCss(ColorId.editorBackground),
    foreground: theme.getColorCss(ColorId.editorForeground),
    cursor: theme.getColorCss(ColorId.foreground),
    selectionForeground: theme.getColorCss(ColorId.selectionForeground),
    selectionBackground: theme.getColorCss(ColorId.selectionBackground),
  };
}

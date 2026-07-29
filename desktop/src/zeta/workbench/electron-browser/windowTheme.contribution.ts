import { ColorId, type IColorTheme } from "../../platform/theme/common/colorTheme.js";
import { IThemeService } from "../../platform/theme/common/themeService.js";
import { registerWorkbenchContribution, WorkbenchPhase } from "../common/contributions.js";
import { INativeHostService } from "../common/services.js";

registerWorkbenchContribution("workbench.contrib.nativeWindowTheme", WorkbenchPhase.BlockStartup, (accessor) => {
  const nativeHost = accessor.get(INativeHostService);
  const themeService = accessor.get(IThemeService);
  const apply = (theme: IColorTheme): void => {
    void nativeHost.setWindowTheme({
      backgroundColor: requiredColor(theme, ColorId.titleBarBackground),
      symbolColor: requiredColor(theme, ColorId.titleBarActionForeground),
    }).catch((error: unknown) => {
      console.error("Failed to apply native window theme", error);
    });
  };
  apply(themeService.getColorTheme());
  return themeService.onDidColorThemeChange(apply);
});

function requiredColor(theme: IColorTheme, id: string): string {
  const color = theme.getColorCss(id);
  if (!color) throw new Error(`Theme '${theme.id}' does not define native window color '${id}'`);
  return color;
}

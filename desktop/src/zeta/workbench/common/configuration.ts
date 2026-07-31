import { ConfigurationsRegistry } from "../../platform/configuration/common/configurationRegistry.js";
import "../../platform/theme/common/themeConfiguration.js";
import { defaultWorkbenchColorThemePreference, SystemColorThemePreference, WorkbenchThemesRegistry } from "./theme.js";

/** Typed configuration keys owned by the workbench layer. */
export const WorkbenchConfiguration = Object.freeze({
  colorTheme: ConfigurationsRegistry.registerConfiguration<string>({
    key: "workbench.colorTheme",
    defaultValue: defaultWorkbenchColorThemePreference,
    parse(value: unknown): string {
      if (
        typeof value !== "string" ||
        (
          value !== SystemColorThemePreference &&
          !WorkbenchThemesRegistry.getColorTheme(value)
        )
      ) {
        throw new TypeError(
          `Unknown workbench color theme preference: ${String(value)}`,
        );
      }
      return value;
    },
  }),
});

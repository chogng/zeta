import { ConfigurationsRegistry } from "../../platform/configuration/common/configurationRegistry.js";
import { AccessibilityConfiguration } from "../../platform/accessibility/common/accessibility.js";
import "../../platform/theme/common/themeConfiguration.js";
import { defaultWorkbenchColorThemePreference, SystemColorThemePreference, WorkbenchThemesRegistry } from "./theme.js";

/** Typed configuration keys owned by the workbench layer. */
export const WorkbenchConfiguration = Object.freeze({
  ...AccessibilityConfiguration,
  colorTheme: ConfigurationsRegistry.registerConfiguration<string>({
    key: "workbench.colorTheme",
    defaultValue: defaultWorkbenchColorThemePreference,
    parse(value: unknown): string {
      if (typeof value !== "string" || !isColorThemePreference(value)) throw new TypeError(`Unknown workbench color theme preference: ${String(value)}`);
      return value;
    },
  }),
});

function isColorThemePreference(value: string): boolean {
  return value === SystemColorThemePreference || WorkbenchThemesRegistry.getColorTheme(value) !== undefined || /^extension-[a-z0-9]+(?:-[a-z0-9]+)*$/u.test(value);
}

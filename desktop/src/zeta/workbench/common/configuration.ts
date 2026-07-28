import {
  ConfigurationsRegistry,
} from "../../platform/configuration/common/configurationRegistry.js";
import {
  defaultWorkbenchColorTheme,
  WorkbenchThemesRegistry,
} from "./theme.js";

/** Typed configuration keys owned by the workbench layer. */
export const WorkbenchConfiguration = Object.freeze({
  colorTheme: ConfigurationsRegistry.registerConfiguration<string>({
    key: "workbench.colorTheme",
    defaultValue: defaultWorkbenchColorTheme.id,
    parse(value: unknown): string {
      if (
        typeof value !== "string" ||
        !WorkbenchThemesRegistry.getColorTheme(value)
      ) {
        throw new TypeError(`Unknown workbench color theme: ${String(value)}`);
      }
      return value;
    },
  }),
});

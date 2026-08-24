import { ConfigurationsRegistry } from "../../platform/configuration/common/configurationRegistry.js";
import { AccessibilityConfiguration } from "../../platform/accessibility/common/accessibility.js";
import { WorkbenchModeConfigurationKey, WorkbenchModeRegistry } from "../../product/common/workbenchMode.js";
import "../../platform/theme/common/themeConfiguration.js";
import { defaultWorkbenchColorThemePreference, SystemColorThemePreference, WorkbenchThemesRegistry } from "./theme.js";

/** Typed configuration keys owned by the workbench layer. */
export const WorkbenchConfiguration = Object.freeze({
	...AccessibilityConfiguration,
	mode: ConfigurationsRegistry.registerConfiguration({
		key: WorkbenchModeConfigurationKey,
		defaultValue: WorkbenchModeRegistry.defaultModeId,
		parse(value: unknown) {
			if (typeof value !== "string") throw new TypeError(`Unknown Workbench mode: ${String(value)}`);
			return WorkbenchModeRegistry.resolveModeId(value);
		},
	}),
	colorTheme: ConfigurationsRegistry.registerConfiguration<string>({
		key: "workbench.colorTheme",
		defaultValue: defaultWorkbenchColorThemePreference,
		parse(value: unknown): string {
			if (typeof value !== "string" || !isColorThemePreference(value)) throw new TypeError(`Unknown workbench color theme preference: ${String(value)}`);
			return value;
		},
		setting: {
			valueType: "select",
			title: "Color theme",
			description: "Choose a built-in theme or follow the operating-system appearance.",
			get options() {
				return [
					{ value: SystemColorThemePreference, label: "System" },
					...WorkbenchThemesRegistry.getColorThemes().map(theme => ({ value: theme.id, label: theme.label })),
				];
			},
		},
	}),
});

function isColorThemePreference(value: string): boolean {
	return value === SystemColorThemePreference || WorkbenchThemesRegistry.getColorTheme(value) !== undefined || /^extension-[a-z0-9]+(?:-[a-z0-9]+)*$/u.test(value);
}

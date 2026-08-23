import { ConfigurationsRegistry } from "../../configuration/common/configurationRegistry.js";

/** Device-local theme selection consumed by terminal presentation surfaces. */
export const TerminalThemeConfiguration = Object.freeze({
	colorTheme: ConfigurationsRegistry.registerConfiguration<string>({
		key: "tui.colorTheme",
		defaultValue: "system",
		parse(value: unknown): string {
			if (typeof value !== "string" || (value !== "system" && !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(value))) throw new TypeError(`Invalid TUI color theme preference: ${String(value)}`);
			return value;
		},
	}),
});

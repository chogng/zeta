import { ConfigurationsRegistry } from "../../../../platform/configuration/common/configurationRegistry.js";

/** Typed defaults owned by the Workbench workspace-search surface. */
export const WorkspaceSearchConfiguration = Object.freeze({
	matchCase: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "search.matchCase",
		defaultValue: false,
		parse: value => parseBoolean(value, "search.matchCase"),
	}),
	smartCase: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "search.smartCase",
		defaultValue: true,
		parse: value => parseBoolean(value, "search.smartCase"),
	}),
	regularExpression: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "search.useRegularExpression",
		defaultValue: false,
		parse: value => parseBoolean(value, "search.useRegularExpression"),
	}),
	includePatterns: ConfigurationsRegistry.registerConfiguration<string>({
		key: "search.includePatterns",
		defaultValue: "",
		parse: value => parsePatternList(value, "search.includePatterns"),
	}),
	excludePatterns: ConfigurationsRegistry.registerConfiguration<string>({
		key: "search.excludePatterns",
		defaultValue: "",
		parse: value => parsePatternList(value, "search.excludePatterns"),
	}),
	maxResults: ConfigurationsRegistry.registerConfiguration<number>({
		key: "search.maxResults",
		defaultValue: 2_000,
		parse(value: unknown): number {
			if (Number.isSafeInteger(value) && (value as number) >= 100 && (value as number) <= 5_000) return value as number;
			throw new RangeError(`search.maxResults must be an integer between 100 and 5000; received ${String(value)}`);
		},
	}),
});

function parseBoolean(value: unknown, key: string): boolean {
	if (typeof value === "boolean") return value;
	throw new TypeError(`${key} must be a boolean; received ${String(value)}`);
}

function parsePatternList(value: unknown, key: string): string {
	if (typeof value === "string" && value.length <= 4_096 && !/[\r\n\0]/u.test(value)) return value;
	throw new TypeError(`${key} must be a single-line string no longer than 4096 characters`);
}

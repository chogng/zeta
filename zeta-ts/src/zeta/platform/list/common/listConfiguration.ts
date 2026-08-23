import { ConfigurationsRegistry } from "../../configuration/common/configurationRegistry.js";

export type ListOpenMode = "doubleClick" | "singleClick";

/** Typed configuration keys owned by Platform List. */
export const ListConfiguration = Object.freeze({
	openMode: ConfigurationsRegistry.registerConfiguration<ListOpenMode>({
		key: "workbench.list.openMode",
		defaultValue: "singleClick",
		parse(value: unknown): ListOpenMode {
			if (value !== "singleClick" && value !== "doubleClick") throw new TypeError(`Unknown list open mode: ${String(value)}`);
			return value;
		},
	}),
});

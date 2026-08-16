import { ConfigurationsRegistry } from "../../platform/configuration/common/configurationRegistry.js";

export const WorkbenchProfileIds = ["code", "academic"] as const;

export type WorkbenchProfileId = typeof WorkbenchProfileIds[number];

/** Default Workbench profile requested for new windows in the unified product host. */
export const WorkbenchProfileConfiguration = Object.freeze({
  defaultProfile: ConfigurationsRegistry.registerConfiguration<WorkbenchProfileId>({
    key: "workbench.defaultProfile",
    defaultValue: "code",
    parse(value: unknown): WorkbenchProfileId {
      if (value === "code" || value === "academic") return value;
      throw new TypeError(`workbench.defaultProfile must be code or academic; received ${String(value)}`);
    },
  }),
});

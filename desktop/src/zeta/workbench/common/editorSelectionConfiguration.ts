import { ConfigurationsRegistry } from "../../platform/configuration/common/configurationRegistry.js";

export type DefaultNewDocumentEditor = "buildMode" | "code" | "academic";

/** Workbench-level editor selection preferences applied before a resource has a durable type. */
export const EditorSelectionConfiguration = Object.freeze({
  defaultNewDocumentEditor: ConfigurationsRegistry.registerConfiguration<DefaultNewDocumentEditor>({
    key: "workbench.editor.defaultNewDocumentEditor",
    defaultValue: "buildMode",
    parse(value: unknown): DefaultNewDocumentEditor {
      if (value === "profile") return "buildMode";
      if (value === "buildMode" || value === "code" || value === "academic") return value;
      throw new TypeError(`workbench.editor.defaultNewDocumentEditor must be buildMode, code, or academic; received ${String(value)}`);
    },
  }),
});

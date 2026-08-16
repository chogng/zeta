import { ConfigurationsRegistry } from "../../platform/configuration/common/configurationRegistry.js";

export type DefaultNewDocumentEditor = "profile" | "code" | "academic";

/** Workbench-level editor selection preferences applied before a resource has a durable type. */
export const EditorSelectionConfiguration = Object.freeze({
  defaultNewDocumentEditor: ConfigurationsRegistry.registerConfiguration<DefaultNewDocumentEditor>({
    key: "workbench.editor.defaultNewDocumentEditor",
    defaultValue: "profile",
    parse(value: unknown): DefaultNewDocumentEditor {
      if (value === "profile" || value === "code" || value === "academic") return value;
      throw new TypeError(`workbench.editor.defaultNewDocumentEditor must be profile, code, or academic; received ${String(value)}`);
    },
  }),
});

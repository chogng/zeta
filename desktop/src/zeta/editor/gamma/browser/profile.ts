import type { DocumentNode } from "../common/document.js";
import type { DocumentOutlineOptions } from "../common/documentOutline.js";
import type { DocumentPlugin } from "../common/plugin.js";
import type { DocumentSchema } from "../common/schema.js";
import type { EditorInput } from "../../../workbench/browser/parts/editor/editorInput.js";
import { EditorPaneMatch } from "../../../workbench/browser/parts/editor/editorPane.js";
import type { DocumentEditorPaneOptions } from "./documentEditorPane.js";
import { matchDocumentEditor, type DocumentEditorInputMatcher } from "./documentEditorInput.js";

/** Product-neutral schema and browser composition for one Gamma document kind. */
export interface DocumentEditorProfile {
  readonly id: string;
  readonly editorId: string;
  readonly editorName: string;
  readonly input: DocumentEditorInputMatcher;
  readonly createSchema: () => DocumentSchema;
  readonly createEmptyDocument?: (schema: DocumentSchema) => DocumentNode;
  readonly outline?: DocumentOutlineOptions;
  readonly outlineNavigator?: boolean;
  readonly nodeViews?: DocumentEditorPaneOptions["nodeViews"];
  readonly inlineNodeViews?: DocumentEditorPaneOptions["inlineNodeViews"];
  readonly toolbarActions?: DocumentEditorPaneOptions["toolbarActions"];
  readonly createPlugins?: () => readonly DocumentPlugin<unknown>[];
}

export interface DocumentEditorPaneRuntimeOptions {
  readonly onSave?: DocumentEditorPaneOptions["onSave"];
  readonly embeddedTextEditorFactory?: DocumentEditorPaneOptions["embeddedTextEditorFactory"];
  readonly workingCopyService?: DocumentEditorPaneOptions["workingCopyService"];
}

/** Selects the first profile that claims one Workbench input. */
export function findDocumentEditorProfile(input: EditorInput, profiles: readonly DocumentEditorProfile[]): DocumentEditorProfile | undefined {
  return profiles.find(profile => matchDocumentEditor(input, profile.input) !== EditorPaneMatch.None);
}

/** Produces the editor-pane match used by a profile registry contribution. */
export function matchDocumentEditorProfiles(input: EditorInput, profiles: readonly DocumentEditorProfile[]): EditorPaneMatch {
  return findDocumentEditorProfile(input, profiles) ? EditorPaneMatch.Default : EditorPaneMatch.None;
}

/** Materializes one profile into pane options while keeping Workbench services at the composition root. */
export function createDocumentEditorPaneOptions(profile: DocumentEditorProfile, runtime: DocumentEditorPaneRuntimeOptions = {}): DocumentEditorPaneOptions {
  const schema = profile.createSchema();
  return {
    ...runtime,
    schema,
    ...(profile.createEmptyDocument ? { createEmptyDocument: () => profile.createEmptyDocument!(schema) } : {}),
    ...(profile.outline === undefined ? {} : { outline: profile.outline }),
    ...(profile.outlineNavigator === undefined ? {} : { outlineNavigator: profile.outlineNavigator }),
    ...(profile.nodeViews === undefined ? {} : { nodeViews: profile.nodeViews }),
    ...(profile.inlineNodeViews === undefined ? {} : { inlineNodeViews: profile.inlineNodeViews }),
    ...(profile.toolbarActions === undefined ? {} : { toolbarActions: profile.toolbarActions }),
    ...(profile.createPlugins === undefined ? {} : { plugins: profile.createPlugins() }),
  };
}

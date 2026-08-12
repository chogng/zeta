import type { DocumentNode } from "../../common/model/document.js";
import type { DocumentOutlineOptions } from "../../common/model/documentOutline.js";
import type { DocumentPlugin } from "../../common/model/documentPlugin.js";
import type { DocumentSchema } from "../../common/model/documentSchema.js";
import type { EditorInput } from "../../../workbench/browser/parts/editor/editorInput.js";
import { EditorPaneMatch } from "../../../workbench/browser/parts/editor/editorPane.js";
import type { EditorWidgetOptions } from "../editorWidget.js";
import type { EditorPaneOptions } from "../documentEditorPane.js";
import { matchGamaEditor, type EditorInputMatcher } from "../documentEditorInput.js";
import type { IDocumentCollaborationService } from "../../common/services/documentCollaborationService.js";

/** Product-neutral schema and browser composition for one Gama document kind. */
export interface EditorProfile {
  readonly id: string;
  readonly editorId: string;
  readonly editorName: string;
  readonly input: EditorInputMatcher;
  readonly createSchema: () => DocumentSchema;
  readonly createEmptyDocument?: (schema: DocumentSchema) => DocumentNode;
  readonly outline?: DocumentOutlineOptions;
  readonly outlineNavigator?: boolean;
  readonly nodeViews?: EditorWidgetOptions["nodeViews"];
  readonly inlineNodeViews?: EditorWidgetOptions["inlineNodeViews"];
  readonly toolbarActions?: EditorWidgetOptions["toolbarActions"];
  readonly createPlugins?: () => readonly DocumentPlugin<unknown>[];
  /** Stable compatibility ID for documents that join the same collaboration room. */
  readonly collaborationSchemaId?: string;
}

export interface EditorRuntimeOptions {
  readonly onSave?: EditorWidgetOptions["onSave"];
  readonly embeddedTextEditorFactory?: EditorWidgetOptions["embeddedTextEditorFactory"];
  readonly workingCopyService?: EditorPaneOptions["workingCopyService"];
  readonly documentCollaborationService?: IDocumentCollaborationService;
}

/** Selects the first profile that claims one Workbench input. */
export function findGamaEditorProfile(input: EditorInput, profiles: readonly EditorProfile[]): EditorProfile | undefined {
  return profiles.find(profile => matchGamaEditor(input, profile.input) !== EditorPaneMatch.None);
}

/** Produces the editor-pane match used by a profile registry contribution. */
export function matchGamaEditorProfiles(input: EditorInput, profiles: readonly EditorProfile[]): EditorPaneMatch {
  return findGamaEditorProfile(input, profiles) ? EditorPaneMatch.Default : EditorPaneMatch.None;
}

/** Materializes one profile into pane options while keeping Workbench services at the composition root. */
export function createGamaEditorPaneOptions(profile: EditorProfile, runtime: EditorRuntimeOptions = {}): EditorPaneOptions {
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
    ...(profile.collaborationSchemaId === undefined ? {} : { collaborationSchemaId: profile.collaborationSchemaId }),
  };
}

import type { DocumentNode } from "../../common/model/document.js";
import type { DocumentOutlineOptions } from "../../common/model/documentOutline.js";
import type { DocumentPlugin } from "../../common/model/documentPlugin.js";
import type { DocumentSchema } from "../../common/model/documentSchema.js";
import type { EditorInput } from "../../../../workbench/browser/parts/editor/editorInput.js";
import { EditorPaneMatch } from "../../../../workbench/browser/parts/editor/editorPane.js";
import type { GamaEditorSessionOptions } from "../gamaEditorSession.js";
import type { GamaEditorPaneOptions } from "../gamaEditorPane.js";
import { matchGamaEditor, type GamaEditorInputMatcher } from "../editorInput.js";

/** Product-neutral schema and browser composition for one Gama document kind. */
export interface GamaEditorProfile {
  readonly id: string;
  readonly editorId: string;
  readonly editorName: string;
  readonly input: GamaEditorInputMatcher;
  readonly createSchema: () => DocumentSchema;
  readonly createEmptyDocument?: (schema: DocumentSchema) => DocumentNode;
  readonly outline?: DocumentOutlineOptions;
  readonly outlineNavigator?: boolean;
  readonly nodeViews?: GamaEditorSessionOptions["nodeViews"];
  readonly inlineNodeViews?: GamaEditorSessionOptions["inlineNodeViews"];
  readonly toolbarActions?: GamaEditorSessionOptions["toolbarActions"];
  readonly createPlugins?: () => readonly DocumentPlugin<unknown>[];
}

export interface GamaEditorRuntimeOptions {
  readonly onSave?: GamaEditorSessionOptions["onSave"];
  readonly embeddedTextEditorFactory?: GamaEditorSessionOptions["embeddedTextEditorFactory"];
  readonly workingCopyService?: GamaEditorPaneOptions["workingCopyService"];
}

/** Selects the first profile that claims one Workbench input. */
export function findGamaEditorProfile(input: EditorInput, profiles: readonly GamaEditorProfile[]): GamaEditorProfile | undefined {
  return profiles.find(profile => matchGamaEditor(input, profile.input) !== EditorPaneMatch.None);
}

/** Produces the editor-pane match used by a profile registry contribution. */
export function matchGamaEditorProfiles(input: EditorInput, profiles: readonly GamaEditorProfile[]): EditorPaneMatch {
  return findGamaEditorProfile(input, profiles) ? EditorPaneMatch.Default : EditorPaneMatch.None;
}

/** Materializes one profile into pane options while keeping Workbench services at the composition root. */
export function createGamaEditorPaneOptions(profile: GamaEditorProfile, runtime: GamaEditorRuntimeOptions = {}): GamaEditorPaneOptions {
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

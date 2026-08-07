import { type EditorInput } from "../../../workbench/browser/parts/editor/editorInput.js";
import { EditorPaneMatch } from "../../../workbench/browser/parts/editor/editorPane.js";

export const GAMA_EDITOR_ID = "zeta.editor.gama";

export interface EditorInputMatcher {
  readonly contentTypes?: readonly string[];
  readonly extensions?: readonly string[];
}

/** Matches Gama's structured document resources without loading its browser view. */
export function matchGamaEditor(input: EditorInput, matcher: EditorInputMatcher): EditorPaneMatch {
  if (matcher.contentTypes?.includes(input.contentType ?? "")) return EditorPaneMatch.Default;
  const path = input.resource.path.toLowerCase();
  if (matcher.extensions?.some(extension => path.endsWith(extension.toLowerCase()))) return EditorPaneMatch.Default;
  return EditorPaneMatch.None;
}

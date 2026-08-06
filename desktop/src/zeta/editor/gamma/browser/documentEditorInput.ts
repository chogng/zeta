import { type EditorInput } from "../../../workbench/browser/parts/editor/editorInput.js";
import { EditorPaneMatch } from "../../../workbench/browser/parts/editor/editorPane.js";

export const DOCUMENT_EDITOR_ID = "zeta.editor.document";

export interface DocumentEditorInputMatcher {
  readonly contentTypes?: readonly string[];
  readonly extensions?: readonly string[];
}

/** Matches Gamma's structured document resources without loading its browser view. */
export function matchDocumentEditor(input: EditorInput, matcher: DocumentEditorInputMatcher): EditorPaneMatch {
  if (matcher.contentTypes?.includes(input.contentType ?? "")) return EditorPaneMatch.Default;
  const path = input.resource.path.toLowerCase();
  if (matcher.extensions?.some(extension => path.endsWith(extension.toLowerCase()))) return EditorPaneMatch.Default;
  return EditorPaneMatch.None;
}

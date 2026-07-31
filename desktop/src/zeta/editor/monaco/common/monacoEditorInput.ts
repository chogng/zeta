import type {
  EditorInput,
} from "../../../workbench/browser/parts/editor/editorInput.js";
import {
  EditorPaneMatch,
} from "../../../workbench/browser/parts/editor/editorPane.js";
import {
  ACADEMIC_DOCUMENT_CONTENT_TYPE,
} from "../../../product/common/documentTypes.js";
import { isTextResourceLanguageInput, resolveTextResourceLanguageId } from "../../common/textResourceLanguage.js";

export const MONACO_EDITOR_ID = "zeta.editor.monaco";

/** Returns the product-level Monaco match without loading Monaco itself. */
export function matchMonacoEditor(input: EditorInput): EditorPaneMatch {
  if (input.contentType === ACADEMIC_DOCUMENT_CONTENT_TYPE) {
    return EditorPaneMatch.None;
  }
  if (isTextResourceLanguageInput(input)) {
    return EditorPaneMatch.Default;
  }
  return input.resource.scheme === "file"
    ? EditorPaneMatch.Optional
    : EditorPaneMatch.None;
}

/** Resolves a stable Monaco language identifier from content metadata. */
export function monacoLanguageForInput(input: EditorInput): string {
  const languageId = resolveTextResourceLanguageId(input);
  if (languageId === "typescriptreact") return "typescript";
  if (languageId === "javascriptreact") return "javascript";
  return languageId;
}

import { ACADEMIC_DOCUMENT_CONTENT_TYPE } from "../../../product/common/documentTypes.js";
import { type EditorInput } from "../../../workbench/browser/parts/editor/editorInput.js";
import { EditorPaneMatch } from "../../../workbench/browser/parts/editor/editorPane.js";
import { isTextResourceLanguageInput, resolveTextResourceLanguageId } from "../../common/textResourceLanguage.js";

export const ALPHA_EDITOR_ID = "zeta.editor.alpha";

/** Selects Alpha as the product's canonical plain-text editor. */
export function matchAlphaEditor(input: EditorInput): EditorPaneMatch {
  if (input.contentType === ACADEMIC_DOCUMENT_CONTENT_TYPE) return EditorPaneMatch.None;
  return isTextResourceLanguageInput(input) ? EditorPaneMatch.Default : EditorPaneMatch.None;
}

/** Resolves the language identity shared by Alpha input, Analysis, and completion. */
export function alphaLanguageForInput(input: EditorInput): string {
  return resolveTextResourceLanguageId(input);
}

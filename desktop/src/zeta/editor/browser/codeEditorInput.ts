import { ACADEMIC_DOCUMENT_CONTENT_TYPE } from "../../product/common/documentTypes.js";
import { type EditorInput } from "../../workbench/browser/parts/editor/editorInput.js";
import { EditorPaneMatch } from "../../workbench/browser/parts/editor/editorPane.js";
import { isTextResourceLanguageInput, resolveTextResourceLanguageId, type TextResourceLanguageResolver } from "../../platform/language/common/textResourceLanguage.js";
import { isDiffEditorInput } from "./diffEditorInput.js";

export const CODE_EDITOR_ID = "aster.editor.code";

/** Selects the canonical editor for plain-text resources. */
export function matchCodeEditor(input: EditorInput): EditorPaneMatch {
  if (isDiffEditorInput(input)) return EditorPaneMatch.None;
  if (input.contentType === ACADEMIC_DOCUMENT_CONTENT_TYPE) return EditorPaneMatch.None;
  if (input.resource.scheme === "untitled") return EditorPaneMatch.Default;
  return input.languageId !== undefined || isTextResourceLanguageInput(input) ? EditorPaneMatch.Default : EditorPaneMatch.None;
}

/** Resolves the language identity shared by editor input, syntax, and completion. */
export function languageForEditorInput(input: EditorInput, resolver?: TextResourceLanguageResolver): string {
  return input.languageId ?? resolveTextResourceLanguageId(input, resolver);
}

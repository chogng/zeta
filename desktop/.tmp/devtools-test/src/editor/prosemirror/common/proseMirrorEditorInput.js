import { EditorPaneMatch, } from "../../../workbench/browser/parts/editor/editorPane.js";
import { ACADEMIC_DOCUMENT_CONTENT_TYPE, } from "../../../product/common/documentTypes.js";
export const PROSEMIRROR_EDITOR_ID = "zeta.editor.prosemirror";
const ACADEMIC_EXTENSIONS = [
    ".zeta-academic",
    ".zeta-paper",
];
/** Returns the Academic editor match without loading ProseMirror itself. */
export function matchProseMirrorEditor(input) {
    if (input.contentType === ACADEMIC_DOCUMENT_CONTENT_TYPE ||
        ACADEMIC_EXTENSIONS.some((extension) => input.resource.path.toLowerCase().endsWith(extension))) {
        return EditorPaneMatch.Default;
    }
    if (input.contentType === "text/markdown" ||
        input.contentType === "text/plain" ||
        input.resource.path.toLowerCase().endsWith(".md") ||
        input.resource.path.toLowerCase().endsWith(".txt")) {
        return EditorPaneMatch.Optional;
    }
    return EditorPaneMatch.None;
}

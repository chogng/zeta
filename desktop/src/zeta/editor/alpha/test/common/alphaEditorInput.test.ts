import assert from "node:assert/strict";
import test from "node:test";
import { URI } from "../../../../base/common/uri.js";
import { ACADEMIC_DOCUMENT_CONTENT_TYPE } from "../../../../product/common/documentTypes.js";
import { EditorPaneMatch } from "../../../../workbench/browser/parts/editor/editorPane.js";
import { ALPHA_EDITOR_ID, alphaLanguageForInput, matchAlphaEditor } from "../../common/alphaEditorInput.js";

test("Alpha is the default text editor with canonical language IDs", () => {
  assert.equal(ALPHA_EDITOR_ID, "zeta.editor.alpha");
  assert.equal(matchAlphaEditor({ resource: URI.file("C:\\project\\view.tsx") }), EditorPaneMatch.Default);
  assert.equal(alphaLanguageForInput({ resource: URI.file("C:\\project\\view.tsx") }), "typescriptreact");
  assert.equal(alphaLanguageForInput({ resource: URI.file("C:\\project\\settings.jsonc") }), "jsonc");
  assert.equal(matchAlphaEditor({ resource: URI.file("C:\\project\\binary.bin") }), EditorPaneMatch.None);
});

test("Alpha excludes structured Academic documents", () => {
  assert.equal(matchAlphaEditor({
    resource: URI.file("C:\\papers\\research.zeta-paper"),
    contentType: ACADEMIC_DOCUMENT_CONTENT_TYPE,
  }), EditorPaneMatch.None);
});

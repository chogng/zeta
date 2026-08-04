import assert from "node:assert/strict";
import test from "node:test";
import { URI } from "../../../../base/common/uri.js";
import { ACADEMIC_DOCUMENT_CONTENT_TYPE } from "../../../../product/common/documentTypes.js";
import { EditorPaneMatch } from "../../../../workbench/browser/parts/editor/editorPane.js";
import { ALPHA_DIFF_EDITOR_ID, createAlphaDiffEditorInput, matchAlphaDiffEditor } from "../../browser/diffEditorInput.js";
import { ALPHA_EDITOR_ID, alphaLanguageForInput, matchAlphaEditor } from "../../browser/editorInput.js";

test("Alpha is the default text editor with canonical language IDs", () => {
  assert.equal(ALPHA_EDITOR_ID, "zeta.editor.alpha");
  assert.equal(matchAlphaEditor({ resource: URI.file("C:\\project\\view.tsx") }), EditorPaneMatch.Default);
  assert.equal(alphaLanguageForInput({ resource: URI.file("C:\\project\\view.tsx") }), "typescriptreact");
  assert.equal(alphaLanguageForInput({ resource: URI.file("C:\\project\\settings.jsonc") }), "jsonc");
  assert.equal(matchAlphaEditor({ resource: URI.parse("untitled:/Untitled-1") }), EditorPaneMatch.Default);
  assert.equal(alphaLanguageForInput({ resource: URI.parse("untitled:/Untitled-1"), languageId: "typescript" }), "typescript");
  assert.equal(matchAlphaEditor({ resource: URI.file("C:\\project\\binary.bin") }), EditorPaneMatch.None);
});

test("Alpha excludes structured Academic documents", () => {
  assert.equal(matchAlphaEditor({
    resource: URI.file("C:\\papers\\research.zeta-paper"),
    contentType: ACADEMIC_DOCUMENT_CONTENT_TYPE,
  }), EditorPaneMatch.None);
});

test("Alpha diff inputs have one stable tab identity and select only the diff pane", () => {
  const original = { resource: URI.file("C:\\project\\before.ts"), label: "before.ts" };
  const modified = { resource: URI.file("C:\\project\\after.ts"), label: "after.ts" };
  const input = createAlphaDiffEditorInput(original, modified, "Review changes");

  assert.equal(ALPHA_DIFF_EDITOR_ID, "zeta.editor.alpha-diff");
  assert.equal(input.label, "Review changes");
  assert.equal(input.readOnly, true);
  assert.equal(matchAlphaDiffEditor(input), EditorPaneMatch.Default);
  assert.equal(matchAlphaEditor(input), EditorPaneMatch.None);
  assert.match(input.resource.toString(), /^zeta-diff:\/compare\?/);
});

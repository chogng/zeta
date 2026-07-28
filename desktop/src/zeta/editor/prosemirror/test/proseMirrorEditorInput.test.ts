import assert from "node:assert/strict";
import test from "node:test";
import {
  URI,
} from "../../../base/common/uri.js";
import {
  ACADEMIC_DOCUMENT_CONTENT_TYPE,
} from "../../../product/common/documentTypes.js";
import {
  EditorPaneMatch,
} from "../../../workbench/browser/parts/editor/editorPane.js";
import {
  matchProseMirrorEditor,
} from "../common/proseMirrorEditorInput.js";
import {
  createProseMirrorEditorState,
  proseMirrorDocumentSchema,
} from "../common/proseMirrorEditorState.js";

test("ProseMirror defaults structured Academic documents", () => {
  assert.equal(
    matchProseMirrorEditor({
      resource: URI.file("C:\\papers\\research.zeta-paper"),
      contentType: ACADEMIC_DOCUMENT_CONTENT_TYPE,
    }),
    EditorPaneMatch.Default,
  );
});

test("ProseMirror remains optional for Markdown and plain text", () => {
  assert.equal(
    matchProseMirrorEditor({
      resource: URI.file("C:\\papers\\notes.md"),
      contentType: "text/markdown",
    }),
    EditorPaneMatch.Optional,
  );
  assert.equal(
    matchProseMirrorEditor({
      resource: URI.file("C:\\papers\\notes.txt"),
    }),
    EditorPaneMatch.Optional,
  );
});

test("ProseMirror rejects unrelated source files", () => {
  assert.equal(
    matchProseMirrorEditor({
      resource: URI.file("C:\\project\\main.ts"),
      contentType: "text/typescript",
    }),
    EditorPaneMatch.None,
  );
});

test("ProseMirror state uses the customizable subsystem schema", () => {
  const state = createProseMirrorEditorState("Title\nBody");
  assert.equal(state.schema, proseMirrorDocumentSchema);
  assert.equal(state.doc.childCount, 2);
  assert.equal(state.doc.child(0).textContent, "Title");
  assert.equal(state.doc.child(1).textContent, "Body");
});

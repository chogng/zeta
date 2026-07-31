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
  matchMonacoEditor,
  monacoLanguageForInput,
} from "../common/monacoEditorInput.js";

test("Monaco defaults Markdown and resolves its language", () => {
  const markdown = {
    resource: URI.file("C:\\project\\README.md"),
    contentType: "text/markdown",
  };
  assert.equal(matchMonacoEditor(markdown), EditorPaneMatch.Default);
  assert.equal(monacoLanguageForInput(markdown), "markdown");
});

test("Monaco excludes structured Academic documents", () => {
  const paper = {
    resource: URI.file("C:\\papers\\research.zeta-paper"),
    contentType: ACADEMIC_DOCUMENT_CONTENT_TYPE,
  };
  assert.equal(matchMonacoEditor(paper), EditorPaneMatch.None);
});

test("Monaco resolves common source languages from extensions", () => {
  assert.equal(
    monacoLanguageForInput({
      resource: URI.file("C:\\project\\main.ts"),
    }),
    "typescript",
  );
  assert.equal(
    monacoLanguageForInput({
      resource: URI.file("C:\\project\\Cargo.toml"),
    }),
    "ini",
  );
  assert.equal(
    monacoLanguageForInput({
      resource: URI.file("C:\\project\\view.tsx"),
    }),
    "typescript",
  );
});

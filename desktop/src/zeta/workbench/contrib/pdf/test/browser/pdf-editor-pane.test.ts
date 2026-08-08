import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { CancellationError } from "../../../../../base/common/cancellation.js";
import { URI } from "../../../../../base/common/uri.js";
import { FileKind, type IFileService } from "../../../../../platform/files/common/files.js";
import type { EditorInput } from "../../../../../workbench/browser/parts/editor/editorInput.js";
import { EditorPaneMatch, EditorPaneVisibility } from "../../../../../workbench/browser/parts/editor/editorPane.js";
import { PdfEditorPane, type IPdfObjectUrlFactory } from "../../../../../workbench/contrib/pdf/browser/pdfEditorPane.js";
import { PDF_CONTENT_TYPE, matchPdfEditor } from "../../../../../workbench/contrib/pdf/browser/pdfEditorInput.js";
import { WorkspacePdfDocumentLoader } from "../../../../../workbench/contrib/pdf/browser/pdfDocumentLoader.js";

test("PDF editor matching selects only application/pdf and .pdf resources", () => {
  assert.equal(matchPdfEditor(input("paper.PDF")), EditorPaneMatch.Default);
  assert.equal(matchPdfEditor({ ...input("paper.bin"), contentType: "Application/PDF; charset=binary" }), EditorPaneMatch.Default);
  assert.equal(matchPdfEditor(input("paper.txt")), EditorPaneMatch.None);
});

test("PDF editor loads bytes into Chromium's viewer and releases object URLs", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const urls = new TestObjectUrls();
  const pane = new PdfEditorPane({ load: async () => new Uint8Array([37, 80, 68, 70]) }, urls);
  pane.create(dom.window.document.body);

  await pane.setInput(input("paper.pdf"), new AbortController().signal);
  const frame = dom.window.document.querySelector<HTMLIFrameElement>(".zeta-pdf-editor-frame");
  assert.ok(frame);
  assert.match(frame.src, /blob:zeta-pdf-1$/);
  assert.equal(frame.title, "paper.pdf PDF reader");
  pane.setVisible(EditorPaneVisibility.Hidden);
  assert.equal(dom.window.document.querySelector<HTMLElement>(".zeta-pdf-editor")?.hidden, true);
  pane.setVisible(EditorPaneVisibility.Visible);
  pane.focus();
  assert.equal(dom.window.document.activeElement, frame);

  await pane.setInput(input("replacement.pdf"), new AbortController().signal);
  assert.deepEqual(urls.revoked, ["blob:zeta-pdf-1"]);
  pane.clearInput();
  assert.deepEqual(urls.revoked, ["blob:zeta-pdf-1", "blob:zeta-pdf-2"]);
  assert.equal(frame.src, "about:blank");
  pane.dispose();
  assert.deepEqual(urls.revoked, ["blob:zeta-pdf-1", "blob:zeta-pdf-2"]);
  dom.window.close();
});

test("PDF editor observes cancellation before installing an object URL", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const urls = new TestObjectUrls();
  const pane = new PdfEditorPane({ load: async () => new Uint8Array([37]) }, urls);
  pane.create(dom.window.document.body);
  const controller = new AbortController();
  controller.abort("closed");

  await assert.rejects(pane.setInput(input("paper.pdf"), controller.signal), CancellationError);
  assert.equal(urls.created.length, 0);
  pane.dispose();
  dom.window.close();
});

test("workspace PDF loader reads only through the binary file contract", async () => {
  const resource = URI.file("C:\\project\\paper.pdf");
  const loader = new WorkspacePdfDocumentLoader({
    onDidChangeFiles: () => ({ dispose() {}, [Symbol.dispose]() {} }),
    stat: async () => ({ resource, kind: FileKind.File, sizeBytes: 4, readonly: true, modifiedAtMillis: undefined }),
    readDirectory: async () => [],
    readFile: async () => { throw new Error("PDF loader must not request text"); },
    readFileBytes: async (requested) => ({ resource: requested, bytes: new Uint8Array([37, 80, 68, 70]), revision: "pdf-revision" }),
    writeFile: async () => { throw new Error("PDF loader is read-only"); },
  } satisfies IFileService);

  assert.deepEqual(await loader.load(input("paper.pdf"), new AbortController().signal), new Uint8Array([37, 80, 68, 70]));
});

function input(name: string): EditorInput {
  return { resource: URI.file(`C:\\project\\${name}`), label: name };
}

class TestObjectUrls implements IPdfObjectUrlFactory {
  readonly created: Uint8Array[] = [];
  readonly revoked: string[] = [];

  create(bytes: Uint8Array): string {
    this.created.push(bytes);
    return `blob:zeta-pdf-${this.created.length}`;
  }

  revoke(url: string): void {
    this.revoked.push(url);
  }
}

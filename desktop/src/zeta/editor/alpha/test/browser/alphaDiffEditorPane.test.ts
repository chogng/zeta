import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { URI } from "../../../../base/common/uri.js";
import { TextFileContentSource, type ITextFileService, type ResolvedTextFileContent, type TextFileResolveRequest } from "../../../../workbench/services/textfile/common/textFileService.js";
import { EditorPaneVisibility } from "../../../../workbench/browser/parts/editor/editorPane.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
  window: browserEnvironment.window,
  document: browserEnvironment.window.document,
  Node: browserEnvironment.window.Node,
  Element: browserEnvironment.window.Element,
  HTMLElement: browserEnvironment.window.HTMLElement,
  Event: browserEnvironment.window.Event,
})) {
  Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { AlphaDiffEditorPane } = await import("../../browser/alphaDiffEditorPane.js");
const { AlphaTextModelService } = await import("../../browser/alphaTextModelService.js");
const { createAlphaDiffEditorInput } = await import("../../common/alphaDiffEditorInput.js");

test("Alpha diff pane acquires both models, lays out the review view, and releases both references", async () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const parent = requiredElement<HTMLElement>(dom.window.document, "main");
  using models = new AlphaTextModelService();
  const pane = new AlphaDiffEditorPane(new BootstrapTextFiles(), { modelService: models });
  pane.create(parent);
  pane.layout({ width: 640, height: 480 });
  await pane.setInput(createAlphaDiffEditorInput(
    { resource: URI.file("C:\\project\\before.ts"), initialText: "const oldValue = 1;", label: "before.ts" },
    { resource: URI.file("C:\\project\\after.ts"), initialText: "const newValue = 2;", label: "after.ts" },
  ), new AbortController().signal);

  assert.equal(parent.querySelectorAll(".zeta-alpha-diff-editor-pane").length, 1);
  assert.equal(parent.querySelectorAll(".zeta-alpha-diff-editor").length, 1);
  assert.match(parent.querySelector(".zeta-alpha-diff-editor")?.getAttribute("aria-label") ?? "", /before\.ts/);
  pane.focus();
  assert.equal(dom.window.document.activeElement?.classList.contains("zeta-alpha-diff-editor"), true);
  pane.setVisible(EditorPaneVisibility.Hidden);
  assert.equal((parent.firstElementChild as HTMLElement).hidden, true);
  pane.clearInput();
  assert.equal(parent.querySelectorAll(".zeta-alpha-diff-editor").length, 0);
  pane.dispose();
  assert.equal(parent.children.length, 0);
  dom.window.close();
});

class BootstrapTextFiles implements ITextFileService {
  readonly onDidChangeFiles = () => ({ dispose() {}, [Symbol.dispose]() {} });

  async resolve(request: TextFileResolveRequest): Promise<ResolvedTextFileContent> {
    return {
      resource: request.resource,
      text: request.bootstrapText ?? "",
      source: request.bootstrapText === undefined ? TextFileContentSource.FileSystem : TextFileContentSource.Bootstrap,
    };
  }

  async save(): Promise<void> {}
}

function requiredElement<T extends Element>(ownerDocument: Document, selector: string): T {
  const element = ownerDocument.querySelector<T>(selector);
  if (!element) throw new Error(`Missing ${selector}`);
  return element;
}

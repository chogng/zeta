import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { URI } from "../../../../base/common/uri.js";
import { type DiffComputationRequest, type IDiffComputationService } from "../../common/diff/diffComputationService.js";
import { type LineDiff } from "../../common/diff/lineDiff.js";
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

const { DiffEditorPane } = await import("../../browser/diffEditorPane.js");
const { BrowserTextModelService } = await import("../../browser/services/browserTextModelService.js");
const { BrowserTextResourceStore } = await import("../../browser/services/browserTextResourceStore.js");
const { createAlphaDiffEditorInput } = await import("../../browser/diffEditorInput.js");

test("Alpha diff pane rejects a missing Rust diff computation service", () => {
  assert.throws(() => new DiffEditorPane(new BrowserTextResourceStore(new BootstrapTextFiles()), undefined as never), /requires the Rust diff computation service/);
});

test("Alpha diff pane acquires both models, lays out the review view, and releases both references", async () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const parent = requiredElement<HTMLElement>(dom.window.document, "main");
  const textFiles = new BootstrapTextFiles();
  const resourceStore = new BrowserTextResourceStore(textFiles);
  using models = new BrowserTextModelService(resourceStore);
  const pane = new DiffEditorPane(resourceStore, {
    modelService: models,
    createComputationService: () => new PaneTestDiffComputationService(),
  });
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

class PaneTestDiffComputationService implements IDiffComputationService {
  async compute(_request: DiffComputationRequest, signal: AbortSignal): Promise<LineDiff> {
    signal.throwIfAborted();
    return Object.freeze({ rows: Object.freeze([]), hunks: Object.freeze([]) });
  }

  dispose(): void {}

  [Symbol.dispose](): void {
    this.dispose();
  }
}

function requiredElement<T extends Element>(ownerDocument: Document, selector: string): T {
  const element = ownerDocument.querySelector<T>(selector);
  if (!element) throw new Error(`Missing ${selector}`);
  return element;
}

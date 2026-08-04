import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { URI } from "../../../../base/common/uri.js";
import { TextFileContentSource, type ITextFileService, type ResolvedTextFileContent, type TextFileResolveRequest } from "../../../../workbench/services/textfile/common/textFileService.js";
import { EditorPaneVisibility } from "../../../../workbench/browser/parts/editor/editorPane.js";
import { TextPosition, TextRange } from "../../common/core/text.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
  window: browserEnvironment.window,
  document: browserEnvironment.window.document,
  Node: browserEnvironment.window.Node,
  Element: browserEnvironment.window.Element,
  HTMLElement: browserEnvironment.window.HTMLElement,
  Event: browserEnvironment.window.Event,
  InputEvent: browserEnvironment.window.InputEvent,
})) {
  Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { AlphaEditorPane } = await import("../../browser/alphaEditorPane.js");
const { BrowserTextModelService } = await import("../../browser/services/browserTextModelService.js");
const { BrowserTextResourceStore } = await import("../../browser/services/browserTextResourceStore.js");
const { AlphaEditorTextDirection } = await import("../../browser/view/editorViewport.js");

test.after(() => browserEnvironment.window.close());

test("Alpha editor pane loads, lays out, focuses, hides, and clears one native session", async () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  dom.window.HTMLCanvasElement.prototype.getContext = () => null;
  const parent = dom.window.document.querySelector<HTMLElement>("main")!;
  const textFiles = new ImmediateTextFiles("from disk");
  const resourceStore = new BrowserTextResourceStore(textFiles);
  using models = new BrowserTextModelService(resourceStore);
  const pane = new AlphaEditorPane(resourceStore, { modelService: models, textDirection: AlphaEditorTextDirection.RightToLeft });
  pane.create(parent);
  pane.layout({ width: 640, height: 480 });
  await pane.setInput({
    resource: URI.file("C:\\project\\main.ts"),
    label: "main.ts",
    initialText: "const alpha = 1;",
  }, new AbortController().signal);

  assert.equal(pane.getValue(), "const alpha = 1;");
  assert.equal(parent.querySelectorAll(".zeta-alpha-editor-pane").length, 1);
  assert.equal(parent.querySelectorAll(".zeta-alpha-editor").length, 1);
  assert.equal(parent.querySelector<HTMLElement>(".zeta-alpha-editor")?.dir, "rtl");
  pane.focus();
  assert.equal(dom.window.document.activeElement?.classList.contains("zeta-alpha-editor-input"), true);
  assert.equal((dom.window.document.activeElement as HTMLTextAreaElement).dir, "rtl");
  pane.setVisible(EditorPaneVisibility.Hidden);
  assert.equal((parent.firstElementChild as HTMLElement).hidden, true);
  pane.setVisible(EditorPaneVisibility.Visible);
  assert.equal((parent.firstElementChild as HTMLElement).hidden, false);

  pane.clearInput();
  assert.equal(pane.getValue(), "");
  assert.equal(parent.querySelectorAll(".zeta-alpha-editor").length, 0);
  pane.dispose();
  assert.equal(parent.children.length, 0);
  dom.window.close();
});

test("Alpha editor pane releases a load cancelled before content resolution", async () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  dom.window.HTMLCanvasElement.prototype.getContext = () => null;
  const parent = dom.window.document.querySelector<HTMLElement>("main")!;
  const pending = deferred<ResolvedTextFileContent>();
  const textFiles = { onDidChangeFiles: inertFileChanges, resolve: () => pending.promise, save: async () => {} };
  const resourceStore = new BrowserTextResourceStore(textFiles);
  using models = new BrowserTextModelService(resourceStore);
  const pane = new AlphaEditorPane(resourceStore, { modelService: models });
  pane.create(parent);
  const controller = new AbortController();
  const opening = pane.setInput({ resource: URI.file("C:\\project\\slow.ts") }, controller.signal);
  controller.abort();
  pending.resolve({
    resource: URI.file("C:\\project\\slow.ts"),
    text: "late",
    source: TextFileContentSource.FileSystem,
  });

  await assert.rejects(opening, error => (error as Error).name === "CancellationError");
  assert.equal(parent.querySelectorAll(".zeta-alpha-editor").length, 0);
  pane.dispose();
  dom.window.close();
});

test("Alpha editor pane saves and reverts its shared model reference", async () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  dom.window.HTMLCanvasElement.prototype.getContext = () => null;
  const parent = dom.window.document.querySelector<HTMLElement>("main")!;
  const textFiles = new ImmediateTextFiles("from disk");
  const resourceStore = new BrowserTextResourceStore(textFiles);
  using models = new BrowserTextModelService(resourceStore);
  const resource = URI.file("C:\\project\\main.ts");
  const reference = await models.acquire({ resource }, new AbortController().signal);
  const pane = new AlphaEditorPane(resourceStore, { modelService: models });
  pane.create(parent);
  await pane.setInput({ resource, label: "main.ts" }, new AbortController().signal);

  reference.model.applyEdits([{
    range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 4)),
    text: "saved",
  }]);
  assert.equal(pane.isDirty, true);
  await pane.save();
  assert.deepEqual(textFiles.savedTexts, ["saved disk"]);
  assert.equal(pane.isDirty, false);

  reference.model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(0, 5)),
    text: " locally",
  }]);
  textFiles.setText("from disk");
  await pane.revert();
  assert.equal(pane.getValue(), "from disk");
  assert.equal(pane.isDirty, false);

  reference.dispose();
  pane.dispose();
  dom.window.close();
});

class ImmediateTextFiles implements ITextFileService {
  readonly savedTexts: string[] = [];
  readonly onDidChangeFiles = inertFileChanges;

  constructor(private text: string) {}

  async resolve(request: TextFileResolveRequest): Promise<ResolvedTextFileContent> {
    return {
      resource: request.resource,
      text: request.bootstrapText ?? this.text,
      source: request.bootstrapText === undefined ? TextFileContentSource.FileSystem : TextFileContentSource.Bootstrap,
    };
  }

  async save(request: { readonly text: string }): Promise<void> {
    this.savedTexts.push(request.text);
    this.text = request.text;
  }

  setText(text: string): void {
    this.text = text;
  }
}

function inertFileChanges() {
  return {
    dispose() {},
    [Symbol.dispose]() {},
  };
}

function deferred<T>(): { readonly promise: Promise<T>; readonly resolve: (value: T) => void } {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(resolver => {
    resolve = resolver;
  });
  return { promise, resolve };
}

import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { URI } from "../../../../base/common/uri.js";
import { TextFileContentSource, type ITextFileService, type ResolvedTextFileContent, type TextFileResolveRequest } from "../../../../workbench/services/textfile/common/textFileService.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
browserEnvironment.window.HTMLCanvasElement.prototype.getContext = () => null;
class NoopWorker {
  addEventListener(): void {}
  removeEventListener(): void {}
  postMessage(): void {}
  terminate(): void {}
}

for (const [name, value] of Object.entries({
  window: browserEnvironment.window,
  document: browserEnvironment.window.document,
  Node: browserEnvironment.window.Node,
  Element: browserEnvironment.window.Element,
  HTMLElement: browserEnvironment.window.HTMLElement,
  Event: browserEnvironment.window.Event,
  InputEvent: browserEnvironment.window.InputEvent,
  KeyboardEvent: browserEnvironment.window.KeyboardEvent,
  MouseEvent: browserEnvironment.window.MouseEvent,
  Worker: NoopWorker,
})) {
  Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { EditorPart } = await import("../../../../workbench/browser/parts/editor/editorPart.js");
const { EditorPanes } = await import("../../../../workbench/browser/parts/editor/editorRegistry.js");
const { EditorPane } = await import("../../browser/editorPane.js");
const { ALPHA_EDITOR_ID } = await import("../../browser/editorInput.js");
await import("../../contrib/editor.contribution.js");

test.after(() => browserEnvironment.window.close());

test("EditorPart opens a real Alpha pane and saves its edited model", async () => {
  const document = browserEnvironment.window.document;
  const textFiles = new InMemoryTextFiles("const alpha = 1;");
  const editor = new EditorPart(document, { textFileService: textFiles });
  document.body.append(editor.element);
  editor.layout({ width: 800, height: 600 });

  try {
    const input = {
      resource: URI.file("C:\\project\\main.ts"),
      label: "main.ts",
      languageId: "typescript",
    };
    assert.equal(EditorPanes.resolve(input).id, ALPHA_EDITOR_ID);

    const pane = await editor.openEditor(input);
    const alphaPane = pane as InstanceType<typeof EditorPane>;

    assert.equal(pane.id, ALPHA_EDITOR_ID);
    assert.equal(editor.activePane, pane);
    assert.equal(pane instanceof EditorPane, true);
    assert.ok(document.querySelector(".zeta-alpha-editor"));
    const textInput = document.querySelector<HTMLTextAreaElement>(".zeta-alpha-editor-input");
    assert.ok(textInput);

    pane.focus();
    textInput.dispatchEvent(new browserEnvironment.window.InputEvent("beforeinput", {
      bubbles: true,
      cancelable: true,
      data: "x",
      inputType: "insertText",
    }));
    assert.equal(alphaPane.getValue(), "xconst alpha = 1;");

    await alphaPane.save();
    assert.deepEqual(textFiles.savedTexts, ["xconst alpha = 1;"]);
  } finally {
    editor.dispose();
  }
});

class InMemoryTextFiles implements ITextFileService {
  readonly savedTexts: string[] = [];
  readonly onDidChangeFiles = inertFileChanges;

  constructor(private text: string) {}

  async resolve(request: TextFileResolveRequest): Promise<ResolvedTextFileContent> {
    return {
      resource: request.resource,
      text: request.bootstrapText ?? this.text,
      source: request.bootstrapText === undefined
        ? TextFileContentSource.FileSystem
        : TextFileContentSource.Bootstrap,
    };
  }

  async save(request: { readonly text: string }): Promise<void> {
    this.savedTexts.push(request.text);
    this.text = request.text;
  }
}

function inertFileChanges() {
  return {
    dispose() {},
    [Symbol.dispose]() {},
  };
}

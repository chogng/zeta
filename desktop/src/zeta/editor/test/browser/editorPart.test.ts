import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { URI } from "../../../base/common/uri.js";
import { type TextModelReference } from "../../common/services/textModelService.js";
import { TextModel } from "../../common/model/textModel.js";

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

await import("../../editor.code.all.js");
const { EditorPart } = await import("../../browser/editorPart.js");

test.after(() => browserEnvironment.window.close());

test("Aster editor part composes native input, local language syntax, and presentation", async () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  dom.window.HTMLCanvasElement.prototype.getContext = () => null;
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  const model = new TextModel("{\"name\": \"alpha\"");
  const reference = modelReference(URI.file("C:\\project\\settings.json"), model);
  const errors: unknown[] = [];
  const editorPart = new EditorPart({
    container,
    input: {
      resource: reference.resource,
      label: "settings.json",
    },
    languageId: "json",
    modelReference: reference,
    onLanguageError: error => errors.push(error),
  });
  editorPart.layout({ width: 500, height: 240 });
  await waitFor(() => container.querySelectorAll(".aster-editor-token.token-string").length > 0);
  await waitFor(() => container.querySelectorAll(".aster-editor-decoration.warning-underline").length > 0);

  assert.equal(container.querySelectorAll(".aster-editor").length, 1);
  assert.equal(container.querySelectorAll(".aster-editor-input").length, 1);
  assert.equal(container.querySelectorAll(".aster-editor-token.token-string").length > 0, true);
  assert.equal(container.querySelectorAll(".aster-editor-bracket-level-1").length > 0, true);
  assert.equal(container.querySelectorAll(".aster-editor-decoration.warning-underline").length > 0, true);
  assert.deepEqual(errors, []);

  editorPart.textInput.element.dispatchEvent(new dom.window.InputEvent("beforeinput", {
    bubbles: true,
    cancelable: true,
    data: "x",
    inputType: "insertText",
  }));
  assert.equal(editorPart.getValue().startsWith("x{"), true);

  editorPart.dispose();
  await Promise.resolve();
  await Promise.resolve();
  assert.deepEqual(errors, []);
  assert.equal(container.children.length, 0);
  assert.throws(() => model.getText(), /disposed/);
  dom.window.close();
});

test("Aster editor part derives indentation folds and projects their gutter controls", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  dom.window.HTMLCanvasElement.prototype.getContext = () => null;
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  const model = new TextModel("root\n  child\nafter");
  const reference = modelReference(URI.file("C:\\project\\fold.txt"), model);
  const editorPart = new EditorPart({
    container,
    input: {
      resource: reference.resource,
      label: "fold.txt",
    },
    languageId: "plaintext",
    modelReference: reference,
  });
  editorPart.layout({ width: 500, height: 120 });

  const foldToggle = container.querySelector<HTMLButtonElement>(".aster-editor-fold-toggle");
  assert.ok(foldToggle);
  foldToggle.dispatchEvent(new dom.window.MouseEvent("pointerdown", { bubbles: true, cancelable: true }));
  assert.deepEqual([...container.querySelectorAll<HTMLElement>(".aster-editor-line")].map(line => line.dataset.logicalLineIndex), ["0", "2"]);

  editorPart.dispose();
  dom.window.close();
});

test("Aster editor part honors a read-only input without disabling selection infrastructure", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  dom.window.HTMLCanvasElement.prototype.getContext = () => null;
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  const model = new TextModel("alpha");
  const reference = modelReference(URI.file("C:\\project\\preview.txt"), model);
  const editorPart = new EditorPart({
    container,
    input: { resource: reference.resource, label: "preview.txt", readOnly: true },
    languageId: "plaintext",
    modelReference: reference,
  });

  const input = editorPart.textInput.element;
  assert.equal(input.readOnly, true);
  assert.equal(input.getAttribute("aria-readonly"), "true");
  const edit = new dom.window.InputEvent("beforeinput", {
    bubbles: true,
    cancelable: true,
    data: "x",
    inputType: "insertText",
  });
  input.dispatchEvent(edit);
  assert.equal(edit.defaultPrevented, true);
  assert.equal(editorPart.getValue(), "alpha");
  editorPart.selections.setSelections(editorPart.selections.selections);

  editorPart.dispose();
  dom.window.close();
});

test("Aster editor part mounts text drop as an optional full-editor contribution", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  dom.window.HTMLCanvasElement.prototype.getContext = () => null;
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  const model = new TextModel("alpha");
  const reference = modelReference(URI.file("C:\\project\\drop.txt"), model);
  const editorPart = new EditorPart({
    container,
    input: { resource: reference.resource, label: "drop.txt" },
    languageId: "plaintext",
    modelReference: reference,
  });
  editorPart.layout({ width: 120, height: 20 });
  editorPart.viewport.element.getBoundingClientRect = () => rectangle(120, 20);
  const drop = textDropEvent(dom.window, "dropped", 100, 5);

  editorPart.viewport.element.dispatchEvent(drop);

  assert.equal(drop.defaultPrevented, true);
  assert.equal(editorPart.getValue(), "alphadropped");
  editorPart.dispose();
  dom.window.close();
});

test("Aster editor part applies selected before-save contributions through explicit save", async () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  dom.window.HTMLCanvasElement.prototype.getContext = () => null;
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  const model = new TextModel("alpha");
  const reference = modelReference(URI.file("C:\\project\\save.txt"), model);
  let savedText = "";
  const editorPart = new EditorPart({
    container,
    input: { resource: reference.resource, label: "save.txt" },
    languageId: "plaintext",
    modelReference: reference,
    insertFinalNewLine: true,
    onSave: async () => { savedText = model.getText(); },
  });
  await editorPart.save();
  assert.equal(savedText, "alpha\n");

  editorPart.dispose();
  dom.window.close();
});

test("Code editor keeps large files editable while disabling full-document background features", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  dom.window.HTMLCanvasElement.prototype.getContext = () => null;
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  const model = new TextModel("let value = 1;\n".repeat(300_001));
  const reference = modelReference(URI.file("C:\\project\\large.ts"), model);
  const editorPart = new EditorPart({ container, input: { resource: reference.resource, label: "large.ts" }, languageId: "typescript", modelReference: reference });
  try {
    editorPart.layout({ width: 500, height: 40 });
    assert.equal(model.largeFile.tooLargeForTokenization, true, "large-file policy");
    assert.equal(container.querySelectorAll(".aster-editor-token").length, 0, "background tokens");
    assert.equal(container.querySelectorAll(".aster-editor-fold-toggle:not([hidden])").length, 0, "folding scan");
    editorPart.textInput.element.dispatchEvent(new dom.window.InputEvent("beforeinput", { bubbles: true, cancelable: true, data: "x", inputType: "insertText" }));
    assert.equal(editorPart.getValue().startsWith("xlet value = 1;\n"), true, "basic editing");
  } finally {
    editorPart.dispose();
    dom.window.close();
  }
});

function modelReference(resource: URI, model: TextModel): TextModelReference {
  let disposed = false;
  const dispose = (): void => {
    if (disposed) return;
    disposed = true;
    model.dispose();
  };
  return {
    resource,
    model,
    get isDirty(): boolean {
      return false;
    },
    onDidChangeDirty: () => ({
      dispose() {},
      [Symbol.dispose]() {},
    }),
    get hasExternalChange(): boolean {
      return false;
    },
    onDidChangeExternalChange: () => ({
      dispose() {},
      [Symbol.dispose]() {},
    }),
    async save(): Promise<void> {},
    async revert(): Promise<void> {},
    dispose,
    [Symbol.dispose]: dispose,
  };
}

function nextTask(): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, 0));
}

function textDropEvent(targetWindow: typeof browserEnvironment.window, text: string, clientX: number, clientY: number): DragEvent {
  const event = new targetWindow.Event("drop", { bubbles: true, cancelable: true });
  Object.defineProperties(event, {
    clientX: { value: clientX },
    clientY: { value: clientY },
    dataTransfer: {
      value: {
        types: ["text/plain"],
        getData(type: string): string {
          return type === "text/plain" ? text : "";
        },
      },
    },
  });
  return event as unknown as DragEvent;
}

function rectangle(width: number, height: number): DOMRect {
  return {
    x: 0,
    y: 0,
    width,
    height,
    top: 0,
    right: width,
    bottom: height,
    left: 0,
    toJSON: () => ({}),
  };
}

async function waitFor(predicate: () => boolean): Promise<void> {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    if (predicate()) return;
    await nextTask();
  }
  assert.fail("Timed out waiting for Aster editor projection");
}

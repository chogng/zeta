import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { URI } from "../../../../base/common/uri.js";
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

const { EditorSession } = await import("../../browser/editorSession.js");

test.after(() => browserEnvironment.window.close());

test("Alpha editor session composes native input, local language analysis, and presentation", async () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  dom.window.HTMLCanvasElement.prototype.getContext = () => null;
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  const model = new TextModel("{\"name\": \"alpha\"");
  const reference = modelReference(URI.file("C:\\project\\settings.json"), model);
  const errors: unknown[] = [];
  const session = new EditorSession({
    container,
    input: {
      resource: reference.resource,
      label: "settings.json",
    },
    languageId: "json",
    modelReference: reference,
    onLanguageError: error => errors.push(error),
  });
  session.layout({ width: 500, height: 240 });
  await waitFor(() => container.querySelectorAll(".zeta-alpha-editor-token.token-string").length > 0);
  await waitFor(() => container.querySelectorAll(".zeta-alpha-editor-decoration.warning-underline").length > 0);

  assert.equal(container.querySelectorAll(".zeta-alpha-editor").length, 1);
  assert.equal(container.querySelectorAll(".zeta-alpha-editor-input").length, 1);
  assert.equal(container.querySelectorAll(".zeta-alpha-editor-token.token-string").length > 0, true);
  assert.equal(container.querySelectorAll(".zeta-alpha-editor-bracket-level-1").length > 0, true);
  assert.equal(container.querySelectorAll(".zeta-alpha-editor-decoration.warning-underline").length > 0, true);
  assert.deepEqual(errors, []);

  session.textInput.element.dispatchEvent(new dom.window.InputEvent("beforeinput", {
    bubbles: true,
    cancelable: true,
    data: "x",
    inputType: "insertText",
  }));
  assert.equal(session.getValue().startsWith("x{"), true);

  session.dispose();
  await Promise.resolve();
  await Promise.resolve();
  assert.deepEqual(errors, []);
  assert.equal(container.children.length, 0);
  assert.throws(() => model.getText(), /disposed/);
  dom.window.close();
});

test("Alpha editor session derives indentation folds and projects their gutter controls", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  dom.window.HTMLCanvasElement.prototype.getContext = () => null;
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  const model = new TextModel("root\n  child\nafter");
  const reference = modelReference(URI.file("C:\\project\\fold.txt"), model);
  const session = new EditorSession({
    container,
    input: {
      resource: reference.resource,
      label: "fold.txt",
    },
    languageId: "plaintext",
    modelReference: reference,
  });
  session.layout({ width: 500, height: 120 });

  const foldToggle = container.querySelector<HTMLButtonElement>(".zeta-alpha-editor-fold-toggle");
  assert.ok(foldToggle);
  foldToggle.dispatchEvent(new dom.window.MouseEvent("pointerdown", { bubbles: true, cancelable: true }));
  assert.deepEqual([...container.querySelectorAll<HTMLElement>(".zeta-alpha-editor-line")].map(line => line.dataset.logicalLineIndex), ["0", "2"]);

  session.dispose();
  dom.window.close();
});

test("Alpha editor session honors a read-only input without disabling selection infrastructure", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  dom.window.HTMLCanvasElement.prototype.getContext = () => null;
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  const model = new TextModel("alpha");
  const reference = modelReference(URI.file("C:\\project\\preview.txt"), model);
  const session = new EditorSession({
    container,
    input: { resource: reference.resource, label: "preview.txt", readOnly: true },
    languageId: "plaintext",
    modelReference: reference,
  });

  const input = session.textInput.element;
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
  assert.equal(session.getValue(), "alpha");
  session.selections.setSelections(session.selections.selections);

  session.dispose();
  dom.window.close();
});

test("Alpha editor session announces save completion and forwards failures", async () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  dom.window.HTMLCanvasElement.prototype.getContext = () => null;
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  const model = new TextModel("alpha");
  const reference = modelReference(URI.file("C:\\project\\save.txt"), model);
  const errors: unknown[] = [];
  let fail = false;
  const session = new EditorSession({
    container,
    input: { resource: reference.resource, label: "save.txt" },
    languageId: "plaintext",
    modelReference: reference,
    onSave: async () => {
      if (fail) throw new Error("conflict");
    },
    onSaveError: error => errors.push(error),
  });

  session.textInput.element.dispatchEvent(new dom.window.KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    ctrlKey: true,
    key: "s",
  }));
  await waitFor(() => container.querySelector(".zeta-alpha-editor-accessibility-status")?.textContent === "Saved");

  fail = true;
  session.textInput.element.dispatchEvent(new dom.window.KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    ctrlKey: true,
    key: "s",
  }));
  await waitFor(() => container.querySelector(".zeta-alpha-editor-accessibility-status")?.textContent === "Save failed: conflict");
  assert.equal(errors.length, 1);

  session.dispose();
  dom.window.close();
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

async function waitFor(predicate: () => boolean): Promise<void> {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    if (predicate()) return;
    await nextTask();
  }
  assert.fail("Timed out waiting for Alpha editor projection");
}

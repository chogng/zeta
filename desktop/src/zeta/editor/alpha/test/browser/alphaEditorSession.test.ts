import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { URI } from "../../../../base/common/uri.js";
import { type AlphaTextModelReference } from "../../browser/alphaTextModelService.js";
import { TextModel } from "../../common/textModel.js";

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

const { AlphaEditorSession } = await import("../../browser/alphaEditorSession.js");

test.after(() => browserEnvironment.window.close());

test("Alpha editor session composes native input, local language analysis, and presentation", async () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  dom.window.HTMLCanvasElement.prototype.getContext = () => null;
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  const model = new TextModel("{\"name\": \"alpha\"");
  const reference = modelReference(URI.file("C:\\project\\settings.json"), model);
  const errors: unknown[] = [];
  const session = new AlphaEditorSession({
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
  assert.equal(container.children.length, 0);
  assert.throws(() => model.getText(), /disposed/);
  dom.window.close();
});

function modelReference(resource: URI, model: TextModel): AlphaTextModelReference {
  let disposed = false;
  const dispose = (): void => {
    if (disposed) return;
    disposed = true;
    model.dispose();
  };
  return {
    resource,
    model,
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

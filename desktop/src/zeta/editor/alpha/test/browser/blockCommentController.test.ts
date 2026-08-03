import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type AlphaTextMeasurer } from "../../browser/fontMetrics.js";
import { LanguageConfigurationRegistry } from "../../language/common/languageConfiguration.js";
import { EditorSelectionController } from "../../common/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../common/selection.js";
import { TextPosition } from "../../common/text.js";
import { TextModel } from "../../common/textModel.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
  window: browserEnvironment.window,
  document: browserEnvironment.window.document,
  Node: browserEnvironment.window.Node,
  Element: browserEnvironment.window.Element,
  HTMLElement: browserEnvironment.window.HTMLElement,
  Event: browserEnvironment.window.Event,
  KeyboardEvent: browserEnvironment.window.KeyboardEvent,
})) {
  Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { AlphaEditorViewport } = await import("../../browser/alphaEditorViewport.js");
const { AlphaBlockCommentController } = await import("../../browser/blockCommentController.js");

test("Block comment shortcut toggles the active language pair locally", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  using model = new TextModel("alpha beta");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(
    TextSelection.from(TextPosition.at(0, 6), TextPosition.at(0, 10)),
  ));
  using configurations = new LanguageConfigurationRegistry();
  using registration = configurations.register("typescript", {
    comments: { blockComment: { open: "/*", close: "*/" } },
  });
  using viewport = new AlphaEditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
  viewport.layout({ width: 200, height: 20 });
  const input = dom.window.document.createElement("textarea");
  container.append(input);
  using controller = new AlphaBlockCommentController(input, viewport, selections, { languageId: "typescript", configurations });

  const toggle = keydown(dom.window, "a", { shiftKey: true, altKey: true });
  input.dispatchEvent(toggle);
  assert.equal(toggle.defaultPrevented, true);
  assert.equal(model.getText(), "alpha /* beta */");
  input.dispatchEvent(keydown(dom.window, "a", { shiftKey: true, altKey: true }));
  assert.equal(model.getText(), "alpha beta");

  dom.window.close();
});

test("Block comment shortcut leaves languages without a block pair alone", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  using model = new TextModel("alpha");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
  using configurations = new LanguageConfigurationRegistry();
  using viewport = new AlphaEditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer() });
  const input = dom.window.document.createElement("textarea");
  container.append(input);
  using controller = new AlphaBlockCommentController(input, viewport, selections, { languageId: "plaintext", configurations });
  const toggle = keydown(dom.window, "a", { shiftKey: true, altKey: true });
  input.dispatchEvent(toggle);
  assert.equal(toggle.defaultPrevented, false);
  assert.equal(model.getText(), "alpha");

  dom.window.close();
});

class FixedTextMeasurer implements AlphaTextMeasurer {
  readonly horizontalPadding = 24;
  readonly contentLeftPadding = 12;

  refresh(): boolean {
    return false;
  }

  measureLineWidth(text: string): number {
    return text.length * 10;
  }
}

function keydown(targetWindow: typeof browserEnvironment.window, key: string, options: { readonly shiftKey?: boolean; readonly altKey?: boolean } = {}): KeyboardEvent {
  return new targetWindow.KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    key,
    shiftKey: options.shiftKey,
    altKey: options.altKey,
  }) as unknown as KeyboardEvent;
}

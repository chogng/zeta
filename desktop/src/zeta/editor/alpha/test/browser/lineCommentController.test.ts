import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type AlphaTextMeasurer } from "../../browser/fontMetrics.js";
import { LanguageConfigurationRegistry } from "../../common/languageConfiguration.js";
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
const { AlphaLineCommentController } = await import("../../browser/lineCommentController.js");

test("Line comment shortcut toggles current language comments through one editor transaction", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  using model = new TextModel("  alpha\nbeta");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(
    TextSelection.from(TextPosition.at(0, 0), TextPosition.at(1, 4)),
  ));
  using configurations = new LanguageConfigurationRegistry();
  using registration = configurations.register("typescript", {
    comments: { lineComment: "//" },
  });
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  viewport.layout({ width: 200, height: 40 });
  const input = dom.window.document.createElement("textarea");
  container.append(input);
  using controller = new AlphaLineCommentController(input, viewport, selections, {
    languageId: "typescript",
    configurations,
  });

  const toggle = keydown(dom.window, "/", { ctrlKey: true });
  input.dispatchEvent(toggle);
  assert.equal(toggle.defaultPrevented, true);
  assert.equal(model.getText(), "  // alpha\n// beta");
  input.dispatchEvent(keydown(dom.window, "/", { metaKey: true }));
  assert.equal(model.getText(), "  alpha\nbeta");

  dom.window.close();
});

test("Line comment shortcut ignores unsupported languages and invalid wiring", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  using model = new TextModel("alpha");
  using other = new TextModel("beta");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
  using otherSelections = new EditorSelectionController(other, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
  using configurations = new LanguageConfigurationRegistry();
  using viewport = new AlphaEditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer() });
  const input = dom.window.document.createElement("textarea");
  container.append(input);
  using controller = new AlphaLineCommentController(input, viewport, selections, { languageId: "plaintext", configurations });
  const toggle = keydown(dom.window, "/", { ctrlKey: true });
  input.dispatchEvent(toggle);
  assert.equal(toggle.defaultPrevented, false);
  assert.equal(model.getText(), "alpha");
  assert.throws(() => new AlphaLineCommentController(input, viewport, otherSelections, { languageId: "plaintext", configurations }), /must share one text model/);

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

function keydown(targetWindow: typeof browserEnvironment.window, key: string, options: { readonly ctrlKey?: boolean; readonly metaKey?: boolean } = {}): KeyboardEvent {
  return new targetWindow.KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    key,
    ctrlKey: options.ctrlKey,
    metaKey: options.metaKey,
  }) as unknown as KeyboardEvent;
}

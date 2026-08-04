import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { EditorIndentationKind } from "../../../indentation/common/indentation.js";
import { EditorSelectionController } from "../../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";
import { type AlphaTextMeasurer } from "../../../../browser/view/fontMetrics.js";

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

const { AlphaEditorViewport } = await import("../../../../browser/view/editorViewport.js");
const { AlphaLineOperationsController } = await import("../../browser/lineOperationsController.js");
const { AlphaTextInputController } = await import("../../../../browser/input/textInputController.js");

test.after(() => browserEnvironment.window.close());

test("Line operations controller routes selected-line Tab and Shift+Tab through indentation commands", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  using model = new TextModel("one\n  two\nthree");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  viewport.layout({ width: 400, height: 100 });
  using input = new AlphaTextInputController(viewport, selections);
  using controller = new AlphaLineOperationsController(input.element, viewport, selections, {
    indentation: { kind: EditorIndentationKind.Spaces, tabSize: 2 },
  });

  const range = TextSelection.from(TextPosition.at(0, 0), TextPosition.at(2, 5));
  selections.setSelections(TextSelectionSet.single(range));

  const indent = keyboardEvent(dom.window, "Tab");
  input.element.dispatchEvent(indent);
  assert.equal(indent.defaultPrevented, true);
  assert.equal(model.getText(), "  one\n    two\n  three");

  const outdent = keyboardEvent(dom.window, "Tab", { shiftKey: true });
  input.element.dispatchEvent(outdent);
  assert.equal(outdent.defaultPrevented, true);
  assert.equal(model.getText(), "one\n  two\nthree");
  dom.window.close();
});

test("Line operations controller validates model ownership and indentation options", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  using model = new TextModel("one");
  using otherModel = new TextModel("two");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
  using otherSelections = new EditorSelectionController(otherModel, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  const input = dom.window.document.createElement("textarea") as unknown as HTMLTextAreaElement;
  assert.throws(() => new AlphaLineOperationsController(input, viewport, otherSelections), /must share one text model/);
  assert.throws(() => new AlphaLineOperationsController(input, viewport, selections, {
    indentation: { tabSize: 0 },
  }), /tab size/);
  dom.window.close();
});

function keyboardEvent(targetWindow: typeof browserEnvironment.window, key: string, options: KeyboardEventInit = {}): KeyboardEvent {
  return new targetWindow.KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    key,
    ...options,
  }) as unknown as KeyboardEvent;
}

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

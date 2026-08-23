import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { OperatingSystem } from "../../../../../base/common/platform.js";
import { type TextMeasurer } from "../../../../browser/measurement/fontMetrics.js";
import { EditorSelectionController } from "../../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";
import { h } from "../../../../../base/browser/dom.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
  window: browserEnvironment.window,
  document: browserEnvironment.window.document,
  Node: browserEnvironment.window.Node,
  Element: browserEnvironment.window.Element,
  HTMLElement: browserEnvironment.window.HTMLElement,
  Event: browserEnvironment.window.Event,
  KeyboardEvent: browserEnvironment.window.KeyboardEvent,
})) Object.defineProperty(globalThis, name, { configurable: true, value });

const { EditorViewport } = await import("../../../../browser/view/editorViewport.js");
const { CursorUndoController, isCursorUndoChord } = await import("../../browser/cursorUndoController.js");

test.after(() => browserEnvironment.window.close());

test("Cursor undo restores macOS multi-cursor history without changing text", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  using model = new TextModel("one\ntwo");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
  using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
  const input = h(dom.window.document, "textarea") as unknown as HTMLTextAreaElement;
  container.append(input);
  using controller = new CursorUndoController(input, viewport, selections, { operatingSystem: OperatingSystem.Macintosh });
  selections.setCursorSelections(TextSelectionSet.withPrimary([
    TextSelection.collapsedAt(TextPosition.at(0, 0)),
    TextSelection.collapsedAt(TextPosition.at(1, 0)),
  ], 1));

  const undo = keydown(dom.window, "u", { metaKey: true });
  input.dispatchEvent(undo);
  assert.equal(undo.defaultPrevented, true);
  assert.deepEqual(selections.selections, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
  assert.equal(model.getText(), "one\ntwo");
  assert.equal(keydown(dom.window, "u", { ctrlKey: true }).defaultPrevented, false);
  dom.window.close();
});

test("Cursor undo chord keeps macOS and Windows modifiers distinct", () => {
  assert.equal(isCursorUndoChord(keydown(browserEnvironment.window, "u", { metaKey: true }), OperatingSystem.Macintosh), true);
  assert.equal(isCursorUndoChord(keydown(browserEnvironment.window, "u", { ctrlKey: true }), OperatingSystem.Macintosh), false);
  assert.equal(isCursorUndoChord(keydown(browserEnvironment.window, "u", { ctrlKey: true }), OperatingSystem.Windows), true);
});

class FixedTextMeasurer implements TextMeasurer {
  readonly horizontalPadding = 24;
  readonly contentLeftPadding = 12;
  refresh(): boolean { return false; }
  measureLineWidth(text: string): number { return text.length * 10; }
}

function keydown(targetWindow: typeof browserEnvironment.window, key: string, options: KeyboardEventInit = {}): KeyboardEvent {
  return new targetWindow.KeyboardEvent("keydown", { bubbles: true, cancelable: true, key, ...options }) as unknown as KeyboardEvent;
}

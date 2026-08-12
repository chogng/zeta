import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { OperatingSystem } from "../../../../../base/common/platform.js";
import { type TextMeasurer } from "../../../../browser/view/fontMetrics.js";
import { EditorSelectionController } from "../../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";

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

const { EditorViewport } = await import("../../../../browser/view/editorViewport.js");
const { TransposeController } = await import("../../browser/transposeController.js");

test("Transpose consumes Ctrl+T only for the VS Code macOS binding", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  using model = new TextModel("abc");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 1))));
  using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
  viewport.layout({ width: 200, height: 60 });
  const input = dom.window.document.createElement("textarea");
  container.append(input);
  using controller = new TransposeController(input, viewport, selections, { operatingSystem: OperatingSystem.Macintosh });

  const transpose = keydown(dom.window, "t", { ctrlKey: true });
  input.dispatchEvent(transpose);
  assert.equal(transpose.defaultPrevented, true);
  assert.equal(model.getText(), "bac");
  const other = keydown(dom.window, "t", { ctrlKey: true, metaKey: true });
  input.dispatchEvent(other);
  assert.equal(other.defaultPrevented, false);

  dom.window.close();
});

class FixedTextMeasurer implements TextMeasurer {
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

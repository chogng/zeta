import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { AlphaDecorationPresentation, createAlphaDecorationSource } from "../../browser/decorationPresentation.js";
import { type AlphaTextMeasurer } from "../../browser/fontMetrics.js";
import { TextDecorationCollection } from "../../common/decoration.js";
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
})) {
  Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { AlphaEditorViewport } = await import("../../browser/alphaEditorViewport.js");
const { AlphaOccurrenceHighlightController } = await import("../../browser/occurrenceHighlightController.js");

test("Occurrence highlight controller projects and clears current-word decorations without changing selections", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  using model = new TextModel("item itemized item\nitem");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 1))));
  using decorations = new TextDecorationCollection<void>(model);
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
    decorationSources: [createAlphaDecorationSource(decorations, () => AlphaDecorationPresentation.OccurrenceHighlight)],
  });
  using controller = new AlphaOccurrenceHighlightController(selections, decorations);
  viewport.layout({ width: 240, height: 40 });

  assert.equal(decorations.size, 3);
  assert.equal(viewport.element.querySelectorAll(".occurrence-highlight").length, 3);
  assert.deepEqual(selections.selections.primary, TextSelection.collapsedAt(TextPosition.at(0, 1)));

  selections.setSelections(TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 4))));
  assert.equal(decorations.size, 0);
  assert.equal(viewport.element.querySelectorAll(".occurrence-highlight").length, 0);
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

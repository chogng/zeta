import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { TextSelection, TextSelectionSet } from "../../common/selection.js";
import { TextPosition } from "../../common/text.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({ window: browserEnvironment.window, document: browserEnvironment.window.document, Node: browserEnvironment.window.Node, Element: browserEnvironment.window.Element, HTMLElement: browserEnvironment.window.HTMLElement, Event: browserEnvironment.window.Event })) {
  Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { isPositionInSelections } = await import("../../browser/pointerSelectionController.js");

test("Context-menu selection policy preserves only non-empty selected content", () => {
  const selections = TextSelectionSet.withPrimary([
    TextSelection.from(TextPosition.at(0, 1), TextPosition.at(1, 2)),
    TextSelection.collapsedAt(TextPosition.at(2, 1)),
  ], 0);
  assert.equal(isPositionInSelections(TextPosition.at(0, 1), selections), true);
  assert.equal(isPositionInSelections(TextPosition.at(1, 1), selections), true);
  assert.equal(isPositionInSelections(TextPosition.at(1, 2), selections), false);
  assert.equal(isPositionInSelections(TextPosition.at(2, 1), selections), false);
});

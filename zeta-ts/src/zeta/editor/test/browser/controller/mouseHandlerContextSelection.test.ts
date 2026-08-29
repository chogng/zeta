import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { Selection } from "../../../common/core/selection.js";
import { SelectionSet } from "../../../common/cursor/selectionSet.js";
import { Position } from "../../../common/core/position.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({ window: browserEnvironment.window, document: browserEnvironment.window.document, Node: browserEnvironment.window.Node, Element: browserEnvironment.window.Element, HTMLElement: browserEnvironment.window.HTMLElement, Event: browserEnvironment.window.Event })) {
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { isPositionInSelections } = await import("../../../browser/controller/mouseHandler.js");

test("Context-menu selection policy preserves only non-empty selected content", () => {
	const selections = SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (1) + 1), new Position((1) + 1, (2) + 1)),
		Selection.fromPositions(new Position((2) + 1, (1) + 1)),
	], 0);
	assert.equal(isPositionInSelections(new Position((0) + 1, (1) + 1), selections), true);
	assert.equal(isPositionInSelections(new Position((1) + 1, (1) + 1), selections), true);
	assert.equal(isPositionInSelections(new Position((1) + 1, (2) + 1), selections), false);
	assert.equal(isPositionInSelections(new Position((2) + 1, (1) + 1), selections), false);
});

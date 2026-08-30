import assert from "node:assert/strict";
import test from "node:test";
import { expandLineSelections } from "../../browser/lineSelection.js";
import { Selection } from "../../../../common/core/selection.js";
import { Position } from "../../../../common/core/position.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Line selection expands through successive physical lines and includes their line breaks", () => {
	using model = new TextModel("zero\none\ntwo");
	let selections: readonly Selection[] = [Selection.fromPositions(new Position((0) + 1, (2) + 1))];

	selections = expandLineSelections(model, selections);
	assert.deepEqual(selections[0]!, Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((1) + 1, (0) + 1)));
	selections = expandLineSelections(model, selections);
	assert.deepEqual(selections[0]!, Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((2) + 1, (0) + 1)));
	selections = expandLineSelections(model, selections);
	assert.deepEqual(selections[0]!, Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((2) + 1, (3) + 1)));
	assert.deepEqual(expandLineSelections(model, selections), selections);
});

test("Line selection normalizes reverse multi-selections while retaining the primary item", () => {
	using model = new TextModel("zero\none\ntwo\nthree");
	const selections = primaryFirst([
		Selection.fromPositions(new Position((2) + 1, (2) + 1), new Position((1) + 1, (1) + 1)),
		Selection.fromPositions(new Position((3) + 1, (4) + 1)),
	], 1);
	assert.deepEqual(expandLineSelections(model, selections), primaryFirst([
		Selection.fromPositions(new Position((1) + 1, (0) + 1), new Position((3) + 1, (0) + 1)),
		Selection.fromPositions(new Position((3) + 1, (0) + 1), new Position((3) + 1, (5) + 1)),
	], 1));
});

function primaryFirst<T>(items: readonly T[], primaryIndex: number): readonly T[] {
	if (primaryIndex === 0) return Object.freeze([...items]);
	return Object.freeze([items[primaryIndex]!, ...items.slice(0, primaryIndex), ...items.slice(primaryIndex + 1)]);
}

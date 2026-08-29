import assert from "node:assert/strict";
import test from "node:test";
import { expandLineSelections } from "../../browser/lineSelection.js";
import { Selection } from "../../../../common/core/selection.js";
import { SelectionSet } from "../../../../common/cursor/selectionSet.js";
import { Position } from "../../../../common/core/position.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Line selection expands through successive physical lines and includes their line breaks", () => {
	using model = new TextModel("zero\none\ntwo");
	let selections = SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (2) + 1)));

	selections = expandLineSelections(model, selections);
	assert.deepEqual(selections.primary, Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((1) + 1, (0) + 1)));
	selections = expandLineSelections(model, selections);
	assert.deepEqual(selections.primary, Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((2) + 1, (0) + 1)));
	selections = expandLineSelections(model, selections);
	assert.deepEqual(selections.primary, Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((2) + 1, (3) + 1)));
	assert.deepEqual(expandLineSelections(model, selections), selections);
});

test("Line selection normalizes reverse multi-selections while retaining the primary item", () => {
	using model = new TextModel("zero\none\ntwo\nthree");
	const selections = SelectionSet.withPrimary([
		Selection.fromPositions(new Position((2) + 1, (2) + 1), new Position((1) + 1, (1) + 1)),
		Selection.fromPositions(new Position((3) + 1, (4) + 1)),
	], 1);
	assert.deepEqual(expandLineSelections(model, selections), SelectionSet.withPrimary([
		Selection.fromPositions(new Position((1) + 1, (0) + 1), new Position((3) + 1, (0) + 1)),
		Selection.fromPositions(new Position((3) + 1, (0) + 1), new Position((3) + 1, (5) + 1)),
	], 1));
});

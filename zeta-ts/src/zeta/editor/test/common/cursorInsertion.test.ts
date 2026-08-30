import assert from "node:assert/strict";
import test from "node:test";
import { CursorMoveCommands } from '../../common/cursor/cursorMoveCommands.js';
import { Selection } from "../../common/core/selection.js";
import { Position } from "../../common/core/position.js";
import { TextModel } from "../../common/model/textModel.js";

test("Adjacent cursor insertion adds clamped carets and preserves existing selection state", () => {
	using model = new TextModel("zero\nx\nthree");
	let selections: readonly Selection[] = [Selection.fromPositions(new Position((1) + 1, (1) + 1))];

	selections = CursorMoveCommands.addCursorDown(model, selections);
	assert.deepEqual(selections, [
		Selection.fromPositions(new Position((1) + 1, (1) + 1)),
		Selection.fromPositions(new Position((2) + 1, (1) + 1)),
	]);
	selections = CursorMoveCommands.addCursorUp(model, selections);
	assert.deepEqual(selections, [
		Selection.fromPositions(new Position((1) + 1, (1) + 1)),
		Selection.fromPositions(new Position((0) + 1, (1) + 1)),
		Selection.fromPositions(new Position((2) + 1, (1) + 1)),
	]);
});

test("Adjacent cursor insertion rejects duplicate or overlapping carets", () => {
	using model = new TextModel("zero\none\ntwo");
	const selections = [Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((2) + 1, (3) + 1))];
	assert.equal(CursorMoveCommands.addCursorDown(model, selections), selections);
});

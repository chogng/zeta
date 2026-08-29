import assert from "node:assert/strict";
import test from "node:test";
import { CursorMoveCommands } from '../../common/cursor/cursorMoveCommands.js';
import { Selection } from "../../common/core/selection.js";
import { SelectionSet } from "../../common/cursor/selectionSet.js";
import { Position } from "../../common/core/position.js";
import { TextModel } from "../../common/model/textModel.js";

test("Adjacent cursor insertion adds clamped carets and preserves existing selection state", () => {
	using model = new TextModel("zero\nx\nthree");
	let selections = SelectionSet.single(Selection.fromPositions(new Position((1) + 1, (1) + 1)));

	selections = CursorMoveCommands.addCursorDown(model, selections);
	assert.deepEqual(selections, SelectionSet.withPrimary([
		Selection.fromPositions(new Position((1) + 1, (1) + 1)),
		Selection.fromPositions(new Position((2) + 1, (1) + 1)),
	], 1));
	selections = CursorMoveCommands.addCursorUp(model, selections);
	assert.deepEqual(selections, SelectionSet.withPrimary([
		Selection.fromPositions(new Position((1) + 1, (1) + 1)),
		Selection.fromPositions(new Position((2) + 1, (1) + 1)),
		Selection.fromPositions(new Position((0) + 1, (1) + 1)),
	], 2));
});

test("Adjacent cursor insertion rejects duplicate or overlapping carets", () => {
	using model = new TextModel("zero\none\ntwo");
	const selections = SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((2) + 1, (3) + 1)));
	assert.equal(CursorMoveCommands.addCursorDown(model, selections), selections);
});

test("Line-end cursor insertion follows selected physical lines and keeps the primary source first", () => {
	using model = new TextModel("zero\none\ntwo\nthree");
	const selections = SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (1) + 1), new Position((2) + 1, (0) + 1)),
		Selection.fromPositions(new Position((2) + 1, (0) + 1), new Position((3) + 1, (2) + 1)),
	], 1);
	assert.deepEqual(CursorMoveCommands.addCursorsToLineEnds(model, selections), SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (4) + 1)),
		Selection.fromPositions(new Position((1) + 1, (3) + 1)),
		Selection.fromPositions(new Position((2) + 1, (3) + 1)),
		Selection.fromPositions(new Position((3) + 1, (2) + 1)),
	], 2));
	const collapsed = SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1)));
	assert.equal(CursorMoveCommands.addCursorsToLineEnds(model, collapsed), collapsed);
});

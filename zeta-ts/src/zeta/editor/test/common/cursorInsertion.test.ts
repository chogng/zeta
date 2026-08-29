import assert from "node:assert/strict";
import test from "node:test";
import { CursorMoveCommands } from '../../common/cursor/cursorMoveCommands.js';
import { TextSelection, TextSelectionSet } from "../../common/core/selection.js";
import { TextPosition } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";

test("Adjacent cursor insertion adds clamped carets and preserves existing selection state", () => {
	using model = new TextModel("zero\nx\nthree");
	let selections = TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(1, 1)));

	selections = CursorMoveCommands.addCursorDown(model, selections);
	assert.deepEqual(selections, TextSelectionSet.withPrimary([
		TextSelection.collapsedAt(TextPosition.at(1, 1)),
		TextSelection.collapsedAt(TextPosition.at(2, 1)),
	], 1));
	selections = CursorMoveCommands.addCursorUp(model, selections);
	assert.deepEqual(selections, TextSelectionSet.withPrimary([
		TextSelection.collapsedAt(TextPosition.at(1, 1)),
		TextSelection.collapsedAt(TextPosition.at(2, 1)),
		TextSelection.collapsedAt(TextPosition.at(0, 1)),
	], 2));
});

test("Adjacent cursor insertion rejects duplicate or overlapping carets", () => {
	using model = new TextModel("zero\none\ntwo");
	const selections = TextSelectionSet.single(TextSelection.from(TextPosition.at(0, 0), TextPosition.at(2, 3)));
	assert.equal(CursorMoveCommands.addCursorDown(model, selections), selections);
});

test("Line-end cursor insertion follows selected physical lines and keeps the primary source first", () => {
	using model = new TextModel("zero\none\ntwo\nthree");
	const selections = TextSelectionSet.withPrimary([
		TextSelection.from(TextPosition.at(0, 1), TextPosition.at(2, 0)),
		TextSelection.from(TextPosition.at(2, 0), TextPosition.at(3, 2)),
	], 1);
	assert.deepEqual(CursorMoveCommands.addCursorsToLineEnds(model, selections), TextSelectionSet.withPrimary([
		TextSelection.collapsedAt(TextPosition.at(0, 4)),
		TextSelection.collapsedAt(TextPosition.at(1, 3)),
		TextSelection.collapsedAt(TextPosition.at(2, 3)),
		TextSelection.collapsedAt(TextPosition.at(3, 2)),
	], 2));
	const collapsed = TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0)));
	assert.equal(CursorMoveCommands.addCursorsToLineEnds(model, collapsed), collapsed);
});

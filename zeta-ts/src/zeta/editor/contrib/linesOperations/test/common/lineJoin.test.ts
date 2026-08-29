import assert from "node:assert/strict";
import test from "node:test";
import { CursorsController } from "../../../../common/cursor/cursor.js";
import { createJoinLinesCommand } from "../../common/lineJoin.js";
import { Selection } from "../../../../common/core/selection.js";
import { SelectionSet } from "../../../../common/cursor/selectionSet.js";
import { Position } from "../../../../common/core/position.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Join lines removes indentation, retains one separator, and restores undo", () => {
	using model = new TextModel("hello\n  world\nhello \n\tworld\n\nlast");
	using selections = new CursorsController(model, SelectionSet.single(caret(0, 2)));

	selections.execute(createJoinLinesCommand(model, selections.selections));
	assert.equal(model.getText(), "hello world\nhello \n\tworld\n\nlast");
	assert.deepEqual(selections.selections.primary, caret(0, 5));
	selections.undo();
	assert.equal(model.getText(), "hello\n  world\nhello \n\tworld\n\nlast");

	selections.setSelections(SelectionSet.single(caret(2, 1)));
	selections.execute(createJoinLinesCommand(model, selections.selections));
	assert.equal(model.getText(), "hello\n  world\nhello world\n\nlast");
	assert.deepEqual(selections.selections.primary, caret(2, 6));
});

test("Join lines joins ranges, reduces overlapping cursors, and preserves the primary group", () => {
	using model = new TextModel("zero\none\ntwo\nthree\nfour\nfive");
	using selections = new CursorsController(model, SelectionSet.withPrimary([
		caret(0, 1),
		caret(1, 2),
		Selection.fromPositions(new Position((3) + 1, (1) + 1), new Position((4) + 1, (2) + 1)),
	], 2));

	selections.execute(createJoinLinesCommand(model, selections.selections));
	assert.equal(model.getText(), "zero one\ntwo\nthree four\nfive");
	assert.deepEqual(selections.selections, SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (1) + 1), new Position((0) + 1, (7) + 1)),
		Selection.fromPositions(new Position((2) + 1, (1) + 1), new Position((2) + 1, (8) + 1)),
	], 1));
});

test("Join lines at the final line leaves collapsed and range selections unchanged", () => {
	using model = new TextModel("first\nlast");
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(
		new Position((1) + 1, (1) + 1),
		new Position((1) + 1, (3) + 1),
	)));
	selections.execute(createJoinLinesCommand(model, selections.selections));
	assert.equal(model.getText(), "first\nlast");
	assert.deepEqual(selections.selections.primary, Selection.fromPositions(
		new Position((1) + 1, (1) + 1),
		new Position((1) + 1, (3) + 1),
	));
});

function caret(lineIndex: number, columnIndex: number): Selection {
	return Selection.fromPositions(new Position((lineIndex) + 1, (columnIndex) + 1));
}

import assert from "node:assert/strict";
import test from "node:test";
import { CursorsController } from "../../../../common/cursor/cursor.js";
import { createJoinLinesCommand } from "../../common/lineJoin.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Join lines removes indentation, retains one separator, and restores undo", () => {
	using model = new TextModel("hello\n  world\nhello \n\tworld\n\nlast");
	using selections = new CursorsController(model, TextSelectionSet.single(caret(0, 2)));

	selections.execute(createJoinLinesCommand(model, selections.selections));
	assert.equal(model.getText(), "hello world\nhello \n\tworld\n\nlast");
	assert.deepEqual(selections.selections.primary, caret(0, 5));
	selections.undo();
	assert.equal(model.getText(), "hello\n  world\nhello \n\tworld\n\nlast");

	selections.setSelections(TextSelectionSet.single(caret(2, 1)));
	selections.execute(createJoinLinesCommand(model, selections.selections));
	assert.equal(model.getText(), "hello\n  world\nhello world\n\nlast");
	assert.deepEqual(selections.selections.primary, caret(2, 6));
});

test("Join lines joins ranges, reduces overlapping cursors, and preserves the primary group", () => {
	using model = new TextModel("zero\none\ntwo\nthree\nfour\nfive");
	using selections = new CursorsController(model, TextSelectionSet.withPrimary([
		caret(0, 1),
		caret(1, 2),
		TextSelection.from(TextPosition.at(3, 1), TextPosition.at(4, 2)),
	], 2));

	selections.execute(createJoinLinesCommand(model, selections.selections));
	assert.equal(model.getText(), "zero one\ntwo\nthree four\nfive");
	assert.deepEqual(selections.selections, TextSelectionSet.withPrimary([
		TextSelection.from(TextPosition.at(0, 1), TextPosition.at(0, 7)),
		TextSelection.from(TextPosition.at(2, 1), TextPosition.at(2, 8)),
	], 1));
});

test("Join lines at the final line leaves collapsed and range selections unchanged", () => {
	using model = new TextModel("first\nlast");
	using selections = new CursorsController(model, TextSelectionSet.single(TextSelection.from(
		TextPosition.at(1, 1),
		TextPosition.at(1, 3),
	)));
	selections.execute(createJoinLinesCommand(model, selections.selections));
	assert.equal(model.getText(), "first\nlast");
	assert.deepEqual(selections.selections.primary, TextSelection.from(
		TextPosition.at(1, 1),
		TextPosition.at(1, 3),
	));
});

function caret(lineIndex: number, columnIndex: number): TextSelection {
	return TextSelection.collapsedAt(TextPosition.at(lineIndex, columnIndex));
}

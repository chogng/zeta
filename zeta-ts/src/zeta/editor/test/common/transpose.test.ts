import assert from "node:assert/strict";
import test from "node:test";
import { CursorsController } from "../../common/cursor/cursor.js";
import { TextSelection, TextSelectionSet } from "../../common/core/selection.js";
import { TextPosition } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";
import { createTransposeCharactersCommand } from "../../contrib/transpose/common/transpose.js";

test("Transpose swaps complete graphemes for multiple carets and undoes atomically", () => {
	using model = new TextModel("a😊b\néx");
	using selections = new CursorsController(model, TextSelectionSet.withPrimary([
		caret(0, 1),
		caret(1, 3),
	], 1));
	const command = createTransposeCharactersCommand(model, selections.selections);
	assert.ok(command);
	selections.execute(command);
	assert.equal(model.getText(), "😊ab\nxé");
	assert.deepEqual(selections.selections, TextSelectionSet.withPrimary([
		caret(0, 3),
		caret(1, 3),
	], 1));
	selections.undo();
	assert.equal(model.getText(), "a😊b\néx");
});

test("Transpose swaps a line break with the following grapheme at a line start", () => {
	using model = new TextModel("ab\ncd");
	using selections = new CursorsController(model, TextSelectionSet.single(caret(1, 0)));
	const command = createTransposeCharactersCommand(model, selections.selections);
	assert.ok(command);
	selections.execute(command);
	assert.equal(model.getText(), "abc\nd");
	assert.deepEqual(selections.selections.primary, caret(1, 0));
});

test("Transpose ignores ranges and resolves overlapping cursor edits in favor of the primary cursor", () => {
	using model = new TextModel("abc");
	using selections = new CursorsController(model, TextSelectionSet.withPrimary([
		caret(0, 1),
		caret(0, 2),
		TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 1)),
	], 1));
	const command = createTransposeCharactersCommand(model, selections.selections);
	assert.ok(command);
	selections.execute(command);
	assert.equal(model.getText(), "acb");
	assert.deepEqual(selections.selections.primary, caret(0, 3));

	selections.setSelections(TextSelectionSet.single(caret(0, 0)));
	assert.equal(createTransposeCharactersCommand(model, selections.selections), undefined);
});

function caret(lineIndex: number, columnIndex: number): TextSelection {
	return TextSelection.collapsedAt(TextPosition.at(lineIndex, columnIndex));
}

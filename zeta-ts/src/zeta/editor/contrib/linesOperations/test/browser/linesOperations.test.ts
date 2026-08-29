import assert from "node:assert/strict";
import test from "node:test";
import { EditorSelectionController } from "../../../../common/cursor/cursor.js";
import { createDeleteLinesCommand, createDuplicateLinesCommand, createInsertLineCommand, createMoveLinesCommand, EditorLineDuplicateDirection, EditorLineInsertDirection, EditorLineMoveDirection } from "../../browser/linesOperations.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Delete lines removes selected physical line groups and keeps a valid final line", () => {
	using model = new TextModel("zero\none\ntwo\nthree\nfour");
	using selections = new EditorSelectionController(model, TextSelectionSet.withPrimary([
		caret(1, 1),
		TextSelection.from(TextPosition.at(3, 0), TextPosition.at(4, 0)),
	], 1));

	selections.execute(createDeleteLinesCommand(model, selections.selections));
	assert.equal(model.getText(), "zero\ntwo\nfour");
	selections.undo();
	assert.equal(model.getText(), "zero\none\ntwo\nthree\nfour");

	selections.setSelections(TextSelectionSet.single(caret(0, 0)));
	selections.execute(createDeleteLinesCommand(model, selections.selections));
	assert.equal(model.getText(), "one\ntwo\nthree\nfour");
	selections.setSelections(TextSelectionSet.single(caret(3, 0)));
	selections.execute(createDeleteLinesCommand(model, selections.selections));
	assert.equal(model.getText(), "one\ntwo\nthree");
	selections.setSelections(TextSelectionSet.single(caret(0, 0)));
	selections.execute(createDeleteLinesCommand(model, selections.selections));
	selections.execute(createDeleteLinesCommand(model, selections.selections));
	selections.execute(createDeleteLinesCommand(model, selections.selections));
	assert.equal(model.getText(), "");
});

test("Duplicate lines supports multi-line groups, document edges, and isolated undo", () => {
	using model = new TextModel("zero\none\ntwo\nthree");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(
		TextSelection.from(TextPosition.at(1, 0), TextPosition.at(3, 0)),
	));

	selections.execute(createDuplicateLinesCommand(
		model,
		selections.selections,
		EditorLineDuplicateDirection.Down,
	));
	assert.equal(model.getText(), "zero\none\ntwo\none\ntwo\nthree");
	selections.undo();
	assert.equal(model.getText(), "zero\none\ntwo\nthree");

	selections.setSelections(TextSelectionSet.single(caret(0, 1)));
	selections.execute(createDuplicateLinesCommand(
		model,
		selections.selections,
		EditorLineDuplicateDirection.Up,
	));
	assert.equal(model.getText(), "zero\nzero\none\ntwo\nthree");
	selections.setSelections(TextSelectionSet.single(caret(4, 2)));
	selections.execute(createDuplicateLinesCommand(
		model,
		selections.selections,
		EditorLineDuplicateDirection.Down,
	));
	assert.equal(model.getText(), "zero\nzero\none\ntwo\nthree\nthree");
});

test("Duplicate line command validates its direction before mutation", () => {
	using model = new TextModel("alpha");
	const selections = TextSelectionSet.single(caret(0, 0));
	assert.throws(() => createDuplicateLinesCommand(
		model,
		selections,
		"sideways" as EditorLineDuplicateDirection,
	), /Unknown editor line duplicate direction/);
	assert.equal(model.getText(), "alpha");
});

test("Move lines swaps selected groups with their neighboring rows and keeps directional selections", () => {
	using model = new TextModel("zero\none\ntwo\nthree\nfour");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(
		TextSelection.from(TextPosition.at(2, 3), TextPosition.at(1, 1)),
	));

	selections.execute(createMoveLinesCommand(model, selections.selections, EditorLineMoveDirection.Down));
	assert.equal(model.getText(), "zero\nthree\none\ntwo\nfour");
	assert.deepEqual(selections.selections.primary, TextSelection.from(
		TextPosition.at(3, 3),
		TextPosition.at(2, 1),
	));
	selections.execute(createMoveLinesCommand(model, selections.selections, EditorLineMoveDirection.Up));
	assert.equal(model.getText(), "zero\none\ntwo\nthree\nfour");
	assert.deepEqual(selections.selections.primary, TextSelection.from(
		TextPosition.at(2, 3),
		TextPosition.at(1, 1),
	));

	selections.setSelections(TextSelectionSet.single(caret(0, 0)));
	selections.execute(createMoveLinesCommand(model, selections.selections, EditorLineMoveDirection.Up));
	assert.equal(model.getText(), "zero\none\ntwo\nthree\nfour");
});

test("Move lines preserves disjoint selected groups and rejects invalid directions", () => {
	using model = new TextModel("zero\none\ntwo\nthree\nfour");
	using selections = new EditorSelectionController(model, TextSelectionSet.withPrimary([
		caret(1, 1),
		caret(3, 2),
	], 1));

	selections.execute(createMoveLinesCommand(model, selections.selections, EditorLineMoveDirection.Down));
	assert.equal(model.getText(), "zero\ntwo\none\nfour\nthree");
	assert.deepEqual(selections.selections, TextSelectionSet.withPrimary([
		caret(2, 1),
		caret(4, 2),
	], 1));
	selections.undo();
	assert.equal(model.getText(), "zero\none\ntwo\nthree\nfour");

	assert.throws(() => createMoveLinesCommand(
		model,
		selections.selections,
		"sideways" as EditorLineMoveDirection,
	), /Unknown editor line move direction/);
});

test("Insert lines deduplicates selected groups, places carets on blank rows, and undoes atomically", () => {
	using model = new TextModel("zero\none\ntwo\nthree");
	using selections = new EditorSelectionController(model, TextSelectionSet.withPrimary([
		caret(0, 1),
		TextSelection.from(TextPosition.at(2, 0), TextPosition.at(3, 0)),
	], 1));

	selections.execute(createInsertLineCommand(model, selections.selections, EditorLineInsertDirection.After));
	assert.equal(model.getText(), "zero\n\none\ntwo\n\nthree");
	assert.deepEqual(selections.selections, TextSelectionSet.withPrimary([
		caret(1, 0),
		caret(4, 0),
	], 1));
	selections.undo();
	assert.equal(model.getText(), "zero\none\ntwo\nthree");

	selections.setSelections(TextSelectionSet.single(caret(0, 0)));
	selections.execute(createInsertLineCommand(model, selections.selections, EditorLineInsertDirection.Before));
	assert.equal(model.getText(), "\nzero\none\ntwo\nthree");
	assert.deepEqual(selections.selections.primary, caret(0, 0));
	assert.throws(() => createInsertLineCommand(
		model,
		selections.selections,
		"nearby" as EditorLineInsertDirection,
	), /Unknown editor line insertion direction/);
});

function caret(lineIndex: number, columnIndex: number): TextSelection {
	return TextSelection.collapsedAt(TextPosition.at(lineIndex, columnIndex));
}

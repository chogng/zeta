import assert from "node:assert/strict";
import test from "node:test";
import { CursorsController } from "../../../../common/cursor/cursor.js";
import { createDeleteLinesCommand, createDuplicateLinesCommand, createInsertLineCommand, createMoveLinesCommand, EditorLineDuplicateDirection, EditorLineInsertDirection, EditorLineMoveDirection } from "../../browser/linesOperations.js";
import { Selection } from "../../../../common/core/selection.js";
import { Position } from "../../../../common/core/position.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Delete lines removes selected physical line groups and keeps a valid final line", () => {
	using model = new TextModel("zero\none\ntwo\nthree\nfour");
	using selections = new CursorsController(model, primaryFirst([
		caret(1, 1),
		Selection.fromPositions(new Position((3) + 1, (0) + 1), new Position((4) + 1, (0) + 1)),
	], 1));

	selections.execute(createDeleteLinesCommand(model, selections.selections));
	assert.equal(model.getText(), "zero\ntwo\nfour");
	selections.undo();
	assert.equal(model.getText(), "zero\none\ntwo\nthree\nfour");

	selections.setSelections([caret(0, 0)]);
	selections.execute(createDeleteLinesCommand(model, selections.selections));
	assert.equal(model.getText(), "one\ntwo\nthree\nfour");
	selections.setSelections([caret(3, 0)]);
	selections.execute(createDeleteLinesCommand(model, selections.selections));
	assert.equal(model.getText(), "one\ntwo\nthree");
	selections.setSelections([caret(0, 0)]);
	selections.execute(createDeleteLinesCommand(model, selections.selections));
	selections.execute(createDeleteLinesCommand(model, selections.selections));
	selections.execute(createDeleteLinesCommand(model, selections.selections));
	assert.equal(model.getText(), "");
});

test("Duplicate lines supports multi-line groups, document edges, and isolated undo", () => {
	using model = new TextModel("zero\none\ntwo\nthree");
	using selections = new CursorsController(model, [Selection.fromPositions(new Position((1) + 1, (0) + 1), new Position((3) + 1, (0) + 1))]);

	selections.execute(createDuplicateLinesCommand(
		model,
		selections.selections,
		EditorLineDuplicateDirection.Down,
	));
	assert.equal(model.getText(), "zero\none\ntwo\none\ntwo\nthree");
	selections.undo();
	assert.equal(model.getText(), "zero\none\ntwo\nthree");

	selections.setSelections([caret(0, 1)]);
	selections.execute(createDuplicateLinesCommand(
		model,
		selections.selections,
		EditorLineDuplicateDirection.Up,
	));
	assert.equal(model.getText(), "zero\nzero\none\ntwo\nthree");
	selections.setSelections([caret(4, 2)]);
	selections.execute(createDuplicateLinesCommand(
		model,
		selections.selections,
		EditorLineDuplicateDirection.Down,
	));
	assert.equal(model.getText(), "zero\nzero\none\ntwo\nthree\nthree");
});

test("Duplicate line command validates its direction before mutation", () => {
	using model = new TextModel("alpha");
	const selections = [caret(0, 0)];
	assert.throws(() => createDuplicateLinesCommand(
		model,
		selections,
		"sideways" as EditorLineDuplicateDirection,
	), /Unknown editor line duplicate direction/);
	assert.equal(model.getText(), "alpha");
});

test("Move lines swaps selected groups with their neighboring rows and keeps directional selections", () => {
	using model = new TextModel("zero\none\ntwo\nthree\nfour");
	using selections = new CursorsController(model, [Selection.fromPositions(new Position((2) + 1, (3) + 1), new Position((1) + 1, (1) + 1))]);

	selections.execute(createMoveLinesCommand(model, selections.selections, EditorLineMoveDirection.Down));
	assert.equal(model.getText(), "zero\nthree\none\ntwo\nfour");
	assert.deepEqual(selections.selections[0]!, Selection.fromPositions(
		new Position((3) + 1, (3) + 1),
		new Position((2) + 1, (1) + 1),
	));
	selections.execute(createMoveLinesCommand(model, selections.selections, EditorLineMoveDirection.Up));
	assert.equal(model.getText(), "zero\none\ntwo\nthree\nfour");
	assert.deepEqual(selections.selections[0]!, Selection.fromPositions(
		new Position((2) + 1, (3) + 1),
		new Position((1) + 1, (1) + 1),
	));

	selections.setSelections([caret(0, 0)]);
	selections.execute(createMoveLinesCommand(model, selections.selections, EditorLineMoveDirection.Up));
	assert.equal(model.getText(), "zero\none\ntwo\nthree\nfour");
});

test("Move lines preserves disjoint selected groups and rejects invalid directions", () => {
	using model = new TextModel("zero\none\ntwo\nthree\nfour");
	using selections = new CursorsController(model, primaryFirst([
		caret(1, 1),
		caret(3, 2),
	], 1));

	selections.execute(createMoveLinesCommand(model, selections.selections, EditorLineMoveDirection.Down));
	assert.equal(model.getText(), "zero\ntwo\none\nfour\nthree");
	assert.deepEqual(selections.selections, primaryFirst([
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
	using selections = new CursorsController(model, primaryFirst([
		caret(0, 1),
		Selection.fromPositions(new Position((2) + 1, (0) + 1), new Position((3) + 1, (0) + 1)),
	], 1));

	selections.execute(createInsertLineCommand(model, selections.selections, EditorLineInsertDirection.After));
	assert.equal(model.getText(), "zero\n\none\ntwo\n\nthree");
	assert.deepEqual(selections.selections, primaryFirst([
		caret(1, 0),
		caret(4, 0),
	], 1));
	selections.undo();
	assert.equal(model.getText(), "zero\none\ntwo\nthree");

	selections.setSelections([caret(0, 0)]);
	selections.execute(createInsertLineCommand(model, selections.selections, EditorLineInsertDirection.Before));
	assert.equal(model.getText(), "\nzero\none\ntwo\nthree");
	assert.deepEqual(selections.selections[0]!, caret(0, 0));
	assert.throws(() => createInsertLineCommand(
		model,
		selections.selections,
		"nearby" as EditorLineInsertDirection,
	), /Unknown editor line insertion direction/);
});

function caret(lineIndex: number, columnIndex: number): Selection {
	return Selection.fromPositions(new Position((lineIndex) + 1, (columnIndex) + 1));
}

function primaryFirst<T>(items: readonly T[], primaryIndex: number): readonly T[] {
	if (primaryIndex === 0) return Object.freeze([...items]);
	return Object.freeze([items[primaryIndex]!, ...items.slice(0, primaryIndex), ...items.slice(primaryIndex + 1)]);
}

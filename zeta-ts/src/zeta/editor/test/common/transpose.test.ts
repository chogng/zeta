import assert from "node:assert/strict";
import test from "node:test";
import { Selection } from "../../common/core/selection.js";
import { Position } from "../../common/core/position.js";
import { TextModel } from "../../common/model/textModel.js";
import { createTransposeCommand } from '../../contrib/linesOperations/browser/linesOperations.js';
import { createTestCursorsController } from './testCursorConfiguration.js';

test("Transpose swaps complete graphemes for multiple carets and undoes atomically", () => {
	using model = new TextModel("a😊b\néx");
	using selections = createTestCursorsController(model, primaryFirst([
		caret(0, 1),
		caret(1, 2),
	], 1));
	const command = createTransposeCommand(model, selections.selections);
	assert.ok(command);
	selections.execute(command);
	assert.equal(model.getText(), "😊ab\nxé");
	assert.deepEqual(selections.selections, primaryFirst([
		caret(0, 3),
		caret(1, 3),
	], 1));
	selections.undo();
	assert.equal(model.getText(), "a😊b\néx");
});

test('Transpose swaps the preceding grapheme with the following line break at a line end', () => {
	using model = new TextModel("ab\ncd");
	using selections = createTestCursorsController(model, [caret(0, 2)]);
	const command = createTransposeCommand(model, selections.selections);
	assert.ok(command);
	selections.execute(command);
	assert.equal(model.getText(), "a\nbcd");
	assert.deepEqual(selections.selections[0]!, caret(1, 1));
});

test("Transpose ignores ranges and resolves overlapping cursor edits in favor of the primary cursor", () => {
	using model = new TextModel("abc");
	using selections = createTestCursorsController(model, primaryFirst([
		caret(0, 1),
		caret(0, 2),
		Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (1) + 1)),
	], 1));
	const command = createTransposeCommand(model, selections.selections);
	assert.ok(command);
	selections.execute(command);
	assert.equal(model.getText(), "acb");
	assert.deepEqual(selections.selections[0]!, caret(0, 3));

	selections.setSelections([caret(0, 3)]);
	assert.equal(createTransposeCommand(model, selections.selections), undefined);
});

function caret(lineIndex: number, columnIndex: number): Selection {
	return Selection.fromPositions(new Position((lineIndex) + 1, (columnIndex) + 1));
}

function primaryFirst<T>(items: readonly T[], primaryIndex: number): readonly T[] {
	if (primaryIndex === 0) return Object.freeze([...items]);
	return Object.freeze([items[primaryIndex]!, ...items.slice(0, primaryIndex), ...items.slice(primaryIndex + 1)]);
}

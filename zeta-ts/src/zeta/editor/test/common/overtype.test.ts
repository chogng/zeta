import assert from "node:assert/strict";
import test from "node:test";
import { AutoClosingOvertypeOperation } from "../../common/cursor/cursorTypeEditOperations.js";
import { Selection } from "../../common/core/selection.js";
import { Position } from "../../common/core/position.js";
import { TextModel } from "../../common/model/textModel.js";
import { createTestCursorsController } from './testCursorConfiguration.js';

test("Overtype replaces complete following graphemes and stops at physical line ends", () => {
	using model = new TextModel("a😊b\ncd");
	using selections = createTestCursorsController(model, primaryFirst([caret(0, 1), caret(1, 1)], 0));
	selections.execute(AutoClosingOvertypeOperation.getEdits(model, selections.getSelections(), "XY"));
	assert.equal(model.getText(), "aXY\ncXY");
	assert.deepEqual(selections.getSelections(), primaryFirst([caret(0, 3), caret(1, 3)], 0));
	selections.undo();
	assert.equal(model.getText(), "a😊b\ncd");
});

function caret(lineIndex: number, columnIndex: number): Selection {
	return Selection.fromPositions(new Position((lineIndex) + 1, (columnIndex) + 1));
}

function primaryFirst<T>(items: readonly T[], primaryIndex: number): readonly T[] {
	if (primaryIndex === 0) return Object.freeze([...items]);
	return Object.freeze([items[primaryIndex]!, ...items.slice(0, primaryIndex), ...items.slice(primaryIndex + 1)]);
}

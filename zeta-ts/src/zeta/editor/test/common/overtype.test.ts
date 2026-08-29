import assert from "node:assert/strict";
import test from "node:test";
import { CursorsController } from "../../common/cursor/cursor.js";
import { AutoClosingOvertypeOperation } from "../../common/cursor/cursorTypeEditOperations.js";
import { TextSelection, TextSelectionSet } from "../../common/core/selection.js";
import { TextPosition } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";

test("Overtype replaces complete following graphemes and stops at physical line ends", () => {
	using model = new TextModel("a😊b\ncd");
	using selections = new CursorsController(model, TextSelectionSet.withPrimary([caret(0, 1), caret(1, 1)], 0));
	selections.execute(AutoClosingOvertypeOperation.getEdits(model, selections.selections, "XY"));
	assert.equal(model.getText(), "aXY\ncXY");
	assert.deepEqual(selections.selections, TextSelectionSet.withPrimary([caret(0, 3), caret(1, 3)], 0));
	selections.undo();
	assert.equal(model.getText(), "a😊b\ncd");
});

function caret(lineIndex: number, columnIndex: number): TextSelection {
	return TextSelection.collapsedAt(TextPosition.at(lineIndex, columnIndex));
}

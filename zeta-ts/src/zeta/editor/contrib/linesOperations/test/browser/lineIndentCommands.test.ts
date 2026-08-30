import assert from "node:assert/strict";
import test from "node:test";
import { EditorIndentationKind } from "../../../../common/core/misc/indentation.js";
import { CursorsController } from "../../../../common/cursor/cursor.js";
import { createLineIndentCommand, EditorLineIndentDirection } from "../../browser/lineIndentCommands.js";
import { Selection } from "../../../../common/core/selection.js";
import { Position } from "../../../../common/core/position.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("line indent touches selected rows once and preserves selection direction", () => {
	using model = new TextModel("one\n  two\nthree");
	const initial = primaryFirst([
		Selection.fromPositions(new Position((1) + 1, (3) + 1), new Position((0) + 1, (0) + 1)),
		Selection.fromPositions(new Position((1) + 1, (2) + 1)),
	], 0);
	using selections = new CursorsController(model, initial);

	selections.execute(createLineIndentCommand(model, selections.selections, EditorLineIndentDirection.Indent, {
		kind: EditorIndentationKind.Spaces,
		tabSize: 2,
	}));

	assert.equal(model.getText(), "  one\n    two\nthree");
	assert.deepEqual(selections.selections[0]!.getSelectionStart(), new Position((1) + 1, (5) + 1));
	assert.deepEqual(selections.selections[0]!.getPosition(), new Position((0) + 1, (2) + 1));
	selections.undo();
	assert.equal(model.getText(), "one\n  two\nthree");
	assert.deepEqual(selections.selections, initial);
});

test("line outdent removes one mixed indentation unit and excludes an ending line start", () => {
	using model = new TextModel("\talpha\n   beta\n  untouched");
	const initial = [Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((2) + 1, (0) + 1))];
	using selections = new CursorsController(model, initial);

	selections.execute(createLineIndentCommand(model, selections.selections, EditorLineIndentDirection.Outdent, { tabSize: 2 }));

	assert.equal(model.getText(), "alpha\n beta\n  untouched");
	assert.deepEqual(selections.selections[0]!.getEndPosition(), new Position((2) + 1, (0) + 1));
});

function primaryFirst<T>(items: readonly T[], primaryIndex: number): readonly T[] {
	if (primaryIndex === 0) return Object.freeze([...items]);
	return Object.freeze([items[primaryIndex]!, ...items.slice(0, primaryIndex), ...items.slice(primaryIndex + 1)]);
}

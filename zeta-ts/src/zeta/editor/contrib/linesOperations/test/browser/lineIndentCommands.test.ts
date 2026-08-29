import assert from "node:assert/strict";
import test from "node:test";
import { EditorIndentationKind } from "../../../../common/core/misc/indentation.js";
import { CursorsController } from "../../../../common/cursor/cursor.js";
import { createLineIndentCommand, EditorLineIndentDirection } from "../../browser/lineIndentCommands.js";
import { Selection } from "../../../../common/core/selection.js";
import { SelectionSet } from "../../../../common/cursor/selectionSet.js";
import { Position } from "../../../../common/core/position.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("line indent touches selected rows once and preserves selection direction", () => {
	using model = new TextModel("one\n  two\nthree");
	const initial = SelectionSet.withPrimary([
		Selection.fromPositions(new Position((1) + 1, (3) + 1), new Position((0) + 1, (0) + 1)),
		Selection.fromPositions(new Position((1) + 1, (2) + 1)),
	], 0);
	using selections = new CursorsController(model, initial);

	selections.execute(createLineIndentCommand(model, selections.selections, EditorLineIndentDirection.Indent, {
		kind: EditorIndentationKind.Spaces,
		tabSize: 2,
	}));

	assert.equal(model.getText(), "  one\n    two\nthree");
	assert.deepEqual(selections.selections.primary.getSelectionStart(), new Position((1) + 1, (5) + 1));
	assert.deepEqual(selections.selections.primary.getPosition(), new Position((0) + 1, (2) + 1));
	selections.undo();
	assert.equal(model.getText(), "one\n  two\nthree");
	assert.deepEqual(selections.selections, initial);
});

test("line outdent removes one mixed indentation unit and excludes an ending line start", () => {
	using model = new TextModel("\talpha\n   beta\n  untouched");
	const initial = SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((2) + 1, (0) + 1)));
	using selections = new CursorsController(model, initial);

	selections.execute(createLineIndentCommand(model, selections.selections, EditorLineIndentDirection.Outdent, { tabSize: 2 }));

	assert.equal(model.getText(), "alpha\n beta\n  untouched");
	assert.deepEqual(selections.selections.primary.getEndPosition(), new Position((2) + 1, (0) + 1));
});

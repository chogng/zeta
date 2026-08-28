import assert from "node:assert/strict";
import test from "node:test";
import { EditorIndentationKind } from "../../../../common/core/misc/indentation.js";
import { EditorSelectionController } from "../../../../common/cursor/editorSelectionController.js";
import { createLineIndentCommand, EditorLineIndentDirection } from "../../browser/lineIndentCommands.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("line indent touches selected rows once and preserves selection direction", () => {
	using model = new TextModel("one\n  two\nthree");
	const initial = TextSelectionSet.withPrimary([
		TextSelection.from(TextPosition.at(1, 3), TextPosition.at(0, 0)),
		TextSelection.collapsedAt(TextPosition.at(1, 2)),
	], 0);
	using selections = new EditorSelectionController(model, initial);

	selections.execute(createLineIndentCommand(model, selections.selections, EditorLineIndentDirection.Indent, {
		kind: EditorIndentationKind.Spaces,
		tabSize: 2,
	}));

	assert.equal(model.getText(), "  one\n    two\nthree");
	assert.deepEqual(selections.selections.primary.anchor, TextPosition.at(1, 5));
	assert.deepEqual(selections.selections.primary.active, TextPosition.at(0, 2));
	selections.undo();
	assert.equal(model.getText(), "one\n  two\nthree");
	assert.deepEqual(selections.selections, initial);
});

test("line outdent removes one mixed indentation unit and excludes an ending line start", () => {
	using model = new TextModel("\talpha\n   beta\n  untouched");
	const initial = TextSelectionSet.single(TextSelection.from(TextPosition.at(0, 0), TextPosition.at(2, 0)));
	using selections = new EditorSelectionController(model, initial);

	selections.execute(createLineIndentCommand(model, selections.selections, EditorLineIndentDirection.Outdent, { tabSize: 2 }));

	assert.equal(model.getText(), "alpha\n beta\n  untouched");
	assert.deepEqual(selections.selections.primary.range.end, TextPosition.at(2, 0));
});

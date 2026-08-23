import assert from "node:assert/strict";
import test from "node:test";
import { createToggleLineCommentCommand } from "../../common/lineCommentCommands.js";
import { EditorSelectionController } from "../../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition, TextRange } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Toggle line comment inserts after indentation and restores one isolated undo step", () => {
	using model = new TextModel("  alpha\n\tbeta\n\n gamma");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(
		TextSelection.from(TextPosition.at(0, 2), TextPosition.at(3, 1)),
	));

	selections.execute(createToggleLineCommentCommand(model, selections.selections, {
		lineComment: "//",
	}));

	assert.equal(model.getText(), "  // alpha\n\t// beta\n//\n // gamma");
	assert.deepEqual(
		selections.selections.primary,
		TextSelection.from(TextPosition.at(0, 5), TextPosition.at(3, 4)),
	);
	selections.undo();
	assert.equal(model.getText(), "  alpha\n\tbeta\n\n gamma");
	selections.redo();
	assert.equal(model.getText(), "  // alpha\n\t// beta\n//\n // gamma");
});

test("Toggle line comment removes only when all selected content lines are commented", () => {
	using model = new TextModel("// alpha\n  // beta\n\n// gamma");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(
		TextSelection.from(TextPosition.at(0, 0), TextPosition.at(3, 8)),
	));

	selections.execute(createToggleLineCommentCommand(model, selections.selections, {
		lineComment: "//",
	}));
	assert.equal(model.getText(), "alpha\n  beta\n\ngamma");

	model.applyEdits([{
		range: TextRange.emptyAt(TextPosition.at(1, 0)),
		text: "x",
	}]);
	selections.setSelections(TextSelectionSet.single(
		TextSelection.from(TextPosition.at(0, 0), TextPosition.at(1, 7)),
	));
	selections.execute(createToggleLineCommentCommand(model, selections.selections, {
		lineComment: "//",
		insertSpace: false,
	}));
	assert.equal(model.getText(), "//alpha\n//x  beta\n\ngamma");
});

test("Toggle line comment validates its contract before mutating", () => {
	using model = new TextModel("alpha");
	const selections = TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0)));
	assert.throws(() => createToggleLineCommentCommand(model, selections, {
		lineComment: "",
	}), /non-empty/);
	assert.throws(() => createToggleLineCommentCommand(model, selections, {
		lineComment: "//\n",
	}), /single-line/);
	assert.equal(model.getText(), "alpha");
});

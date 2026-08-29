import assert from "node:assert/strict";
import test from "node:test";
import { createToggleLineCommentCommand } from "../../common/lineCommentCommands.js";
import { CursorsController } from "../../../../common/cursor/cursor.js";
import { Selection } from "../../../../common/core/selection.js";
import { SelectionSet } from "../../../../common/cursor/selectionSet.js";
import { Position } from "../../../../common/core/position.js";
import { Range } from "../../../../common/core/range.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Toggle line comment inserts after indentation and restores one isolated undo step", () => {
	using model = new TextModel("  alpha\n\tbeta\n\n gamma");
	using selections = new CursorsController(model, SelectionSet.single(
		Selection.fromPositions(new Position((0) + 1, (2) + 1), new Position((3) + 1, (1) + 1)),
	));

	selections.execute(createToggleLineCommentCommand(model, selections.selections, {
		lineComment: "//",
	}));

	assert.equal(model.getText(), "  // alpha\n\t// beta\n//\n // gamma");
	assert.deepEqual(
		selections.selections.primary,
		Selection.fromPositions(new Position((0) + 1, (5) + 1), new Position((3) + 1, (4) + 1)),
	);
	selections.undo();
	assert.equal(model.getText(), "  alpha\n\tbeta\n\n gamma");
	selections.redo();
	assert.equal(model.getText(), "  // alpha\n\t// beta\n//\n // gamma");
});

test("Toggle line comment removes only when all selected content lines are commented", () => {
	using model = new TextModel("// alpha\n  // beta\n\n// gamma");
	using selections = new CursorsController(model, SelectionSet.single(
		Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((3) + 1, (8) + 1)),
	));

	selections.execute(createToggleLineCommentCommand(model, selections.selections, {
		lineComment: "//",
	}));
	assert.equal(model.getText(), "alpha\n  beta\n\ngamma");

	model.applyEdits([{
		range: Range.fromPositions(new Position((1) + 1, (0) + 1)),
		text: "x",
	}]);
	selections.setSelections(SelectionSet.single(
		Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((1) + 1, (7) + 1)),
	));
	selections.execute(createToggleLineCommentCommand(model, selections.selections, {
		lineComment: "//",
		insertSpace: false,
	}));
	assert.equal(model.getText(), "//alpha\n//x  beta\n\ngamma");
});

test("Toggle line comment validates its contract before mutating", () => {
	using model = new TextModel("alpha");
	const selections = SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1)));
	assert.throws(() => createToggleLineCommentCommand(model, selections, {
		lineComment: "",
	}), /non-empty/);
	assert.throws(() => createToggleLineCommentCommand(model, selections, {
		lineComment: "//\n",
	}), /single-line/);
	assert.equal(model.getText(), "alpha");
});

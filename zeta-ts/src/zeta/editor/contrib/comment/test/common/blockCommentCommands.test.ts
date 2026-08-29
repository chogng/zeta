import assert from "node:assert/strict";
import test from "node:test";
import { createToggleBlockCommentCommand } from "../../common/blockCommentCommands.js";
import { CursorsController } from "../../../../common/cursor/cursor.js";
import { Selection } from "../../../../common/core/selection.js";
import { SelectionSet } from "../../../../common/cursor/selectionSet.js";
import { Position } from "../../../../common/core/position.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Block comments wrap and unwrap directional selections in isolated undo steps", () => {
	using model = new TextModel("alpha beta");
	using selections = new CursorsController(model, SelectionSet.single(
		Selection.fromPositions(new Position((0) + 1, (10) + 1), new Position((0) + 1, (6) + 1)),
	));
	const options = { open: "/*", close: "*/" };

	selections.execute(createToggleBlockCommentCommand(model, selections.selections, options));
	assert.equal(model.getText(), "alpha /* beta */");
	assert.deepEqual(selections.selections.primary, Selection.fromPositions(
		new Position((0) + 1, (13) + 1),
		new Position((0) + 1, (9) + 1),
	));
	selections.execute(createToggleBlockCommentCommand(model, selections.selections, options));
	assert.equal(model.getText(), "alpha beta");
	assert.deepEqual(selections.selections.primary, Selection.fromPositions(
		new Position((0) + 1, (10) + 1),
		new Position((0) + 1, (6) + 1),
	));
	selections.undo();
	assert.equal(model.getText(), "alpha /* beta */");
});

test("Block comments place collapsed carets inside the generated pair and support independent cursors", () => {
	using model = new TextModel("one two");
	using selections = new CursorsController(model, SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (0) + 1)),
		Selection.fromPositions(new Position((0) + 1, (4) + 1)),
	], 1));
	selections.execute(createToggleBlockCommentCommand(model, selections.selections, {
		open: "/*",
		close: "*/",
	}));
	assert.equal(model.getText(), "/* */one /* */two");
	assert.deepEqual(selections.selections, SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (3) + 1)),
		Selection.fromPositions(new Position((0) + 1, (12) + 1)),
	], 1));
});

test("Block comments reject overlapping selections and invalid tokens before mutation", () => {
	using model = new TextModel("alpha");
	const options = { open: "/*", close: "*/" };
	const overlap = SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (3) + 1)),
		Selection.fromPositions(new Position((0) + 1, (2) + 1), new Position((0) + 1, (5) + 1)),
	], 0);
	assert.throws(() => createToggleBlockCommentCommand(model, overlap, options), /must not overlap/);
	assert.throws(() => createToggleBlockCommentCommand(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))), {
		open: "",
		close: "*/",
	}), /non-empty/);
	assert.equal(model.getText(), "alpha");
});

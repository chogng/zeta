import assert from "node:assert/strict";
import test from "node:test";
import { ColumnSelection } from "../../common/cursor/cursorColumnSelection.js";
import { Selection } from "../../common/core/selection.js";
import { Position } from "../../common/core/position.js";
import { TextModel } from "../../common/model/textModel.js";

test("Column selection creates directional same-column selections for every physical line", () => {
	using model = new TextModel("abcdef\nab\n12345\nxy");
	const selections = ColumnSelection.columnSelect(
		model,
		new Position((3) + 1, (2) + 1),
		new Position((0) + 1, (5) + 1),
	);

	assert.deepEqual(selections.selections, [
		Selection.fromPositions(new Position((0) + 1, (2) + 1), new Position((0) + 1, (5) + 1)),
		Selection.fromPositions(new Position((1) + 1, (2) + 1), new Position((1) + 1, (2) + 1)),
		Selection.fromPositions(new Position((2) + 1, (2) + 1), new Position((2) + 1, (5) + 1)),
		Selection.fromPositions(new Position((3) + 1, (2) + 1), new Position((3) + 1, (2) + 1)),
	]);
	assert.equal(selections.primaryIndex, 0);
});

test("Column selection validates both positions against its text model", () => {
	using model = new TextModel("one");
	assert.throws(() => ColumnSelection.columnSelect(model, new Position((0) + 1, (0) + 1), new Position((1) + 1, (0) + 1)), /lineIndex/);
});

import assert from "node:assert/strict";
import test from "node:test";
import { ColumnSelection } from "../../common/cursor/cursorColumnSelection.js";
import { Selection } from "../../common/core/selection.js";
import { Position } from "../../common/core/position.js";
import { TextModel } from "../../common/model/textModel.js";
import { TestLanguageConfigurationService } from './modes/testLanguageConfigurationService.js';
import { createTestCursorConfiguration } from "./testCursorConfiguration.js";

test("Column selection creates directional same-column selections for every physical line", () => {
	using model = new TextModel("abcdef\nab\n12345\nxy");
	using configurations = new TestLanguageConfigurationService();
	const config = createTestCursorConfiguration(model, configurations);
	const result = ColumnSelection.columnSelect(
		config,
		model,
		4,
		2,
		1,
		5,
	);

	assert.deepEqual(result.viewStates.map(state => state.selection), [
		Selection.fromPositions(new Position(4, 3), new Position(4, 3)),
		Selection.fromPositions(new Position(3, 3), new Position(3, 6)),
		Selection.fromPositions(new Position(2, 3), new Position(2, 3)),
		Selection.fromPositions(new Position(1, 3), new Position(1, 6)),
	]);
	assert.equal(result.reversed, true);
});

test("Column selection validates both positions against its text model", () => {
	using model = new TextModel("one");
	using configurations = new TestLanguageConfigurationService();
	const config = createTestCursorConfiguration(model, configurations);
	assert.throws(() => ColumnSelection.columnSelect(config, model, 1, 0, 2, 0), /lineNumber/);
});

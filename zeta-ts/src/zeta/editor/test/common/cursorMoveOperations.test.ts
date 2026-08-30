import assert from 'node:assert/strict';
import test from 'node:test';
import { MoveOperations } from '../../common/cursor/cursorMoveOperations.js';
import { SelectionStartKind, SingleCursorState } from '../../common/cursorCommon.js';
import { Position } from '../../common/core/position.js';
import { Range } from '../../common/core/range.js';
import { Selection } from '../../common/core/selection.js';
import { TestLanguageConfigurationService } from './modes/testLanguageConfigurationService.js';
import { TextModel } from '../../common/model/textModel.js';
import { createTestCursorConfiguration } from './testCursorConfiguration.js';

test('MoveOperations collapses a selection at the movement edge', () => {
	using model = new TextModel('abcdef');
	using languageConfigurationService = new TestLanguageConfigurationService();
	const configuration = createTestCursorConfiguration(model, languageConfigurationService);
	const cursor = cursorState(1, 2, 1, 5);

	const left = MoveOperations.moveLeft(configuration, model, cursor, false, 1);
	const right = MoveOperations.moveRight(configuration, model, cursor, false, 1);

	assert.deepEqual({
		left: left.position,
		right: right.position,
		leftHasSelection: left.hasSelection(),
		rightHasSelection: right.hasSelection(),
	}, {
		left: new Position(1, 2),
		right: new Position(1, 5),
		leftHasSelection: false,
		rightHasSelection: false,
	});
});

test('MoveOperations preserves the target visible column across a short line', () => {
	using model = new TextModel('abcdef\nx\nabcdef');
	using languageConfigurationService = new TestLanguageConfigurationService();
	const configuration = createTestCursorConfiguration(model, languageConfigurationService);
	const first = cursorState(1, 6, 1, 6);

	const shortLine = MoveOperations.moveDown(configuration, model, first, false, 1);
	const restoredColumn = MoveOperations.moveDown(configuration, model, shortLine, false, 1);

	assert.deepEqual({
		shortLine: shortLine.position,
		shortLineLeftover: shortLine.leftoverVisibleColumns,
		restoredColumn: restoredColumn.position,
		restoredLeftover: restoredColumn.leftoverVisibleColumns,
	}, {
		shortLine: new Position(2, 2),
		shortLineLeftover: 4,
		restoredColumn: new Position(3, 6),
		restoredLeftover: 0,
	});
});

test('MoveOperations toggles line start between indentation and the minimum column', () => {
	using model = new TextModel('   value');
	using languageConfigurationService = new TestLanguageConfigurationService();
	const configuration = createTestCursorConfiguration(model, languageConfigurationService);
	const cursor = cursorState(1, 7, 1, 7);

	const indentation = MoveOperations.moveToBeginningOfLine(configuration, model, cursor, false);
	const lineStart = MoveOperations.moveToBeginningOfLine(configuration, model, indentation, false);

	assert.deepEqual({
		indentation: indentation.position,
		lineStart: lineStart.position,
	}, {
		indentation: new Position(1, 4),
		lineStart: new Position(1, 1),
	});
});

test('MoveOperations translates both selection endpoints with their visible columns', () => {
	using model = new TextModel('abcdef\nx\nabcdef');
	using languageConfigurationService = new TestLanguageConfigurationService();
	const configuration = createTestCursorConfiguration(model, languageConfigurationService);
	const cursor = cursorState(1, 6, 2, 2);

	const down = MoveOperations.translateDown(configuration, model, cursor);
	const up = MoveOperations.translateUp(configuration, model, down);

	assert.deepEqual({
		downSelection: down.selection,
		downStartLeftover: down.selectionStartLeftoverVisibleColumns,
		upSelection: up.selection,
		upStartLeftover: up.selectionStartLeftoverVisibleColumns,
	}, {
		downSelection: new Selection(2, 2, 3, 2),
		downStartLeftover: 4,
		upSelection: cursor.selection,
		upStartLeftover: 0,
	});
});

function cursorState(anchorLineNumber: number, anchorColumn: number, lineNumber: number, column: number): SingleCursorState {
	return new SingleCursorState(
		new Range(anchorLineNumber, anchorColumn, anchorLineNumber, anchorColumn),
		SelectionStartKind.Simple,
		0,
		new Position(lineNumber, column),
		0,
	);
}

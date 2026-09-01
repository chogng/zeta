import assert from 'node:assert/strict';
import test from 'node:test';
import { DeleteOperations } from '../../common/cursor/cursorDeleteOperations.js';
import { EditOperationType } from '../../common/cursorCommon.js';
import { Position } from '../../common/core/position.js';
import { Selection } from '../../common/core/selection.js';
import { TextModel } from '../../common/model/textModel.js';
import { TestLanguageConfigurationService } from './modes/testLanguageConfigurationService.js';
import { createTestCursorConfiguration } from './testCursorConfiguration.js';
import { createTestCursorsController } from './testCursorConfiguration.js';

test('delete-left removes one complete grapheme', () => {
	using model = new TextModel('a😀b');
	using languages = new TestLanguageConfigurationService();
	using controller = createTestCursorsController(model, [Selection.fromPositions(new Position(1, 4))]);
	const operation = DeleteOperations.deleteLeft(EditOperationType.Other, createTestCursorConfiguration(model, languages), model, [...controller.selections], []);
	controller.executeCommands(operation[1]);
	assert.equal(model.getText(), 'ab');
	assert.deepEqual(controller.selections, [Selection.fromPositions(new Position(1, 2))]);
});

test('delete-right joins adjacent physical lines', () => {
	using model = new TextModel('a\nb');
	using languages = new TestLanguageConfigurationService();
	using controller = createTestCursorsController(model, [Selection.fromPositions(new Position(1, 2))]);
	const operation = DeleteOperations.deleteRight(EditOperationType.Other, createTestCursorConfiguration(model, languages), model, [...controller.selections]);
	controller.executeCommands(operation[1]);
	assert.equal(model.getText(), 'ab');
	assert.deepEqual(controller.selections, [Selection.fromPositions(new Position(1, 2))]);
});

import assert from 'node:assert/strict';
import test from 'node:test';
import { EditorCursorNavigationCommand, EditorCursorNavigationMode, MoveOperations } from '../../../common/cursor/cursorMoveOperations.js';
import { Selection } from '../../../common/core/selection.js';
import { TextModel } from '../../../common/model/textModel.js';

test('vertical navigation retains the requested visual column', () => {
	using model = new TextModel('12345\n12\n12345');
	const result = MoveOperations.navigate(model, [new Selection(1, 5, 1, 5)], {
		command: EditorCursorNavigationCommand.LineDown,
		mode: EditorCursorNavigationMode.Move,
	});
	assert.deepEqual([result.selections[0]!.positionLineNumber, result.selections[0]!.positionColumn], [2, 3]);
	assert.deepEqual(result.preferredColumns, [5]);
});

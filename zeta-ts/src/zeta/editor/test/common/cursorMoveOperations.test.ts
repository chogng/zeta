import assert from 'node:assert/strict';
import test from 'node:test';
import { EditorCursorNavigationCommand, EditorCursorNavigationMode, MoveOperations } from '../../common/cursor/cursorMoveOperations.js';
import { Selection } from '../../common/core/selection.js';
import { TextModel } from '../../common/model/textModel.js';

test('MoveOperations collapses a selection at the requested edge', () => {
	using model = new TextModel('abcdef');
	const source = [new Selection(1, 2, 1, 5)];
	const left = MoveOperations.navigate(model, source, { command: EditorCursorNavigationCommand.CharacterLeft, mode: EditorCursorNavigationMode.Move });
	const right = MoveOperations.navigate(model, source, { command: EditorCursorNavigationCommand.CharacterRight, mode: EditorCursorNavigationMode.Move });
	assert.deepEqual([left.selections[0]!.positionColumn, right.selections[0]!.positionColumn], [2, 5]);
});

test('MoveOperations extends word navigation without changing the model', () => {
	using model = new TextModel('alpha beta');
	const result = MoveOperations.navigate(model, [new Selection(1, 11, 1, 11)], {
		command: EditorCursorNavigationCommand.WordLeft,
		mode: EditorCursorNavigationMode.Extend,
	});
	assert.deepEqual([result.selections[0]!.startColumn, result.selections[0]!.endColumn], [7, 11]);
});

import assert from 'node:assert/strict';
import test from 'node:test';
import { DeleteOperations } from '../../common/cursor/cursorDeleteOperations.js';
import { Position } from '../../common/core/position.js';
import { Range } from '../../common/core/range.js';
import { Selection } from '../../common/core/selection.js';
import { TextModel } from '../../common/model/textModel.js';

test('delete-left removes one complete grapheme', () => {
	using model = new TextModel('a😀b');
	const command = DeleteOperations.deleteLeft(model, [Selection.fromPositions(new Position(1, 4))]);
	assert.deepEqual(command.edits, [{ range: new Range(1, 2, 1, 4), text: '' }]);
});

test('delete-right joins adjacent physical lines', () => {
	using model = new TextModel('a\nb');
	const command = DeleteOperations.deleteRight(model, [Selection.fromPositions(new Position(1, 2))]);
	assert.deepEqual(command.edits, [{ range: new Range(1, 2, 2, 1), text: '' }]);
});

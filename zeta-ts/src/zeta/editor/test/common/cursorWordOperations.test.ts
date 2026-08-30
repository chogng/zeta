import assert from 'node:assert/strict';
import test from 'node:test';
import { WordOperations } from '../../common/cursor/cursorWordOperations.js';
import { Position } from '../../common/core/position.js';
import { Range } from '../../common/core/range.js';
import { Selection } from '../../common/core/selection.js';
import { TextModel } from '../../common/model/textModel.js';

test('WordOperations resolves Unicode word ranges at a position', () => {
	using model = new TextModel('alpha beta');
	assert.deepEqual(WordOperations.getWordSelectionRange(model, new Position(1, 8)), new Range(1, 7, 1, 11));
});

test('WordOperations creates one delete command per selection set', () => {
	using model = new TextModel('alpha beta');
	const command = WordOperations.deleteWordLeft(model, [Selection.fromPositions(new Position(1, 11))]);
	assert.deepEqual(command.edits, [{ range: new Range(1, 7, 1, 11), text: '' }]);
});

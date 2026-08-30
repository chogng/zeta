import assert from 'node:assert/strict';
import test from 'node:test';
import { CursorsController } from '../../../common/cursor/cursor.js';
import { Range } from '../../../common/core/range.js';
import { Selection } from '../../../common/core/selection.js';
import { TextModel } from '../../../common/model/textModel.js';

test('cursor edit updates the shared model and resulting selection together', () => {
	using model = new TextModel('bc');
	using cursors = new CursorsController(model, [new Selection(1, 1, 1, 1)]);
	cursors.execute({
		edits: [{ range: new Range(1, 1, 1, 1), text: 'a' }],
		selectionsAfter: [{ anchorOffset: 1, activeOffset: 1 }],
		primarySelectionIndex: 0,
	});
	assert.equal(model.getValue(), 'abc');
	assert.equal(cursors.selections[0]!.positionColumn, 2);
});

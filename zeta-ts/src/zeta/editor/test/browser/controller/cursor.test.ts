import assert from 'node:assert/strict';
import test from 'node:test';
import { CursorsController } from '../../../common/cursor/cursor.js';
import { Selection } from '../../../common/core/selection.js';
import { TextModel } from '../../../common/model/textModel.js';

test('CursorsController owns per-editor selections and cursor-only undo', () => {
	using model = new TextModel('alpha\nbeta');
	const initial = [new Selection(1, 1, 1, 1)];
	using cursors = new CursorsController(model, initial);
	cursors.setCursorSelections([new Selection(2, 3, 2, 3)]);
	assert.equal(cursors.selections[0]!.positionLineNumber, 2);
	assert.equal(cursors.undoCursorOperation(), true);
	assert.equal(cursors.selections[0]!.positionLineNumber, 1);
});

test('CursorsController validates read-only ownership', () => {
	using model = new TextModel('text');
	using cursors = new CursorsController(model, [new Selection(1, 1, 1, 1)], { readOnly: true });
	assert.equal(cursors.readOnly, true);
	assert.equal(cursors.textModel, model);
});

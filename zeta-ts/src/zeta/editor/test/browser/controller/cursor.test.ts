import assert from 'node:assert/strict';
import test from 'node:test';
import { CursorsController } from '../../../common/cursor/cursor.js';
import { Selection } from '../../../common/core/selection.js';
import { TextModel } from '../../../common/model/textModel.js';
import { createTestCursorsController } from '../../common/testCursorConfiguration.js';

test('CursorsController owns per-editor selections and cursor-only undo', () => {
	using model = new TextModel('alpha\nbeta');
	const initial = [new Selection(1, 1, 1, 1), new Selection(2, 3, 2, 3)];
	using cursors = createTestCursorsController(model, initial);
	assert.deepEqual({
		selection: cursors.getSelection(),
		position: cursors.getPosition(),
		states: cursors.getCursorStates().map(state => state.modelState.selection),
		primary: cursors.getPrimaryCursorState().modelState.selection,
		top: cursors.getTopMostViewPosition(),
		bottom: cursors.getBottomMostViewPosition(),
		lastAdded: cursors.getLastAddedCursorIndex(),
	}, {
		selection: initial[0],
		position: initial[0]!.getPosition(),
		states: initial,
		primary: initial[0],
		top: initial[0]!.getPosition(),
		bottom: initial[1]!.getPosition(),
		lastAdded: 1,
	});
	cursors.setCursorSelections([new Selection(2, 3, 2, 3)]);
	assert.equal(cursors.undoCursorOperation(), true);
	assert.deepEqual(cursors.getSelections(), initial);
});

test('CursorsController validates read-only ownership', () => {
	using model = new TextModel('text');
	using cursors = createTestCursorsController(model, [new Selection(1, 1, 1, 1)], { readOnly: true });
	assert.equal(cursors.readOnly, true);
	assert.equal(cursors.context.model, model);
});

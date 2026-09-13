import assert from 'node:assert/strict';
import test from 'node:test';
import { Range } from '../../../common/core/range.js';
import { Selection } from '../../../common/core/selection.js';
import { TextModel } from '../../../common/model/textModel.js';
import { ReplaceCommand } from '../../../common/commands/replaceCommand.js';
import { createTestCursorsController } from '../../common/testCursorConfiguration.js';

test('cursor edit updates the shared model and resulting selection together', () => {
	using model = new TextModel('bc');
	using cursors = createTestCursorsController(model, [new Selection(1, 1, 1, 1)]);
	cursors.executeCommand(new ReplaceCommand(new Range(1, 1, 1, 1), 'a'));
	assert.equal(model.getValue(), 'abc');
	assert.equal(cursors.getSelections()[0]!.positionColumn, 2);
});

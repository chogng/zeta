import assert from 'node:assert/strict';
import test from 'node:test';
import { TextAreaState } from '../../../browser/controller/editContext/textArea/textAreaEditContextState.js';

test('textarea state derives inserted text from two snapshots', () => {
	const before = new TextAreaState('ac', 1, 1, null, 0);
	const after = new TextAreaState('abc', 2, 2, null, 0);
	assert.deepEqual(TextAreaState.deduceInput(before, after, false), {
		text: 'b', replacePrevCharCnt: 0, replaceNextCharCnt: 0, positionDelta: 0,
	});
});

test('textarea state reports Android selection-only composition moves', () => {
	const before = new TextAreaState('abc', 1, 1, null, 0);
	const after = new TextAreaState('abc', 2, 2, null, 0);
	assert.equal(TextAreaState.deduceAndroidCompositionInput(before, after).positionDelta, 1);
});

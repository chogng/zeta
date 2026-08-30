import assert from 'node:assert/strict';
import test from 'node:test';
import { CursorCollection } from '../../common/cursor/cursorCollection.js';
import { Selection } from '../../common/core/selection.js';
import { TextModel } from '../../common/model/textModel.js';

test('CursorCollection preserves primary-first selection order', () => {
	using model = new TextModel('abcdef\nsecond');
	const selections = primaryFirst([new Selection(1, 1, 1, 3), new Selection(2, 2, 2, 2)], 1);
	using cursors = new CursorCollection(model, selections);
	assert.deepEqual(cursors.getSelections()[0], selections[0]);
	assert.equal(CursorCollection.selectionsEqual(cursors.getSelections(), selections), true);
});

test('CursorCollection converts validated offsets after an edit', () => {
	using model = new TextModel('abc');
	const selections = CursorCollection.selectionsFromOffsets(model, [{ anchorOffset: 1, activeOffset: 2 }], 0);
	assert.deepEqual([selections[0]!.selectionStartColumn, selections[0]!.positionColumn], [2, 3]);
});

function primaryFirst<T>(items: readonly T[], primaryIndex: number): readonly T[] {
	if (primaryIndex === 0) return Object.freeze([...items]);
	return Object.freeze([items[primaryIndex]!, ...items.slice(0, primaryIndex), ...items.slice(primaryIndex + 1)]);
}

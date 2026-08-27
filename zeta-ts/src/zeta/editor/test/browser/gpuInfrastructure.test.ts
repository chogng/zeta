import assert from 'node:assert/strict';
import test from 'node:test';
import { BufferDirtyTracker } from '../../browser/gpu/bufferDirtyTracker.js';
import { createContentSegmenter } from '../../browser/gpu/contentSegmenter.js';
import { createObjectCollectionBuffer } from '../../browser/gpu/objectCollectionBuffer.js';

test('BufferDirtyTracker exposes one inclusive dirty range', () => {
	const tracker = new BufferDirtyTracker();
	assert.equal(tracker.isDirty, false);

	tracker.flag(8, 3);
	tracker.flag(2, 2);

	assert.deepEqual({ offset: tracker.dataOffset, size: tracker.dirtySize, dirty: tracker.isDirty }, { offset: 2, size: 9, dirty: true });
	tracker.clear();
	assert.deepEqual({ offset: tracker.dataOffset, size: tracker.dirtySize, dirty: tracker.isDirty }, { offset: undefined, size: undefined, dirty: false });
});

test('ObjectCollectionBuffer grows and compacts managed entries', () => {
	using collection = createObjectCollectionBuffer([{ name: 'x' }, { name: 'y' }] as const, 1);
	using first = collection.createEntry({ x: 1, y: 2 });
	using second = collection.createEntry({ x: 3, y: 4 });

	assert.equal(collection.entryCount, 2);
	assert.deepEqual([...collection.view.slice(0, collection.viewUsedSize)], [1, 2, 3, 4]);
	first.dispose();
	assert.equal(collection.entryCount, 1);
	assert.deepEqual([...collection.view.slice(0, collection.viewUsedSize)], [3, 4]);

	second.set('y', 9);
	assert.equal(second.get('y'), 9);
});

test('ContentSegmenter returns one entry for a complete grapheme', () => {
	const text = 'A👩‍💻B';
	const segmenter = createContentSegmenter(text, { isBasicASCII: false, useMonospaceOptimizations: false });

	assert.equal(segmenter.getSegmentAtIndex(0), 'A');
	assert.equal(segmenter.getSegmentAtIndex(1), '👩‍💻');
	assert.equal(segmenter.getSegmentAtIndex(2), undefined);
	assert.equal(segmenter.getSegmentAtIndex(text.length - 1), 'B');
});

import assert from 'node:assert/strict';
import test from 'node:test';
import { CustomLineHeightData, LineHeightsManager } from '../../../common/viewLayout/lineHeights.js';

test('line heights resolve the tallest overlapping decoration', () => {
	const heights = new LineHeightsManager(10, [
		new CustomLineHeightData('wide', 2, 4, 18),
		new CustomLineHeightData('tall', 3, 5, 24),
	]);
	assert.deepEqual([1, 2, 3, 4, 5, 6].map(line => heights.heightForLineNumber(line)), [10, 18, 24, 24, 24, 10]);
	assert.equal(heights.getAccumulatedLineHeightsIncludingLineNumber(6), 110);
});

test('line insertions shift later ranges and extend containing ranges', () => {
	const heights = new LineHeightsManager(10, []);
	heights.insertOrChangeCustomLineHeight('range', 3, 5, 20);
	heights.onLinesInserted(4, 5);
	assert.deepEqual([3, 4, 5, 6, 7, 8].map(line => heights.heightForLineNumber(line)), [20, 20, 20, 20, 20, 10]);
	heights.onLinesInserted(1, 1);
	assert.deepEqual([3, 4, 8].map(line => heights.heightForLineNumber(line)), [10, 20, 20]);
});

test('line deletions preserve surviving spans and collapse fully removed ranges', () => {
	const heights = new LineHeightsManager(10, []);
	heights.insertOrChangeCustomLineHeight('partial', 3, 7, 20);
	heights.insertOrChangeCustomLineHeight('removed', 9, 10, 30);
	heights.onLinesDeleted(5, 10);
	assert.deepEqual([3, 4, 5, 6].map(line => heights.heightForLineNumber(line)), [20, 20, 30, 10]);
});

test('changing and removing a decoration takes effect immediately', () => {
	const heights = new LineHeightsManager(12, []);
	heights.insertOrChangeCustomLineHeight('active', 2, 2, 20);
	heights.insertOrChangeCustomLineHeight('active', 4, 4, 28);
	assert.deepEqual([2, 4].map(line => heights.heightForLineNumber(line)), [12, 28]);
	heights.removeCustomLineHeight('active');
	assert.equal(heights.heightForLineNumber(4), 12);
});

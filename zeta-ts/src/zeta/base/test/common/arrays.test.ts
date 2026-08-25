import assert from 'node:assert/strict';
import test from 'node:test';
import { arraysEqual, commonPrefixLength, commonSuffixLength } from '../../common/arrays.js';

test('arraysEqual compares complete sequences', () => {
	assert.equal(arraysEqual([1, 2], [1, 2]), true);
	assert.equal(arraysEqual([1, 2], [1, 3]), false);
	assert.equal(arraysEqual([1], [1, 2]), false);
});

test('arraysEqual accepts domain equality', () => {
	assert.equal(arraysEqual(['A'], ['a'], (left, right) => left.toLowerCase() === right.toLowerCase()), true);
});

test('commonPrefixLength returns the shared leading run', () => {
	assert.equal(commonPrefixLength(['a', 'b', 'c'], ['a', 'b', 'd']), 2);
	assert.equal(commonPrefixLength([], ['a']), 0);
});

test('commonSuffixLength excludes an already matched prefix', () => {
	assert.equal(commonSuffixLength(['a', 'x', 'c'], ['a', 'y', 'c'], 1), 1);
	assert.equal(commonSuffixLength(['a'], ['a'], 1), 0);
	assert.throws(() => commonSuffixLength(['a'], ['a'], 2), RangeError);
});

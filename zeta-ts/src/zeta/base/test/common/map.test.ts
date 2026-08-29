import assert from 'node:assert/strict';
import test from 'node:test';
import { getOrSet, LRUCache } from '../../common/map.js';

test('getOrSet returns existing values without replacing them', () => {
	const map = new Map([['key', 'existing']]);

	assert.equal(getOrSet(map, 'key', 'replacement'), 'existing');
	assert.equal(map.get('key'), 'existing');
});

test('getOrSet inserts absent and undefined values', () => {
	const map = new Map<string, string | undefined>([['present', undefined]]);

	assert.equal(getOrSet(map, 'missing', 'created'), 'created');
	assert.equal(map.get('missing'), 'created');
	assert.equal(getOrSet(map, 'present', 'replacement'), 'replacement');
	assert.equal(map.get('present'), 'replacement');
});

test('LRUCache trims oldest entries and touches values on read', () => {
	const cache = new LRUCache<string, number>(4, 0.5);
	cache.set('one', 1).set('two', 2).set('three', 3).set('four', 4);
	assert.equal(cache.get('one'), 1);

	cache.set('five', 5);

	assert.deepEqual([...cache], [['one', 1], ['five', 5]]);
});

test('LRUCache peek preserves age and validates its bounds', () => {
	const cache = new LRUCache<string, number>(2);
	cache.set('one', 1).set('two', 2);
	assert.equal(cache.peek('one'), 1);
	cache.set('three', 3);

	assert.deepEqual([...cache], [['two', 2], ['three', 3]]);
	assert.throws(() => new LRUCache(-1), RangeError);
	assert.throws(() => new LRUCache(1, 2), RangeError);
});

test('LRUCache iteration remains finite while reads update recency', () => {
	const cache = new LRUCache<string, number>(3);
	cache.set('one', 1).set('two', 2).set('three', 3);
	const seen: string[] = [];

	cache.forEach((_value, key) => {
		seen.push(key);
		cache.get(key);
	});

	assert.deepEqual(seen, ['one', 'two', 'three']);
});

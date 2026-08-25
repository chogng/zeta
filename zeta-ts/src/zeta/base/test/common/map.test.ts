import assert from 'node:assert/strict';
import test from 'node:test';
import { getOrSet } from '../../common/map.js';

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

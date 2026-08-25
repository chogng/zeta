import assert from 'node:assert/strict';
import test from 'node:test';
import { toError } from '../../common/errors.js';

test('toError preserves Error instances', () => {
	const error = new TypeError('failure');
	assert.equal(toError(error), error);
});

test('toError wraps non-Error values', () => {
	assert.deepEqual(toError('failure'), new Error('failure'));
	assert.deepEqual(toError(undefined), new Error('undefined'));
});

import assert from 'node:assert/strict';
import test from 'node:test';
import { getErrorMessage, toError } from '../../common/errors.js';

test('toError preserves Error instances', () => {
	const error = new TypeError('failure');
	assert.equal(toError(error), error);
});

test('toError wraps non-Error values', () => {
	assert.deepEqual(toError('failure'), new Error('failure'));
	assert.deepEqual(toError(undefined), new Error('undefined'));
});

test('getErrorMessage reads messages, stack headers, and fallback values', () => {
	assert.equal(getErrorMessage(new Error('failure')), 'failure');
	assert.equal(getErrorMessage('failure'), 'failure');
	assert.equal(getErrorMessage({ stack: 'Failure\nsecond line' }), 'Failure');
	assert.equal(getErrorMessage(undefined), 'Error');
});

import assert from 'node:assert/strict';
import test from 'node:test';
import { timeout } from '../../common/async.js';
import { isCancellationError } from '../../common/errors.js';

test('timeout settles asynchronously', async () => {
	let settled = false;
	const pending = timeout(0).then(() => {
		settled = true;
	});

	assert.equal(settled, false);
	await pending;
	assert.equal(settled, true);
});

test('timeout can be cancelled', async () => {
	const pending = timeout(10_000);
	pending.cancel();

	await assert.rejects(pending, isCancellationError);
});

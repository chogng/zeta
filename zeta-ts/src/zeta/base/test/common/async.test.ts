import assert from 'node:assert/strict';
import test from 'node:test';
import { RunOnceScheduler, TimeoutTimer, timeout } from '../../common/async.js';
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

test('TimeoutTimer replaces work and rejects scheduling after disposal', () => {
	const calls: string[] = [];
	const timer = new TimeoutTimer();
	timer.cancelAndSet(() => calls.push('first'), 10_000);
	timer.cancelAndSet(() => calls.push('second'), 10_000);
	timer.cancel();
	assert.deepEqual(calls, []);
	timer.dispose();
	assert.throws(() => timer.cancelAndSet(() => undefined, 0), ReferenceError);
});

test('RunOnceScheduler debounces, flushes, and cancels owned work', () => {
	let runs = 0;
	const scheduler = new RunOnceScheduler(() => runs += 1, 10_000);
	scheduler.schedule();
	scheduler.schedule();
	assert.equal(scheduler.isScheduled(), true);
	scheduler.flush();
	assert.equal(runs, 1);
	assert.equal(scheduler.isScheduled(), false);
	scheduler.schedule();
	scheduler.dispose();
	assert.equal(scheduler.isScheduled(), false);
	assert.throws(() => scheduler.schedule(), ReferenceError);
});

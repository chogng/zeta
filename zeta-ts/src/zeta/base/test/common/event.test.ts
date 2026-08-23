import { strict as assert } from "node:assert";
import test from "node:test";
import { Emitter, runWithBufferedEvents } from "../../common/event.js";
import type { IDisposable } from "../../common/lifecycle.js";

test("Emitter delivers synchronously and subscriptions are disposable", () => {
	const emitter = new Emitter<number>();
	const received: number[] = [];
	const subscription = emitter.event((event) => received.push(event));

	emitter.fire(1);
	assert.deepEqual(received, [1]);

	subscription.dispose();
	emitter.fire(2);
	assert.deepEqual(received, [1]);

	emitter.dispose();
	assert.throws(() => emitter.event(() => undefined), ReferenceError);
	emitter.fire(3);
});

test("Emitter treats repeated listener registrations independently", () => {
	const emitter = new Emitter<void>();
	let calls = 0;
	const listener = (): void => {
		calls += 1;
	};
	const first = emitter.event(listener);
	const second = emitter.event(listener);

	emitter.fire();
	assert.equal(calls, 2);

	first.dispose();
	emitter.fire();
	assert.equal(calls, 3);

	second.dispose();
	emitter.dispose();
});

test("Emitter snapshots registrations before delivering an event", () => {
	const emitter = new Emitter<number>();
	const received: string[] = [];
	let second: IDisposable;
	const first = emitter.event((event) => {
		received.push(`first:${event}`);
		second.dispose();
		second = emitter.event((next) => received.push(`late:${next}`));
	});
	second = emitter.event((event) => received.push(`second:${event}`));

	emitter.fire(1);
	assert.deepEqual(received, ["first:1"]);

	emitter.fire(2);
	assert.deepEqual(received, ["first:1", "first:2"]);

	first.dispose();
	second.dispose();
	emitter.dispose();
});

test("Emitter queues reentrant events in FIFO order", () => {
	const emitter = new Emitter<number>();
	const received: string[] = [];
	const first = emitter.event((event) => {
		received.push(`first:${event}`);
		if (event === 1) emitter.fire(2);
	});
	const second = emitter.event((event) => {
		received.push(`second:${event}`);
	});

	emitter.fire(1);

	assert.deepEqual(received, [
		"first:1",
		"second:1",
		"first:2",
		"second:2",
	]);
	first.dispose();
	second.dispose();
	emitter.dispose();
});

test("Emitter reports listener errors and continues delivery", () => {
	const expected = new Error("listener failed");
	const errors: unknown[] = [];
	const emitter = new Emitter<void>({
		onListenerError: (error) => errors.push(error),
	});
	const failing = emitter.event(() => {
		throw expected;
	});
	let delivered = false;
	const succeeding = emitter.event(() => {
		delivered = true;
	});

	assert.doesNotThrow(() => emitter.fire());
	assert.equal(delivered, true);
	assert.deepEqual(errors, [expected]);

	failing.dispose();
	succeeding.dispose();
	emitter.dispose();
});

test("runWithBufferedEvents publishes only after every state mutation completes", () => {
	const first = new Emitter<void>();
	const second = new Emitter<void>();
	const state = { first: 0, second: 0 };
	const observations: string[] = [];
	first.event(() => observations.push(`${state.first}:${state.second}`));
	second.event(() => observations.push(`${state.first}:${state.second}`));

	runWithBufferedEvents(() => {
		state.first = 1;
		first.fire();
		state.second = 1;
		second.fire();
		assert.deepEqual(observations, []);
	});

	assert.deepEqual(observations, ["1:1", "1:1"]);
	first.dispose();
	second.dispose();
});

test("runWithBufferedEvents discards notifications from failed mutations", () => {
	const emitter = new Emitter<void>();
	let calls = 0;
	emitter.event(() => { calls += 1; });

	assert.throws(() => runWithBufferedEvents(() => {
		emitter.fire();
		throw new Error("failed");
	}), /failed/);

	assert.equal(calls, 0);
	emitter.dispose();
});

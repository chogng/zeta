import { strict as assert } from "node:assert";
import test from "node:test";
import {
	CancellationError,
	isCancellationError,
	raceCancellation,
	throwIfCancelled,
} from "../../common/cancellation.js";

test("live AbortSignals do not cancel an operation", () => {
	const controller = new AbortController();

	assert.doesNotThrow(() => throwIfCancelled(controller.signal));
});

test("aborted signals produce a classified error and preserve their reason", () => {
	const controller = new AbortController();
	const reason = new Error("superseded");
	controller.abort(reason);

	assert.throws(
		() => throwIfCancelled(controller.signal, "Request cancelled"),
		(error: unknown) => {
			assert.ok(isCancellationError(error));
			assert.equal(error.message, "Request cancelled");
			assert.equal(error.reason, reason);
			assert.equal(error.cause, reason);
			return true;
		},
	);
});

test("unrelated errors are not classified as cancellation", () => {
	assert.equal(isCancellationError(new CancellationError()), true);
	assert.equal(isCancellationError(new Error("cancelled")), false);
});

test("raceCancellation preserves settlement and classifies caller cancellation", async () => {
	const live = new AbortController();
	assert.equal(await raceCancellation(Promise.resolve(42), live.signal), 42);

	const cancelled = new AbortController();
	const reason = new Error("stop waiting");
	const pending = raceCancellation(new Promise<number>(() => undefined), cancelled.signal, "Module wait cancelled");
	cancelled.abort(reason);
	await assert.rejects(pending, error => (
		isCancellationError(error) &&
		error.message === "Module wait cancelled" &&
		error.reason === reason
	));

	const alreadyCancelled = new AbortController();
	alreadyCancelled.abort("already");
	await assert.rejects(raceCancellation(Promise.resolve(1), alreadyCancelled.signal), error => (
		isCancellationError(error) && error.reason === "already"
	));
});

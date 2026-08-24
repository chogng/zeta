import { strict as assert } from "node:assert";
import test from "node:test";
import {
	AbstractDisposable,
	AsyncDisposableStore,
	DisposableSlot,
	DisposableStore,
	ResettableDisposableGroup,
	DisposableOwner,
	toDisposable,
} from "../../common/lifecycle.js";

test("AbstractDisposable runs leaf cleanup once", () => {
	class Resource extends AbstractDisposable {
		public cleanupCalls = 0;

		protected override disposeCore(): void {
			this.cleanupCalls += 1;
		}
	}

	const resource = new Resource();
	resource.dispose();
	resource[Symbol.dispose]();

	assert.equal(resource.cleanupCalls, 1);
});

test("project disposables support explicit disposal and using", () => {
	let calls = 0;

	{
		using resource = toDisposable(() => {
			calls += 1;
		});
		resource.dispose();
	}

	assert.equal(calls, 1);
});

test("DisposableStore releases resources in LIFO order and is idempotent", () => {
	const released: number[] = [];
	const store = new DisposableStore();
	store.add(toDisposable(() => released.push(1)));
	store.add(null);
	store.add(toDisposable(() => released.push(2)));
	store.add(undefined);

	store.dispose();
	store.dispose();

	assert.deepEqual(released, [2, 1]);
});

test("a disposed store rejects resources without taking ownership", () => {
	const store = new DisposableStore();
	store.dispose();
	let disposed = false;
	const resource = toDisposable(() => {
		disposed = true;
	});

	assert.throws(() => store.add(resource), ReferenceError);
	assert.equal(disposed, false);
	resource.dispose();
});

test("DisposableStore supports adopted values and deferred cleanup", () => {
	const released: string[] = [];
	const store = new DisposableStore();
	const value = store.adopt("value", (current) => released.push(current));
	store.defer(() => released.push("deferred"));

	assert.equal(value, "value");
	store.dispose();
	assert.deepEqual(released, ["deferred", "value"]);
});

test("DisposableStore attempts every cleanup and preserves suppressed errors", () => {
	const released: number[] = [];
	const store = new DisposableStore();
	store.add(toDisposable(() => {
		released.push(1);
		throw new Error("first cleanup failed");
	}));
	store.add(toDisposable(() => {
		released.push(2);
		throw new Error("second cleanup failed");
	}));

	assert.throws(
		() => store.dispose(),
		(error: unknown) => error instanceof SuppressedError,
	);
	assert.deepEqual(released, [2, 1]);
	assert.equal(store.disposed, true);
});

test("DisposableOwner owns standard Disposable resources", () => {
	class Owner extends DisposableOwner {
		take<T extends Disposable>(resource: T): T {
			return this.own(resource);
		}
	}

	let disposed = false;
	const owner = new Owner();
	owner.take(toDisposable(() => {
		disposed = true;
	}));
	owner.dispose();

	assert.equal(disposed, true);
});

test("DisposableSlot releases replaced and current values", () => {
	const released: number[] = [];
	const slot = new DisposableSlot();
	slot.replace(toDisposable(() => released.push(1)));
	slot.replace(toDisposable(() => released.push(2)));

	assert.deepEqual(released, [1]);
	slot.dispose();
	assert.deepEqual(released, [1, 2]);
});

test("a disposed DisposableSlot rejects values without taking ownership", () => {
	const slot = new DisposableSlot();
	slot.dispose();
	let disposed = false;
	const resource = toDisposable(() => {
		disposed = true;
	});

	assert.throws(() => slot.replace(resource), ReferenceError);
	assert.equal(disposed, false);
	resource.dispose();
});

test("ResettableDisposableGroup clears, rebuilds, and then closes", () => {
	const released: number[] = [];
	const group = new ResettableDisposableGroup();
	group.add(toDisposable(() => released.push(1)));
	group.clear();
	group.add(toDisposable(() => released.push(2)));
	group.dispose();

	assert.deepEqual(released, [1, 2]);
	assert.throws(
		() => group.add(toDisposable(() => released.push(3))),
		ReferenceError,
	);
});

test("AsyncDisposableStore owns sync and async resources in LIFO order", async () => {
	const released: number[] = [];

	await (async () => {
		await using store = new AsyncDisposableStore();
		store.add(toDisposable(() => released.push(1)));
		store.add({
			async [Symbol.asyncDispose](): Promise<void> {
				await Promise.resolve();
				released.push(2);
			},
		});
	})();

	assert.deepEqual(released, [2, 1]);
});

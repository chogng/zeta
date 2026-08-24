import { strict as assert } from "node:assert";
import test from "node:test";
import {
	AbstractDisposable,
	AsyncDisposableStore,
	DisposableMap,
	DisposableSlot,
	DisposableStore,
	ResettableDisposableGroup,
	DisposableOwner,
	noneDisposable,
	toDisposable,
} from "../../common/lifecycle.js";

test("noneDisposable is a reusable no-op", () => {
	noneDisposable.dispose();
	noneDisposable[Symbol.dispose]();
	assert.equal(Object.isFrozen(noneDisposable), true);
});

test("AsyncDisposableStore shares one cleanup operation", async () => {
	let release!: () => void;
	const gate = new Promise<void>(resolve => {
		release = resolve;
	});
	let cleanupCalls = 0;
	const store = new AsyncDisposableStore();
	store.defer(async () => {
		cleanupCalls += 1;
		await gate;
	});
	const first = store.disposeAsync();
	const second = store[Symbol.asyncDispose]();
	assert.equal(first, second);
	release();
	await first;
	assert.equal(cleanupCalls, 1);
});

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

test("DisposableMap replaces, removes, leaks, and disposes keyed resources", () => {
	const released: string[] = [];
	const resources = new DisposableMap<string>();
	resources.set("first", toDisposable(() => released.push("first:old")));
	resources.set("first", toDisposable(() => released.push("first:new")));
	resources.set("second", toDisposable(() => released.push("second")));
	resources.set("third", toDisposable(() => released.push("third")));

	assert.deepEqual(released, ["first:old"]);
	assert.equal(resources.deleteAndDispose("first"), true);
	assert.equal(resources.deleteAndDispose("missing"), false);
	const leaked = resources.set("leaked", toDisposable(() => released.push("leaked")));
	assert.equal(resources.deleteAndLeak("leaked"), leaked);
	resources.dispose();
	resources.dispose();
	assert.deepEqual(released, ["first:old", "first:new", "third", "second"]);
	leaked[Symbol.dispose]();
	assert.deepEqual(released, ["first:old", "first:new", "third", "second", "leaked"]);

	const rejected = toDisposable(() => released.push("rejected"));
	assert.throws(() => resources.set("rejected", rejected), ReferenceError);
	rejected.dispose();
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

test("AsyncDisposableStore closes registration when disposal starts", async () => {
	const store = new AsyncDisposableStore();
	const disposal = store.disposeAsync();
	const rejected = toDisposable(() => undefined);

	assert.equal(store.disposed, true);
	assert.throws(() => store.add(rejected), ReferenceError);
	rejected.dispose();
	await disposal;
});

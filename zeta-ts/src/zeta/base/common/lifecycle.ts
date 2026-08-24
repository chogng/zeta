/**
 * A project resource that supports both explicit `.dispose()` calls and the
 * ECMAScript `using` protocol.
 *
 * Implementations must release synchronously and idempotently.
 */
export interface IDisposable extends Disposable {
	dispose(): void;
}

/**
 * A project resource that supports both explicit `.disposeAsync()` calls and
 * the ECMAScript `await using` protocol.
 */
export interface IAsyncDisposable extends AsyncDisposable {
	disposeAsync(): Promise<void>;
}

/** Reusable cleanup handle for contracts that intentionally own no resource. */
export const noneDisposable: IDisposable = Object.freeze({
	dispose(): void {},
	[Symbol.dispose](): void {},
});

/** @internal */
export type TrackableDisposable = Disposable | AsyncDisposable;

function isNoneDisposable(disposable: TrackableDisposable): boolean {
	return Object.is(disposable, noneDisposable);
}

/**
 * Internal observation contract used by development-time disposable tracking.
 *
 * Implementations must never affect lifecycle correctness when no tracker is
 * installed.
 *
 * @internal
 */
export interface IDisposableTracker {
	trackDisposable(
		disposable: TrackableDisposable,
		label?: string,
	): void;
	validateDisposableOwner(
		disposable: TrackableDisposable,
		owner: TrackableDisposable,
	): void;
	setDisposableOwner(
		disposable: TrackableDisposable,
		owner: TrackableDisposable,
	): void;
	clearDisposableOwner(disposable: TrackableDisposable): void;
	markAsDisposed(disposable: TrackableDisposable): void;
}

let disposableTracker: IDisposableTracker | undefined;

/** @internal */
export function registerDisposableTracker(
	tracker: IDisposableTracker,
): Disposable {
	if (disposableTracker) {
		throw new Error("A disposable tracker is already installed");
	}
	disposableTracker = tracker;
	let disposed = false;
	return {
		[Symbol.dispose](): void {
			if (disposed) return;
			if (disposableTracker !== tracker) {
				throw new Error("The installed disposable tracker changed unexpectedly");
			}
			disposed = true;
			disposableTracker = undefined;
		},
	};
}

/** @internal */
export function trackDisposable<T extends TrackableDisposable>(
	disposable: T,
	label?: string,
): T {
	if (!isNoneDisposable(disposable)) disposableTracker?.trackDisposable(disposable, label);
	return disposable;
}

/** @internal */
export function validateDisposableOwner(
	disposable: TrackableDisposable,
	owner: TrackableDisposable,
): void {
	if (!isNoneDisposable(disposable)) disposableTracker?.validateDisposableOwner(disposable, owner);
}

/** @internal */
export function setDisposableOwner(
	disposable: TrackableDisposable,
	owner: TrackableDisposable,
): void {
	if (!isNoneDisposable(disposable)) disposableTracker?.setDisposableOwner(disposable, owner);
}

function clearDisposableOwner(disposable: TrackableDisposable): void {
	if (!isNoneDisposable(disposable)) disposableTracker?.clearDisposableOwner(disposable);
}

/** @internal */
export function markAsDisposed(disposable: TrackableDisposable): void {
	if (!isNoneDisposable(disposable)) disposableTracker?.markAsDisposed(disposable);
}

/**
 * Provides tracked, idempotent synchronous disposal without owning a resource
 * collection. Subclasses implement cleanup in `disposeCore`.
 */
export abstract class AbstractDisposable implements IDisposable {
	private disposedState = false;

	constructor() {
		trackDisposable(this);
	}

	protected get isDisposed(): boolean {
		return this.disposedState;
	}

	protected assertNotDisposed(): void {
		if (this.disposedState) throw new ReferenceError(`${this.constructor.name} is already disposed`);
	}

	public dispose(): void {
		if (this.disposedState) return;
		this.disposedState = true;
		try {
			this.disposeCore();
		} finally {
			markAsDisposed(this);
		}
	}

	public [Symbol.dispose](): void {
		this.dispose();
	}

	protected abstract disposeCore(): void;
}

/**
 * Creates an idempotent project disposable from a cleanup callback.
 */
export function toDisposable(fn: () => void): IDisposable {
	let disposed = false;
	const dispose = (): void => {
		if (disposed) return;
		disposed = true;
		try {
			fn();
		} finally {
			markAsDisposed(resource);
		}
	};
	const resource: IDisposable = {
		dispose,
		[Symbol.dispose]: dispose,
	};
	trackDisposable(resource, "toDisposable");
	return resource;
}

/**
 * Owns synchronous resources with the standard `DisposableStack` semantics.
 *
 * Resources are released in reverse registration order. Once disposed, new
 * registrations throw `ReferenceError`, matching the standard stack.
 */
export class DisposableStore extends AbstractDisposable {
	private readonly stack = new DisposableStack();

	get disposed(): boolean {
		return this.stack.disposed;
	}

	add<T extends Disposable | null | undefined>(resource: T): T {
		if (resource) validateDisposableOwner(resource, this);
		const owned = this.stack.use(resource);
		if (resource) setDisposableOwner(resource, this);
		return owned;
	}

	adopt<T>(value: T, onDispose: (value: T) => void): T {
		return this.stack.adopt(value, onDispose);
	}

	defer(onDispose: () => void): void {
		this.stack.defer(onDispose);
	}

	protected override disposeCore(): void {
		this.stack.dispose();
	}
}

/**
 * Owns asynchronous and synchronous resources with the standard
 * `AsyncDisposableStack` semantics.
 */
export class AsyncDisposableStore implements IAsyncDisposable {
	private readonly stack = new AsyncDisposableStack();
	private disposePromise: Promise<void> | undefined;

	constructor() {
		trackDisposable(this);
	}

	get disposed(): boolean {
		return this.stack.disposed;
	}

	add<
		T extends AsyncDisposable | Disposable | null | undefined,
	>(resource: T): T {
		if (resource) validateDisposableOwner(resource, this);
		const owned = this.stack.use(resource);
		if (resource) setDisposableOwner(resource, this);
		return owned;
	}

	adopt<T>(
		value: T,
		onDisposeAsync: (value: T) => PromiseLike<void> | void,
	): T {
		return this.stack.adopt(value, onDisposeAsync);
	}

	defer(onDisposeAsync: () => PromiseLike<void> | void): void {
		this.stack.defer(onDisposeAsync);
	}

	public disposeAsync(): Promise<void> {
		if (!this.disposePromise) {
			this.disposePromise = this.stack.disposeAsync().finally(() => markAsDisposed(this));
		}
		return this.disposePromise;
	}

	public [Symbol.asyncDispose](): Promise<void> {
		return this.disposeAsync();
	}
}

/**
 * Optional base class for composite objects that own independently created
 * synchronous resources. Leaf cleanup adapters extend `AbstractDisposable`.
 *
 * Subclasses register cleanup through `own`, `adopt`, or `defer` and must not
 * override the two disposal entry points.
 */
export abstract class DisposableOwner extends AbstractDisposable {
	private readonly resources = new DisposableStore();

	constructor() {
		super();
		setDisposableOwner(this.resources, this);
	}

	protected own<T extends Disposable | null | undefined>(resource: T): T {
		return this.resources.add(resource);
	}

	protected adopt<T>(value: T, onDispose: (value: T) => void): T {
		return this.resources.adopt(value, onDispose);
	}

	protected defer(onDispose: () => void): void {
		this.resources.defer(onDispose);
	}

	protected override disposeCore(): void {
		this.resources.dispose();
	}
}

/**
 * Owns one replaceable synchronous resource.
 */
export class DisposableSlot<T extends Disposable> extends AbstractDisposable {
	private _value: T | undefined;

	get value(): T | undefined {
		return this._value;
	}

	get disposed(): boolean {
		return this.isDisposed;
	}

	replace(next: T | undefined): void {
		this.assertNotDisposed();
		if (next === this._value) return;
		if (next) validateDisposableOwner(next, this);
		const previous = this._value;
		this._value = next;
		if (next) setDisposableOwner(next, this);
		if (previous) {
			try {
				previous[Symbol.dispose]();
			} finally {
				markAsDisposed(previous);
			}
		}
	}

	clear(): void {
		this.replace(undefined);
	}

	protected override disposeCore(): void {
		const current = this._value;
		this._value = undefined;
		try {
			current?.[Symbol.dispose]();
		} finally {
			if (current) markAsDisposed(current);
		}
	}
}

/** Owns disposable resources whose lifetime follows stable keys. */
export class DisposableMap<K, V extends Disposable = Disposable> extends AbstractDisposable implements Iterable<[K, V]> {
	private readonly resources = new Map<K, V>();

	public has(key: K): boolean {
		return this.resources.has(key);
	}

	public set(key: K, resource: V): V {
		this.assertNotDisposed();
		const previous = this.resources.get(key);
		if (previous === resource) return resource;
		validateDisposableOwner(resource, this);
		if (previous) {
			this.resources.delete(key);
			this.disposeResource(previous);
		}
		setDisposableOwner(resource, this);
		this.resources.set(key, resource);
		return resource;
	}

	public deleteAndDispose(key: K): boolean {
		const resource = this.resources.get(key);
		if (!resource) return false;
		this.resources.delete(key);
		this.disposeResource(resource);
		return true;
	}

	public deleteAndLeak(key: K): V | undefined {
		const resource = this.resources.get(key);
		if (!resource) return undefined;
		this.resources.delete(key);
		clearDisposableOwner(resource);
		return resource;
	}

	public keys(): IterableIterator<K> {
		return this.resources.keys();
	}

	public [Symbol.iterator](): IterableIterator<[K, V]> {
		return this.resources[Symbol.iterator]();
	}

	protected override disposeCore(): void {
		if (this.resources.size === 0) return;
		const resources = [...this.resources.values()];
		this.resources.clear();
		const stack = new DisposableStack();
		for (const resource of resources) stack.use(resource);
		try {
			stack.dispose();
		} finally {
			for (const resource of resources) markAsDisposed(resource);
		}
	}

	private disposeResource(resource: V): void {
		try {
			resource[Symbol.dispose]();
		} finally {
			markAsDisposed(resource);
		}
	}
}

/**
 * Owns a reusable group of synchronous resources.
 *
 * Prefer `DisposableStore`. Use this type only when one owner must repeatedly
 * discard and rebuild a complete group during its lifetime.
 */
export class ResettableDisposableGroup extends AbstractDisposable {
	private stack = new DisposableStack();
	private readonly resources = new Set<Disposable>();

	get disposed(): boolean {
		return this.isDisposed;
	}

	add<T extends Disposable | null | undefined>(resource: T): T {
		if (resource) validateDisposableOwner(resource, this);
		const owned = this.current().use(resource);
		if (resource) {
			this.resources.add(resource);
			setDisposableOwner(resource, this);
		}
		return owned;
	}

	adopt<T>(value: T, onDispose: (value: T) => void): T {
		return this.current().adopt(value, onDispose);
	}

	defer(onDispose: () => void): void {
		this.current().defer(onDispose);
	}

	clear(): void {
		const current = this.current();
		this.stack = new DisposableStack();
		try {
			current.dispose();
		} finally {
			for (const resource of this.resources) {
				markAsDisposed(resource);
			}
			this.resources.clear();
		}
	}

	protected override disposeCore(): void {
		const current = this.stack;
		try {
			current.dispose();
		} finally {
			for (const resource of this.resources) {
				markAsDisposed(resource);
			}
			this.resources.clear();
		}
	}

	private current(): DisposableStack {
		this.assertNotDisposed();
		return this.stack;
	}
}

/**
 * Combines resources into one project disposable.
 */
export function combinedDisposable(
	...resources: readonly Disposable[]
): IDisposable {
	const store = new DisposableStore();
	for (const resource of resources) store.add(resource);
	return store;
}

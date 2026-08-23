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

/** @internal */
export type TrackableDisposable = Disposable | AsyncDisposable;

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
	disposableTracker?.trackDisposable(disposable, label);
	return disposable;
}

/** @internal */
export function validateDisposableOwner(
	disposable: TrackableDisposable,
	owner: TrackableDisposable,
): void {
	disposableTracker?.validateDisposableOwner(disposable, owner);
}

/** @internal */
export function setDisposableOwner(
	disposable: TrackableDisposable,
	owner: TrackableDisposable,
): void {
	disposableTracker?.setDisposableOwner(disposable, owner);
}

/** @internal */
export function markAsDisposed(disposable: TrackableDisposable): void {
	disposableTracker?.markAsDisposed(disposable);
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
export class DisposableStore implements IDisposable {
	private readonly stack = new DisposableStack();

	constructor() {
		trackDisposable(this);
	}

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

	dispose(): void {
		try {
			this.stack.dispose();
		} finally {
			markAsDisposed(this);
		}
	}

	[Symbol.dispose](): void {
		this.dispose();
	}
}

/**
 * Owns asynchronous and synchronous resources with the standard
 * `AsyncDisposableStack` semantics.
 */
export class AsyncDisposableStore implements IAsyncDisposable {
	private readonly stack = new AsyncDisposableStack();

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

	async disposeAsync(): Promise<void> {
		try {
			await this.stack.disposeAsync();
		} finally {
			markAsDisposed(this);
		}
	}

	[Symbol.asyncDispose](): Promise<void> {
		return this.disposeAsync();
	}
}

/**
 * Optional base class for long-lived objects that own synchronous resources.
 *
 * Subclasses register cleanup through `own`, `adopt`, or `defer` and must not
 * override the two disposal entry points.
 */
export abstract class DisposableOwner implements IDisposable {
	private readonly resources = new DisposableStore();

	constructor() {
		trackDisposable(this);
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

	dispose(): void {
		try {
			this.resources.dispose();
		} finally {
			markAsDisposed(this);
		}
	}

	[Symbol.dispose](): void {
		this.dispose();
	}
}

/**
 * Owns one replaceable synchronous resource.
 */
export class DisposableSlot<T extends Disposable> implements IDisposable {
	private _value: T | undefined;
	private _disposed = false;

	constructor() {
		trackDisposable(this);
	}

	get value(): T | undefined {
		return this._value;
	}

	get disposed(): boolean {
		return this._disposed;
	}

	replace(next: T | undefined): void {
		if (this._disposed) {
			throw new ReferenceError("DisposableSlot is already disposed");
		}
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

	dispose(): void {
		if (this._disposed) return;
		this._disposed = true;
		const current = this._value;
		this._value = undefined;
		try {
			current?.[Symbol.dispose]();
		} finally {
			if (current) markAsDisposed(current);
			markAsDisposed(this);
		}
	}

	[Symbol.dispose](): void {
		this.dispose();
	}
}

/**
 * Owns a reusable group of synchronous resources.
 *
 * Prefer `DisposableStore`. Use this type only when one owner must repeatedly
 * discard and rebuild a complete group during its lifetime.
 */
export class ResettableDisposableGroup implements IDisposable {
	private stack: DisposableStack | undefined = new DisposableStack();
	private readonly resources = new Set<Disposable>();

	constructor() {
		trackDisposable(this);
	}

	get disposed(): boolean {
		return this.stack === undefined;
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

	dispose(): void {
		const current = this.stack;
		if (!current) return;
		this.stack = undefined;
		try {
			current.dispose();
		} finally {
			for (const resource of this.resources) {
				markAsDisposed(resource);
			}
			this.resources.clear();
			markAsDisposed(this);
		}
	}

	[Symbol.dispose](): void {
		this.dispose();
	}

	private current(): DisposableStack {
		if (!this.stack) {
			throw new ReferenceError("ResettableDisposableGroup is already disposed");
		}
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

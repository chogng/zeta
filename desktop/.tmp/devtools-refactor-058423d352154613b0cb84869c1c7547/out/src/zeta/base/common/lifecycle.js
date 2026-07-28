let disposableTracker;
/** @internal */
export function registerDisposableTracker(tracker) {
    if (disposableTracker) {
        throw new Error("A disposable tracker is already installed");
    }
    disposableTracker = tracker;
    let disposed = false;
    return {
        [Symbol.dispose]() {
            if (disposed)
                return;
            if (disposableTracker !== tracker) {
                throw new Error("The installed disposable tracker changed unexpectedly");
            }
            disposed = true;
            disposableTracker = undefined;
        },
    };
}
/** @internal */
export function trackDisposable(disposable, label) {
    disposableTracker?.trackDisposable(disposable, label);
    return disposable;
}
/** @internal */
export function validateDisposableOwner(disposable, owner) {
    disposableTracker?.validateDisposableOwner(disposable, owner);
}
/** @internal */
export function setDisposableOwner(disposable, owner) {
    disposableTracker?.setDisposableOwner(disposable, owner);
}
/** @internal */
export function markAsDisposed(disposable) {
    disposableTracker?.markAsDisposed(disposable);
}
/**
 * Creates an idempotent project disposable from a cleanup callback.
 */
export function toDisposable(fn) {
    let disposed = false;
    const dispose = () => {
        if (disposed)
            return;
        disposed = true;
        try {
            fn();
        }
        finally {
            markAsDisposed(resource);
        }
    };
    const resource = {
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
export class DisposableStore {
    #stack = new DisposableStack();
    constructor() {
        trackDisposable(this);
    }
    get disposed() {
        return this.#stack.disposed;
    }
    add(resource) {
        if (resource)
            validateDisposableOwner(resource, this);
        const owned = this.#stack.use(resource);
        if (resource)
            setDisposableOwner(resource, this);
        return owned;
    }
    adopt(value, onDispose) {
        return this.#stack.adopt(value, onDispose);
    }
    defer(onDispose) {
        this.#stack.defer(onDispose);
    }
    dispose() {
        try {
            this.#stack.dispose();
        }
        finally {
            markAsDisposed(this);
        }
    }
    [Symbol.dispose]() {
        this.dispose();
    }
}
/**
 * Owns asynchronous and synchronous resources with the standard
 * `AsyncDisposableStack` semantics.
 */
export class AsyncDisposableStore {
    #stack = new AsyncDisposableStack();
    constructor() {
        trackDisposable(this);
    }
    get disposed() {
        return this.#stack.disposed;
    }
    add(resource) {
        if (resource)
            validateDisposableOwner(resource, this);
        const owned = this.#stack.use(resource);
        if (resource)
            setDisposableOwner(resource, this);
        return owned;
    }
    adopt(value, onDisposeAsync) {
        return this.#stack.adopt(value, onDisposeAsync);
    }
    defer(onDisposeAsync) {
        this.#stack.defer(onDisposeAsync);
    }
    async disposeAsync() {
        try {
            await this.#stack.disposeAsync();
        }
        finally {
            markAsDisposed(this);
        }
    }
    [Symbol.asyncDispose]() {
        return this.disposeAsync();
    }
}
/**
 * Optional base class for long-lived objects that own synchronous resources.
 *
 * Subclasses register cleanup through `own`, `adopt`, or `defer` and must not
 * override the two disposal entry points.
 */
export class DisposableOwner {
    #resources = new DisposableStore();
    constructor() {
        trackDisposable(this);
        setDisposableOwner(this.#resources, this);
    }
    own(resource) {
        return this.#resources.add(resource);
    }
    adopt(value, onDispose) {
        return this.#resources.adopt(value, onDispose);
    }
    defer(onDispose) {
        this.#resources.defer(onDispose);
    }
    dispose() {
        try {
            this.#resources.dispose();
        }
        finally {
            markAsDisposed(this);
        }
    }
    [Symbol.dispose]() {
        this.dispose();
    }
}
/**
 * Owns one replaceable synchronous resource.
 */
export class DisposableSlot {
    #value;
    #disposed = false;
    constructor() {
        trackDisposable(this);
    }
    get value() {
        return this.#value;
    }
    get disposed() {
        return this.#disposed;
    }
    replace(next) {
        if (this.#disposed) {
            throw new ReferenceError("DisposableSlot is already disposed");
        }
        if (next === this.#value)
            return;
        if (next)
            validateDisposableOwner(next, this);
        const previous = this.#value;
        this.#value = next;
        if (next)
            setDisposableOwner(next, this);
        if (previous) {
            try {
                previous[Symbol.dispose]();
            }
            finally {
                markAsDisposed(previous);
            }
        }
    }
    clear() {
        this.replace(undefined);
    }
    dispose() {
        if (this.#disposed)
            return;
        this.#disposed = true;
        const current = this.#value;
        this.#value = undefined;
        try {
            current?.[Symbol.dispose]();
        }
        finally {
            if (current)
                markAsDisposed(current);
            markAsDisposed(this);
        }
    }
    [Symbol.dispose]() {
        this.dispose();
    }
}
/**
 * Owns a reusable group of synchronous resources.
 *
 * Prefer `DisposableStore`. Use this type only when one owner must repeatedly
 * discard and rebuild a complete group during its lifetime.
 */
export class ResettableDisposableGroup {
    #stack = new DisposableStack();
    #resources = new Set();
    constructor() {
        trackDisposable(this);
    }
    get disposed() {
        return this.#stack === undefined;
    }
    add(resource) {
        if (resource)
            validateDisposableOwner(resource, this);
        const owned = this.#current().use(resource);
        if (resource) {
            this.#resources.add(resource);
            setDisposableOwner(resource, this);
        }
        return owned;
    }
    adopt(value, onDispose) {
        return this.#current().adopt(value, onDispose);
    }
    defer(onDispose) {
        this.#current().defer(onDispose);
    }
    clear() {
        const current = this.#current();
        this.#stack = new DisposableStack();
        try {
            current.dispose();
        }
        finally {
            for (const resource of this.#resources) {
                markAsDisposed(resource);
            }
            this.#resources.clear();
        }
    }
    dispose() {
        const current = this.#stack;
        if (!current)
            return;
        this.#stack = undefined;
        try {
            current.dispose();
        }
        finally {
            for (const resource of this.#resources) {
                markAsDisposed(resource);
            }
            this.#resources.clear();
            markAsDisposed(this);
        }
    }
    [Symbol.dispose]() {
        this.dispose();
    }
    #current() {
        if (!this.#stack) {
            throw new ReferenceError("ResettableDisposableGroup is already disposed");
        }
        return this.#stack;
    }
}
/**
 * Combines resources into one project disposable.
 */
export function combinedDisposable(...resources) {
    const store = new DisposableStore();
    for (const resource of resources)
        store.add(resource);
    return store;
}

var __addDisposableResource = (this && this.__addDisposableResource) || function (env, value, async) {
    if (value !== null && value !== void 0) {
        if (typeof value !== "object" && typeof value !== "function") throw new TypeError("Object expected.");
        var dispose, inner;
        if (async) {
            if (!Symbol.asyncDispose) throw new TypeError("Symbol.asyncDispose is not defined.");
            dispose = value[Symbol.asyncDispose];
        }
        if (dispose === void 0) {
            if (!Symbol.dispose) throw new TypeError("Symbol.dispose is not defined.");
            dispose = value[Symbol.dispose];
            if (async) inner = dispose;
        }
        if (typeof dispose !== "function") throw new TypeError("Object not disposable.");
        if (inner) dispose = function() { try { inner.call(this); } catch (e) { return Promise.reject(e); } };
        env.stack.push({ value: value, dispose: dispose, async: async });
    }
    else if (async) {
        env.stack.push({ async: true });
    }
    return value;
};
var __disposeResources = (this && this.__disposeResources) || (function (SuppressedError) {
    return function (env) {
        function fail(e) {
            env.error = env.hasError ? new SuppressedError(e, env.error, "An error was suppressed during disposal.") : e;
            env.hasError = true;
        }
        var r, s = 0;
        function next() {
            while (r = env.stack.pop()) {
                try {
                    if (!r.async && s === 1) return s = 0, env.stack.push(r), Promise.resolve().then(next);
                    if (r.dispose) {
                        var result = r.dispose.call(r.value);
                        if (r.async) return s |= 2, Promise.resolve(result).then(next, function(e) { fail(e); return next(); });
                    }
                    else s |= 1;
                }
                catch (e) {
                    fail(e);
                }
            }
            if (s === 1) return env.hasError ? Promise.reject(env.error) : Promise.resolve();
            if (env.hasError) throw env.error;
        }
        return next();
    };
})(typeof SuppressedError === "function" ? SuppressedError : function (error, suppressed, message) {
    var e = new Error(message);
    return e.name = "SuppressedError", e.error = error, e.suppressed = suppressed, e;
});
import { strict as assert } from "node:assert";
import test from "node:test";
import { AsyncDisposableStore, DisposableSlot, DisposableStore, ResettableDisposableGroup, DisposableOwner, toDisposable, } from "../../common/lifecycle.js";
test("project disposables support explicit disposal and using", () => {
    let calls = 0;
    {
        const env_1 = { stack: [], error: void 0, hasError: false };
        try {
            const resource = __addDisposableResource(env_1, toDisposable(() => {
                calls += 1;
            }), false);
            resource.dispose();
        }
        catch (e_1) {
            env_1.error = e_1;
            env_1.hasError = true;
        }
        finally {
            __disposeResources(env_1);
        }
    }
    assert.equal(calls, 1);
});
test("DisposableStore releases resources in LIFO order and is idempotent", () => {
    const released = [];
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
    const released = [];
    const store = new DisposableStore();
    const value = store.adopt("value", (current) => released.push(current));
    store.defer(() => released.push("deferred"));
    assert.equal(value, "value");
    store.dispose();
    assert.deepEqual(released, ["deferred", "value"]);
});
test("DisposableStore attempts every cleanup and preserves suppressed errors", () => {
    const released = [];
    const store = new DisposableStore();
    store.add(toDisposable(() => {
        released.push(1);
        throw new Error("first cleanup failed");
    }));
    store.add(toDisposable(() => {
        released.push(2);
        throw new Error("second cleanup failed");
    }));
    assert.throws(() => store.dispose(), (error) => error instanceof SuppressedError);
    assert.deepEqual(released, [2, 1]);
    assert.equal(store.disposed, true);
});
test("DisposableOwner owns standard Disposable resources", () => {
    class Owner extends DisposableOwner {
        take(resource) {
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
    const released = [];
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
    const released = [];
    const group = new ResettableDisposableGroup();
    group.add(toDisposable(() => released.push(1)));
    group.clear();
    group.add(toDisposable(() => released.push(2)));
    group.dispose();
    assert.deepEqual(released, [1, 2]);
    assert.throws(() => group.add(toDisposable(() => released.push(3))), ReferenceError);
});
test("AsyncDisposableStore owns sync and async resources in LIFO order", async () => {
    const released = [];
    await (async () => {
        const env_2 = { stack: [], error: void 0, hasError: false };
        try {
            const store = __addDisposableResource(env_2, new AsyncDisposableStore(), true);
            store.add(toDisposable(() => released.push(1)));
            store.add({
                async [Symbol.asyncDispose]() {
                    await Promise.resolve();
                    released.push(2);
                },
            });
        }
        catch (e_2) {
            env_2.error = e_2;
            env_2.hasError = true;
        }
        finally {
            const result_1 = __disposeResources(env_2);
            if (result_1)
                await result_1;
        }
    })();
    assert.deepEqual(released, [2, 1]);
});

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
import { DisposableStore, DisposableOwner, toDisposable, } from "../../common/lifecycle.js";
import { DisposableTracker, installDisposableTracker, } from "../../common/disposableTracker.js";
test("DisposableTracker reports an unowned disposable until it is disposed", () => {
    const env_1 = { stack: [], error: void 0, hasError: false };
    try {
        const tracker = new DisposableTracker();
        const installation = __addDisposableResource(env_1, installDisposableTracker(tracker), false);
        const resource = toDisposable(() => { });
        const [leak] = tracker.leaks();
        assert.equal(leak?.label, "toDisposable");
        assert.equal(leak?.ownerLabel, undefined);
        assert.match(leak?.createdAt ?? "", /disposableTracker\.test/);
        assert.throws(() => tracker.assertNoLeaks(), /1 undisposed disposable/);
        resource.dispose();
        tracker.assertNoLeaks();
    }
    catch (e_1) {
        env_1.error = e_1;
        env_1.hasError = true;
    }
    finally {
        __disposeResources(env_1);
    }
});
test("DisposableTracker records ownership and closes the complete subtree", () => {
    const env_2 = { stack: [], error: void 0, hasError: false };
    try {
        const tracker = new DisposableTracker();
        const installation = __addDisposableResource(env_2, installDisposableTracker(tracker), false);
        const store = new DisposableStore();
        store.add(toDisposable(() => { }));
        const child = tracker.leaks().find((leak) => leak.label === "toDisposable");
        assert.equal(child?.ownerLabel, "DisposableStore");
        store.dispose();
        tracker.assertNoLeaks();
    }
    catch (e_2) {
        env_2.error = e_2;
        env_2.hasError = true;
    }
    finally {
        __disposeResources(env_2);
    }
});
test("DisposableTracker follows DisposableOwner through its internal store", () => {
    const env_3 = { stack: [], error: void 0, hasError: false };
    try {
        class Owner extends DisposableOwner {
            take(resource) {
                this.own(resource);
            }
        }
        const tracker = new DisposableTracker();
        const installation = __addDisposableResource(env_3, installDisposableTracker(tracker), false);
        const owner = new Owner();
        owner.take(toDisposable(() => { }));
        const store = tracker.leaks().find((leak) => leak.label === "DisposableStore");
        assert.equal(store?.ownerLabel, "Owner");
        owner.dispose();
        tracker.assertNoLeaks();
    }
    catch (e_3) {
        env_3.error = e_3;
        env_3.hasError = true;
    }
    finally {
        __disposeResources(env_3);
    }
});
test("DisposableTracker rejects multiple owners before ownership transfers", () => {
    const env_4 = { stack: [], error: void 0, hasError: false };
    try {
        const tracker = new DisposableTracker();
        const installation = __addDisposableResource(env_4, installDisposableTracker(tracker), false);
        const first = new DisposableStore();
        const second = new DisposableStore();
        const resource = first.add(toDisposable(() => { }));
        assert.throws(() => second.add(resource), /already belongs to DisposableStore/);
        first.dispose();
        second.dispose();
        tracker.assertNoLeaks();
    }
    catch (e_4) {
        env_4.error = e_4;
        env_4.hasError = true;
    }
    finally {
        __disposeResources(env_4);
    }
});
test("DisposableTracker rejects ownership cycles", () => {
    const env_5 = { stack: [], error: void 0, hasError: false };
    try {
        const tracker = new DisposableTracker();
        const installation = __addDisposableResource(env_5, installDisposableTracker(tracker), false);
        const parent = new DisposableStore();
        const child = parent.add(new DisposableStore());
        assert.throws(() => child.add(parent), /ownership cannot contain a cycle/);
        parent.dispose();
        tracker.assertNoLeaks();
    }
    catch (e_5) {
        env_5.error = e_5;
        env_5.hasError = true;
    }
    finally {
        __disposeResources(env_5);
    }
});
test("DisposableTracker closes ownership records even when cleanup throws", () => {
    const env_6 = { stack: [], error: void 0, hasError: false };
    try {
        const tracker = new DisposableTracker();
        const installation = __addDisposableResource(env_6, installDisposableTracker(tracker), false);
        const store = new DisposableStore();
        store.add(toDisposable(() => {
            throw new Error("cleanup failed");
        }));
        assert.throws(() => store.dispose(), /cleanup failed/);
        tracker.assertNoLeaks();
    }
    catch (e_6) {
        env_6.error = e_6;
        env_6.hasError = true;
    }
    finally {
        __disposeResources(env_6);
    }
});
test("tracking is disabled outside an installed development scope", () => {
    const tracker = new DisposableTracker();
    const resource = toDisposable(() => { });
    assert.equal(tracker.leaks().length, 0);
    resource.dispose();
});
test("only one DisposableTracker can be installed in a JavaScript realm", () => {
    const env_7 = { stack: [], error: void 0, hasError: false };
    try {
        const tracker = new DisposableTracker();
        const installation = __addDisposableResource(env_7, installDisposableTracker(tracker), false);
        assert.throws(() => installDisposableTracker(new DisposableTracker()), /already installed/);
    }
    catch (e_7) {
        env_7.error = e_7;
        env_7.hasError = true;
    }
    finally {
        __disposeResources(env_7);
    }
});

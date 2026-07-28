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
import assert from "node:assert/strict";
import test from "node:test";
import { StatusbarAlignment, StatusbarService, } from "../src/workbench/services/statusbar/browser/statusbar.js";
test("status bar entries are grouped and ordered by alignment", () => {
    const env_1 = { stack: [], error: void 0, hasError: false };
    try {
        const service = __addDisposableResource(env_1, new StatusbarService(), false);
        const lowPriority = __addDisposableResource(env_1, service.addEntry({ text: "Low" }, {
            id: "test.low",
            alignment: StatusbarAlignment.Left,
            priority: 1,
        }), false);
        const right = __addDisposableResource(env_1, service.addEntry({ text: "Right" }, {
            id: "test.right",
            alignment: StatusbarAlignment.Right,
        }), false);
        const highPriority = __addDisposableResource(env_1, service.addEntry({ text: "High" }, {
            id: "test.high",
            alignment: StatusbarAlignment.Left,
            priority: 10,
        }), false);
        assert.deepEqual(service.getEntries(StatusbarAlignment.Left).map(({ id }) => id), ["test.high", "test.low"]);
        assert.deepEqual(service.getEntries(StatusbarAlignment.Right).map(({ id }) => id), ["test.right"]);
    }
    catch (e_1) {
        env_1.error = e_1;
        env_1.hasError = true;
    }
    finally {
        __disposeResources(env_1);
    }
});
test("status bar entry accessors update and remove their entry", () => {
    const env_2 = { stack: [], error: void 0, hasError: false };
    try {
        const service = __addDisposableResource(env_2, new StatusbarService(), false);
        let changes = 0;
        const listener = __addDisposableResource(env_2, service.onDidChangeEntries(() => {
            changes += 1;
        }), false);
        const entry = service.addEntry({ text: "Connecting" }, {
            id: "test.connection",
            alignment: StatusbarAlignment.Left,
        });
        entry.update({
            text: "Connected",
            tooltip: "The app server is connected",
        });
        assert.deepEqual(service.getEntries(StatusbarAlignment.Left)[0]?.entry, {
            text: "Connected",
            tooltip: "The app server is connected",
        });
        entry.dispose();
        assert.deepEqual(service.getEntries(StatusbarAlignment.Left), []);
        assert.equal(changes, 3);
    }
    catch (e_2) {
        env_2.error = e_2;
        env_2.hasError = true;
    }
    finally {
        __disposeResources(env_2);
    }
});
test("status bar entry ids are unique while registered", () => {
    const env_3 = { stack: [], error: void 0, hasError: false };
    try {
        const service = __addDisposableResource(env_3, new StatusbarService(), false);
        const entry = __addDisposableResource(env_3, service.addEntry({ text: "First" }, {
            id: "test.unique",
            alignment: StatusbarAlignment.Left,
        }), false);
        assert.throws(() => service.addEntry({ text: "Duplicate" }, {
            id: "test.unique",
            alignment: StatusbarAlignment.Right,
        }), /already exists/);
    }
    catch (e_3) {
        env_3.error = e_3;
        env_3.hasError = true;
    }
    finally {
        __disposeResources(env_3);
    }
});

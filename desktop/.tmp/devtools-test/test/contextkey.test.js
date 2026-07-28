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
import { ContextKeyExpr, ContextKeyService, RawContextKey, } from "../src/platform/contextkey/common/contextkey.js";
test("typed context keys reset to their declared default", () => {
    const env_1 = { stack: [], error: void 0, hasError: false };
    try {
        const service = __addDisposableResource(env_1, new ContextKeyService(), false);
        const ready = new RawContextKey("test.ready.typed", false).bindTo(service);
        assert.equal(ready.get(), false);
        ready.set(true);
        assert.equal(ready.get(), true);
        ready.reset();
        assert.equal(ready.get(), false);
    }
    catch (e_1) {
        env_1.error = e_1;
        env_1.hasError = true;
    }
    finally {
        __disposeResources(env_1);
    }
});
test("scoped contexts inherit values and override the nearest DOM subtree", () => {
    const env_2 = { stack: [], error: void 0, hasError: false };
    try {
        const root = __addDisposableResource(env_2, new ContextKeyService(), false);
        const scopeElement = fakeNode();
        const childElement = fakeNode(scopeElement);
        const nestedElement = fakeNode(childElement);
        const scope = __addDisposableResource(env_2, root.createScoped(scopeElement), false);
        const nestedScope = __addDisposableResource(env_2, root.createScoped(nestedElement), false);
        root.setContext("test.language", "global");
        scope.setContext("test.language", "local");
        scope.setContext("test.focused", true);
        nestedScope.setContext("test.nested", true);
        const childContext = root.getContext(childElement);
        const nestedContext = root.getContext(nestedElement);
        assert.equal(childContext.getValue("test.language"), "local");
        assert.equal(childContext.getValue("test.focused"), true);
        assert.equal(nestedContext.getValue("test.language"), "local");
        assert.equal(nestedContext.getValue("test.nested"), true);
        assert.equal(root.contextMatchesRules(ContextKeyExpr.equals("test.language", "local"), childElement), true);
        scope.removeContext("test.language");
        assert.equal(childContext.getValue("test.language"), "global");
    }
    catch (e_2) {
        env_2.error = e_2;
        env_2.hasError = true;
    }
    finally {
        __disposeResources(env_2);
    }
});
function fakeNode(parentNode = null) {
    return {
        nodeType: 1,
        parentNode,
        getRootNode: () => parentNode?.getRootNode() ?? {},
    };
}

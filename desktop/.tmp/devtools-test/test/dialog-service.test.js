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
import { DialogResult, DialogSeverity, } from "../src/platform/dialogs/common/dialogs.js";
import { DialogService, } from "../src/workbench/services/dialogs/common/dialogService.js";
test("dialog service publishes requests through its owned model", async () => {
    const env_1 = { stack: [], error: void 0, hasError: false };
    try {
        const service = __addDisposableResource(env_1, new DialogService(), false);
        const confirmation = service.confirm({
            message: "Continue?",
        });
        const item = service.model.dialogs[0];
        assert.equal(service.model.dialogs.length, 1);
        assert.equal(item?.request.kind, "confirmation");
        item?.close(DialogResult.Primary);
        assert.equal(await confirmation, true);
        assert.equal(service.model.dialogs.length, 0);
    }
    catch (e_1) {
        env_1.error = e_1;
        env_1.hasError = true;
    }
    finally {
        __disposeResources(env_1);
    }
});
test("dialog service maps cancellation to a false confirmation", async () => {
    const env_2 = { stack: [], error: void 0, hasError: false };
    try {
        const service = __addDisposableResource(env_2, new DialogService(), false);
        const confirmation = service.confirm({
            message: "Delete item?",
            primaryButton: "Delete",
        });
        service.model.dialogs[0]?.cancel();
        assert.equal(await confirmation, false);
    }
    catch (e_2) {
        env_2.error = e_2;
        env_2.hasError = true;
    }
    finally {
        __disposeResources(env_2);
    }
});
test("disposing dialog service cancels every queued model request", async () => {
    const service = new DialogService();
    const confirmation = service.confirm({ message: "Active" });
    const message = service.showMessage({
        severity: DialogSeverity.Warning,
        message: "Queued",
    });
    assert.equal(service.model.dialogs.length, 2);
    service.dispose();
    assert.equal(await confirmation, false);
    await message;
    assert.equal(service.model.dialogs.length, 0);
});
test("dialog service propagates model presentation failures", async () => {
    const env_3 = { stack: [], error: void 0, hasError: false };
    try {
        const service = __addDisposableResource(env_3, new DialogService(), false);
        const result = service.showMessage({
            severity: DialogSeverity.Error,
            message: "Failure",
        });
        service.model.dialogs[0]?.fail(new Error("render failed"));
        await assert.rejects(result, /render failed/);
        assert.equal(service.model.dialogs.length, 0);
    }
    catch (e_3) {
        env_3.error = e_3;
        env_3.hasError = true;
    }
    finally {
        __disposeResources(env_3);
    }
});

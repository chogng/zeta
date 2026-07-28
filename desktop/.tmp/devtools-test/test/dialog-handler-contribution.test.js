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
import { ServiceCollection, } from "../src/platform/instantiation/common/instantiation.js";
import { DialogHandlerContribution, } from "../src/workbench/browser/parts/dialogs/dialog.contribution.js";
import { IDialogsModel, IWorkbenchDialogHandler, } from "../src/workbench/common/dialogs.js";
import { WorkbenchContributionsRegistry, WorkbenchPhase, } from "../src/workbench/common/contributions.js";
import { DialogService, } from "../src/workbench/services/dialogs/common/dialogService.js";
class TestDialogHandler {
    calls = [];
    showDialog(request, signal) {
        return new Promise((resolve, reject) => {
            this.calls.push({ request, signal, resolve, reject });
        });
    }
}
test("dialog handler contribution starts at BlockStartup", async () => {
    const env_1 = { stack: [], error: void 0, hasError: false };
    try {
        const service = __addDisposableResource(env_1, new DialogService(), false);
        const handler = new TestDialogHandler();
        const services = new ServiceCollection();
        services.set(IDialogsModel, service.model);
        services.set(IWorkbenchDialogHandler, handler);
        const host = __addDisposableResource(env_1, WorkbenchContributionsRegistry.createHost(services), false);
        host.advance(WorkbenchPhase.BlockStartup);
        const confirmation = service.confirm({ message: "Ready?" });
        assert.equal(handler.calls.length, 1);
        handler.calls[0]?.resolve(DialogResult.Primary);
        assert.equal(await confirmation, true);
    }
    catch (e_1) {
        env_1.error = e_1;
        env_1.hasError = true;
    }
    finally {
        __disposeResources(env_1);
    }
});
test("dialog handler contribution presents the model queue serially", async () => {
    const env_2 = { stack: [], error: void 0, hasError: false };
    try {
        const service = __addDisposableResource(env_2, new DialogService(), false);
        const handler = new TestDialogHandler();
        const contribution = __addDisposableResource(env_2, new DialogHandlerContribution(service.model, handler), false);
        const confirmation = service.confirm({ message: "Continue?" });
        const message = service.showMessage({
            severity: DialogSeverity.Info,
            message: "Finished",
        });
        assert.equal(service.model.dialogs.length, 2);
        assert.equal(handler.calls.length, 1);
        assert.equal(handler.calls[0]?.request.kind, "confirmation");
        handler.calls[0]?.resolve(DialogResult.Primary);
        assert.equal(await confirmation, true);
        assert.equal(handler.calls.length, 2);
        assert.equal(handler.calls[1]?.request.kind, "message");
        handler.calls[1]?.resolve(DialogResult.Primary);
        await message;
        assert.equal(service.model.dialogs.length, 0);
    }
    catch (e_2) {
        env_2.error = e_2;
        env_2.hasError = true;
    }
    finally {
        __disposeResources(env_2);
    }
});
test("dialog handler contribution picks up existing model items", async () => {
    const env_3 = { stack: [], error: void 0, hasError: false };
    try {
        const service = __addDisposableResource(env_3, new DialogService(), false);
        const confirmation = service.confirm({ message: "Pending" });
        const handler = new TestDialogHandler();
        const contribution = __addDisposableResource(env_3, new DialogHandlerContribution(service.model, handler), false);
        assert.equal(handler.calls.length, 1);
        assert.equal(handler.calls[0]?.request.message, "Pending");
        handler.calls[0]?.resolve(DialogResult.Cancel);
        assert.equal(await confirmation, false);
    }
    catch (e_3) {
        env_3.error = e_3;
        env_3.hasError = true;
    }
    finally {
        __disposeResources(env_3);
    }
});
test("closing the active model item aborts its handler", async () => {
    const env_4 = { stack: [], error: void 0, hasError: false };
    try {
        const service = __addDisposableResource(env_4, new DialogService(), false);
        const handler = new TestDialogHandler();
        const contribution = __addDisposableResource(env_4, new DialogHandlerContribution(service.model, handler), false);
        const confirmation = service.confirm({ message: "Cancel?" });
        const item = service.model.dialogs[0];
        const call = handler.calls[0];
        item?.cancel();
        assert.equal(await confirmation, false);
        assert.equal(call?.signal.aborted, true);
        call?.resolve(DialogResult.Cancel);
    }
    catch (e_4) {
        env_4.error = e_4;
        env_4.hasError = true;
    }
    finally {
        __disposeResources(env_4);
    }
});
test("dialog handler contribution continues after a handler failure", async () => {
    const env_5 = { stack: [], error: void 0, hasError: false };
    try {
        const service = __addDisposableResource(env_5, new DialogService(), false);
        const handler = new TestDialogHandler();
        const contribution = __addDisposableResource(env_5, new DialogHandlerContribution(service.model, handler), false);
        const failed = service.showMessage({
            severity: DialogSeverity.Error,
            message: "Failure",
        });
        const next = service.confirm({ message: "Retry?" });
        handler.calls[0]?.reject(new Error("render failed"));
        await assert.rejects(failed, /render failed/);
        assert.equal(handler.calls.length, 2);
        handler.calls[1]?.resolve(DialogResult.Primary);
        assert.equal(await next, true);
    }
    catch (e_5) {
        env_5.error = e_5;
        env_5.hasError = true;
    }
    finally {
        __disposeResources(env_5);
    }
});
test("disposing the contribution cancels its active model item", async () => {
    const env_6 = { stack: [], error: void 0, hasError: false };
    try {
        const service = __addDisposableResource(env_6, new DialogService(), false);
        const handler = new TestDialogHandler();
        const contribution = new DialogHandlerContribution(service.model, handler);
        const confirmation = service.confirm({ message: "Active" });
        const call = handler.calls[0];
        contribution.dispose();
        assert.equal(call?.signal.aborted, true);
        assert.equal(await confirmation, false);
        call?.resolve(DialogResult.Cancel);
    }
    catch (e_6) {
        env_6.error = e_6;
        env_6.hasError = true;
    }
    finally {
        __disposeResources(env_6);
    }
});

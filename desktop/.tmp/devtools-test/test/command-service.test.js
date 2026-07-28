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
import { CommandRegistry, } from "../src/platform/commands/common/commands.js";
import { ServiceCollection, } from "../src/platform/instantiation/common/instantiation.js";
import { CommandService, } from "../src/workbench/services/commands/common/commandService.js";
test("command service emits execution events around the handler call", async () => {
    const env_1 = { stack: [], error: void 0, hasError: false };
    try {
        const registry = new CommandRegistry();
        const order = [];
        const commandRegistration = __addDisposableResource(env_1, registry.register("test.command.events", async () => {
            order.push("handler");
            return "result";
        }), false);
        const service = __addDisposableResource(env_1, new CommandService(new ServiceCollection(), registry), false);
        const willListener = __addDisposableResource(env_1, service.onWillExecuteCommand((event) => {
            order.push(`will:${event.commandId}:${String(event.args[0])}`);
        }), false);
        const didListener = __addDisposableResource(env_1, service.onDidExecuteCommand((event) => {
            order.push(`did:${event.commandId}:${String(event.args[0])}`);
        }), false);
        const result = await service.executeCommand("test.command.events", "argument");
        assert.equal(result, "result");
        assert.deepEqual(order, [
            "will:test.command.events:argument",
            "handler",
            "did:test.command.events:argument",
        ]);
    }
    catch (e_1) {
        env_1.error = e_1;
        env_1.hasError = true;
    }
    finally {
        __disposeResources(env_1);
    }
});
test("command service does not emit did when a handler throws", async () => {
    const env_2 = { stack: [], error: void 0, hasError: false };
    try {
        const registry = new CommandRegistry();
        const commandRegistration = __addDisposableResource(env_2, registry.register("test.command.failure", () => {
            throw new Error("failed");
        }), false);
        const service = __addDisposableResource(env_2, new CommandService(new ServiceCollection(), registry), false);
        let willCount = 0;
        let didCount = 0;
        const willListener = __addDisposableResource(env_2, service.onWillExecuteCommand(() => {
            willCount += 1;
        }), false);
        const didListener = __addDisposableResource(env_2, service.onDidExecuteCommand(() => {
            didCount += 1;
        }), false);
        await assert.rejects(service.executeCommand("test.command.failure"), /failed/);
        assert.equal(willCount, 1);
        assert.equal(didCount, 0);
    }
    catch (e_2) {
        env_2.error = e_2;
        env_2.hasError = true;
    }
    finally {
        __disposeResources(env_2);
    }
});

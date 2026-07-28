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
import { MenuId, } from "../src/platform/actions/common/actions.js";
import { MenuService, } from "../src/platform/actions/common/menuService.js";
import { ContextKeyService, } from "../src/platform/contextkey/common/contextkey.js";
import { ServiceCollection, } from "../src/platform/instantiation/common/instantiation.js";
import { NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL, } from "../src/platform/native/common/nativeHost.js";
import { nativeHostIpcRoutes, } from "../src/platform/native/electron-main/nativeHostIpc.js";
import { INativeHostService, } from "../src/workbench/common/services.js";
import { ToggleDeveloperToolsCommandId, } from "../src/workbench/electron-browser/developerToolsActions.js";
import { CommandService, } from "../src/workbench/services/commands/common/commandService.js";
test("native host route validates and toggles developer tools", () => {
    let toggles = 0;
    const [route] = nativeHostIpcRoutes({
        toggleDeveloperTools: () => {
            toggles += 1;
        },
    });
    assert.equal(route.channel, NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL);
    assert.throws(() => route.validate(null), /does not accept parameters/);
    route.invoke(route.validate(undefined));
    assert.equal(toggles, 1);
});
test("developer tools command is available from the command palette", async () => {
    const env_1 = { stack: [], error: void 0, hasError: false };
    try {
        const services = new ServiceCollection();
        let toggles = 0;
        services.set(INativeHostService, {
            async toggleDeveloperTools() {
                toggles += 1;
            },
        });
        const commands = __addDisposableResource(env_1, new CommandService(services), false);
        const contexts = __addDisposableResource(env_1, new ContextKeyService(), false);
        const paletteActions = new MenuService(commands, contexts)
            .getMenuActions(MenuId.CommandPalette)
            .flatMap(([, actions]) => actions);
        const action = paletteActions.find(({ id }) => id === ToggleDeveloperToolsCommandId);
        assert.equal(action?.label, "Developer: Toggle Developer Tools");
        await action?.run();
        assert.equal(toggles, 1);
    }
    catch (e_1) {
        env_1.error = e_1;
        env_1.hasError = true;
    }
    finally {
        __disposeResources(env_1);
    }
});

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
import { toDisposable } from "../src/base/common/lifecycle.js";
import { URI } from "../src/base/common/uri.js";
import { ConfigurationsRegistry, } from "../src/platform/configuration/common/configurationRegistry.js";
import { ContextKeyService, } from "../src/platform/contextkey/common/contextkey.js";
import { createServiceIdentifier, ServiceCollection, SyncDescriptor, } from "../src/platform/instantiation/common/instantiation.js";
import { darkColorTheme, lightColorTheme, } from "../src/platform/theme/common/colorTheme.js";
import { bindWorkbenchContextKeys, getVisibleViewContextKey, } from "../src/workbench/common/contextkeys.js";
import { WorkbenchContributionRegistry, WorkbenchPhase, } from "../src/workbench/common/contributions.js";
import { WorkbenchConfiguration } from "../src/workbench/common/configuration.js";
import { DialogsModel } from "../src/workbench/common/dialogs.js";
import { getWorkbenchColorTheme, WorkbenchThemeRegistry, } from "../src/workbench/common/theme.js";
import { ViewContainerLocation, WorkbenchViewRegistry, } from "../src/workbench/common/views.js";
import { DialogResult, DialogSeverity, } from "../src/platform/dialogs/common/dialogs.js";
test("workbench context keys describe the current workspace", () => {
    const env_1 = { stack: [], error: void 0, hasError: false };
    try {
        const contextKeys = __addDisposableResource(env_1, new ContextKeyService(), false);
        const workspace = {
            getWorkbenchState: () => 2 /* WorkbenchState.FOLDER */,
            getWorkspace: () => ({
                id: "workspace",
                folders: [{
                        index: 0,
                        name: "project",
                        uri: URI.file("C:\\project"),
                    }],
            }),
        };
        const bindings = __addDisposableResource(env_1, bindWorkbenchContextKeys(contextKeys, workspace), false);
        assert.equal(contextKeys.getValue("workbenchState"), "folder");
        assert.equal(contextKeys.getValue("workspaceFolderCount"), 1);
        assert.equal(contextKeys.getValue("sideBarVisible"), true);
        assert.equal(getVisibleViewContextKey("zeta.explorer"), "view.zeta.explorer.visible");
    }
    catch (e_1) {
        env_1.error = e_1;
        env_1.hasError = true;
    }
    finally {
        __disposeResources(env_1);
    }
});
test("workbench contributions start once at their declared phases", () => {
    const env_2 = { stack: [], error: void 0, hasError: false };
    try {
        const serviceId = createServiceIdentifier("testService");
        const services = new ServiceCollection();
        services.set(serviceId, "ready");
        const registry = new WorkbenchContributionRegistry();
        const calls = [];
        const startupRegistration = __addDisposableResource(env_2, registry.register("test.startup", WorkbenchPhase.BlockStartup, (accessor) => {
            calls.push(`startup:${accessor.get(serviceId)}`);
            return toDisposable(() => calls.push("dispose:startup"));
        }), false);
        const restoredRegistration = __addDisposableResource(env_2, registry.register("test.restored", WorkbenchPhase.AfterRestored, () => {
            calls.push("restored");
            return toDisposable(() => calls.push("dispose:restored"));
        }), false);
        {
            const env_3 = { stack: [], error: void 0, hasError: false };
            try {
                const host = __addDisposableResource(env_3, registry.createHost(services), false);
                host.advance(WorkbenchPhase.BlockStartup);
                host.advance(WorkbenchPhase.BlockRestore);
                host.advance(WorkbenchPhase.AfterRestored);
                host.advance(WorkbenchPhase.AfterRestored);
                assert.deepEqual(calls, ["startup:ready", "restored"]);
            }
            catch (e_2) {
                env_3.error = e_2;
                env_3.hasError = true;
            }
            finally {
                __disposeResources(env_3);
            }
        }
        assert.deepEqual(calls, [
            "startup:ready",
            "restored",
            "dispose:restored",
            "dispose:startup",
        ]);
    }
    catch (e_3) {
        env_2.error = e_3;
        env_2.hasError = true;
    }
    finally {
        __disposeResources(env_2);
    }
});
test("workbench configuration resolves registered color themes", () => {
    assert.equal(ConfigurationsRegistry.owns(WorkbenchConfiguration.colorTheme), true);
    assert.equal(WorkbenchConfiguration.colorTheme.defaultValue, darkColorTheme.id);
    assert.equal(WorkbenchConfiguration.colorTheme.parse(lightColorTheme.id), lightColorTheme.id);
    assert.throws(() => WorkbenchConfiguration.colorTheme.parse("missing-theme"), /Unknown workbench color theme/);
    assert.equal(getWorkbenchColorTheme(lightColorTheme.id), lightColorTheme);
});
test("workbench theme registries reject duplicate themes", () => {
    const env_4 = { stack: [], error: void 0, hasError: false };
    try {
        const registry = new WorkbenchThemeRegistry([darkColorTheme]);
        assert.equal(registry.getColorTheme(darkColorTheme.id), darkColorTheme);
        assert.throws(() => registry.registerColorTheme(darkColorTheme), /already registered/);
        const registration = __addDisposableResource(env_4, registry.registerColorTheme(lightColorTheme), false);
        assert.deepEqual(registry.getColorThemes().map((theme) => theme.id), [darkColorTheme.id, lightColorTheme.id]);
    }
    catch (e_4) {
        env_4.error = e_4;
        env_4.hasError = true;
    }
    finally {
        __disposeResources(env_4);
    }
});
test("dialogs model publishes and settles renderer items", async () => {
    const env_5 = { stack: [], error: void 0, hasError: false };
    try {
        const model = __addDisposableResource(env_5, new DialogsModel(), false);
        const events = [];
        const willShow = __addDisposableResource(env_5, model.onWillShowDialog((item) => events.push(`show:${item.request.kind}`)), false);
        const didClose = __addDisposableResource(env_5, model.onDidCloseDialog((event) => events.push(event.kind === "result"
            ? `close:${event.result}`
            : "close:error")), false);
        const handle = model.show({
            kind: "message",
            severity: DialogSeverity.Info,
            message: "Saved",
        });
        assert.equal(model.dialogs.length, 1);
        handle.item.close(DialogResult.Primary);
        assert.equal(await handle.result, DialogResult.Primary);
        assert.equal(model.dialogs.length, 0);
        assert.deepEqual(events, ["show:message", "close:primary"]);
    }
    catch (e_5) {
        env_5.error = e_5;
        env_5.hasError = true;
    }
    finally {
        __disposeResources(env_5);
    }
});
test("view registrations are ordered and disposed atomically", () => {
    const env_6 = { stack: [], error: void 0, hasError: false };
    try {
        const registry = new WorkbenchViewRegistry();
        const changes = [];
        const registered = __addDisposableResource(env_6, registry.onDidRegisterViews((event) => changes.push(`add:${event.views.map((view) => view.id).join(",")}`)), false);
        const removed = __addDisposableResource(env_6, registry.onDidDeregisterViews((event) => changes.push(`remove:${event.views.map((view) => view.id).join(",")}`)), false);
        const container = __addDisposableResource(env_6, registry.registerViewContainer({
            id: "zeta.sidebar",
            title: "Navigation",
            location: ViewContainerLocation.Sidebar,
        }), false);
        const views = __addDisposableResource(env_6, registry.registerViews("zeta.sidebar", [
            {
                id: "zeta.search",
                title: "Search",
                order: 20,
                ctorDescriptor: new SyncDescriptor(TestView, {
                    staticArguments: ["zeta.search"],
                }),
            },
            {
                id: "zeta.explorer",
                title: "Explorer",
                order: 10,
                ctorDescriptor: new SyncDescriptor(TestView, {
                    staticArguments: ["zeta.explorer"],
                }),
            },
        ]), false);
        assert.deepEqual(registry.getViews("zeta.sidebar").map((view) => view.id), ["zeta.explorer", "zeta.search"]);
        assert.equal(registry.getViewContainerForView("zeta.explorer")?.id, "zeta.sidebar");
        assert.throws(() => registry.registerViews("zeta.sidebar", [
            {
                id: "zeta.explorer",
                title: "Duplicate",
                ctorDescriptor: new SyncDescriptor(TestView, {
                    staticArguments: ["zeta.explorer"],
                }),
            },
        ]), /already registered/);
        assert.deepEqual(registry.getViews("zeta.sidebar").map((view) => view.id), ["zeta.explorer", "zeta.search"]);
        views.dispose();
        assert.deepEqual(changes, [
            "add:zeta.explorer,zeta.search",
            "remove:zeta.explorer,zeta.search",
        ]);
    }
    catch (e_6) {
        env_6.error = e_6;
        env_6.hasError = true;
    }
    finally {
        __disposeResources(env_6);
    }
});
class TestView {
    id;
    #visible = true;
    constructor(id) {
        this.id = id;
    }
    focus() { }
    isVisible() {
        return this.#visible;
    }
    setVisible(visible) {
        this.#visible = visible;
    }
}

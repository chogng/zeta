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
import { ContextKeyExpr, ContextKeyService, } from "../src/platform/contextkey/common/contextkey.js";
import { SyncDescriptor, } from "../src/platform/instantiation/common/instantiation.js";
import { ViewContainerLocation, WorkbenchViewRegistry, } from "../src/workbench/common/views.js";
import { ViewDescriptorService, } from "../src/workbench/services/views/common/viewDescriptorService.js";
test("view descriptor models project registry and context visibility", () => {
    const env_1 = { stack: [], error: void 0, hasError: false };
    try {
        const contextKeys = __addDisposableResource(env_1, new ContextKeyService(), false);
        const registry = new WorkbenchViewRegistry();
        const descriptors = __addDisposableResource(env_1, new ViewDescriptorService({
            contextKeyService: contextKeys,
            registry,
        }), false);
        const containerRegistration = __addDisposableResource(env_1, registry.registerViewContainer({
            id: "test.sidebar",
            title: "Test",
            location: ViewContainerLocation.Sidebar,
            isDefault: true,
        }), false);
        const model = descriptors.getViewContainerModel("test.sidebar");
        const changes = [];
        const listener = __addDisposableResource(env_1, model.onDidChangeVisibleViewDescriptors((event) => {
            changes.push(`+${event.added.map((view) => view.id).join(",")}` +
                ` -${event.removed.map((view) => view.id).join(",")}`);
        }), false);
        const viewRegistrations = __addDisposableResource(env_1, registry.registerViews("test.sidebar", [
            testView("test.always", "Always", {
                order: 10,
            }),
            testView("test.conditional", "Conditional", {
                order: 20,
                when: ContextKeyExpr.has("test.featureEnabled"),
            }),
            testView("test.hidden", "Hidden", {
                order: 30,
                hideByDefault: true,
            }),
        ]), false);
        assert.deepEqual(model.visibleViewDescriptors.map((view) => view.id), ["test.always"]);
        assert.equal(contextKeys.getValue("view.test.always.visible"), true);
        assert.equal(contextKeys.getValue("view.test.conditional.visible"), false);
        contextKeys.setContext("test.featureEnabled", true);
        assert.deepEqual(model.visibleViewDescriptors.map((view) => view.id), ["test.always", "test.conditional"]);
        model.setVisible("test.hidden", true);
        assert.deepEqual(model.visibleViewDescriptors.map((view) => view.id), ["test.always", "test.conditional", "test.hidden"]);
        model.setVisible("test.always", false);
        assert.equal(contextKeys.getValue("view.test.always.visible"), false);
        assert.deepEqual(changes, [
            "+test.always -",
            "+test.conditional -",
            "+test.hidden -",
            "+ -test.always",
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
test("view descriptor service resolves default containers by location", () => {
    const env_2 = { stack: [], error: void 0, hasError: false };
    try {
        const contextKeys = __addDisposableResource(env_2, new ContextKeyService(), false);
        const registry = new WorkbenchViewRegistry();
        const first = __addDisposableResource(env_2, registry.registerViewContainer({
            id: "test.first",
            title: "First",
            location: ViewContainerLocation.Sidebar,
            order: 20,
        }), false);
        const defaultContainer = __addDisposableResource(env_2, registry.registerViewContainer({
            id: "test.default",
            title: "Default",
            location: ViewContainerLocation.Sidebar,
            order: 30,
            isDefault: true,
        }), false);
        const descriptors = __addDisposableResource(env_2, new ViewDescriptorService({
            contextKeyService: contextKeys,
            registry,
        }), false);
        assert.equal(descriptors.getDefaultViewContainer(ViewContainerLocation.Sidebar)?.id, "test.default");
        assert.deepEqual(descriptors.getViewContainers(ViewContainerLocation.Sidebar)
            .map((container) => container.id), ["test.first", "test.default"]);
    }
    catch (e_2) {
        env_2.error = e_2;
        env_2.hasError = true;
    }
    finally {
        __disposeResources(env_2);
    }
});
function testView(id, title, options = {}) {
    return {
        id,
        title,
        ctorDescriptor: new SyncDescriptor(TestView, {
            staticArguments: [id],
        }),
        ...options,
    };
}
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

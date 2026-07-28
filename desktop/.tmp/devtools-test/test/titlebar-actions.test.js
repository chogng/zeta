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
import { JSDOM } from "jsdom";
const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
    window: browserEnvironment.window,
    document: browserEnvironment.window.document,
    Node: browserEnvironment.window.Node,
    Element: browserEnvironment.window.Element,
    HTMLElement: browserEnvironment.window.HTMLElement,
    Event: browserEnvironment.window.Event,
    MouseEvent: browserEnvironment.window.MouseEvent,
    navigator: browserEnvironment.window.navigator,
})) {
    Object.defineProperty(globalThis, name, {
        configurable: true,
        value,
    });
}
const { DisposableStore, toDisposable } = await import("../src/base/common/lifecycle.js");
const { MenuId, MenusRegistry } = await import("../src/platform/actions/common/actions.js");
const { MenuService } = await import("../src/platform/actions/common/menuService.js");
const { CommandsRegistry } = await import("../src/platform/commands/common/commands.js");
const { ContextKeyService } = await import("../src/platform/contextkey/common/contextkey.js");
const { ServiceCollection } = await import("../src/platform/instantiation/common/instantiation.js");
const { CommandService } = await import("../src/workbench/services/commands/common/commandService.js");
const { BrowserTitlebarPart } = await import("../src/workbench/browser/parts/titlebar/titlebarPart.js");
const { BrowserMenubarControl } = await import("../src/workbench/browser/parts/titlebar/menubarControl.js");
const noEvent = () => toDisposable(() => { });
const contextMenuService = {
    onDidShowContextMenu: noEvent,
    onDidHideContextMenu: noEvent,
    showContextMenu() { },
    hideContextMenu() { },
};
test("titlebar owns a menu-driven actions container", async () => {
    const env_1 = { stack: [], error: void 0, hasError: false };
    try {
        const disposables = __addDisposableResource(env_1, new DisposableStore(), false);
        const ownerDocument = browserEnvironment.window.document;
        ownerDocument.body.replaceChildren();
        const commandService = disposables.add(new CommandService(new ServiceCollection()));
        const contextKeyService = disposables.add(new ContextKeyService());
        const menuService = new MenuService(commandService, contextKeyService);
        let runs = 0;
        const commandId = "test.titlebar.action";
        disposables.add(CommandsRegistry.register(commandId, () => {
            runs += 1;
        }));
        disposables.add(MenusRegistry.appendMenuItem(MenuId.TitleBar, {
            command: {
                id: commandId,
                title: "Title action",
            },
            group: "navigation",
        }));
        const menubarElement = ownerDocument.createElement("nav");
        let menubarDisposed = false;
        const menubar = {
            element: menubarElement,
            dispose() {
                menubarDisposed = true;
                menubarElement.remove();
            },
            [Symbol.dispose]() {
                this.dispose();
            },
        };
        const titlebar = disposables.add(new BrowserTitlebarPart({
            menuService,
            contextMenuService,
            ownerDocument,
            title: "Zeta",
        }, menubar));
        ownerDocument.body.append(titlebar.element);
        const actionsContainer = titlebar.element.querySelector(".zeta-workbench-part-content > .zeta-titlebar-actions");
        assert.ok(actionsContainer);
        assert.equal(actionsContainer.querySelector(".zeta-action-bar")
            ?.getAttribute("role"), "toolbar");
        const button = actionsContainer.querySelector("button");
        assert.equal(button?.textContent, "Title action");
        button?.click();
        await Promise.resolve();
        assert.equal(runs, 1);
        const secondMenuRegistration = disposables.add(MenusRegistry.appendMenuItem(MenuId.TitleBar, {
            command: {
                id: commandId,
                title: "Second title action",
            },
            group: "navigation",
            order: 20,
        }));
        assert.deepEqual([...actionsContainer.querySelectorAll("button")]
            .map((element) => element.textContent), ["Title action", "Second title action"]);
        secondMenuRegistration.dispose();
        titlebar.dispose();
        assert.equal(titlebar.element.isConnected, false);
        assert.equal(menubarDisposed, true);
    }
    catch (e_1) {
        env_1.error = e_1;
        env_1.hasError = true;
    }
    finally {
        __disposeResources(env_1);
    }
});
test("titlebar renders left actions before the application menu", () => {
    const env_2 = { stack: [], error: void 0, hasError: false };
    try {
        const disposables = __addDisposableResource(env_2, new DisposableStore(), false);
        const ownerDocument = browserEnvironment.window.document;
        ownerDocument.body.replaceChildren();
        const commandService = disposables.add(new CommandService(new ServiceCollection()));
        const contextKeyService = disposables.add(new ContextKeyService());
        const menuService = new MenuService(commandService, contextKeyService);
        disposables.add(MenusRegistry.appendMenuItem(MenuId.TitleBarLeft, {
            command: {
                id: "test.titlebar.leftAction",
                title: "Left title action",
            },
            group: "navigation",
        }));
        const menubarElement = ownerDocument.createElement("nav");
        const titlebar = disposables.add(new BrowserTitlebarPart({
            menuService,
            contextMenuService,
            ownerDocument,
            title: "Zeta",
        }, {
            element: menubarElement,
            dispose() {
                menubarElement.remove();
            },
            [Symbol.dispose]() {
                this.dispose();
            },
        }));
        const titleChildren = [...titlebar.element.querySelector(".zeta-workbench-part-title")?.children ?? []];
        assert.equal(titleChildren[0]?.classList.contains("zeta-titlebar-left-actions"), true);
        assert.equal(titleChildren[1], menubarElement);
        assert.equal(titleChildren[0]?.querySelector("button")?.textContent, "Left title action");
    }
    catch (e_2) {
        env_2.error = e_2;
        env_2.hasError = true;
    }
    finally {
        __disposeResources(env_2);
    }
});
test("browser titlebar uses one icon trigger for the application menus", () => {
    const env_3 = { stack: [], error: void 0, hasError: false };
    try {
        const disposables = __addDisposableResource(env_3, new DisposableStore(), false);
        const ownerDocument = browserEnvironment.window.document;
        ownerDocument.body.replaceChildren();
        const commandService = disposables.add(new CommandService(new ServiceCollection()));
        const contextKeyService = disposables.add(new ContextKeyService());
        const menuService = new MenuService(commandService, contextKeyService);
        const emptyFileMenu = new MenuId("test.titlebar.file");
        const emptyEditMenu = new MenuId("test.titlebar.edit");
        disposables.add(MenusRegistry.appendMenuItem(MenuId.MenubarMainMenu, {
            title: "File",
            submenu: emptyFileMenu,
            group: "navigation",
            order: 1,
        }));
        disposables.add(MenusRegistry.appendMenuItem(MenuId.MenubarMainMenu, {
            title: "Edit",
            submenu: emptyEditMenu,
            group: "navigation",
            order: 2,
        }));
        let menuLabels = [];
        const menuContextService = {
            onDidShowContextMenu: noEvent,
            onDidHideContextMenu: noEvent,
            showContextMenu(options) {
                if ("actions" in options) {
                    menuLabels = options.actions.map((action) => action.label);
                }
            },
            hideContextMenu() { },
        };
        const menubar = disposables.add(new BrowserMenubarControl(menuService, menuContextService, ownerDocument));
        ownerDocument.body.append(menubar.element);
        const button = menubar.element.querySelector("button");
        assert.ok(button);
        assert.equal(button.title, "Application menu");
        assert.ok(button.querySelector(".zeta-icon"));
        assert.equal(menubar.element.querySelectorAll("button").length, 1);
        button.click();
        assert.deepEqual(menuLabels, ["File", "Edit"]);
        assert.equal(button.getAttribute("aria-expanded"), "true");
    }
    catch (e_3) {
        env_3.error = e_3;
        env_3.hasError = true;
    }
    finally {
        __disposeResources(env_3);
    }
});

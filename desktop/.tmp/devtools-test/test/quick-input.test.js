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
import { Action2, registerAction2, } from "../src/platform/actions/common/actions.js";
import { IMenuService, MenuService, } from "../src/platform/actions/common/menuService.js";
import { ICommandService, } from "../src/platform/commands/common/commands.js";
import { ContextKeyService, IContextKeyService, } from "../src/platform/contextkey/common/contextkey.js";
import { ServiceCollection, } from "../src/platform/instantiation/common/instantiation.js";
import { IKeybindingService, } from "../src/platform/keybinding/common/keybinding.js";
import { filterQuickPickItems, QuickInputList, } from "../src/platform/quickinput/browser/quickInputList.js";
import { IQuickInputService, } from "../src/platform/quickinput/common/quickInput.js";
import { CommandService, } from "../src/workbench/services/commands/common/commandService.js";
import { InQuickInputContext, WorkbenchQuickInputService, } from "../src/workbench/services/quickinput/browser/quickInputService.js";
import { ShowAllCommandsCommandId, } from "../src/workbench/contrib/quickaccess/browser/commandsQuickAccess.js";
test("Quick Pick filtering matches ordered characters and favors labels", () => {
    const items = [
        { label: "Open Folder", description: "workbench.openFolder" },
        { label: "Format Document", description: "editor.formatDocument" },
        { label: "Focus Sidebar", description: "workbench.focusSidebar" },
    ];
    assert.deepEqual(filterQuickPickItems(items, "open f").map((item) => item.label), ["Open Folder"]);
    assert.deepEqual(filterQuickPickItems(items, "format").map((item) => item.label), ["Format Document"]);
    assert.deepEqual(filterQuickPickItems(items, "missing"), []);
});
test("QuickInputList owns filtering, looping focus, and acceptance", () => {
    const dom = new JSDOM("<!doctype html><body></body>");
    installDomGlobals(dom);
    const list = new QuickInputList(dom.window.document);
    dom.window.document.body.append(list.element);
    const activeLabels = [];
    const acceptedLabels = [];
    const activeListener = list.onDidChangeActive(({ item }) => {
        activeLabels.push(item?.label);
    });
    const acceptListener = list.onDidAccept((item) => {
        acceptedLabels.push(item.label);
    });
    list.items = [
        { label: "First" },
        { label: "Second" },
        { label: "Third" },
    ];
    assert.equal(list.activeItem?.label, "First");
    list.focusPrevious();
    assert.equal(list.activeItem?.label, "Third");
    list.acceptActive();
    assert.deepEqual(acceptedLabels, ["Third"]);
    list.filter("second");
    assert.deepEqual(list.visibleItems.map((item) => item.label), ["Second"]);
    assert.equal(list.activeItem?.label, "Second");
    list.filter("missing");
    assert.equal(list.activeItem, undefined);
    assert.equal(list.element.querySelector(".zeta-quick-pick-empty")?.textContent, "No matching results");
    assert.equal(activeLabels.at(-1), undefined);
    acceptListener.dispose();
    activeListener.dispose();
    list.dispose();
    dom.window.close();
});
test("Command Palette filters, executes, closes, and restores focus", async () => {
    const env_1 = { stack: [], error: void 0, hasError: false };
    try {
        const dom = new JSDOM("<!doctype html><body><main></main></body>");
        installDomGlobals(dom);
        const container = dom.window.document.querySelector("main");
        assert.ok(container);
        const focusTarget = dom.window.document.createElement("button");
        focusTarget.textContent = "Restore focus";
        container.append(focusTarget);
        focusTarget.focus();
        const services = new ServiceCollection();
        const contextKeys = new ContextKeyService();
        services.set(IContextKeyService, contextKeys);
        const commands = new CommandService(services);
        services.set(ICommandService, commands);
        const menus = new MenuService(commands, contextKeys);
        services.set(IMenuService, menus);
        const quickInput = new WorkbenchQuickInputService({
            container,
            contextKeyService: contextKeys,
        });
        services.set(IQuickInputService, quickInput);
        services.set(IKeybindingService, emptyKeybindingService());
        let executions = 0;
        class PaletteTargetAction extends Action2 {
            constructor() {
                super({
                    id: "test.quickInput.target",
                    title: "Run Palette Target",
                    f1: true,
                });
            }
            run() {
                executions += 1;
            }
        }
        const actionRegistration = __addDisposableResource(env_1, registerAction2(PaletteTargetAction), false);
        await commands.executeCommand(ShowAllCommandsCommandId);
        assert.equal(contextKeys.getValue(InQuickInputContext.key), true);
        const input = container.querySelector(".zeta-quick-pick-input input");
        assert.ok(input);
        input.value = "palette target";
        input.dispatchEvent(new dom.window.Event("input", { bubbles: true }));
        assert.deepEqual([...container.querySelectorAll(".zeta-quick-pick-row-label")]
            .map((label) => label.textContent), ["Run Palette Target"]);
        input.dispatchEvent(new dom.window.KeyboardEvent("keydown", {
            bubbles: true,
            cancelable: true,
            key: "Enter",
        }));
        await Promise.resolve();
        assert.equal(executions, 1);
        assert.equal(contextKeys.getValue(InQuickInputContext.key), false);
        assert.equal(container.querySelector(".zeta-quick-pick"), null);
        assert.equal(dom.window.document.activeElement, focusTarget);
        quickInput.dispose();
        commands.dispose();
        contextKeys.dispose();
        dom.window.close();
    }
    catch (e_1) {
        env_1.error = e_1;
        env_1.hasError = true;
    }
    finally {
        __disposeResources(env_1);
    }
});
function emptyKeybindingService() {
    return {
        inChordMode: false,
        onDidUpdateKeybindings: () => ({
            dispose() { },
            [Symbol.dispose]() { },
        }),
        resolveKeybinding() {
            throw new Error("Not needed by Command Palette test");
        },
        resolveUserBinding: () => undefined,
        lookupKeybindings: () => [],
        lookupKeybinding: () => undefined,
    };
}
function installDomGlobals(dom) {
    for (const [name, value] of Object.entries({
        window: dom.window,
        document: dom.window.document,
        Node: dom.window.Node,
        Element: dom.window.Element,
        HTMLElement: dom.window.HTMLElement,
        Event: dom.window.Event,
        MouseEvent: dom.window.MouseEvent,
        KeyboardEvent: dom.window.KeyboardEvent,
        navigator: dom.window.navigator,
    })) {
        Object.defineProperty(globalThis, name, {
            configurable: true,
            value,
        });
    }
}

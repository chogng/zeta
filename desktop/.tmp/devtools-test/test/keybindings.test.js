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
import { IME } from "../src/base/common/ime.js";
import { Keybinding, logicalKey, physicalKey, resolveKeybinding, } from "../src/base/common/keybindings.js";
import { DisposableStore } from "../src/base/common/lifecycle.js";
import { OperatingSystem } from "../src/base/common/platform.js";
import { getKeybindingLabel, } from "../src/base/common/keybindingLabels.js";
import { CommandRegistry, } from "../src/platform/commands/common/commands.js";
import { ContextKeyExpr, ContextKeyService, } from "../src/platform/contextkey/common/contextkey.js";
import { parseContextKeyExpression, } from "../src/platform/contextkey/common/contextKeyExpressionParser.js";
import { ServiceCollection, } from "../src/platform/instantiation/common/instantiation.js";
import { KeybindingResolveKind, KeybindingResolver, } from "../src/platform/keybinding/common/keybindingResolver.js";
import { KeybindingRegistry, KeybindingWeight, } from "../src/platform/keybinding/common/keybindingsRegistry.js";
import { BrowserKeyboardLayoutService, } from "../src/workbench/services/keybinding/browser/keyboardLayoutService.js";
import { WorkbenchKeybindingService, } from "../src/workbench/services/keybinding/browser/keybindingService.js";
import { CommandService, } from "../src/workbench/services/commands/common/commandService.js";
import { KeybindingsResourceContribution, } from "../src/workbench/services/keybinding/browser/keybindingsResourceContribution.js";
import { WorkbenchKeybindingsResourceService, } from "../src/workbench/services/keybinding/browser/keybindingsResourceService.js";
import { StatusbarAlignment, StatusbarService, } from "../src/workbench/services/statusbar/browser/statusbar.js";
test("resolver applies context, weight, and latest-registration precedence", () => {
    const env_1 = { stack: [], error: void 0, hasError: false };
    try {
        const registrations = __addDisposableResource(env_1, new DisposableStore(), false);
        const registry = new KeybindingRegistry();
        const contexts = registrations.add(new ContextKeyService());
        const keybinding = Keybinding.single(logicalKey("p", {
            ctrlKey: true,
        }));
        registrations.add(registry.registerKeybindingRule({
            command: "test.low",
            keybinding,
            weight: KeybindingWeight.Builtin,
        }));
        registrations.add(registry.registerKeybindingRule({
            command: "test.disabled",
            keybinding,
            when: ContextKeyExpr.has("test.enabled"),
            weight: KeybindingWeight.User,
        }));
        registrations.add(registry.registerKeybindingRule({
            command: "test.latest",
            keybinding,
        }));
        const resolver = new KeybindingResolver({
            registry,
            resolveKeybinding: (keybinding) => resolveKeybinding(keybinding, OperatingSystem.Windows),
        });
        const event = keyEventData();
        let result = resolver.resolve(contexts, [event]);
        assert.equal(result.kind, KeybindingResolveKind.Command);
        assert.equal(result.kind === KeybindingResolveKind.Command
            ? result.command
            : undefined, "test.latest");
        contexts.setContext("test.enabled", true);
        result = resolver.resolve(contexts, [event]);
        assert.equal(result.kind === KeybindingResolveKind.Command
            ? result.command
            : undefined, "test.disabled");
    }
    catch (e_1) {
        env_1.error = e_1;
        env_1.hasError = true;
    }
    finally {
        __disposeResources(env_1);
    }
});
test("when expressions preserve boolean precedence and comparisons", () => {
    const env_2 = { stack: [], error: void 0, hasError: false };
    try {
        const contexts = __addDisposableResource(env_2, new ContextKeyService(), false);
        const expression = parseContextKeyExpression("editorFocus && (mode == edit || !readOnly)");
        contexts.setContext("editorFocus", true);
        contexts.setContext("mode", "preview");
        contexts.setContext("readOnly", false);
        assert.equal(expression.evaluate(contexts), true);
        contexts.setContext("readOnly", true);
        assert.equal(expression.evaluate(contexts), false);
        contexts.setContext("mode", "edit");
        assert.equal(expression.evaluate(contexts), true);
        assert.throws(() => parseContextKeyExpression("editorFocus &&"), /Expected/);
    }
    catch (e_2) {
        env_2.error = e_2;
        env_2.hasError = true;
    }
    finally {
        __disposeResources(env_2);
    }
});
test("browser service executes chords and restores IME state", async () => {
    const env_3 = { stack: [], error: void 0, hasError: false };
    try {
        const registrations = __addDisposableResource(env_3, new DisposableStore(), false);
        const registry = new KeybindingRegistry();
        const commands = new CommandRegistry();
        const contexts = registrations.add(new ContextKeyService());
        let executions = 0;
        const executed = new Promise((resolve) => {
            registrations.add(commands.register("test.chord", () => {
                executions += 1;
                resolve();
            }));
        });
        registrations.add(registry.registerKeybindingRule({
            command: "test.chord",
            keybinding: Keybinding.chord(physicalKey("KeyK", { ctrlKey: true }), physicalKey("KeyC", { ctrlKey: true })),
        }));
        const keyboardLayout = registrations.add(new BrowserKeyboardLayoutService({
            navigator: fakeNavigator(),
            operatingSystem: OperatingSystem.Windows,
        }));
        const statusbar = registrations.add(new StatusbarService());
        const service = registrations.add(new WorkbenchKeybindingService({
            ownerDocument: new EventTarget(),
            commandService: new CommandService(new ServiceCollection(), commands),
            contextKeyService: contexts,
            keyboardLayoutService: keyboardLayout,
            statusbarService: statusbar,
            registry,
        }));
        IME.enable();
        const first = keyboardEvent({ code: "KeyK", key: "k" });
        assert.equal(service.dispatchEvent(first.event), true);
        assert.equal(first.prevented, true);
        assert.equal(IME.enabled, false);
        assert.equal(service.inChordMode, true);
        assert.equal(contexts.getValue("keybinding.inChordMode"), true);
        assert.match(statusbar.getEntries(StatusbarAlignment.Left)[0].entry.text, /Waiting for another key/);
        const second = keyboardEvent({ code: "KeyC", key: "c" });
        assert.equal(service.dispatchEvent(second.event), true);
        await executed;
        assert.equal(executions, 1);
        assert.equal(IME.enabled, true);
        assert.equal(service.inChordMode, false);
        assert.equal(contexts.getValue("keybinding.inChordMode"), false);
        assert.deepEqual(statusbar.getEntries(StatusbarAlignment.Left), []);
        assert.equal(getKeybindingLabel(service.resolveUserBinding("ctrl+k")), "Ctrl+K");
    }
    catch (e_3) {
        env_3.error = e_3;
        env_3.hasError = true;
    }
    finally {
        __disposeResources(env_3);
    }
});
test("browser service dispatches Ctrl+Shift+P with a shifted key value", async () => {
    const env_4 = { stack: [], error: void 0, hasError: false };
    try {
        const registrations = __addDisposableResource(env_4, new DisposableStore(), false);
        const registry = new KeybindingRegistry();
        const commands = new CommandRegistry();
        const contexts = registrations.add(new ContextKeyService());
        const commandId = "workbench.action.showCommands";
        const executed = new Promise((resolve) => {
            registrations.add(commands.register(commandId, () => resolve()));
        });
        registrations.add(registry.registerKeybindingRule({
            command: commandId,
            keybinding: Keybinding.single(logicalKey("p", {
                ctrlKey: true,
                shiftKey: true,
            })),
        }));
        const keyboardLayout = registrations.add(new BrowserKeyboardLayoutService({
            navigator: fakeNavigator(),
            operatingSystem: OperatingSystem.Windows,
        }));
        const service = registrations.add(new WorkbenchKeybindingService({
            ownerDocument: new EventTarget(),
            commandService: new CommandService(new ServiceCollection(), commands),
            contextKeyService: contexts,
            keyboardLayoutService: keyboardLayout,
            registry,
        }));
        const shortcut = keyboardEvent({
            code: "KeyP",
            key: "P",
            shiftKey: true,
        });
        assert.equal(service.dispatchEvent(shortcut.event), true);
        await executed;
        assert.equal(shortcut.prevented, true);
    }
    catch (e_4) {
        env_4.error = e_4;
        env_4.hasError = true;
    }
    finally {
        __disposeResources(env_4);
    }
});
test("browser keyboard layouts provide physical key labels", async () => {
    const env_5 = { stack: [], error: void 0, hasError: false };
    try {
        const service = __addDisposableResource(env_5, new BrowserKeyboardLayoutService({
            navigator: fakeNavigator(new Map([["KeyY", "z"]])),
            operatingSystem: OperatingSystem.Windows,
        }), false);
        await service.refreshKeyboardLayout();
        const resolved = service.getKeyboardMapper().resolveKeybinding(Keybinding.single(physicalKey("KeyY", { ctrlKey: true })));
        assert.equal(getKeybindingLabel(resolved), "Ctrl+Z");
        assert.equal(service.getCurrentKeyboardLayout().source, "browser");
    }
    catch (e_5) {
        env_5.error = e_5;
        env_5.hasError = true;
    }
    finally {
        __disposeResources(env_5);
    }
});
test("keybindings resource applies conditions, arguments, OS keys, and blockers", async () => {
    const env_6 = { stack: [], error: void 0, hasError: false };
    try {
        const registrations = __addDisposableResource(env_6, new DisposableStore(), false);
        const registry = new KeybindingRegistry();
        const contexts = registrations.add(new ContextKeyService());
        registrations.add(registry.registerKeybindingRule({
            command: "test.builtin",
            keybinding: Keybinding.single(logicalKey("p", {
                ctrlKey: true,
            })),
            weight: KeybindingWeight.Builtin,
        }));
        const keybindingsResource = registrations.add(new WorkbenchKeybindingsResourceService());
        registrations.add(new KeybindingsResourceContribution({
            service: keybindingsResource,
            registry,
            operatingSystem: "windows",
        }));
        const resolver = new KeybindingResolver({
            registry,
            resolveKeybinding: (keybinding) => resolveKeybinding(keybinding, OperatingSystem.Windows),
        });
        await keybindingsResource.updateKeybindings([{
                key: "ctrl+q",
                win: "ctrl+p",
                command: "test.user",
                when: "test.enabled && mode == edit",
                args: { source: "user" },
            }]);
        let result = resolver.resolve(contexts, [keyEventData()]);
        assert.equal(result.kind, KeybindingResolveKind.Command);
        assert.equal(result.kind === KeybindingResolveKind.Command
            ? result.command
            : undefined, "test.builtin");
        contexts.setContext("test.enabled", true);
        contexts.setContext("mode", "edit");
        result = resolver.resolve(contexts, [keyEventData()]);
        assert.equal(result.kind, KeybindingResolveKind.Command);
        assert.equal(result.kind === KeybindingResolveKind.Command
            ? result.command
            : undefined, "test.user");
        assert.deepEqual(result.kind === KeybindingResolveKind.Command
            ? result.args
            : undefined, [{ source: "user" }]);
        assert.equal(resolver.lookupKeybinding("test.builtin", contexts), undefined);
        await keybindingsResource.updateKeybindings([{
                key: "ctrl+p",
                command: null,
            }]);
        result = resolver.resolve(contexts, [keyEventData()]);
        assert.equal(result.kind, KeybindingResolveKind.Blocked);
        assert.equal(resolver.lookupKeybinding("test.user", contexts), undefined);
    }
    catch (e_6) {
        env_6.error = e_6;
        env_6.hasError = true;
    }
    finally {
        __disposeResources(env_6);
    }
});
function keyEventData() {
    return {
        key: "p",
        code: "KeyP",
        ctrlKey: true,
        shiftKey: false,
        altKey: false,
        metaKey: false,
    };
}
function keyboardEvent(overrides = {}) {
    let prevented = false;
    const event = {
        key: "p",
        code: "KeyP",
        ctrlKey: true,
        shiftKey: false,
        altKey: false,
        metaKey: false,
        repeat: false,
        isComposing: false,
        target: null,
        composedPath: () => [],
        getModifierState: () => false,
        preventDefault: () => {
            prevented = true;
        },
        stopPropagation: () => { },
        stopImmediatePropagation: () => { },
        ...overrides,
    };
    return {
        event,
        get prevented() {
            return prevented;
        },
    };
}
function fakeNavigator(layout) {
    const keyboard = layout
        ? {
            async getLayoutMap() {
                return layout;
            },
        }
        : undefined;
    return {
        language: "en-US",
        keyboard,
    };
}

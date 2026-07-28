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
import { mkdtemp, readFile, rm, } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { ConfigurationMainService, } from "../src/platform/configuration/electron-main/configurationMainService.js";
import { validateKeybindingsResource, validateKeybindingsResourceSnapshot, } from "../src/platform/keybinding/common/keybindingsResource.js";
import { migrateLegacyKeybindings, } from "../src/platform/keybinding/electron-main/migrateLegacyKeybindings.js";
import { KeybindingsResourceMainService, } from "../src/platform/keybinding/electron-main/keybindingsResourceMainService.js";
import { WorkbenchKeybindingsResourceService, } from "../src/workbench/services/keybinding/browser/keybindingsResourceService.js";
test("keybinding resource wire data validates complete ordered rules", () => {
    assert.deepEqual(validateKeybindingsResourceSnapshot({
        revision: 3,
        bindings: [{
                key: "primary+k primary+c",
                command: "zeta.comment",
                when: "editorFocus && mode == edit",
                args: { source: "keyboard" },
                mac: "cmd+k cmd+c",
                linux: null,
            }],
    }), {
        revision: 3,
        bindings: [{
                key: "primary+k primary+c",
                command: "zeta.comment",
                when: "editorFocus && mode == edit",
                args: { source: "keyboard" },
                mac: "cmd+k cmd+c",
                linux: null,
            }],
    });
    assert.throws(() => validateKeybindingsResource([{
            key: "ctrl+k",
            command: "zeta.test",
            unknown: true,
        }]), /unknown field/);
    assert.throws(() => validateKeybindingsResource([{
            key: "ctrl+k",
            command: "zeta.test",
            when: "editorFocus &&",
        }]), /Expected/);
});
test("workbench keybindings resource accepts host snapshots and CAS updates", async () => {
    const env_1 = { stack: [], error: void 0, hasError: false };
    try {
        const api = new TestKeybindingsResourceApi({
            revision: 0,
            bindings: [{
                    key: "primary+n",
                    command: "zeta.new",
                }],
        });
        const service = __addDisposableResource(env_1, new WorkbenchKeybindingsResourceService({ api }), false);
        const observed = [];
        const listener = __addDisposableResource(env_1, service.onDidChangeKeybindings((bindings) => {
            observed.push(bindings.map(({ key }) => key));
        }), false);
        assert.deepEqual(service.getKeybindings(), []);
        await service.reload();
        assert.equal(service.getKeybindings()[0].command, "zeta.new");
        await service.updateKeybindings([{
                key: "primary+shift+n",
                command: "zeta.newWindow",
            }]);
        assert.equal(service.getKeybindings()[0].command, "zeta.newWindow");
        api.emit({
            revision: 2,
            bindings: [{
                    key: "primary+w",
                    command: null,
                }],
        });
        assert.equal(service.getKeybindings()[0].command, null);
        assert.deepEqual(observed, [
            ["primary+n"],
            ["primary+shift+n"],
            ["primary+w"],
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
test("main keybindings resource persists a standalone top-level array", async (context) => {
    const directory = await mkdtemp(join(tmpdir(), "zeta-keybindings-"));
    context.after(async () => {
        await rm(directory, { recursive: true, force: true });
    });
    const filePath = join(directory, "keybindings.json");
    const service = await KeybindingsResourceMainService.create({ filePath });
    const bindings = [{
            key: "primary+n",
            command: "zeta.new",
            when: "windowFocused",
        }];
    const updated = await service.update({
        expectedRevision: 0,
        bindings,
    });
    assert.equal(updated.revision, 1);
    await assert.rejects(() => service.update({
        expectedRevision: 0,
        bindings: [],
    }), /revision conflict/);
    await service.close();
    assert.deepEqual(JSON.parse(await readFile(filePath, "utf8")), bindings);
    const reopened = await KeybindingsResourceMainService.create({ filePath });
    assert.deepEqual(reopened.read(), {
        revision: 0,
        bindings,
    });
    await reopened.close();
});
test("legacy configuration keybindings migrate into the standalone resource", async (context) => {
    const directory = await mkdtemp(join(tmpdir(), "zeta-keybinding-migration-"));
    context.after(async () => {
        await rm(directory, { recursive: true, force: true });
    });
    const configuration = await ConfigurationMainService.create({
        filePath: join(directory, "configuration.json"),
    });
    const keybindings = await KeybindingsResourceMainService.create({
        filePath: join(directory, "keybindings.json"),
    });
    await configuration.update({
        expectedRevision: 0,
        document: {
            version: 1,
            values: {
                "editor.fontSize": 14,
                "keyboard.keybindings": [{
                        key: "primary+n",
                        command: "zeta.new",
                    }],
            },
        },
    });
    assert.equal(await migrateLegacyKeybindings(configuration, keybindings), true);
    assert.deepEqual(keybindings.read().bindings, [{
            key: "primary+n",
            command: "zeta.new",
        }]);
    assert.deepEqual(configuration.read().document.values, {
        "editor.fontSize": 14,
    });
    await Promise.all([
        configuration.close(),
        keybindings.close(),
    ]);
});
class TestKeybindingsResourceApi {
    #listeners = new Set();
    #snapshot;
    constructor(snapshot) {
        this.#snapshot = snapshot;
    }
    read() {
        return Promise.resolve(this.#snapshot);
    }
    update(request) {
        if (request.expectedRevision !== this.#snapshot.revision) {
            return Promise.reject(new Error("revision conflict"));
        }
        this.#snapshot = {
            revision: this.#snapshot.revision + 1,
            bindings: request.bindings,
        };
        return Promise.resolve(this.#snapshot);
    }
    onDidChange(listener) {
        this.#listeners.add(listener);
        return {
            dispose: () => this.#listeners.delete(listener),
        };
    }
    emit(snapshot) {
        this.#snapshot = snapshot;
        for (const listener of this.#listeners)
            listener(snapshot);
    }
}

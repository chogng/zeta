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
import { ConfigurationRegistry, } from "../src/platform/configuration/common/configurationRegistry.js";
import { validateConfigurationDocument, validateConfigurationSnapshot, validateConfigurationUpdateRequest, } from "../src/platform/configuration/common/configuration.js";
import { ConfigurationMainService, } from "../src/platform/configuration/electron-main/configurationMainService.js";
import { WorkbenchConfigurationService, } from "../src/workbench/services/configuration/browser/configurationService.js";
test("configuration validators bound the complete wire document", () => {
    assert.deepEqual(validateConfigurationSnapshot({
        revision: 2,
        document: {
            version: 1,
            values: {
                "editor.fontSize": 14,
            },
        },
    }), {
        revision: 2,
        document: {
            version: 1,
            values: {
                "editor.fontSize": 14,
            },
        },
    });
    assert.throws(() => validateConfigurationDocument({
        version: 1,
        values: { "__proto__.unsafe": true },
    }), /invalid configuration key/);
    assert.throws(() => validateConfigurationUpdateRequest({
        expectedRevision: -1,
        document: { version: 1, values: {} },
    }), /non-negative safe integer/);
});
test("workbench configuration resolves typed defaults and live snapshots", async () => {
    const env_1 = { stack: [], error: void 0, hasError: false };
    try {
        const registry = new ConfigurationRegistry();
        const fontSize = registry.registerConfiguration({
            key: "editor.fontSize",
            defaultValue: 12,
            parse(value) {
                if (!Number.isInteger(value) || value < 8) {
                    throw new TypeError("font size must be an integer of at least 8");
                }
                return value;
            },
        });
        const api = new TestConfigurationApi({
            revision: 0,
            document: {
                version: 1,
                values: { "editor.fontSize": 16 },
            },
        });
        const service = __addDisposableResource(env_1, new WorkbenchConfigurationService({
            api,
            registry,
        }), false);
        let changes = 0;
        const listener = __addDisposableResource(env_1, service.onDidChangeConfiguration((event) => {
            if (event.affectsConfiguration(fontSize))
                changes += 1;
        }), false);
        assert.equal(service.getValue(fontSize), 12);
        await service.reload();
        assert.equal(service.getValue(fontSize), 16);
        await service.updateValue(fontSize, 18);
        assert.equal(service.getValue(fontSize), 18);
        api.emit({
            revision: 2,
            document: {
                version: 1,
                values: { "editor.fontSize": 20 },
            },
        });
        assert.equal(service.getValue(fontSize), 20);
        await service.resetValue(fontSize);
        assert.equal(service.getValue(fontSize), 12);
        assert.equal(changes, 4);
        await assert.rejects(() => service.updateValue(fontSize, 4), /font size/);
    }
    catch (e_1) {
        env_1.error = e_1;
        env_1.hasError = true;
    }
    finally {
        __disposeResources(env_1);
    }
});
test("main configuration service persists atomic revisions", async (context) => {
    const directory = await mkdtemp(join(tmpdir(), "zeta-configuration-"));
    context.after(async () => {
        await rm(directory, { recursive: true, force: true });
    });
    const filePath = join(directory, "configuration.json");
    const service = await ConfigurationMainService.create({ filePath });
    const updated = await service.update({
        expectedRevision: 0,
        document: {
            version: 1,
            values: {
                "editor.fontSize": 14,
            },
        },
    });
    assert.equal(updated.revision, 1);
    await assert.rejects(() => service.update({
        expectedRevision: 0,
        document: { version: 1, values: {} },
    }), /revision conflict/);
    await service.close();
    assert.deepEqual(JSON.parse(await readFile(filePath, "utf8")), updated.document);
    const reopened = await ConfigurationMainService.create({ filePath });
    assert.deepEqual(reopened.read(), {
        revision: 0,
        document: updated.document,
    });
    await reopened.close();
});
class TestConfigurationApi {
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
            document: request.document,
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

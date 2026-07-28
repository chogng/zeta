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
import { strict as assert } from "node:assert";
import test from "node:test";
import { ColorId, colorCssVariable, colorIdentifiers, darkColorTheme, lightColorTheme, } from "../../common/colorTheme.js";
import { ThemeService } from "../../common/themeService.js";
test("ThemeService exposes its initial theme and emits actual changes", () => {
    const env_1 = { stack: [], error: void 0, hasError: false };
    try {
        const service = __addDisposableResource(env_1, new ThemeService(darkColorTheme), false);
        const changes = [];
        const listener = __addDisposableResource(env_1, service.onDidColorThemeChange((theme) => {
            changes.push(theme.id);
        }), false);
        service.setColorTheme(darkColorTheme);
        service.setColorTheme(lightColorTheme);
        assert.equal(service.getColorTheme(), lightColorTheme);
        assert.deepEqual(changes, ["zeta-light"]);
    }
    catch (e_1) {
        env_1.error = e_1;
        env_1.hasError = true;
    }
    finally {
        __disposeResources(env_1);
    }
});
test("built-in themes define every registered color", () => {
    for (const id of colorIdentifiers) {
        assert.equal(typeof darkColorTheme.colors[id], "string");
        assert.equal(typeof lightColorTheme.colors[id], "string");
    }
});
test("color identifiers map to stable CSS custom properties", () => {
    assert.equal(colorCssVariable(ColorId.primaryButtonHoverBackground), "--zeta-primary-button-hover-background");
    assert.equal(colorCssVariable(ColorId.titleBarForeground), "--zeta-title-bar-foreground");
});

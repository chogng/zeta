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
import { bindColorTheme } from "../../browser/themeStyles.js";
import { ColorId, colorCssVariable, darkColorTheme, lightColorTheme, } from "../../common/colorTheme.js";
import { ThemeService } from "../../common/themeService.js";
test("color theme binding applies changes and restores prior root styles", () => {
    const env_1 = { stack: [], error: void 0, hasError: false };
    try {
        const service = __addDisposableResource(env_1, new ThemeService(darkColorTheme), false);
        const target = new FakeThemeTarget();
        const foreground = colorCssVariable(ColorId.foreground);
        const background = colorCssVariable(ColorId.workbenchBackground);
        target.style.setProperty(foreground, "hotpink", "important");
        target.style.setProperty("color-scheme", "only light");
        target.setAttribute("data-color-theme", "host-theme");
        const binding = bindColorTheme(service, target);
        assert.equal(target.style.getPropertyValue(foreground), "#cccccc");
        assert.equal(target.style.getPropertyValue(background), "#1e1e1e");
        assert.equal(target.style.getPropertyValue("color-scheme"), "dark");
        assert.equal(target.getAttribute("data-color-theme"), "zeta-dark");
        service.setColorTheme(lightColorTheme);
        assert.equal(target.style.getPropertyValue(background), "#ffffff");
        assert.equal(target.style.getPropertyValue("color-scheme"), "light");
        assert.equal(target.getAttribute("data-color-theme"), "zeta-light");
        binding.dispose();
        assert.equal(target.style.getPropertyValue(foreground), "hotpink");
        assert.equal(target.style.getPropertyPriority(foreground), "important");
        assert.equal(target.style.getPropertyValue(background), "");
        assert.equal(target.style.getPropertyValue("color-scheme"), "only light");
        assert.equal(target.getAttribute("data-color-theme"), "host-theme");
        assert.equal(target.getAttribute("data-color-scheme"), null);
        service.setColorTheme(darkColorTheme);
        assert.equal(target.style.getPropertyValue(background), "");
    }
    catch (e_1) {
        env_1.error = e_1;
        env_1.hasError = true;
    }
    finally {
        __disposeResources(env_1);
    }
});
class FakeStyle {
    #properties = new Map();
    getPropertyValue(name) {
        return this.#properties.get(name)?.value ?? "";
    }
    getPropertyPriority(name) {
        return this.#properties.get(name)?.priority ?? "";
    }
    setProperty(name, value, priority = "") {
        this.#properties.set(name, { value, priority });
    }
    removeProperty(name) {
        const previous = this.getPropertyValue(name);
        this.#properties.delete(name);
        return previous;
    }
}
class FakeThemeTarget {
    style = new FakeStyle();
    #attributes = new Map();
    getAttribute(name) {
        return this.#attributes.get(name) ?? null;
    }
    setAttribute(name, value) {
        this.#attributes.set(name, value);
    }
    removeAttribute(name) {
        this.#attributes.delete(name);
    }
}

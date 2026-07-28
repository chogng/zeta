import { Emitter } from "../../../../base/common/event.js";
import { resolveKeybinding, } from "../../../../base/common/keybindings.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { operatingSystem, } from "../../../../base/common/platform.js";
/**
 * Uses the browser Keyboard Map capability when available and falls back to
 * stable `KeyboardEvent.code` labels when the capability is unavailable.
 */
export class BrowserKeyboardLayoutService extends DisposableOwner {
    #onDidChangeKeyboardLayout = this.own(new Emitter());
    #navigator;
    #operatingSystem;
    #physicalKeyLabels = new Map();
    #mapper;
    #refreshing;
    #disposed = false;
    onDidChangeKeyboardLayout = this.#onDidChangeKeyboardLayout.event;
    constructor(options) {
        super();
        this.#navigator = options.navigator;
        this.#operatingSystem = options.operatingSystem ?? operatingSystem;
        this.#mapper = this.#createMapper();
        this.defer(() => {
            this.#disposed = true;
            this.#physicalKeyLabels.clear();
        });
        void this.refreshKeyboardLayout();
    }
    getCurrentKeyboardLayout() {
        const browserMapping = this.#physicalKeyLabels.size > 0;
        const language = this.#navigator.language || "unknown";
        return {
            id: browserMapping ? `browser.${language}` : "fallback",
            label: browserMapping ? language : "Fallback keyboard layout",
            source: browserMapping ? "browser" : "fallback",
        };
    }
    getKeyboardMapper() {
        return this.#mapper;
    }
    validateCurrentKeyboardMapping(event) {
        if (!this.#navigator.keyboard ||
            event.ctrlKey ||
            event.shiftKey ||
            event.altKey ||
            event.metaKey ||
            event.key.length !== 1) {
            return;
        }
        const expected = this.#physicalKeyLabels.get(event.code);
        if (expected === undefined ||
            expected.toLocaleLowerCase("en-US") !==
                event.key.toLocaleLowerCase("en-US")) {
            void this.refreshKeyboardLayout();
        }
    }
    refreshKeyboardLayout() {
        if (!this.#navigator.keyboard || this.#disposed) {
            return Promise.resolve();
        }
        if (this.#refreshing)
            return this.#refreshing;
        const refreshing = this.#readKeyboardLayout()
            .finally(() => {
            if (this.#refreshing === refreshing)
                this.#refreshing = undefined;
        });
        this.#refreshing = refreshing;
        return refreshing;
    }
    async #readKeyboardLayout() {
        try {
            const layoutMap = await this.#navigator.keyboard.getLayoutMap();
            if (this.#disposed)
                return;
            const nextLabels = new Map();
            for (const [code, label] of layoutMap) {
                if (code && label)
                    nextLabels.set(code, label);
            }
            if (mapsEqual(this.#physicalKeyLabels, nextLabels))
                return;
            this.#physicalKeyLabels = nextLabels;
            this.#mapper = this.#createMapper();
            this.#onDidChangeKeyboardLayout.fire();
        }
        catch {
            // Browsers may expose the API but deny it without focus or permission.
            // The fallback mapper remains valid and can be refreshed on a later key.
        }
    }
    #createMapper() {
        const labels = new Map(this.#physicalKeyLabels);
        const targetOperatingSystem = this.#operatingSystem;
        return {
            resolveKeybinding(keybinding) {
                return resolveKeybinding(keybinding, targetOperatingSystem, labels);
            },
        };
    }
}
function mapsEqual(first, second) {
    if (first.size !== second.size)
        return false;
    for (const [key, value] of first) {
        if (second.get(key) !== value)
            return false;
    }
    return true;
}

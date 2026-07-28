import { Emitter } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { createServiceIdentifier, } from "../../instantiation/common/instantiation.js";
export const IThemeService = createServiceIdentifier("themeService");
/** Owns the active color theme and notifies consumers after it changes. */
export class ThemeService extends DisposableOwner {
    #onDidColorThemeChange = this.own(new Emitter());
    #colorTheme;
    onDidColorThemeChange = this.#onDidColorThemeChange.event;
    constructor(initialColorTheme) {
        super();
        this.#colorTheme = initialColorTheme;
    }
    getColorTheme() {
        return this.#colorTheme;
    }
    setColorTheme(theme) {
        if (theme === this.#colorTheme)
            return;
        this.#colorTheme = theme;
        this.#onDidColorThemeChange.fire(theme);
    }
}

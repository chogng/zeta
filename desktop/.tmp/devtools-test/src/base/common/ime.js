import { Emitter } from "./event.js";
/**
 * Coordinates whether text-input surfaces should currently accept IME
 * composition.
 *
 * Keybinding dispatchers temporarily disable IME while waiting for another
 * chord. Text-input implementations observe the state and suppress composition
 * without coupling themselves to the keybinding service.
 */
export class InputMethodEditorState {
    #onDidChange = new Emitter();
    #enabled = true;
    onDidChange = this.#onDidChange.event;
    get enabled() {
        return this.#enabled;
    }
    enable() {
        this.#setEnabled(true);
    }
    disable() {
        this.#setEnabled(false);
    }
    #setEnabled(enabled) {
        if (this.#enabled === enabled)
            return;
        this.#enabled = enabled;
        this.#onDidChange.fire(enabled);
    }
}
/** Shared IME coordination state for the current JavaScript realm. */
export const IME = new InputMethodEditorState();

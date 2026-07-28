import { Emitter, type Event } from "./event.js";

/**
 * Coordinates whether text-input surfaces should currently accept IME
 * composition.
 *
 * Keybinding dispatchers temporarily disable IME while waiting for another
 * chord. Text-input implementations observe the state and suppress composition
 * without coupling themselves to the keybinding service.
 */
export class InputMethodEditorState {
  readonly #onDidChange = new Emitter<boolean>();
  #enabled = true;

  readonly onDidChange: Event<boolean> = this.#onDidChange.event;

  get enabled(): boolean {
    return this.#enabled;
  }

  enable(): void {
    this.#setEnabled(true);
  }

  disable(): void {
    this.#setEnabled(false);
  }

  #setEnabled(enabled: boolean): void {
    if (this.#enabled === enabled) return;
    this.#enabled = enabled;
    this.#onDidChange.fire(enabled);
  }
}

/** Shared IME coordination state for the current JavaScript realm. */
export const IME = new InputMethodEditorState();

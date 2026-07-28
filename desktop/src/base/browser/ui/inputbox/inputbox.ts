import { addDisposableListener } from "../../dom.js";
import { IME } from "../../../common/ime.js";
import { DisposableOwner } from "../../../common/lifecycle.js";

export interface InputBoxOptions {
  readonly placeholder?: string;
  readonly type?: "text" | "password" | "search";
  readonly ownerDocument?: Document;
  readonly readOnly?: boolean;
  readonly onInput?: (value: string) => void;
}

/** A text input with a separately rendered validation message. */
export class InputBox extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly input: HTMLInputElement;
  readonly #message: HTMLDivElement;
  #readOnly: boolean;

  constructor(options: InputBoxOptions = {}) {
    super();
    const ownerDocument = options.ownerDocument ?? document;
    const element = ownerDocument.createElement("div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-input-box";
    this.input = ownerDocument.createElement("input");
    this.input.type = options.type ?? "text";
    this.input.placeholder = options.placeholder ?? "";
    this.#readOnly = options.readOnly ?? false;
    this.#message = ownerDocument.createElement("div");
    this.#message.hidden = true;
    element.append(this.input, this.#message);
    this.#syncReadOnly();
    this.own(IME.onDidChange(() => this.#syncReadOnly()));
    if (options.onInput) {
      this.own(addDisposableListener(this.input, "input", () =>
        options.onInput?.(this.input.value),
      ));
    }
  }

  get value(): string { return this.input.value; }
  set value(value: string) { this.input.value = value; }
  get readOnly(): boolean { return this.#readOnly; }
  set readOnly(value: boolean) {
    this.#readOnly = value;
    this.#syncReadOnly();
  }
  showValidation(message: string): void { this.#message.textContent = message; this.#message.hidden = !message; }

  #syncReadOnly(): void {
    this.input.readOnly = this.#readOnly || !IME.enabled;
  }
}

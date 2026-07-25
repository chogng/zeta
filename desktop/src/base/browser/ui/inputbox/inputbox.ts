import { Component } from "../common/component.js";

export interface InputBoxOptions { placeholder?: string; type?: "text" | "password" | "search"; onInput?: (value: string) => void; }

/** A text input with a separately rendered validation message. */
export class InputBox extends Component<HTMLDivElement> {
  readonly input: HTMLInputElement;
  #message: HTMLDivElement;

  constructor(options: InputBoxOptions = {}) {
    const element = document.createElement("div");
    element.className = "zeta-input-box";
    super(element);
    this.input = document.createElement("input");
    this.input.type = options.type ?? "text";
    this.input.placeholder = options.placeholder ?? "";
    this.#message = document.createElement("div");
    this.#message.hidden = true;
    element.append(this.input, this.#message);
    if (options.onInput) this.listen(this.input, "input", () => options.onInput?.(this.input.value));
  }

  get value(): string { return this.input.value; }
  set value(value: string) { this.input.value = value; }
  showValidation(message: string): void { this.#message.textContent = message; this.#message.hidden = !message; }
}

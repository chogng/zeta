import { addDisposableListener } from "../../dom.js";
import {
  type AriaAutoComplete,
  type AriaRole,
  getAriaAttribute,
  setAriaAttribute,
  setRole,
} from "../aria/aria.js";
import { Emitter, type Event } from "../../../common/event.js";
import { IME } from "../../../common/ime.js";
import { DisposableOwner } from "../../../common/lifecycle.js";

export interface InputBoxOptions {
  readonly placeholder?: string;
  readonly type?: "text" | "password" | "search";
  readonly ownerDocument?: Document;
  readonly readOnly?: boolean;
  readonly enabled?: boolean;
  readonly ariaLabel?: string;
  readonly role?: AriaRole;
  readonly ariaAutoComplete?: AriaAutoComplete;
  readonly ariaControls?: string;
  readonly ariaExpanded?: boolean;
}

export interface InputSelection {
  readonly start: number;
  readonly end: number;
}

/** A text input foundation with events, focus control, and validation state. */
export class InputBox extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly inputElement: HTMLInputElement;
  readonly #message: HTMLDivElement;
  readonly #onDidChange = this.own(new Emitter<string>());
  readonly #onDidFocus = this.own(new Emitter<void>());
  readonly #onDidBlur = this.own(new Emitter<void>());
  readonly #onKeyDown = this.own(new Emitter<KeyboardEvent>());
  #readOnly: boolean;

  readonly onDidChange: Event<string> = this.#onDidChange.event;
  readonly onDidFocus: Event<void> = this.#onDidFocus.event;
  readonly onDidBlur: Event<void> = this.#onDidBlur.event;
  readonly onKeyDown: Event<KeyboardEvent> = this.#onKeyDown.event;

  constructor(options: InputBoxOptions = {}) {
    super();
    const ownerDocument = options.ownerDocument ?? document;
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-input-box";
    this.defer(() => this.element.remove());

    this.inputElement = ownerDocument.createElement("input");
    this.inputElement.type = options.type ?? "text";
    this.inputElement.placeholder = options.placeholder ?? "";
    this.inputElement.disabled = options.enabled === false;
    this.element.classList.toggle("is-disabled", this.inputElement.disabled);
    this.inputElement.autocomplete = "off";
    this.inputElement.autocapitalize = "off";
    this.inputElement.spellcheck = false;
    if (options.ariaLabel) {
      setAriaAttribute(this.inputElement, "label", options.ariaLabel);
    }
    setRole(this.inputElement, options.role);
    if (options.ariaAutoComplete) {
      setAriaAttribute(
        this.inputElement,
        "autocomplete",
        options.ariaAutoComplete,
      );
    }
    if (options.ariaControls) {
      setAriaAttribute(
        this.inputElement,
        "controls",
        options.ariaControls,
      );
    }
    if (options.ariaExpanded !== undefined) {
      setAriaAttribute(
        this.inputElement,
        "expanded",
        options.ariaExpanded,
      );
    }

    this.#readOnly = options.readOnly ?? false;
    this.#message = ownerDocument.createElement("div");
    this.#message.id = `zeta-input-message-${inputBoxSequence++}`;
    this.#message.className = "zeta-input-box-message";
    setRole(this.#message, "alert");
    this.#message.hidden = true;
    this.element.append(this.inputElement, this.#message);
    this.#syncReadOnly();
    this.own(IME.onDidChange(() => this.#syncReadOnly()));
    this.own(addDisposableListener(
      this.inputElement,
      "input",
      () => this.#onDidChange.fire(this.value),
    ));
    this.own(addDisposableListener(
      this.inputElement,
      "focus",
      () => {
        this.element.classList.add("is-focused");
        this.#onDidFocus.fire();
      },
    ));
    this.own(addDisposableListener(
      this.inputElement,
      "blur",
      () => {
        this.element.classList.remove("is-focused");
        this.#onDidBlur.fire();
      },
    ));
    this.own(addDisposableListener(
      this.inputElement,
      "keydown",
      (event: KeyboardEvent) => this.#onKeyDown.fire(event),
    ));
  }

  get value(): string {
    return this.inputElement.value;
  }

  set value(value: string) {
    if (this.inputElement.value === value) return;
    this.inputElement.value = value;
    this.#onDidChange.fire(value);
  }

  get placeholder(): string {
    return this.inputElement.placeholder;
  }

  set placeholder(value: string) {
    this.inputElement.placeholder = value;
  }

  get readOnly(): boolean {
    return this.#readOnly;
  }

  set readOnly(value: boolean) {
    this.#readOnly = value;
    this.#syncReadOnly();
  }

  get enabled(): boolean {
    return !this.inputElement.disabled;
  }

  set enabled(value: boolean) {
    this.inputElement.disabled = !value;
    this.element.classList.toggle("is-disabled", !value);
  }

  get ariaActiveDescendant(): string | undefined {
    return getAriaAttribute(this.inputElement, "activedescendant");
  }

  set ariaActiveDescendant(value: string | undefined) {
    if (value) {
      setAriaAttribute(this.inputElement, "activedescendant", value);
    } else {
      setAriaAttribute(this.inputElement, "activedescendant", undefined);
    }
  }

  focus(): void {
    this.inputElement.focus();
  }

  blur(): void {
    this.inputElement.blur();
  }

  hasFocus(): boolean {
    return this.inputElement.ownerDocument.activeElement === this.inputElement;
  }

  select(selection?: InputSelection): void {
    if (selection) {
      this.inputElement.setSelectionRange(selection.start, selection.end);
    } else {
      this.inputElement.select();
    }
  }

  showValidation(message: string): void {
    this.#message.textContent = message;
    this.#message.hidden = !message;
    this.element.classList.toggle("has-validation", Boolean(message));
    if (message) {
      setAriaAttribute(this.inputElement, "invalid", true);
      setAriaAttribute(
        this.inputElement,
        "describedby",
        this.#message.id,
      );
    } else {
      setAriaAttribute(this.inputElement, "invalid", undefined);
      setAriaAttribute(this.inputElement, "describedby", undefined);
    }
  }

  #syncReadOnly(): void {
    this.inputElement.readOnly = this.#readOnly || !IME.enabled;
  }
}

let inputBoxSequence = 1;

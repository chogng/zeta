import { Emitter, type Event } from "../../../common/event.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { addDisposableListener, h, text as createText } from "../../dom.js";

export interface ToggleOptions {
  readonly ownerDocument?: Document;
  readonly checked?: boolean;
  readonly disabled?: boolean;
  readonly ariaLabel?: string;
  readonly content?: Node;
  readonly label?: string;
  readonly contentPlacement?: "after-control" | "before-control";
  readonly onChange?: (checked: boolean) => void;
}

type ToggleOptionsOrLabel = ToggleOptions | string;

/** A reusable two-state boolean control shared by checkbox and switch presentations. */
export class Toggle extends DisposableOwner {
  readonly element: HTMLLabelElement;
  readonly input: HTMLInputElement;
  protected readonly contentElement: HTMLSpanElement | undefined;
  private readonly _onDidChange = this.own(new Emitter<boolean>());
  readonly onDidChange: Event<boolean> = this._onDidChange.event;

  constructor(options?: ToggleOptions);
  constructor(label: string, checked?: boolean, onChange?: (checked: boolean) => void);
  constructor(optionsOrLabel: ToggleOptionsOrLabel, checked?: boolean, onChange?: (checked: boolean) => void);
  constructor(optionsOrLabel: ToggleOptionsOrLabel = {}, checked = false, onChange?: (checked: boolean) => void) {
    super();
    const options: ToggleOptions = typeof optionsOrLabel === "string"
      ? { label: optionsOrLabel, checked, onChange }
      : optionsOrLabel;
    const ownerDocument = options.ownerDocument ?? document;
    const element = h(ownerDocument, "label");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-toggle";

    const input = h(ownerDocument, "input");
    this.input = input;
    input.type = "checkbox";
    input.checked = options.checked ?? false;
    input.disabled = options.disabled === true;
    if (options.ariaLabel) input.setAttribute("aria-label", options.ariaLabel);
    element.append(input);
    const content = options.content ?? (options.label ? createText(ownerDocument, options.label) : undefined);
    if (content) {
      if (options.contentPlacement === "before-control") {
        const contentElement = h(ownerDocument, "span");
        this.contentElement = contentElement;
        contentElement.className = "zeta-toggle-content";
        contentElement.append(content);
        element.append(contentElement);
      } else {
        this.contentElement = undefined;
        element.append(content);
      }
    } else {
      this.contentElement = undefined;
    }
    if (options.contentPlacement === "before-control") element.classList.add("zeta-toggle-content-before-control");

    this.own(addDisposableListener(input, "change", () => {
      this.syncState();
      this._onDidChange.fire(input.checked);
      options.onChange?.(input.checked);
    }));
    this.syncState();
  }

  get checked(): boolean { return this.input.checked; }

  set checked(value: boolean) {
    this.input.checked = value;
    this.syncState();
  }

  get enabled(): boolean { return !this.input.disabled; }

  set enabled(value: boolean) {
    this.input.disabled = !value;
    this.syncState();
  }

  focus(): void { this.input.focus(); }

  blur(): void { this.input.blur(); }

  setAriaLabel(label: string): void {
    this.input.setAttribute("aria-label", label);
  }

  protected syncState(): void {
    this.element.classList.toggle("checked", this.input.checked);
    this.element.classList.toggle("disabled", this.input.disabled);
    if (this.input.getAttribute("role") === "switch") {
      this.input.setAttribute("aria-checked", String(this.input.checked));
    }
  }
}

/** A native checkbox presentation backed by the shared Toggle state model. */
export class Checkbox extends Toggle {
  constructor(options?: ToggleOptions);
  constructor(label: string, checked?: boolean, onChange?: (checked: boolean) => void);
  constructor(optionsOrLabel: ToggleOptionsOrLabel = {}, checked = false, onChange?: (checked: boolean) => void) {
    super(optionsOrLabel, checked, onChange);
    this.element.classList.add("zeta-checkbox");
  }
}

/** A compact on/off switch presentation backed by the shared Toggle state model. */
export class Switch extends Toggle {
  readonly track: HTMLSpanElement;

  constructor(options?: ToggleOptions);
  constructor(label: string, checked?: boolean, onChange?: (checked: boolean) => void);
  constructor(optionsOrLabel: ToggleOptionsOrLabel = {}, checked = false, onChange?: (checked: boolean) => void) {
    super(optionsOrLabel, checked, onChange);
    this.element.classList.add("zeta-switch");
    this.input.setAttribute("role", "switch");
    const track = h(this.element.ownerDocument, "span");
    this.track = track;
    track.className = "zeta-switch-track";
    track.setAttribute("aria-hidden", "true");
    const contentPlacement = typeof optionsOrLabel === "string" ? undefined : optionsOrLabel.contentPlacement;
    if (contentPlacement === "before-control" && this.contentElement) this.element.append(track);
    else this.element.insertBefore(track, this.input.nextSibling);
    this.syncState();
  }
}

import { addDisposableListener } from "../../dom.js";
import type { Icon } from "../../../common/icon.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { setAriaAttribute } from "../aria/aria.js";
import { appendIcon } from "../icon/icon.js";

export interface ButtonOptions {
  label: string;
  ownerDocument?: Document;
  icon?: Icon;
  title?: string;
  enabled?: boolean;
  checked?: boolean;
  onClick?: () => void;
}

/** A semantic button with an explicit enabled state. */
export class Button extends DisposableOwner {
  readonly element: HTMLButtonElement;

  constructor(options: ButtonOptions) {
    super();
    const ownerDocument = options.ownerDocument ?? document;
    const element = ownerDocument.createElement("button");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-button";
    element.type = "button";
    if (options.icon) appendIcon(options.icon, element);
    const label = ownerDocument.createElement("span");
    label.className = "zeta-button-label";
    label.textContent = options.label;
    element.append(label);
    element.title = options.title ?? options.label;
    element.disabled = options.enabled === false;
    if (options.checked !== undefined) {
      this.checked = options.checked;
    }
    if (options.onClick) {
      this.own(addDisposableListener(element, "click", options.onClick));
    }
  }

  set enabled(value: boolean) { this.element.disabled = !value; }
  get enabled(): boolean { return !this.element.disabled; }

  set checked(value: boolean) {
    this.element.classList.toggle("checked", value);
    setAriaAttribute(this.element, "pressed", value);
  }

  get checked(): boolean { return this.element.classList.contains("checked"); }
}

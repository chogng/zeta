import { Component } from "../common/component.js";

export interface ButtonOptions { label: string; title?: string; enabled?: boolean; onClick?: () => void; }

/** A semantic button with an explicit enabled state. */
export class Button extends Component<HTMLButtonElement> {
  constructor(options: ButtonOptions) {
    const element = document.createElement("button");
    element.className = "zeta-button";
    element.type = "button";
    element.textContent = options.label;
    element.title = options.title ?? options.label;
    element.disabled = options.enabled === false;
    super(element);
    if (options.onClick) this.listen(element, "click", () => options.onClick?.());
  }

  set enabled(value: boolean) { this.element.disabled = !value; }
  get enabled(): boolean { return !this.element.disabled; }
}

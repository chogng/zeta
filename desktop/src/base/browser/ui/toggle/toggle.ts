import { Component } from "../common/component.js";

/** A labelled checkbox used for boolean preferences and toolbar states. */
export class Toggle extends Component<HTMLLabelElement> {
  readonly input: HTMLInputElement;

  constructor(label: string, checked = false, onChange?: (checked: boolean) => void) {
    const element = document.createElement("label");
    element.className = "zeta-toggle";
    super(element);
    this.input = document.createElement("input");
    this.input.type = "checkbox";
    this.input.checked = checked;
    element.append(this.input, document.createTextNode(label));
    if (onChange) this.listen(this.input, "change", () => onChange(this.input.checked));
  }

  get checked(): boolean { return this.input.checked; }
  set checked(value: boolean) { this.input.checked = value; }
}

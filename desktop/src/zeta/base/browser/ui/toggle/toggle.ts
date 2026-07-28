import { addDisposableListener } from "../../dom.js";
import { DisposableOwner } from "../../../common/lifecycle.js";

/** A labelled checkbox used for boolean preferences and toolbar states. */
export class Toggle extends DisposableOwner {
  readonly element: HTMLLabelElement;
  readonly input: HTMLInputElement;

  constructor(label: string, checked = false, onChange?: (checked: boolean) => void) {
    super();
    const element = document.createElement("label");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-toggle";
    this.input = document.createElement("input");
    this.input.type = "checkbox";
    this.input.checked = checked;
    element.append(this.input, document.createTextNode(label));
    if (onChange) {
      this.own(addDisposableListener(this.input, "change", () =>
        onChange(this.input.checked),
      ));
    }
  }

  get checked(): boolean { return this.input.checked; }
  set checked(value: boolean) { this.input.checked = value; }
}

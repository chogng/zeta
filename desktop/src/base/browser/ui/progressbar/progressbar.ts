import { Component } from "../common/component.js";

/** A determinate or indeterminate progress indicator. */
export class ProgressBar extends Component<HTMLProgressElement> {
  constructor() {
    const element = document.createElement("progress");
    element.className = "zeta-progress-bar";
    element.max = 1;
    super(element);
  }

  set value(value: number | undefined) {
    if (value === undefined) this.element.removeAttribute("value");
    else this.element.value = Math.max(0, Math.min(1, value));
  }
}

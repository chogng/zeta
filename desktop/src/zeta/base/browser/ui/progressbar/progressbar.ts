import { DisposableOwner } from "../../../common/lifecycle.js";
import { h } from "../../dom.js";

/** A determinate or indeterminate progress indicator. */
export class ProgressBar extends DisposableOwner {
  readonly element: HTMLProgressElement;

  constructor(ownerDocument: Document) {
    super();
    const element = h(ownerDocument, "progress");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-progress-bar";
    element.max = 1;
  }

  set value(value: number | undefined) {
    if (value === undefined) this.element.removeAttribute("value");
    else this.element.value = Math.max(0, Math.min(1, value));
  }
}

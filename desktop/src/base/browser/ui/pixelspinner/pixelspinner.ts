import {
  DisposableOwner,
} from "../../../common/lifecycle.js";
import { disposableWindowInterval } from "../../scheduler.js";
import { getWindow } from "../../window.js";

/** A compact four-pixel activity indicator with no image asset dependency. */
export class PixelSpinner extends DisposableOwner {
  readonly element: HTMLSpanElement;
  #step = 0;

  constructor() {
    super();
    const element = document.createElement("span");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-pixel-spinner";
    element.setAttribute("role", "status");
    element.setAttribute("aria-label", "Loading");
    element.append(...Array.from({ length: 4 }, () => document.createElement("i")));
    this.own(disposableWindowInterval(
      getWindow(element),
      () => this.render(),
      120,
    ));
    this.render();
  }

  private render(): void {
    [...this.element.children].forEach((pixel, index) => pixel.classList.toggle("active", index === this.#step));
    this.#step = (this.#step + 1) % 4;
  }
}

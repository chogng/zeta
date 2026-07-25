import { Component } from "../common/component.js";

/** A compact four-pixel activity indicator with no image asset dependency. */
export class PixelSpinner extends Component<HTMLSpanElement> {
  #timer: ReturnType<typeof setInterval>;
  #step = 0;

  constructor() {
    const element = document.createElement("span");
    element.className = "zeta-pixel-spinner";
    element.setAttribute("role", "status");
    element.setAttribute("aria-label", "Loading");
    super(element);
    element.append(...Array.from({ length: 4 }, () => document.createElement("i")));
    this.#timer = setInterval(() => this.render(), 120);
    this.render();
  }

  override dispose(): void { clearInterval(this.#timer); super.dispose(); }

  private render(): void {
    [...this.element.children].forEach((pixel, index) => pixel.classList.toggle("active", index === this.#step));
    this.#step = (this.#step + 1) % 4;
  }
}

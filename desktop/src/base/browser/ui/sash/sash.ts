import { Component } from "../common/component.js";

export type SashOrientation = "vertical" | "horizontal";

/** A draggable separator that reports pointer movement to its owning layout. */
export class Sash extends Component<HTMLDivElement> {
  #listeners = new Set<(delta: number) => void>();

  constructor(readonly orientation: SashOrientation) {
    const element = document.createElement("div");
    element.className = `zeta-sash zeta-sash-${orientation}`;
    element.tabIndex = 0;
    super(element);
    this.listen(element, "pointerdown", (event: PointerEvent) => this.beginDrag(event));
  }

  onDidDrag(listener: (delta: number) => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  private beginDrag(event: PointerEvent): void {
    event.preventDefault();
    let previous = this.orientation === "vertical" ? event.clientX : event.clientY;
    const move = (next: PointerEvent) => {
      const position = this.orientation === "vertical" ? next.clientX : next.clientY;
      const delta = position - previous;
      previous = position;
      for (const listener of this.#listeners) listener(delta);
    };
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", stop);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop, { once: true });
  }
}

import { addDisposableListener } from "../../dom.js";
import { getWindow } from "../../window.js";
import {
  type IDisposable,
  DisposableOwner,
  ResettableDisposableGroup,
  toDisposable,
} from "../../../common/lifecycle.js";

export type SashOrientation = "vertical" | "horizontal";

/** A draggable separator that reports pointer movement to its owning layout. */
export class Sash extends DisposableOwner {
  readonly element: HTMLDivElement;
  #listeners = new Set<(delta: number) => void>();
  readonly #dragListeners: ResettableDisposableGroup;

  constructor(
    readonly orientation: SashOrientation,
    ownerDocument: Document = document,
  ) {
    super();
    const element = ownerDocument.createElement("div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = `zeta-sash zeta-sash-${orientation}`;
    element.tabIndex = 0;
    this.#dragListeners = this.own(new ResettableDisposableGroup());
    this.own(toDisposable(() => this.#listeners.clear()));
    this.own(addDisposableListener(element, "pointerdown", (event: PointerEvent) =>
      this.beginDrag(event),
    ));
  }

  onDidDrag(listener: (delta: number) => void): IDisposable {
    this.#listeners.add(listener);
    return toDisposable(() => this.#listeners.delete(listener));
  }

  private beginDrag(event: PointerEvent): void {
    event.preventDefault();
    this.#dragListeners.clear();
    let previous = this.orientation === "vertical" ? event.clientX : event.clientY;
    const move = (next: PointerEvent) => {
      const position = this.orientation === "vertical" ? next.clientX : next.clientY;
      const delta = position - previous;
      previous = position;
      for (const listener of this.#listeners) listener(delta);
    };
    const stop = () => {
      this.#dragListeners.clear();
    };
    const targetWindow = getWindow(this.element);
    this.#dragListeners.add(addDisposableListener(
      targetWindow,
      "pointermove",
      move,
    ));
    this.#dragListeners.add(addDisposableListener(
      targetWindow,
      "pointerup",
      stop,
      { once: true },
    ));
  }
}

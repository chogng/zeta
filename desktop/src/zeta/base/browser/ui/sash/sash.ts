import { addDisposableListener } from "../../dom.js";
import { getWindow } from "../../window.js";
import {
  type IDisposable,
  DisposableOwner,
  ResettableDisposableGroup,
  toDisposable,
} from "../../../common/lifecycle.js";

export type SashOrientation = "vertical" | "horizontal";

export interface SashDragEvent {
  /** Signed movement from the position where the current drag started. */
  readonly delta: number;
}

/** A draggable and keyboard-operable separator owned by a layout control. */
export class Sash extends DisposableOwner {
  readonly element: HTMLDivElement;
  #startListeners = new Set<() => void>();
  #changeListeners = new Set<(event: SashDragEvent) => void>();
  #endListeners = new Set<() => void>();
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
    element.setAttribute("role", "separator");
    element.setAttribute("aria-orientation", orientation);
    element.tabIndex = 0;
    this.#dragListeners = this.own(new ResettableDisposableGroup());
    this.own(toDisposable(() => {
      this.#startListeners.clear();
      this.#changeListeners.clear();
      this.#endListeners.clear();
    }));
    this.own(addDisposableListener(element, "pointerdown", (event: PointerEvent) =>
      this.beginDrag(event),
    ));
    this.own(addDisposableListener(element, "keydown", (event: KeyboardEvent) =>
      this.handleKeydown(event),
    ));
  }

  onDidStart(listener: () => void): IDisposable {
    this.#startListeners.add(listener);
    return toDisposable(() => this.#startListeners.delete(listener));
  }

  onDidChange(listener: (event: SashDragEvent) => void): IDisposable {
    this.#changeListeners.add(listener);
    return toDisposable(() => this.#changeListeners.delete(listener));
  }

  onDidEnd(listener: () => void): IDisposable {
    this.#endListeners.add(listener);
    return toDisposable(() => this.#endListeners.delete(listener));
  }

  private beginDrag(event: PointerEvent): void {
    if (event.button !== 0) return;
    event.preventDefault();
    this.#dragListeners.clear();
    const start = this.coordinate(event);
    this.fire(this.#startListeners);
    if (
      typeof event.pointerId === "number" &&
      typeof this.element.setPointerCapture === "function"
    ) {
      this.element.setPointerCapture(event.pointerId);
    }
    const move = (next: PointerEvent) => {
      const dragEvent = { delta: this.coordinate(next) - start };
      for (const listener of this.#changeListeners) listener(dragEvent);
    };
    const stop = () => {
      this.#dragListeners.clear();
      if (
        typeof event.pointerId === "number" &&
        typeof this.element.hasPointerCapture === "function" &&
        this.element.hasPointerCapture(event.pointerId)
      ) {
        this.element.releasePointerCapture(event.pointerId);
      }
      this.fire(this.#endListeners);
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
    this.#dragListeners.add(addDisposableListener(
      targetWindow,
      "pointercancel",
      stop,
      { once: true },
    ));
    this.#dragListeners.add(addDisposableListener(
      targetWindow,
      "blur",
      stop,
      { once: true },
    ));
  }

  private handleKeydown(event: KeyboardEvent): void {
    const delta = this.keyboardDelta(event);
    if (delta === undefined) return;
    event.preventDefault();
    this.fire(this.#startListeners);
    for (const listener of this.#changeListeners) listener({ delta });
    this.fire(this.#endListeners);
  }

  private coordinate(event: Pick<PointerEvent, "clientX" | "clientY">): number {
    return this.orientation === "vertical" ? event.clientX : event.clientY;
  }

  private keyboardDelta(event: KeyboardEvent): number | undefined {
    const step = event.altKey ? 1 : 10;
    if (this.orientation === "vertical") {
      if (event.key === "ArrowLeft") return -step;
      if (event.key === "ArrowRight") return step;
      return undefined;
    }
    if (event.key === "ArrowUp") return -step;
    if (event.key === "ArrowDown") return step;
    return undefined;
  }

  private fire(listeners: ReadonlySet<() => void>): void {
    for (const listener of listeners) listener();
  }
}

import { addDisposableListener } from "../../dom.js";
import { ManagedStyleSheet } from "../../domStylesheets.js";
import { getWindow } from "../../window.js";
import { type IDisposable, DisposableSlot, DisposableOwner, ResettableDisposableGroup, toDisposable } from "../../../common/lifecycle.js";

export type SashOrientation = "vertical" | "horizontal";

/** Visual and interaction settings applied to every Sash below one element. */
export interface SashSettings {
  readonly dragAreaSize: number;
  readonly hoverFeedbackSize: number;
  readonly hoverDelay: number;
}

export interface SashDragEvent {
  /** Signed movement from the position where the current drag started. */
  readonly delta: number;
}

const SashDragAreaSizeProperty = "--zeta-sash-drag-area-size";
const SashHoverFeedbackSizeProperty = "--zeta-sash-hover-feedback-size";
const SashHoverDelayProperty = "--zeta-sash-hover-delay";
const DefaultSashHoverDelay = 300;
let nextSashSettingsBindingId = 1;

/**
 * Projects Sash settings onto one DOM subtree and restores prior values when
 * disposed.
 */
export class SashSettingsBinding extends DisposableOwner {
  readonly #scopeClass: string;
  readonly #styleSheet: ManagedStyleSheet;

  constructor(container: HTMLElement) {
    super();
    this.#scopeClass =
      `zeta-sash-settings-${nextSashSettingsBindingId++}`;
    container.classList.add(this.#scopeClass);
    this.defer(() => container.classList.remove(this.#scopeClass));
    this.#styleSheet = this.own(new ManagedStyleSheet(
      container.ownerDocument,
    ));
  }

  update(settings: SashSettings): void {
    assertPositiveFinite(settings.dragAreaSize, "drag area size");
    assertPositiveFinite(settings.hoverFeedbackSize, "hover feedback size");
    assertNonNegativeFinite(settings.hoverDelay, "hover delay");
    this.#styleSheet.setText(`
      .${this.#scopeClass} .zeta-sash {
        ${SashDragAreaSizeProperty}: ${settings.dragAreaSize}px;
        ${SashHoverFeedbackSizeProperty}: ${settings.hoverFeedbackSize}px;
        ${SashHoverDelayProperty}: ${settings.hoverDelay}ms;
      }
    `);
  }
}

/** A draggable and keyboard-operable separator owned by a layout control. */
export class Sash extends DisposableOwner {
  readonly element: HTMLDivElement;
  #startListeners = new Set<() => void>();
  #changeListeners = new Set<(event: SashDragEvent) => void>();
  #endListeners = new Set<() => void>();
  readonly #dragListeners: ResettableDisposableGroup;
  readonly #hoverTimer: DisposableSlot<IDisposable>;

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
    this.#hoverTimer = this.own(new DisposableSlot<IDisposable>());
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
    this.own(addDisposableListener(element, "pointerenter", () =>
      this.beginHover(),
    ));
    this.own(addDisposableListener(element, "pointerleave", () =>
      this.endHover(),
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
    this.endHover();
    this.element.classList.add("zeta-sash-active");
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
      this.element.classList.remove("zeta-sash-active");
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

  private beginHover(): void {
    this.#hoverTimer.clear();
    const delay = sashHoverDelay(this.element);
    if (delay === 0) {
      this.element.classList.add("zeta-sash-hover");
      return;
    }
    const targetWindow = getWindow(this.element);
    const handle = targetWindow.setTimeout(() => {
      this.element.classList.add("zeta-sash-hover");
    }, delay);
    this.#hoverTimer.replace(toDisposable(() => {
      targetWindow.clearTimeout(handle);
    }));
  }

  private endHover(): void {
    this.#hoverTimer.clear();
    this.element.classList.remove("zeta-sash-hover");
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

function sashHoverDelay(element: HTMLElement): number {
  const value = getWindow(element).getComputedStyle(element)
    .getPropertyValue(SashHoverDelayProperty)
    .trim();
  if (value === "") return DefaultSashHoverDelay;
  const milliseconds = value.endsWith("ms")
    ? Number(value.slice(0, -2))
    : Number.NaN;
  return Number.isFinite(milliseconds) && milliseconds >= 0
    ? milliseconds
    : DefaultSashHoverDelay;
}

function assertPositiveFinite(value: number, name: string): void {
  if (!Number.isFinite(value) || value <= 0) {
    throw new RangeError(`Sash ${name} must be a positive finite number`);
  }
}

function assertNonNegativeFinite(value: number, name: string): void {
  if (!Number.isFinite(value) || value < 0) {
    throw new RangeError(
      `Sash ${name} must be a non-negative finite number`,
    );
  }
}

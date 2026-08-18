import { addDisposableListener, h } from "../../dom.js";
import { ManagedStyleSheet } from "../../domStylesheets.js";
import { disposableWindowTimeout } from "../../scheduler.js";
import { getWindow } from "../../window.js";
import { type IDisposable, DisposableSlot, DisposableOwner, ResettableDisposableGroup, toDisposable } from "../../../common/lifecycle.js";

export type SashOrientation = "vertical" | "horizontal";

/** The directions in which a Sash can currently resize its owner. */
export enum SashState {
  Disabled,
  AtMinimum,
  AtMaximum,
  Enabled,
}

/**
 * An opt-in placement treatment for a Sash hosted between inset surfaces.
 *
 * The Sash continues to use its configured drag target and hover feedback,
 * while its own cross-axis footprint expands to the full visual gap. Its
 * long-axis endpoints are clipped by half the gap, preventing perpendicular
 * Sashes from competing for pointer events at grid intersections.
 */
export interface InsetSashPresentation {
  readonly type: "inset";
  readonly gap: number;
}

export type SashPresentation = InsetSashPresentation | undefined;

/** Visual and interaction settings applied to every Sash below one element. */
export interface SashSettings {
  readonly dragAreaSize: number;
  readonly hoverFeedbackSize: number;
  readonly hoverDelay: number;
}

export interface SashDragEvent {
  /** Signed movement from the position where the current drag started. */
  readonly delta: number;
  readonly input: "pointer" | "keyboard";
  readonly altKey: boolean;
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
  private readonly scopeClass: string;
  private readonly styleSheet: ManagedStyleSheet;

  constructor(container: HTMLElement) {
    super();
    this.scopeClass =
      `zeta-sash-settings-${nextSashSettingsBindingId++}`;
    container.classList.add(this.scopeClass);
    this.defer(() => container.classList.remove(this.scopeClass));
    this.styleSheet = this.own(new ManagedStyleSheet(
      container.ownerDocument,
    ));
  }

  update(settings: SashSettings): void {
    assertPositiveFinite(settings.dragAreaSize, "drag area size");
    assertPositiveFinite(settings.hoverFeedbackSize, "hover feedback size");
    assertNonNegativeFinite(settings.hoverDelay, "hover delay");
    this.styleSheet.setText(`
      .${this.scopeClass} .zeta-sash {
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
  private _state = SashState.Enabled;
  private readonly startListeners = new Set<() => void>();
  private readonly changeListeners = new Set<(event: SashDragEvent) => void>();
  private readonly resetListeners = new Set<() => void>();
  private readonly endListeners = new Set<() => void>();
  private readonly stateListeners = new Set<(state: SashState) => void>();
  private readonly dragListeners: ResettableDisposableGroup;
  private readonly hoverTimer: DisposableSlot<IDisposable>;
  private readonly orthogonalStartListener: DisposableSlot<IDisposable>;
  private readonly orthogonalEndListener: DisposableSlot<IDisposable>;
  private readonly orthogonalStartResources: ResettableDisposableGroup;
  private readonly orthogonalEndResources: ResettableDisposableGroup;
  private _orthogonalStartSash: Sash | undefined;
  private _orthogonalEndSash: Sash | undefined;
  private _linkedSash: Sash | undefined;

  constructor(
    readonly orientation: SashOrientation,
    ownerDocument: Document = document,
    presentation: SashPresentation = undefined,
  ) {
    super();
    const element = h(ownerDocument, "div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = `zeta-sash zeta-sash-${orientation}`;
    if (presentation?.type === "inset") {
      assertPositiveFinite(presentation.gap, "inset gap");
      element.classList.add("zeta-sash-inset");
      element.style.setProperty("--zeta-sash-inset-gap", `${presentation.gap}px`);
    }
    element.setAttribute("role", "separator");
    element.setAttribute("aria-orientation", orientation);
    element.setAttribute("aria-disabled", "false");
    element.tabIndex = 0;
    this.dragListeners = this.own(new ResettableDisposableGroup());
    this.hoverTimer = this.own(new DisposableSlot<IDisposable>());
    this.orthogonalStartListener = this.own(new DisposableSlot<IDisposable>());
    this.orthogonalEndListener = this.own(new DisposableSlot<IDisposable>());
    this.orthogonalStartResources = this.own(new ResettableDisposableGroup());
    this.orthogonalEndResources = this.own(new ResettableDisposableGroup());
    this.own(toDisposable(() => {
      const linkedSash = this._linkedSash;
      this._linkedSash = undefined;
      if (linkedSash?.linkedSash === this) linkedSash.linkedSash = undefined;
      this.startListeners.clear();
      this.changeListeners.clear();
      this.resetListeners.clear();
      this.endListeners.clear();
      this.stateListeners.clear();
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
    this.own(addDisposableListener(element, "dblclick", () => this.reset()));
  }

  onDidStart(listener: () => void): IDisposable {
    this.startListeners.add(listener);
    return toDisposable(() => this.startListeners.delete(listener));
  }

  onDidChange(listener: (event: SashDragEvent) => void): IDisposable {
    this.changeListeners.add(listener);
    return toDisposable(() => this.changeListeners.delete(listener));
  }

  onDidReset(listener: () => void): IDisposable {
    this.resetListeners.add(listener);
    return toDisposable(() => this.resetListeners.delete(listener));
  }

  onDidEnd(listener: () => void): IDisposable {
    this.endListeners.add(listener);
    return toDisposable(() => this.endListeners.delete(listener));
  }

  onDidChangeState(listener: (state: SashState) => void): IDisposable {
    this.stateListeners.add(listener);
    return toDisposable(() => this.stateListeners.delete(listener));
  }

  get state(): SashState {
    return this._state;
  }

  set state(state: SashState) {
    if (this._state === state) return;
    this._state = state;
    const disabled = state === SashState.Disabled;
    this.element.classList.toggle("zeta-sash-disabled", disabled);
    this.element.classList.toggle("zeta-sash-minimum", state === SashState.AtMinimum);
    this.element.classList.toggle("zeta-sash-maximum", state === SashState.AtMaximum);
    this.element.setAttribute("aria-disabled", String(disabled));
    this.element.tabIndex = disabled ? -1 : 0;
    if (disabled) this.clearSashHoverState();
    this.updateOrthogonalHandle("start");
    this.updateOrthogonalHandle("end");
    for (const listener of this.stateListeners) listener(state);
  }

  get orthogonalStartSash(): Sash | undefined {
    return this._orthogonalStartSash;
  }

  set orthogonalStartSash(sash: Sash | undefined) {
    this.setOrthogonalSash("start", sash);
  }

  get orthogonalEndSash(): Sash | undefined {
    return this._orthogonalEndSash;
  }

  set orthogonalEndSash(sash: Sash | undefined) {
    this.setOrthogonalSash("end", sash);
  }

  /** A same-orientation Sash which receives this Sash's interactions. */
  get linkedSash(): Sash | undefined {
    return this._linkedSash;
  }

  set linkedSash(sash: Sash | undefined) {
    if (sash === this) throw new TypeError("A Sash cannot link to itself");
    if (sash && sash.orientation !== this.orientation) {
      throw new TypeError("Linked Sashes must use the same orientation");
    }
    this._linkedSash = sash;
  }

  clearSashHoverState(): void {
    this.hoverTimer.clear();
    this.element.classList.remove("zeta-sash-hover");
  }

  private beginDrag(event: PointerEvent, fromLinkedSash = false): void {
    if (event.button !== 0 || this._state === SashState.Disabled) return;
    event.preventDefault();
    this.endHover(fromLinkedSash);
    this.element.classList.add("zeta-sash-active");
    this.dragListeners.clear();
    const start = this.coordinate(event);
    this.fire(this.startListeners);
    if (
      !fromLinkedSash &&
      typeof event.pointerId === "number" &&
      typeof this.element.setPointerCapture === "function"
    ) {
      this.element.setPointerCapture(event.pointerId);
    }
    const move = (next: PointerEvent) => {
      const dragEvent: SashDragEvent = {
        delta: this.coordinate(next) - start,
        input: "pointer",
        altKey: next.altKey,
      };
      for (const listener of this.changeListeners) listener(dragEvent);
    };
    const stop = () => {
      this.dragListeners.clear();
      this.element.classList.remove("zeta-sash-active");
      if (
        !fromLinkedSash &&
        typeof event.pointerId === "number" &&
        typeof this.element.hasPointerCapture === "function" &&
        this.element.hasPointerCapture(event.pointerId)
      ) {
        this.element.releasePointerCapture(event.pointerId);
      }
      this.fire(this.endListeners);
    };
    const targetWindow = getWindow(this.element);
    this.dragListeners.add(addDisposableListener(
      targetWindow,
      "pointermove",
      move,
    ));
    this.dragListeners.add(addDisposableListener(
      targetWindow,
      "pointerup",
      stop,
      { once: true },
    ));
    this.dragListeners.add(addDisposableListener(
      targetWindow,
      "pointercancel",
      stop,
      { once: true },
    ));
    this.dragListeners.add(addDisposableListener(
      targetWindow,
      "blur",
      stop,
      { once: true },
    ));
    if (!fromLinkedSash) this._linkedSash?.beginDrag(event, true);
  }

  private beginHover(fromLinkedSash = false): void {
    if (this._state === SashState.Disabled) return;
    if (!fromLinkedSash) this._linkedSash?.beginHover(true);
    this.hoverTimer.clear();
    const delay = sashHoverDelay(this.element);
    if (delay === 0) {
      this.element.classList.add("zeta-sash-hover");
      return;
    }
    const targetWindow = getWindow(this.element);
    this.hoverTimer.replace(disposableWindowTimeout(targetWindow, () => {
      this.element.classList.add("zeta-sash-hover");
    }, delay));
  }

  private endHover(fromLinkedSash = false): void {
    this.hoverTimer.clear();
    this.element.classList.remove("zeta-sash-hover");
    if (!fromLinkedSash) this._linkedSash?.endHover(true);
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (this._state === SashState.Disabled) return;
    const delta = this.keyboardDelta(event);
    if (delta === undefined) return;
    event.preventDefault();
    this.fire(this.startListeners);
    for (const listener of this.changeListeners) {
      listener({ delta, input: "keyboard", altKey: false });
    }
    this.fire(this.endListeners);
  }

  private coordinate(event: Pick<PointerEvent, "clientX" | "clientY">): number {
    return this.orientation === "vertical" ? event.clientX : event.clientY;
  }

  private setOrthogonalSash(edge: "start" | "end", sash: Sash | undefined): void {
    if (sash?.orientation === this.orientation) {
      throw new TypeError("Orthogonal Sashes must use different orientations");
    }
    const current = edge === "start" ? this._orthogonalStartSash : this._orthogonalEndSash;
    if (current === sash) return;
    if (edge === "start") {
      this._orthogonalStartSash = sash;
      this.orthogonalStartListener.replace(sash?.onDidChangeState(() => this.updateOrthogonalHandle("start")));
    } else {
      this._orthogonalEndSash = sash;
      this.orthogonalEndListener.replace(sash?.onDidChangeState(() => this.updateOrthogonalHandle("end")));
    }
    this.updateOrthogonalHandle(edge);
  }

  private updateOrthogonalHandle(edge: "start" | "end"): void {
    const resources = edge === "start" ? this.orthogonalStartResources : this.orthogonalEndResources;
    const orthogonalSash = edge === "start" ? this._orthogonalStartSash : this._orthogonalEndSash;
    const stateClass = `zeta-sash-has-orthogonal-${edge}`;
    resources.clear();
    this.element.classList.remove(stateClass);
    if (!orthogonalSash || this.state === SashState.Disabled || orthogonalSash.state === SashState.Disabled) return;
    const handle = h(this.element.ownerDocument, "div");
    handle.className = `zeta-sash-orthogonal-handle zeta-sash-orthogonal-handle-${edge}`;
    this.element.append(handle);
    this.element.classList.add(stateClass);
    resources.add(toDisposable(() => {
      handle.remove();
      this.element.classList.remove(stateClass);
    }));
    resources.add(addDisposableListener(handle, "pointerdown", (event: PointerEvent) => {
      event.stopPropagation();
      this.beginDrag(event);
      orthogonalSash.beginDrag(event);
    }));
    resources.add(addDisposableListener(handle, "dblclick", (event: MouseEvent) => {
      event.stopPropagation();
      this.reset();
      orthogonalSash.reset();
    }));
    resources.add(addDisposableListener(handle, "pointerenter", () => orthogonalSash.beginHover()));
    resources.add(addDisposableListener(handle, "pointerleave", () => orthogonalSash.endHover()));
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

  private reset(fromLinkedSash = false): void {
    if (this._state === SashState.Disabled) return;
    this.fire(this.resetListeners);
    if (!fromLinkedSash) this._linkedSash?.reset(true);
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

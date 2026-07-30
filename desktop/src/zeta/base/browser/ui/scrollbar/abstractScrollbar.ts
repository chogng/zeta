import { addDisposableListener } from "../../dom.js";
import { StandardPointerEvent } from "../../mouseEvent.js";
import {
  DisposableOwner,
  ResettableDisposableGroup,
} from "../../../common/lifecycle.js";
import type { ScrollbarAxisMetrics } from "./scrollbarState.js";

export type ScrollbarAxis = "horizontal" | "vertical";

export interface AbstractScrollbarOptions {
  readonly ownerDocument: Document;
  readonly viewport: HTMLElement;
  readonly trackClickBehavior: "jump" | "page";
  readonly getMetrics: () => ScrollbarAxisMetrics;
  readonly setPosition: (position: number) => void;
}

/** Owns the DOM and direct input behavior for one scrollbar axis. */
export abstract class AbstractScrollbar extends DisposableOwner {
  readonly track: HTMLDivElement;
  readonly thumb: HTMLDivElement;
  private readonly trackClickBehavior: "jump" | "page";
  private readonly getMetrics: () => ScrollbarAxisMetrics;
  private readonly setPosition: (position: number) => void;
  private readonly dragListeners: ResettableDisposableGroup;

  protected constructor(
    axis: ScrollbarAxis,
    options: AbstractScrollbarOptions,
  ) {
    super();
    this.trackClickBehavior = options.trackClickBehavior;
    this.getMetrics = options.getMetrics;
    this.setPosition = options.setPosition;
    const track = options.ownerDocument.createElement("div");
    const thumb = options.ownerDocument.createElement("div");
    this.track = track;
    this.thumb = thumb;
    this.dragListeners = this.own(new ResettableDisposableGroup());
    this.defer(() => track.remove());

    track.className =
      `zeta-scrollbar-track zeta-scrollbar-track-${axis}`;
    track.setAttribute("role", "scrollbar");
    track.setAttribute("aria-label", `${capitalize(axis)} scrollbar`);
    track.setAttribute("aria-orientation", axis);
    track.setAttribute("aria-controls", options.viewport.id);
    track.setAttribute("aria-valuemin", "0");
    thumb.className = "zeta-scrollbar-thumb";
    track.append(thumb);

    this.own(addDisposableListener(
      track,
      "pointerdown",
      (event: PointerEvent) => {
        if (event.target === thumb) this.beginThumbDrag(event);
        else this.handleTrackPointerDown(event);
      },
    ));
    this.own(addDisposableListener(
      track,
      "keydown",
      (event: KeyboardEvent) => this.handleKeydown(event),
    ));
  }

  get rendered(): boolean {
    return !this.track.hidden;
  }

  protected abstract applyThumbMetrics(
    metrics: ScrollbarAxisMetrics,
  ): void;

  protected abstract pointerCoordinate(
    event: Pick<PointerEvent, "clientX" | "clientY">,
  ): number;

  protected abstract trackPointerCoordinate(
    event: Pick<PointerEvent, "clientX" | "clientY">,
    bounds: DOMRect,
  ): number;

  protected abstract keyboardDelta(
    key: string,
    step: number,
  ): number | undefined;

  render(metrics: ScrollbarAxisMetrics, rendered: boolean): void {
    this.track.hidden = !rendered;
    this.track.tabIndex = rendered && metrics.maximumPosition > 0
      ? 0
      : -1;
    this.track.setAttribute(
      "aria-valuemax",
      String(Math.round(metrics.maximumPosition)),
    );
    this.track.setAttribute(
      "aria-valuenow",
      String(Math.round(metrics.position)),
    );
    this.track.setAttribute(
      "aria-disabled",
      String(metrics.maximumPosition === 0),
    );
    this.applyThumbMetrics(metrics);
  }

  private handleKeydown(event: KeyboardEvent): void {
    const metrics = this.getMetrics();
    const step = event.altKey ? 10 : 40;
    const delta = this.keyboardDelta(event.key, step);
    let next = metrics.position;
    if (delta !== undefined) {
      next += delta;
    } else {
      switch (event.key) {
        case "PageUp":
          next = metrics.position - metrics.viewportSize;
          break;
        case "PageDown":
          next = metrics.position + metrics.viewportSize;
          break;
        case "Home":
          next = 0;
          break;
        case "End":
          next = metrics.maximumPosition;
          break;
        default:
          return;
      }
    }
    event.preventDefault();
    event.stopPropagation();
    this.setPosition(next);
  }

  private beginThumbDrag(browserEvent: PointerEvent): void {
    const event = new StandardPointerEvent(browserEvent);
    if (!event.leftButton) return;
    event.stop();
    const startCoordinate = this.pointerCoordinate(event);
    const startMetrics = this.getMetrics();
    const thumbTravel = startMetrics.trackSize - startMetrics.thumbSize;
    if (thumbTravel <= 0 || startMetrics.maximumPosition <= 0) return;
    this.dragListeners.clear();
    this.track.dataset.active = "true";
    if (
      typeof this.track.setPointerCapture === "function" &&
      browserEvent.pointerId !== undefined
    ) {
      this.track.setPointerCapture(event.pointerId);
    }
    const targetWindow = this.track.ownerDocument.defaultView;
    if (!targetWindow) {
      throw new Error("Scrollbar drag requires a browser window");
    }
    const move = (nextBrowserEvent: PointerEvent) => {
      const next = new StandardPointerEvent(nextBrowserEvent);
      if (
        browserEvent.pointerId !== undefined &&
        nextBrowserEvent.pointerId !== browserEvent.pointerId
      ) return;
      next.preventDefault();
      const pointerDelta =
        this.pointerCoordinate(next) - startCoordinate;
      this.setPosition(
        startMetrics.position +
          pointerDelta * startMetrics.maximumPosition / thumbTravel,
      );
    };
    const stop = () => {
      if (
        typeof this.track.hasPointerCapture === "function" &&
        this.track.hasPointerCapture(event.pointerId)
      ) {
        this.track.releasePointerCapture(event.pointerId);
      }
      delete this.track.dataset.active;
      this.dragListeners.clear();
    };
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
  }

  private handleTrackPointerDown(browserEvent: PointerEvent): void {
    const event = new StandardPointerEvent(browserEvent);
    if (!event.leftButton) return;
    event.stop();
    const bounds = this.track.getBoundingClientRect();
    const coordinate = this.trackPointerCoordinate(event, bounds);
    const metrics = this.getMetrics();
    let next: number;
    if (this.trackClickBehavior === "page") {
      next = coordinate < metrics.thumbPosition
        ? metrics.position - metrics.viewportSize
        : metrics.position + metrics.viewportSize;
    } else {
      const targetThumbPosition = coordinate - metrics.thumbSize / 2;
      const thumbTravel = metrics.trackSize - metrics.thumbSize;
      next = thumbTravel <= 0
        ? 0
        : targetThumbPosition *
          metrics.maximumPosition /
          thumbTravel;
    }
    this.setPosition(next);
  }
}

function capitalize(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1);
}

import { addDisposableListener, h } from "../../dom.js";
import { FastDomNode } from "../../fastDomNode.js";
import { StandardWheelEvent } from "../../mouseEvent.js";
import { observeResize } from "../../observer.js";
import { disposableWindowTimeout } from "../../scheduler.js";
import { Emitter, type Event } from "../../../common/event.js";
import { DisposableOwner, DisposableSlot, type IDisposable } from "../../../common/lifecycle.js";
import type { ScrollbarAxis } from "./abstractScrollbar.js";
import { HorizontalScrollbar } from "./horizontalScrollbar.js";
import {
  clampScrollbarPosition,
  createScrollbarAxisMetrics,
  type ScrollbarAxisMetrics,
} from "./scrollbarState.js";
import {
  resolveScrollableElementOptions,
  type ResolvedScrollableElementOptions,
  type ScrollableElementOptions,
  type ScrollbarVisibility,
} from "./scrollableElementOptions.js";
import { VerticalScrollbar } from "./verticalScrollbar.js";

export type {
  ScrollableElementOptions,
  ScrollbarVisibility,
  ScrollbarWheelOptions,
  ScrollDirection,
} from "./scrollableElementOptions.js";
export type { ScrollbarAxis } from "./abstractScrollbar.js";

export interface ScrollPosition {
  readonly left: number;
  readonly top: number;
}

export interface ScrollableElementState extends ScrollPosition {
  readonly width: number;
  readonly height: number;
  readonly scrollWidth: number;
  readonly scrollHeight: number;
  readonly maximumLeft: number;
  readonly maximumTop: number;
}

export interface ScrollableScrollEvent {
  readonly previous: ScrollPosition;
  readonly current: ScrollableElementState;
}

const initialState: ScrollableElementState = {
  left: 0,
  top: 0,
  width: 0,
  height: 0,
  scrollWidth: 0,
  scrollHeight: 0,
  maximumLeft: 0,
  maximumTop: 0,
};
let nextScrollableId = 1;

/**
 * Themeable two-axis scroll container with managed wheel and pointer input.
 *
 * Content remains in a native scrolling viewport for keyboard, touch, focus
 * reveal, and accessibility behavior. The native bars are hidden and mirrored
 * by stable DOM tracks whose visibility and interaction are controlled here.
 */
export class ScrollableElement extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly scrollableElement: HTMLDivElement;
  readonly contentElement: HTMLDivElement;
  readonly onDidScroll: Event<ScrollableScrollEvent>;
  private readonly horizontal: HorizontalScrollbar;
  private readonly vertical: VerticalScrollbar;
  private readonly corner: HTMLDivElement;
  private readonly horizontalTrackNode: FastDomNode<HTMLDivElement>;
  private readonly verticalTrackNode: FastDomNode<HTMLDivElement>;
  private readonly cornerNode: FastDomNode<HTMLDivElement>;
  private readonly options: ResolvedScrollableElementOptions;
  private readonly onScrollOption: ((position: ScrollPosition) => void) | undefined;
  private readonly onDidScrollEmitter: Emitter<ScrollableScrollEvent>;
  private _state = initialState;
  private pendingReveal: Element | undefined;
  private readonly scrollActivityTimeout = this.own(new DisposableSlot<IDisposable>());

  constructor(container: HTMLElement, options: ScrollableElementOptions = {}) {
    super();
    this.options = resolveScrollableElementOptions(options);
    this.onScrollOption = options.onScroll;
    const ownerDocument = container.ownerDocument;
    const element = h(ownerDocument, "div");
    const viewport = h(ownerDocument, "div");
    const content = h(ownerDocument, "div");
    viewport.id = `zeta-scrollable-${nextScrollableId++}`;
    const horizontal = this.own(new HorizontalScrollbar(element, {
      viewport,
      trackClickBehavior: this.options.trackClickBehavior,
      getMetrics: () => this.axisMetrics("horizontal"),
      setPosition: (position) =>
        this.setAxisPosition("horizontal", position),
    }));
    const vertical = this.own(new VerticalScrollbar(element, {
      viewport,
      trackClickBehavior: this.options.trackClickBehavior,
      getMetrics: () => this.axisMetrics("vertical"),
      setPosition: (position) =>
        this.setAxisPosition("vertical", position),
    }));
    const corner = h(ownerDocument, "div");
    this.element = element;
    this.scrollableElement = viewport;
    this.contentElement = content;
    this.horizontal = horizontal;
    this.vertical = vertical;
    this.corner = corner;
    this.horizontalTrackNode = new FastDomNode(horizontal.track);
    this.verticalTrackNode = new FastDomNode(vertical.track);
    this.cornerNode = new FastDomNode(corner);
    this.onDidScrollEmitter = this.own(new Emitter<ScrollableScrollEvent>());
    this.onDidScroll = this.onDidScrollEmitter.event;

    element.className = "zeta-scrollable-element zeta-scrollbar";
    element.dataset.scrollDirection = this.options.direction;
    element.tabIndex = options.tabIndex ?? 0;
    element.style.setProperty(
      "--zeta-scrollbar-size",
      `${this.options.scrollbarSize}px`,
    );
    if (options.ariaLabel) {
      element.setAttribute("role", "region");
      element.setAttribute("aria-label", options.ariaLabel);
    }
    viewport.className = "zeta-scrollbar-viewport";
    content.className = "zeta-scrollbar-content";
    horizontal.track.dataset.visibility = this.options.horizontal;
    vertical.track.dataset.visibility = this.options.vertical;
    corner.className = "zeta-scrollbar-corner";
    viewport.append(content);
    element.append(
      viewport,
      horizontal.track,
      vertical.track,
      corner,
    );
    container.append(element);

    this.defer(() => {
      this.pendingReveal = undefined;
      element.remove();
    });
    this.own(addDisposableListener(viewport, "scroll", () =>
      this.handleNativeScroll(),
    ));
    this.own(addDisposableListener(viewport, "wheel", (event: WheelEvent) =>
      this.handleWheel(event),
    { passive: false }));
    this.own(addDisposableListener(element, "keydown", (event: KeyboardEvent) =>
      this.handleContainerKeydown(event),
    ));

    this.own(observeResize([element, content], () => this.layout()));
    this.layout();
  }

  get state(): ScrollableElementState {
    return this._state;
  }

  setContent(content: Element): void {
    this.replaceChildren(content);
  }

  append(...children: readonly (Node | string)[]): void {
    this.contentElement.append(...children);
    this.layout();
  }

  replaceChildren(...children: readonly (Node | string)[]): void {
    this.contentElement.replaceChildren(...children);
    this.layout();
  }

  layout(): void {
    const width = Math.max(0, this.scrollableElement.clientWidth);
    const height = Math.max(0, this.scrollableElement.clientHeight);
    const scrollWidth = this.options.direction === "vertical"
      ? width
      : Math.max(width, this.scrollableElement.scrollWidth);
    const scrollHeight = this.options.direction === "horizontal"
      ? height
      : Math.max(height, this.scrollableElement.scrollHeight);
    const maximumLeft = Math.max(0, scrollWidth - width);
    const maximumTop = Math.max(0, scrollHeight - height);
    const left = clampScrollbarPosition(
      this.scrollableElement.scrollLeft,
      maximumLeft,
    );
    const top = clampScrollbarPosition(
      this.scrollableElement.scrollTop,
      maximumTop,
    );
    if (
      left !== this.scrollableElement.scrollLeft ||
      top !== this.scrollableElement.scrollTop
    ) {
      this.scrollableElement.scrollLeft = left;
      this.scrollableElement.scrollTop = top;
    }
    this.commitState({
      left,
      top,
      width,
      height,
      scrollWidth,
      scrollHeight,
      maximumLeft,
      maximumTop,
    });
    this.applyPendingReveal();
  }

  scrollTo(left: number, top: number): void {
    this.setScrollPosition(left, top);
  }

  scrollBy(deltaLeft: number, deltaTop: number): void {
    this.setScrollPosition(
      this._state.left + deltaLeft,
      this._state.top + deltaTop,
    );
  }

  /** Reveals a descendant at the nearest visible edge on the enabled axes. */
  reveal(element: Element): void {
    if (!this.contentElement.contains(element)) {
      throw new RangeError("ScrollableElement can only reveal its descendants");
    }
    this.pendingReveal = element;
    this.layout();
  }

  private applyPendingReveal(): void {
    const element = this.pendingReveal;
    if (!element) return;
    if (!this.contentElement.contains(element)) {
      this.pendingReveal = undefined;
      return;
    }
    if (
      (this.options.direction !== "vertical" && this._state.width <= 0) ||
      (this.options.direction !== "horizontal" && this._state.height <= 0)
    ) return;
    this.pendingReveal = undefined;
    const viewportBounds = this.scrollableElement.getBoundingClientRect();
    const elementBounds = element.getBoundingClientRect();
    let left = this._state.left;
    let top = this._state.top;
    if (this.options.direction !== "vertical") {
      const viewportLeft = viewportBounds.left;
      const viewportRight = viewportBounds.left + this._state.width -
        (this.vertical.rendered ? this.options.scrollbarSize : 0);
      if (elementBounds.left < viewportLeft) {
        left += elementBounds.left - viewportLeft;
      } else if (elementBounds.right > viewportRight) {
        left += elementBounds.right - viewportRight;
      }
    }
    if (this.options.direction !== "horizontal") {
      const viewportTop = viewportBounds.top;
      const viewportBottom = viewportBounds.top + this._state.height -
        (this.horizontal.rendered ? this.options.scrollbarSize : 0);
      if (elementBounds.top < viewportTop) {
        top += elementBounds.top - viewportTop;
      } else if (elementBounds.bottom > viewportBottom) {
        top += elementBounds.bottom - viewportBottom;
      }
    }
    this.setScrollPosition(left, top);
  }

  private handleNativeScroll(): void {
    const previous = this._state;
    this.layout();
    if (
      previous.left === this._state.left &&
      previous.top === this._state.top
    ) return;
    this.showScrollbars();
  }

  private handleWheel(browserEvent: WheelEvent): void {
    const wheel = new StandardWheelEvent(browserEvent, {
      pageWidth: this._state.width,
      pageHeight: this._state.height,
    });
    let deltaX = wheel.deltaX;
    let deltaY = wheel.deltaY;
    if (
      wheel.shiftKey &&
      this.options.wheel.shift === "horizontal" &&
      deltaX === 0
    ) {
      deltaX = deltaY;
      deltaY = 0;
    }
    if (this.options.direction === "horizontal") {
      if (deltaX === 0) deltaX = deltaY;
      deltaY = 0;
    } else if (this.options.direction === "vertical") {
      deltaX = 0;
    }
    if (
      this.options.wheel.axis === "predominant" &&
      deltaX !== 0 &&
      deltaY !== 0
    ) {
      if (Math.abs(deltaX) > Math.abs(deltaY)) deltaY = 0;
      else deltaX = 0;
    }
    const sensitivity = this.options.wheel.sensitivity * (
      wheel.altKey ? this.options.wheel.fastSensitivity : 1
    );
    const changed = this.setScrollPosition(
      this._state.left + deltaX * sensitivity,
      this._state.top + deltaY * sensitivity,
    );
    if (
      changed ||
      this.options.wheel.consume === "always"
    ) {
      wheel.stop();
    }
  }

  private handleContainerKeydown(event: KeyboardEvent): void {
    if (event.target !== this.element) return;
    const step = event.altKey ? 10 : 40;
    let left = this._state.left;
    let top = this._state.top;
    switch (event.key) {
      case "ArrowLeft":
        if (this.options.direction === "vertical") return;
        left -= step;
        break;
      case "ArrowRight":
        if (this.options.direction === "vertical") return;
        left += step;
        break;
      case "ArrowUp":
        if (this.options.direction === "horizontal") return;
        top -= step;
        break;
      case "ArrowDown":
        if (this.options.direction === "horizontal") return;
        top += step;
        break;
      case "PageUp":
        if (this.options.direction === "horizontal") {
          left -= this._state.width;
        } else {
          top -= this._state.height;
        }
        break;
      case "PageDown":
        if (this.options.direction === "horizontal") {
          left += this._state.width;
        } else {
          top += this._state.height;
        }
        break;
      case "Home":
        if (this.options.direction === "horizontal") left = 0;
        else top = 0;
        break;
      case "End":
        if (this.options.direction === "horizontal") {
          left = this._state.maximumLeft;
        } else {
          top = this._state.maximumTop;
        }
        break;
      default: return;
    }
    event.preventDefault();
    this.setScrollPosition(left, top);
  }

  private setAxisPosition(axis: ScrollbarAxis, position: number): boolean {
    return axis === "horizontal"
      ? this.setScrollPosition(position, this._state.top)
      : this.setScrollPosition(this._state.left, position);
  }

  private setScrollPosition(left: number, top: number): boolean {
    left = clampScrollbarPosition(left, this._state.maximumLeft);
    top = clampScrollbarPosition(top, this._state.maximumTop);
    if (left === this._state.left && top === this._state.top) return false;
    this.scrollableElement.scrollLeft = left;
    this.scrollableElement.scrollTop = top;
    this.commitState({ ...this._state, left, top });
    this.showScrollbars();
    return true;
  }

  private commitState(next: ScrollableElementState): void {
    const previous = this._state;
    this._state = next;
    this.render();
    if (previous.left === next.left && previous.top === next.top) return;
    const previousPosition = {
      left: previous.left,
      top: previous.top,
    };
    this.onDidScrollEmitter.fire({
      previous: previousPosition,
      current: next,
    });
    this.onScrollOption?.({ left: next.left, top: next.top });
  }

  private render(): void {
    const horizontalNeeded = this._state.maximumLeft > 0;
    const verticalNeeded = this._state.maximumTop > 0;
    const horizontalRendered = isRendered(
      this.options.horizontal,
      horizontalNeeded,
    );
    const verticalRendered = isRendered(
      this.options.vertical,
      verticalNeeded,
    );
    this.horizontalTrackNode.setRight(verticalRendered ? this.options.scrollbarSize : 0);
    this.verticalTrackNode.setBottom(horizontalRendered ? this.options.scrollbarSize : 0);
    this.cornerNode.setHidden(!(horizontalRendered && verticalRendered));
    const horizontalTrackSize = Math.max(
      0,
      this._state.width -
        (verticalRendered ? this.options.scrollbarSize : 0),
    );
    const verticalTrackSize = Math.max(
      0,
      this._state.height -
        (horizontalRendered ? this.options.scrollbarSize : 0),
    );
    this.horizontal.render(
      createScrollbarAxisMetrics(
        this._state.width,
        this._state.scrollWidth,
        this._state.left,
        horizontalTrackSize,
        this.options.minimumThumbSize,
      ),
      horizontalRendered,
    );
    this.vertical.render(
      createScrollbarAxisMetrics(
        this._state.height,
        this._state.scrollHeight,
        this._state.top,
        verticalTrackSize,
        this.options.minimumThumbSize,
      ),
      verticalRendered,
    );
  }

  private axisMetrics(axis: ScrollbarAxis): ScrollbarAxisMetrics {
    const oppositeRendered = axis === "horizontal"
      ? this.vertical.rendered
      : this.horizontal.rendered;
    const trackSize = (
      axis === "horizontal" ? this._state.width : this._state.height
    ) - (oppositeRendered ? this.options.scrollbarSize : 0);
    return axis === "horizontal"
      ? createScrollbarAxisMetrics(
        this._state.width,
        this._state.scrollWidth,
        this._state.left,
        trackSize,
        this.options.minimumThumbSize,
      )
      : createScrollbarAxisMetrics(
        this._state.height,
        this._state.scrollHeight,
        this._state.top,
        trackSize,
        this.options.minimumThumbSize,
      );
  }

  private showScrollbars(): void {
    const targetWindow = ownerWindow(this.element);
    this.element.classList.add("zeta-scrollbar-scrolling");
    this.scrollActivityTimeout.replace(disposableWindowTimeout(targetWindow, () => {
      this.scrollActivityTimeout.clear();
      this.element.classList.remove("zeta-scrollbar-scrolling");
    }, 700));
  }
}

function isRendered(
  visibility: ScrollbarVisibility,
  needed: boolean,
): boolean {
  return visibility === "visible" ||
    (visibility === "auto" && needed);
}

function ownerWindow(element: HTMLElement): Window {
  const targetWindow = element.ownerDocument.defaultView;
  if (!targetWindow) throw new Error("ScrollableElement requires a browser window");
  return targetWindow;
}

import { addDisposableListener } from "../../dom.js";
import { StandardWheelEvent } from "../../mouseEvent.js";
import { Emitter, type Event } from "../../../common/event.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
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
  readonly #horizontal: HorizontalScrollbar;
  readonly #vertical: VerticalScrollbar;
  readonly #corner: HTMLDivElement;
  readonly #options: ResolvedScrollableElementOptions;
  readonly #onScrollOption: ((position: ScrollPosition) => void) | undefined;
  readonly #onDidScrollEmitter: Emitter<ScrollableScrollEvent>;
  #state = initialState;
  #scrollActivityTimer: number | undefined;

  constructor(options: ScrollableElementOptions = {}) {
    super();
    this.#options = resolveScrollableElementOptions(options);
    this.#onScrollOption = options.onScroll;
    const ownerDocument = options.ownerDocument ?? document;
    const element = ownerDocument.createElement("div");
    const viewport = ownerDocument.createElement("div");
    const content = ownerDocument.createElement("div");
    viewport.id = `zeta-scrollable-${nextScrollableId++}`;
    const horizontal = this.own(new HorizontalScrollbar({
      ownerDocument,
      viewport,
      trackClickBehavior: this.#options.trackClickBehavior,
      getMetrics: () => this.#axisMetrics("horizontal"),
      setPosition: (position) =>
        this.#setAxisPosition("horizontal", position),
    }));
    const vertical = this.own(new VerticalScrollbar({
      ownerDocument,
      viewport,
      trackClickBehavior: this.#options.trackClickBehavior,
      getMetrics: () => this.#axisMetrics("vertical"),
      setPosition: (position) =>
        this.#setAxisPosition("vertical", position),
    }));
    const corner = ownerDocument.createElement("div");
    this.element = element;
    this.scrollableElement = viewport;
    this.contentElement = content;
    this.#horizontal = horizontal;
    this.#vertical = vertical;
    this.#corner = corner;
    this.#onDidScrollEmitter = this.own(new Emitter<ScrollableScrollEvent>());
    this.onDidScroll = this.#onDidScrollEmitter.event;

    element.className = "zeta-scrollable-element zeta-scrollbar";
    element.dataset.scrollDirection = this.#options.direction;
    element.tabIndex = options.tabIndex ?? 0;
    element.style.setProperty(
      "--zeta-scrollbar-size",
      `${this.#options.scrollbarSize}px`,
    );
    if (options.ariaLabel) {
      element.setAttribute("role", "region");
      element.setAttribute("aria-label", options.ariaLabel);
    }
    viewport.className = "zeta-scrollbar-viewport";
    content.className = "zeta-scrollbar-content";
    horizontal.track.dataset.visibility = this.#options.horizontal;
    vertical.track.dataset.visibility = this.#options.vertical;
    corner.className = "zeta-scrollbar-corner";
    viewport.append(content);
    element.append(
      viewport,
      horizontal.track,
      vertical.track,
      corner,
    );

    this.defer(() => {
      const timer = this.#scrollActivityTimer;
      if (timer !== undefined) ownerWindow(element).clearTimeout(timer);
      element.remove();
    });
    this.own(addDisposableListener(viewport, "scroll", () =>
      this.#handleNativeScroll(),
    ));
    this.own(addDisposableListener(viewport, "wheel", (event: WheelEvent) =>
      this.#handleWheel(event),
    { passive: false }));
    this.own(addDisposableListener(element, "keydown", (event: KeyboardEvent) =>
      this.#handleContainerKeydown(event),
    ));

    const ResizeObserverConstructor = ownerDocument.defaultView?.ResizeObserver;
    if (ResizeObserverConstructor) {
      const observer = new ResizeObserverConstructor(() => this.layout());
      observer.observe(element);
      observer.observe(content);
      this.defer(() => observer.disconnect());
    }
    this.layout();
  }

  get state(): ScrollableElementState {
    return this.#state;
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
    const scrollWidth = this.#options.direction === "vertical"
      ? width
      : Math.max(width, this.scrollableElement.scrollWidth);
    const scrollHeight = this.#options.direction === "horizontal"
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
    this.#commitState({
      left,
      top,
      width,
      height,
      scrollWidth,
      scrollHeight,
      maximumLeft,
      maximumTop,
    });
  }

  scrollTo(left: number, top: number): void {
    this.#setScrollPosition(left, top);
  }

  scrollBy(deltaLeft: number, deltaTop: number): void {
    this.#setScrollPosition(
      this.#state.left + deltaLeft,
      this.#state.top + deltaTop,
    );
  }

  #handleNativeScroll(): void {
    const previous = this.#state;
    this.layout();
    if (
      previous.left === this.#state.left &&
      previous.top === this.#state.top
    ) return;
    this.#showScrollbars();
  }

  #handleWheel(browserEvent: WheelEvent): void {
    const wheel = new StandardWheelEvent(browserEvent, {
      pageWidth: this.#state.width,
      pageHeight: this.#state.height,
    });
    let deltaX = wheel.deltaX;
    let deltaY = wheel.deltaY;
    if (
      wheel.shiftKey &&
      this.#options.wheel.shift === "horizontal" &&
      deltaX === 0
    ) {
      deltaX = deltaY;
      deltaY = 0;
    }
    if (this.#options.direction === "horizontal") {
      if (deltaX === 0) deltaX = deltaY;
      deltaY = 0;
    } else if (this.#options.direction === "vertical") {
      deltaX = 0;
    }
    if (
      this.#options.wheel.axis === "predominant" &&
      deltaX !== 0 &&
      deltaY !== 0
    ) {
      if (Math.abs(deltaX) > Math.abs(deltaY)) deltaY = 0;
      else deltaX = 0;
    }
    const sensitivity = this.#options.wheel.sensitivity * (
      wheel.altKey ? this.#options.wheel.fastSensitivity : 1
    );
    const changed = this.#setScrollPosition(
      this.#state.left + deltaX * sensitivity,
      this.#state.top + deltaY * sensitivity,
    );
    if (
      changed ||
      this.#options.wheel.consume === "always"
    ) {
      wheel.stop();
    }
  }

  #handleContainerKeydown(event: KeyboardEvent): void {
    if (event.target !== this.element) return;
    const step = event.altKey ? 10 : 40;
    let left = this.#state.left;
    let top = this.#state.top;
    switch (event.key) {
      case "ArrowLeft":
        if (this.#options.direction === "vertical") return;
        left -= step;
        break;
      case "ArrowRight":
        if (this.#options.direction === "vertical") return;
        left += step;
        break;
      case "ArrowUp":
        if (this.#options.direction === "horizontal") return;
        top -= step;
        break;
      case "ArrowDown":
        if (this.#options.direction === "horizontal") return;
        top += step;
        break;
      case "PageUp":
        if (this.#options.direction === "horizontal") {
          left -= this.#state.width;
        } else {
          top -= this.#state.height;
        }
        break;
      case "PageDown":
        if (this.#options.direction === "horizontal") {
          left += this.#state.width;
        } else {
          top += this.#state.height;
        }
        break;
      case "Home":
        if (this.#options.direction === "horizontal") left = 0;
        else top = 0;
        break;
      case "End":
        if (this.#options.direction === "horizontal") {
          left = this.#state.maximumLeft;
        } else {
          top = this.#state.maximumTop;
        }
        break;
      default: return;
    }
    event.preventDefault();
    this.#setScrollPosition(left, top);
  }

  #setAxisPosition(axis: ScrollbarAxis, position: number): boolean {
    return axis === "horizontal"
      ? this.#setScrollPosition(position, this.#state.top)
      : this.#setScrollPosition(this.#state.left, position);
  }

  #setScrollPosition(left: number, top: number): boolean {
    left = clampScrollbarPosition(left, this.#state.maximumLeft);
    top = clampScrollbarPosition(top, this.#state.maximumTop);
    if (left === this.#state.left && top === this.#state.top) return false;
    this.scrollableElement.scrollLeft = left;
    this.scrollableElement.scrollTop = top;
    this.#commitState({ ...this.#state, left, top });
    this.#showScrollbars();
    return true;
  }

  #commitState(next: ScrollableElementState): void {
    const previous = this.#state;
    this.#state = next;
    this.#render();
    if (previous.left === next.left && previous.top === next.top) return;
    const previousPosition = {
      left: previous.left,
      top: previous.top,
    };
    this.#onDidScrollEmitter.fire({
      previous: previousPosition,
      current: next,
    });
    this.#onScrollOption?.({ left: next.left, top: next.top });
  }

  #render(): void {
    const horizontalNeeded = this.#state.maximumLeft > 0;
    const verticalNeeded = this.#state.maximumTop > 0;
    const horizontalRendered = isRendered(
      this.#options.horizontal,
      horizontalNeeded,
    );
    const verticalRendered = isRendered(
      this.#options.vertical,
      verticalNeeded,
    );
    this.#horizontal.track.style.right = verticalRendered
      ? `${this.#options.scrollbarSize}px`
      : "0px";
    this.#vertical.track.style.bottom = horizontalRendered
      ? `${this.#options.scrollbarSize}px`
      : "0px";
    this.#corner.hidden = !(horizontalRendered && verticalRendered);
    const horizontalTrackSize = Math.max(
      0,
      this.#state.width -
        (verticalRendered ? this.#options.scrollbarSize : 0),
    );
    const verticalTrackSize = Math.max(
      0,
      this.#state.height -
        (horizontalRendered ? this.#options.scrollbarSize : 0),
    );
    this.#horizontal.render(
      createScrollbarAxisMetrics(
        this.#state.width,
        this.#state.scrollWidth,
        this.#state.left,
        horizontalTrackSize,
        this.#options.minimumThumbSize,
      ),
      horizontalRendered,
    );
    this.#vertical.render(
      createScrollbarAxisMetrics(
        this.#state.height,
        this.#state.scrollHeight,
        this.#state.top,
        verticalTrackSize,
        this.#options.minimumThumbSize,
      ),
      verticalRendered,
    );
  }

  #axisMetrics(axis: ScrollbarAxis): ScrollbarAxisMetrics {
    const oppositeRendered = axis === "horizontal"
      ? this.#vertical.rendered
      : this.#horizontal.rendered;
    const trackSize = (
      axis === "horizontal" ? this.#state.width : this.#state.height
    ) - (oppositeRendered ? this.#options.scrollbarSize : 0);
    return axis === "horizontal"
      ? createScrollbarAxisMetrics(
        this.#state.width,
        this.#state.scrollWidth,
        this.#state.left,
        trackSize,
        this.#options.minimumThumbSize,
      )
      : createScrollbarAxisMetrics(
        this.#state.height,
        this.#state.scrollHeight,
        this.#state.top,
        trackSize,
        this.#options.minimumThumbSize,
      );
  }

  #showScrollbars(): void {
    const targetWindow = ownerWindow(this.element);
    if (this.#scrollActivityTimer !== undefined) {
      targetWindow.clearTimeout(this.#scrollActivityTimer);
    }
    this.element.classList.add("zeta-scrollbar-scrolling");
    this.#scrollActivityTimer = targetWindow.setTimeout(() => {
      this.element.classList.remove("zeta-scrollbar-scrolling");
      this.#scrollActivityTimer = undefined;
    }, 700);
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

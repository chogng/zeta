import "./editorScrollbar.css";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { FastDomNode } from "../../../../base/browser/fastDomNode.js";
import { HorizontalScrollbar } from "../../../../base/browser/ui/scrollbar/horizontalScrollbar.js";
import { VerticalScrollbar } from "../../../../base/browser/ui/scrollbar/verticalScrollbar.js";
import { createScrollbarAxisMetrics, type ScrollbarAxisMetrics } from "../../../../base/browser/ui/scrollbar/scrollbarState.js";
import { type EditorScrollPosition, type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { type EditorViewPart } from "../viewPart.js";

export type EditorScrollbarVisibility = "auto" | "visible" | "hidden";

export interface EditorScrollbarPartOptions {
  readonly container: HTMLElement;
  readonly viewport: HTMLElement;
  readonly scrollTo: (position: EditorScrollPosition) => void;
  readonly scrollbarSize?: number;
  readonly minimumThumbSize?: number;
  readonly horizontal?: EditorScrollbarVisibility;
  readonly vertical?: EditorScrollbarVisibility;
}

/**
 * Projects the editor viewport's canonical scroll state into two themed
 * scrollbar axes. Native scrolling remains the editor's input and
 * accessibility fallback; this part owns only the visible custom tracks.
 */
export class EditorScrollbarPart extends DisposableOwner implements EditorViewPart {
  private static nextViewportId = 1;
  private readonly container: HTMLElement;
  private readonly horizontal: HorizontalScrollbar;
  private readonly vertical: VerticalScrollbar;
  private readonly horizontalTrackNode: FastDomNode<HTMLDivElement>;
  private readonly verticalTrackNode: FastDomNode<HTMLDivElement>;
  private readonly scrollbarSize: number;
  private readonly minimumThumbSize: number;
  private readonly horizontalVisibility: EditorScrollbarVisibility;
  private readonly verticalVisibility: EditorScrollbarVisibility;
  private horizontalMetrics: ScrollbarAxisMetrics;
  private verticalMetrics: ScrollbarAxisMetrics;
  private lastScrollPosition: EditorScrollPosition | undefined;
  private scrollActivityTimer: ReturnType<typeof setTimeout> | undefined;

  constructor(options: EditorScrollbarPartOptions) {
    super();
    if (!options.container || !options.viewport) {
      throw new TypeError("Editor scrollbar part requires a container and viewport");
    }
    if (typeof options.scrollTo !== "function") {
      throw new TypeError("Editor scrollbar part requires a scroll callback");
    }
    this.container = options.container;
    this.scrollbarSize = positiveFinite(options.scrollbarSize ?? 10, "scrollbarSize");
    this.minimumThumbSize = positiveFinite(options.minimumThumbSize ?? 20, "minimumThumbSize");
    this.horizontalVisibility = options.horizontal ?? "auto";
    this.verticalVisibility = options.vertical ?? "auto";
    if (!options.viewport.id) {
      options.viewport.id = `aster-editor-scroll-viewport-${EditorScrollbarPart.nextViewportId++}`;
    }
    options.container.style.setProperty("--aster-editor-scrollbar-size", `${this.scrollbarSize}px`);
    this.horizontalMetrics = createScrollbarAxisMetrics(0, 0, 0, 0, 0);
    this.verticalMetrics = createScrollbarAxisMetrics(0, 0, 0, 0, 0);
    this.horizontal = this.own(new HorizontalScrollbar(options.container, {
      viewport: options.viewport,
      trackClickBehavior: "jump",
      getMetrics: () => this.horizontalMetrics,
      setPosition: position => options.scrollTo({
        left: position,
        top: this.verticalMetrics.position,
      }),
    }));
    this.vertical = this.own(new VerticalScrollbar(options.container, {
      viewport: options.viewport,
      trackClickBehavior: "jump",
      getMetrics: () => this.verticalMetrics,
      setPosition: position => options.scrollTo({
        left: this.horizontalMetrics.position,
        top: position,
      }),
    }));
    this.horizontalTrackNode = new FastDomNode(this.horizontal.track);
    this.verticalTrackNode = new FastDomNode(this.vertical.track);
    this.configureTrack(this.horizontal.track, "horizontal", this.horizontalVisibility);
    this.configureTrack(this.vertical.track, "vertical", this.verticalVisibility);
    this.defer(() => {
      if (this.scrollActivityTimer !== undefined) clearTimeout(this.scrollActivityTimer);
      this.container.classList.remove("aster-editor-scrolling");
    });
  }

  render(layout: EditorViewportLayout): void {
    if (
      this.lastScrollPosition !== undefined &&
      (this.lastScrollPosition.left !== layout.scrollPosition.left ||
        this.lastScrollPosition.top !== layout.scrollPosition.top)
    ) {
      this.showScrollbars();
    }
    this.lastScrollPosition = layout.scrollPosition;
    const horizontalRendered = isRendered(
      this.horizontalVisibility,
      layout.maximumScrollPosition.left > 0,
    );
    const verticalRendered = isRendered(
      this.verticalVisibility,
      layout.maximumScrollPosition.top > 0,
    );
    const horizontalTrackSize = Math.max(
      0,
      layout.viewportSize.width - (verticalRendered ? this.scrollbarSize : 0),
    );
    const verticalTrackSize = Math.max(
      0,
      layout.viewportSize.height - (horizontalRendered ? this.scrollbarSize : 0),
    );
    this.horizontalTrackNode.setRight(verticalRendered ? this.scrollbarSize : 0);
    this.verticalTrackNode.setBottom(horizontalRendered ? this.scrollbarSize : 0);
    const scrollTransform = `translate3d(${layout.scrollPosition.left}px, ${layout.scrollPosition.top}px, 0)`;
    this.horizontalTrackNode.setTransform(scrollTransform);
    this.verticalTrackNode.setTransform(scrollTransform);
    this.horizontalMetrics = createScrollbarAxisMetrics(
      layout.viewportSize.width,
      layout.contentSize.width,
      layout.scrollPosition.left,
      horizontalTrackSize,
      this.minimumThumbSize,
    );
    this.verticalMetrics = createScrollbarAxisMetrics(
      layout.viewportSize.height,
      layout.contentSize.height,
      layout.scrollPosition.top,
      verticalTrackSize,
      this.minimumThumbSize,
    );
    this.horizontal.render(this.horizontalMetrics, horizontalRendered);
    this.vertical.render(this.verticalMetrics, verticalRendered);
  }

  private configureTrack(
    track: HTMLDivElement,
    axis: "horizontal" | "vertical",
    visibility: EditorScrollbarVisibility,
  ): void {
    track.classList.add("aster-editor-scrollbar-track", `aster-editor-scrollbar-track-${axis}`);
    track.dataset.visibility = visibility;
  }

  private showScrollbars(): void {
    this.container.classList.add("aster-editor-scrolling");
    if (this.scrollActivityTimer !== undefined) clearTimeout(this.scrollActivityTimer);
    this.scrollActivityTimer = setTimeout(() => {
      this.scrollActivityTimer = undefined;
      this.container.classList.remove("aster-editor-scrolling");
    }, 700);
  }
}

function isRendered(visibility: EditorScrollbarVisibility, needed: boolean): boolean {
  return visibility === "visible" || (visibility === "auto" && needed);
}

function positiveFinite(value: number, name: string): number {
  if (!Number.isFinite(value) || value <= 0) throw new RangeError(`${name} must be positive and finite`);
  return value;
}

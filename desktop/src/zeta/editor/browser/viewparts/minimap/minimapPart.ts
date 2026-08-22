import "./minimap.css";
import { addDisposableListener, h, reset, fragment as createFragment } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type EditorScrollPosition, type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { type DiagnosticOverviewMarker } from "../overviewRuler/diagnosticOverviewMarkers.js";
import { type DiffOverviewMarker } from "../overviewRuler/diffOverviewMarkers.js";
import { GpuMinimapRenderer } from "./gpuMinimapRenderer.js";
import { MinimapNavigationController } from "./minimapNavigationController.js";
import { MINIMAP_LINE_HEIGHT, MINIMAP_WIDTH, createMinimapSliderLayout, minimapContentWidth } from "./minimapPresentation.js";
import { createMinimapRows } from "./minimapProjection.js";
import { type EditorViewPart } from "../viewPart.js";

export type MinimapMarker = DiagnosticOverviewMarker | DiffOverviewMarker;

export interface MinimapPartOptions {
  readonly container: HTMLElement;
  readonly model: TextModel;
  readonly readLayout: () => EditorViewportLayout;
  readonly scrollTo: (position: EditorScrollPosition) => void;
  readonly readMarkers: () => readonly MinimapMarker[];
  readonly readMarkersRevision: () => number;
  readonly enabled: boolean;
}

/** Owns the minimap preview, its bounded density projection, and navigation. */
export class MinimapPart extends DisposableOwner implements EditorViewPart {
  readonly element: HTMLDivElement;
  private readonly canvas: HTMLCanvasElement;
  private readonly viewportElement: HTMLDivElement;
  private readonly model: TextModel;
  private readonly readLayout: () => EditorViewportLayout;
  private readonly readMarkers: () => readonly MinimapMarker[];
  private readonly readMarkersRevision: () => number;
  private readonly gpuRenderer: GpuMinimapRenderer | undefined;
  private renderedMarkersRevision = -1;

  constructor(options: MinimapPartOptions) {
    super();
    this.model = options.model;
    this.readLayout = options.readLayout;
    this.readMarkers = options.readMarkers;
    this.readMarkersRevision = options.readMarkersRevision;
    const ownerDocument = options.container.ownerDocument;
    this.element = h(ownerDocument, "div");
    this.canvas = h(ownerDocument, "canvas");
    this.viewportElement = h(ownerDocument, "div");
    this.element.className = "aster-editor-minimap";
    this.element.hidden = !options.enabled;
    this.element.setAttribute("aria-hidden", "true");
    this.canvas.className = "aster-editor-minimap-gpu";
    this.canvas.setAttribute("aria-hidden", "true");
    this.viewportElement.className = "aster-editor-minimap-viewport";
    this.element.append(this.canvas, this.viewportElement);
    options.container.append(this.element);
    this.gpuRenderer = options.enabled
      ? GpuMinimapRenderer.tryCreate(this.canvas)
      : undefined;
    this.own(new MinimapNavigationController(
      this.element,
      options.readLayout,
      options.scrollTo,
    ));
    this.own(addDisposableListener<globalThis.Event>(this.canvas, "webglcontextlost", event => {
      event.preventDefault();
      this.gpuRenderer?.disable();
      this.renderedMarkersRevision = -1;
      this.render(this.readLayout());
    }));
  }

  render(layout: EditorViewportLayout): void {
    if (this.element.hidden) return;
    this.element.style.transform = `translate3d(${layout.scrollPosition.left + Math.max(0, layout.viewportSize.width - MINIMAP_WIDTH)}px, ${layout.scrollPosition.top}px, 0)`;
    this.element.style.height = `${layout.viewportSize.height}px`;
    const slider = createMinimapSliderLayout(
      layout.viewportSize.height,
      layout.contentSize.height,
      layout.scrollPosition.top,
    );
    this.viewportElement.style.height = `${slider.height}px`;
    this.viewportElement.style.transform = `translate3d(0, ${slider.top}px, 0)`;
    this.gpuRenderer?.resize(MINIMAP_WIDTH, layout.viewportSize.height);
    const markersRevision = this.readMarkersRevision();
    if (this.renderedMarkersRevision === markersRevision) return;
    const fragment = createFragment(this.element.ownerDocument);
    const rows = createMinimapRows(this.model);
    if (this.gpuRenderer?.isAvailable) {
      this.gpuRenderer.setRows(rows, this.model.lineCount);
    } else {
      for (const row of rows) {
        const marker = h(this.element.ownerDocument, "span");
        marker.className = "aster-editor-minimap-row";
        marker.style.top = `${row.startLineIndex / this.model.lineCount * 100}%`;
        marker.style.width = `${minimapContentWidth(row.density)}px`;
        marker.style.height = `${MINIMAP_LINE_HEIGHT}px`;
        fragment.append(marker);
      }
    }
    for (const marker of this.readMarkers()) {
      const element = h(this.element.ownerDocument, "span");
      element.className = "aster-editor-minimap-diagnostic-marker";
      element.classList.add(marker.presentation);
      element.style.top = `${marker.startLineIndex / this.model.lineCount * 100}%`;
      element.style.height = `${Math.max(1, (marker.endLineIndexExclusive - marker.startLineIndex) / this.model.lineCount * 100)}%`;
      if (marker.hoverText !== undefined) element.title = marker.hoverText;
      fragment.append(element);
    }
    reset(this.element, this.canvas, fragment, this.viewportElement);
    this.renderedMarkersRevision = markersRevision;
  }
}

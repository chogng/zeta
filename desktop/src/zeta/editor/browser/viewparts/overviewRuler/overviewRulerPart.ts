import "./overviewRuler.css";
import { h, reset, fragment as createFragment } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { type DiagnosticOverviewMarker } from "./diagnosticOverviewMarkers.js";
import { type DiffOverviewMarker } from "./diffOverviewMarkers.js";
import { MINIMAP_WIDTH } from "../minimap/minimapPresentation.js";
import { type EditorViewPart } from "../viewPart.js";

const OVERVIEW_RULER_WIDTH = 6;

export type OverviewRulerMarker = DiagnosticOverviewMarker | DiffOverviewMarker;

export interface OverviewRulerPartOptions {
  readonly container: HTMLElement;
  readonly minimapEnabled: boolean;
  readonly readLineCount: () => number;
  readonly readMarkers: () => readonly OverviewRulerMarker[];
  readonly readMarkersRevision: () => number;
}

/** Projects diagnostic and diff markers into the editor's overview ruler. */
export class OverviewRulerPart extends DisposableOwner implements EditorViewPart {
  readonly element: HTMLDivElement;
  private readonly minimapEnabled: boolean;
  private readonly readLineCount: () => number;
  private readonly readMarkers: () => readonly OverviewRulerMarker[];
  private readonly readMarkersRevision: () => number;
  private renderedMarkersRevision = -1;

  constructor(options: OverviewRulerPartOptions) {
    super();
    this.minimapEnabled = options.minimapEnabled;
    this.readLineCount = options.readLineCount;
    this.readMarkers = options.readMarkers;
    this.readMarkersRevision = options.readMarkersRevision;
    this.element = h(options.container.ownerDocument, "div");
    this.element.className = "aster-editor-overview-ruler";
    this.element.setAttribute("aria-hidden", "true");
    options.container.append(this.element);
    this.defer(() => this.element.remove());
  }

  render(layout: EditorViewportLayout): void {
    const rightOffset = this.minimapEnabled ? MINIMAP_WIDTH + 4 : 0;
    this.element.style.left = `${layout.scrollPosition.left + Math.max(0, layout.viewportSize.width - OVERVIEW_RULER_WIDTH - rightOffset)}px`;
    this.element.style.top = `${layout.scrollPosition.top}px`;
    this.element.style.height = `${layout.viewportSize.height}px`;
    const markersRevision = this.readMarkersRevision();
    if (this.renderedMarkersRevision === markersRevision) return;
    const lineCount = Math.max(1, this.readLineCount());
    const markers = this.readMarkers();
    const fragment = createFragment(this.element.ownerDocument);
    for (const marker of markers) {
      const element = h(this.element.ownerDocument, "span");
      element.className = "aster-editor-overview-marker";
      element.classList.add(marker.presentation);
      element.style.top = `${marker.startLineIndex / lineCount * 100}%`;
      element.style.height = `${Math.max(1, (marker.endLineIndexExclusive - marker.startLineIndex) / lineCount * 100)}%`;
      if (marker.hoverText !== undefined) element.title = marker.hoverText;
      fragment.append(element);
    }
    reset(this.element, fragment);
    this.renderedMarkersRevision = markersRevision;
  }
}

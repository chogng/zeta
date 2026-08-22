import "./decorations.css";
import { type Event, Emitter } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { type DiagnosticOverviewMarker } from "../overviewRuler/diagnosticOverviewMarkers.js";
import { createAsterDiagnosticOverviewMarkers } from "../overviewRuler/diagnosticOverviewMarkers.js";
import { type DiffOverviewMarker } from "../overviewRuler/diffOverviewMarkers.js";
import { createAsterDiffOverviewMarkers } from "../overviewRuler/diffOverviewMarkers.js";
import { type DecorationSource, type ResolvedDecoration } from "../decorations/decorationPresentation.js";
import { type ViewportOverlayContext } from "../viewportOverlay/viewportOverlayPresentation.js";
import { projectAsterDecorationOverlays } from "./decorationProjection.js";
import { DecorationLineIndex } from "./decorationLineIndex.js";
import { type EditorViewPart } from "../viewPart.js";

export type DecorationsPartMarker = DiagnosticOverviewMarker | DiffOverviewMarker;

export interface DecorationsPartOptions {
  readonly model: TextModel;
  readonly decorationSources: readonly DecorationSource[];
  readonly readOverlayContext: (layout: EditorViewportLayout) => ViewportOverlayContext;
}

/** Owns decoration snapshots, visible-line lookup, inline DOM projection, and overview aggregation. */
export class DecorationsPart extends DisposableOwner implements EditorViewPart {
  private readonly model: TextModel;
  private readonly decorationSources: readonly DecorationSource[];
  private readonly readOverlayContext: (layout: EditorViewportLayout) => ViewportOverlayContext;
  private readonly decorationSnapshots = new Map<DecorationSource, readonly ResolvedDecoration[]>();
  private readonly changeEmitter = this.own(new Emitter<void>());
  private decorationLineIndex = new DecorationLineIndex([]);
  private markerRevision = 0;

  readonly onDidChange: Event<void> = this.changeEmitter.event;

  constructor(options: DecorationsPartOptions) {
    super();
    this.model = options.model;
    this.decorationSources = Object.freeze([...options.decorationSources]);
    this.readOverlayContext = options.readOverlayContext;
    this.own(this.model.onDidChange(() => {
      this.markerRevision += 1;
    }));
    for (const source of this.decorationSources) {
      this.decorationSnapshots.set(source, source.decorations);
      this.own(source.onDidChange(() => {
        this.decorationSnapshots.set(source, source.decorations);
        this.rebuildDecorationLineIndex();
        this.changeEmitter.fire();
      }));
    }
    this.rebuildDecorationLineIndex();
  }

  get markersRevision(): number {
    return this.markerRevision;
  }

  render(layout: EditorViewportLayout): void {
    const context = this.readOverlayContext(layout);
    if (context.visualLineProjection.modelVersion !== context.model.version) return;
    projectAsterDecorationOverlays(context, this.resolveVisibleDecorations(context));
  }

  /** Returns the current visible snapshot for another decoration-owned view part. */
  visibleDecorations(layout: EditorViewportLayout): readonly ResolvedDecoration[] {
    return this.resolveVisibleDecorations(this.readOverlayContext(layout));
  }

  overviewMarkers(): readonly DecorationsPartMarker[] {
    const decorations = this.decorationSources.flatMap(source => this.decorationSnapshots.get(source) ?? []);
    return Object.freeze([
      ...createAsterDiagnosticOverviewMarkers(decorations, this.model.lineCount),
      ...createAsterDiffOverviewMarkers(decorations, this.model.lineCount),
    ]);
  }

  private rebuildDecorationLineIndex(): void {
    this.decorationLineIndex = new DecorationLineIndex(this.decorationSources.flatMap(
      source => this.decorationSnapshots.get(source) ?? [],
    ));
    this.markerRevision += 1;
  }

  private resolveVisibleDecorations(context: ViewportOverlayContext): readonly ResolvedDecoration[] {
    const renderLines = context.renderLines;
    let minimumLogicalLineIndex = Number.POSITIVE_INFINITY;
    let maximumLogicalLineIndex = -1;
    for (let visualLineIndex = renderLines.startLineIndex; visualLineIndex < renderLines.endLineIndexExclusive; visualLineIndex += 1) {
      const visualLine = context.visualLineProjection.lineAt(visualLineIndex);
      if (!visualLine) continue;
      minimumLogicalLineIndex = Math.min(minimumLogicalLineIndex, visualLine.logicalLineIndex);
      maximumLogicalLineIndex = Math.max(maximumLogicalLineIndex, visualLine.logicalLineIndex);
    }
    return maximumLogicalLineIndex < 0
      ? []
      : this.decorationLineIndex.getIntersectingLines(minimumLogicalLineIndex, maximumLogicalLineIndex);
  }
}

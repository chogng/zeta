import "./decorations.css";
import { type Event, Emitter } from "../../../../base/common/event.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { type DiagnosticOverviewMarker } from "../overviewRuler/diagnosticOverviewMarkers.js";
import { createStanzaDiagnosticOverviewMarkers } from "../overviewRuler/diagnosticOverviewMarkers.js";
import { type DiffOverviewMarker } from "../overviewRuler/diffOverviewMarkers.js";
import { createStanzaDiffOverviewMarkers } from "../overviewRuler/diffOverviewMarkers.js";
import { type DecorationSource, type ResolvedDecoration } from "../decorations/decorationPresentation.js";
import { type ViewportOverlayContext } from "../viewportOverlay/viewportOverlayPresentation.js";
import { projectStanzaDecorationOverlays } from "./decorationProjection.js";
import { DecorationLineIndex } from "./decorationLineIndex.js";
import { EditorOverlayPart, EditorViewContext } from "../viewPart.js";

export type DecorationsPartMarker = DiagnosticOverviewMarker | DiffOverviewMarker;

/** Owns decoration snapshots, visible-line lookup, inline DOM projection, and overview aggregation. */
export class DecorationsPart extends EditorOverlayPart {
	private readonly model: TextModel;
	private readonly decorationSources: readonly DecorationSource[];
	private readonly decorationSnapshots = new Map<DecorationSource, readonly ResolvedDecoration[]>();
	private readonly changeEmitter = this.own(new Emitter<void>());
	private decorationLineIndex = new DecorationLineIndex([]);
	private markerRevision = 0;

	public readonly onDidChange: Event<void> = this.changeEmitter.event;

	constructor(context: EditorViewContext, model: TextModel, decorationSources: readonly DecorationSource[]) {
		super(context);
		this.model = model;
		this.decorationSources = Object.freeze([...decorationSources]);
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

	public get markersRevision(): number {
		return this.markerRevision;
	}

	public render(layout: EditorViewportLayout): void {
		const context = this.context.overlayContext(layout);
		if (!context) {
			return;
		}
		projectStanzaDecorationOverlays(context, this.resolveVisibleDecorations(context));
	}

	public visibleDecorations(context: ViewportOverlayContext): readonly ResolvedDecoration[] {
		return this.resolveVisibleDecorations(context);
	}

	public overviewMarkers(): readonly DecorationsPartMarker[] {
		const decorations = this.allDecorations().filter(decoration => decoration.overviewRuler !== false);
		return markersForDecorations(decorations, this.model.lineCount);
	}

	public minimapMarkers(): readonly DecorationsPartMarker[] {
		const decorations = this.allDecorations().filter(decoration => decoration.minimap !== false);
		return markersForDecorations(decorations, this.model.lineCount);
	}

	private allDecorations(): readonly ResolvedDecoration[] {
		return this.decorationSources.flatMap(source => this.decorationSnapshots.get(source) ?? []);
	}

	private rebuildDecorationLineIndex(): void {
		this.decorationLineIndex = new DecorationLineIndex(this.allDecorations());
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

function markersForDecorations(decorations: readonly ResolvedDecoration[], lineCount: number): readonly DecorationsPartMarker[] {
	return Object.freeze([
		...createStanzaDiagnosticOverviewMarkers(decorations, lineCount),
		...createStanzaDiffOverviewMarkers(decorations, lineCount),
	]);
}

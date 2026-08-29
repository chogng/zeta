import "./decorations.css";
import { type Event, Emitter } from "../../../../base/common/event.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type DiagnosticOverviewMarker } from "../overviewRuler/diagnosticOverviewMarkers.js";
import { createStanzaDiagnosticOverviewMarkers } from "../overviewRuler/diagnosticOverviewMarkers.js";
import { type DiffOverviewMarker } from "../overviewRuler/diffOverviewMarkers.js";
import { createStanzaDiffOverviewMarkers } from "../overviewRuler/diffOverviewMarkers.js";
import { type DecorationSource, type ResolvedDecoration } from "../decorations/decorationPresentation.js";
import { type EditorOverlayContext } from "../../view/renderingContext.js";
import { projectStanzaDecorationOverlays } from "./decorationProjection.js";
import { DecorationLineIndex } from "./decorationLineIndex.js";
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";
import { ViewPartRows } from '../../view/viewPartRows.js';

export type DecorationsPartMarker = DiagnosticOverviewMarker | DiffOverviewMarker;

/** Owns decoration snapshots, visible-line lookup, inline DOM projection, and overview aggregation. */
export class DecorationsPart extends DynamicViewOverlay {
	public readonly domNode: HTMLElement;
	private readonly model: TextModel;
	private readonly decorationSources: readonly DecorationSource[];
	private readonly decorationSnapshots = new Map<DecorationSource, readonly ResolvedDecoration[]>();
	private readonly changeEmitter = this._register(new Emitter<void>());
	private decorationLineIndex = new DecorationLineIndex([]);
	private decorationModelVersion: number;
	private markerRevision = 0;
	private readonly rows: ViewPartRows;

	public readonly onDidChange: Event<void> = this.changeEmitter.event;

	constructor(context: EditorViewContext, host: HTMLElement, model: TextModel, decorationSources: readonly DecorationSource[]) {
		super(context);
		this.rows = this._register(new ViewPartRows(host, 'stanza-editor-decorations-layer', 'stanza-editor-line-decorations'));
		this.domNode = this.rows.domNode;
		this.model = model;
		this.decorationSources = Object.freeze([...decorationSources]);
		for (const source of this.decorationSources) {
			this.decorationSnapshots.set(source, source.decorations);
			this._register(source.onDidChange(() => {
				this.decorationSnapshots.set(source, source.decorations);
				this.decorationModelVersion = this.model.version;
				this.rebuildDecorationLineIndex();
				this.changeEmitter.fire();
			}));
		}
		this.decorationModelVersion = this.model.version;
		this.rebuildDecorationLineIndex();
	}

	public get markersRevision(): number {
		this.synchronizeDecorationSnapshots();
		return this.markerRevision;
	}

	public render(context: EditorRenderingContext): void {
		this.synchronizeDecorationSnapshots();
		const overlay = context.overlay;
		if (!overlay) {
			return;
		}
		projectStanzaDecorationOverlays(overlay, this.resolveVisibleDecorations(overlay), this.rows.render(context));
	}

	public visibleDecorations(context: EditorOverlayContext): readonly ResolvedDecoration[] {
		this.synchronizeDecorationSnapshots();
		return this.resolveVisibleDecorations(context);
	}

	public overviewMarkers(): readonly DecorationsPartMarker[] {
		this.synchronizeDecorationSnapshots();
		const decorations = this.allDecorations().filter(decoration => decoration.overviewRuler !== false);
		return markersForDecorations(decorations, this.model.lineCount);
	}

	public minimapMarkers(): readonly DecorationsPartMarker[] {
		this.synchronizeDecorationSnapshots();
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

	private synchronizeDecorationSnapshots(): void {
		if (this.decorationModelVersion === this.model.version) return;
		for (const source of this.decorationSources) {
			this.decorationSnapshots.set(source, source.decorations);
		}
		this.decorationModelVersion = this.model.version;
		this.rebuildDecorationLineIndex();
	}

	private resolveVisibleDecorations(context: EditorOverlayContext): readonly ResolvedDecoration[] {
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

import "./decorations.css";
import { type Event, Emitter } from "../../../../base/common/event.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type DiagnosticOverviewMarker } from "../overviewRuler/diagnosticOverviewMarkers.js";
import { createStanzaDiagnosticOverviewMarkers } from "../overviewRuler/diagnosticOverviewMarkers.js";
import { type DiffOverviewMarker } from "../overviewRuler/diffOverviewMarkers.js";
import { createStanzaDiffOverviewMarkers } from "../overviewRuler/diffOverviewMarkers.js";
import { createStanzaVisualDecorationRectangles, type DecorationSource, type ResolvedDecoration } from "../decorations/decorationPresentation.js";
import { type EditorOverlayContext } from "../../view/renderingContext.js";
import { h, reset } from '../../../../base/browser/dom.js';
import { DecorationPresentation } from './decorationPresentation.js';
import { DecorationLineIndex } from "./decorationLineIndex.js";
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";
import { ViewPartRows } from '../../view/viewPartRows.js';

export type DecorationsOverlayMarker = DiagnosticOverviewMarker | DiffOverviewMarker;

/** Owns decoration snapshots, visible-line lookup, inline DOM projection, and overview aggregation. */
export class DecorationsOverlay extends DynamicViewOverlay {
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

	public overviewMarkers(): readonly DecorationsOverlayMarker[] {
		this.synchronizeDecorationSnapshots();
		const decorations = this.allDecorations().filter(decoration => decoration.overviewRuler !== false);
		return markersForDecorations(decorations, this.model.lineCount);
	}

	public minimapMarkers(): readonly DecorationsOverlayMarker[] {
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

function markersForDecorations(decorations: readonly ResolvedDecoration[], lineCount: number): readonly DecorationsOverlayMarker[] {
	return Object.freeze([
		...createStanzaDiagnosticOverviewMarkers(decorations, lineCount),
		...createStanzaDiffOverviewMarkers(decorations, lineCount),
	]);
}

function projectStanzaDecorationOverlays(context: EditorOverlayContext, decorations: readonly ResolvedDecoration[], rows: ReadonlyMap<number, HTMLElement>): void {
	const inlineDecorations = decorations.filter(decoration => (
		decoration.presentation !== DecorationPresentation.GlyphMargin
		&& decoration.presentation !== DecorationPresentation.LineDecoration
	));
	const rectangles = createStanzaVisualDecorationRectangles(context.model, inlineDecorations, context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
	const domRectangles = new Map(inlineDecorations.map(decoration => [decoration.id, context.linesVisibleRangesForRange(decoration.range, false)] as const));
	const decorationsById = new Map(inlineDecorations.map(decoration => [decoration.id, decoration] as const));
	for (const row of rows.values()) reset(row);
	const ownerDocument = context.ownerDocument;
	for (const rectangle of rectangles) {
		if (domRectangles.get(rectangle.id)) continue;
		const row = rows.get(rectangle.visualLineIndex);
		if (!row) continue;
		row.append(createDecorationElement(ownerDocument, decorationsById.get(rectangle.id)!, rectangle.left, rectangle.width));
	}
	for (const decoration of inlineDecorations) {
		const geometry = domRectangles.get(decoration.id);
		if (!geometry) continue;
		for (const rectangle of geometry) {
			const row = rows.get(rectangle.visualLineIndex);
			if (!row) continue;
			row.append(createDecorationElement(ownerDocument, decoration, rectangle.left, rectangle.width));
		}
	}
}

function createDecorationElement(ownerDocument: Document, decoration: ResolvedDecoration, left: number, width: number): HTMLElement {
	const element = h(ownerDocument, 'div');
	element.className = 'stanza-editor-decoration';
	element.classList.add(decoration.presentation);
	element.dataset.decorationId = String(decoration.id);
	if (decoration.hoverText !== undefined) element.title = decoration.hoverText;
	if (decoration.presentation === DecorationPresentation.ColorSwatch) {
		element.setAttribute('role', 'button');
		element.setAttribute('aria-label', decoration.hoverText ?? 'Edit color');
		element.tabIndex = -1;
		element.style.setProperty('--stanza-editor-color-swatch', decoration.color!);
		element.style.left = `${left - 14}px`;
		element.style.width = '10px';
	} else {
		element.style.left = `${left}px`;
		element.style.width = `${width}px`;
	}
	return element;
}

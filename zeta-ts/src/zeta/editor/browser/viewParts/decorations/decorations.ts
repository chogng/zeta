import "./decorations.css";
import { type Icon } from '../../../../base/common/icon.js';
import { type Event, Emitter } from "../../../../base/common/event.js";
import { type IDisposable } from '../../../../base/common/lifecycle.js';
import { type Range } from '../../../common/core/range.js';
import { type TextDecorationCollection, type TextDecorationId, type TextDecorationSnapshot } from '../../../common/model/decorationCollection.js';
import { type TextModel } from "../../../common/model/textModel.js";
import { GlyphMarginLane } from '../../../common/model.js';
import { EmptyRangeRendering, createStanzaRangeRectangles } from '../../../common/viewModel/rangeGeometry.js';
import { type EditorVisualLineProjection } from '../../../common/viewModel/modelLineProjection.js';
import { type TextMeasurer } from '../../../common/viewModel/textMeasurer.js';
import { createStanzaVisualRangeRectangles } from '../../../common/viewModel/visualRangeGeometry.js';
import { type EditorLineRange } from '../../../common/viewModel/editorViewportContracts.js';
import { type DiagnosticOverviewMarker, type DiffOverviewMarker } from "../overviewRuler/overviewRuler.js";
import { type RenderingContext } from "../../view/renderingContext.js";
import { h, reset } from '../../../../base/browser/dom.js';
import { DynamicViewOverlay } from '../../view/dynamicViewOverlay.js';
import { type ViewContext } from '../../../common/viewModel/viewContext.js';
import * as viewEvents from '../../../common/viewEvents.js';
import { renderViewPartRows } from '../../view/viewLayer.js';

export type DecorationsOverlayMarker = DiagnosticOverviewMarker | DiffOverviewMarker;

/** Owns decoration snapshots, visible-line lookup, inline DOM projection, and overview aggregation. */
export enum DecorationPresentation {
	SearchMatch = "search-match",
	WordHighlight = "word-highlight",
	WordHighlightStrong = "word-highlight-strong",
	WordHighlightText = "word-highlight-text",
	SelectionHighlight = "selection-highlight",
	SelectionAnchor = "selection-anchor",
	BracketMatch = "bracket-match",
	ErrorUnderline = "error-underline",
	WarningUnderline = "warning-underline",
	InformationUnderline = "information-underline",
	HintUnderline = "hint-underline",
	UnicodeHighlight = "unicode-highlight",
	UnusualLineTerminator = "unusual-line-terminator",
	DiffAdded = "diff-added",
	DiffModified = "diff-modified",
	DiffDeleted = "diff-deleted",
	ColorSwatch = "color-swatch",
	GlyphMargin = "glyph-margin",
	LineDecoration = "line-decoration",
}

export interface DecorationGlyphMarginPresentation {
	readonly owner: string;
	readonly lane: GlyphMarginLane;
	readonly icon?: Icon;
	readonly className?: string;
	readonly ariaLabel: string;
	readonly title?: string;
	readonly expanded?: boolean;
	readonly pressed?: boolean;
	readonly zIndex?: number;
}

export interface DecorationGlyphMarginLane {
	readonly owner: string;
	readonly lane: GlyphMarginLane;
}

export interface DecorationSourceOptions {
	readonly glyphMarginLanes?: readonly DecorationGlyphMarginLane[];
	readonly linesDecorationLanes?: readonly DecorationLinesLane[];
}

/** Describes an optional class projected into the editor's line-side decoration lane. */
export interface DecorationLinesPresentation {
	readonly owner: string;
	readonly className?: string;
	readonly firstLineClassName?: string;
	readonly tooltip?: string;
	readonly icon?: Icon;
	readonly ariaLabel?: string;
	readonly expanded?: boolean;
}

export interface DecorationLinesLane {
	readonly owner: string;
	readonly width: number;
}

/** Describes an optional block-level background or outline projected across visual rows. */
export interface DecorationBlockPresentation {
	readonly className: string;
	readonly isAfterEnd?: boolean;
	readonly doesNotCollapse?: boolean;
	readonly padding?: readonly [number, number, number, number];
}

/** Groups the visual details that a decoration source resolves for one model decoration. */
export interface DecorationPresentationResolution {
	readonly presentation: DecorationPresentation;
	readonly color?: string;
	readonly linesDecoration?: DecorationLinesPresentation;
	readonly blockDecoration?: DecorationBlockPresentation;
	readonly glyphMargin?: DecorationGlyphMarginPresentation;
	readonly overviewRuler?: boolean;
	readonly minimap?: boolean;
}

export interface ResolvedDecoration {
	readonly id: TextDecorationId;
	readonly range: Range;
	readonly presentation: DecorationPresentation;
	readonly hoverText?: string;
	readonly color?: string;
	readonly linesDecoration?: DecorationLinesPresentation;
	readonly blockDecoration?: DecorationBlockPresentation;
	readonly glyphMargin?: DecorationGlyphMarginPresentation;
	readonly overviewRuler?: boolean;
	readonly minimap?: boolean;
}

export interface DecorationSource {
	readonly onDidChange: Event<void>;
	readonly decorations: readonly ResolvedDecoration[];
	readonly glyphMarginLanes: readonly DecorationGlyphMarginLane[];
	readonly linesDecorationLanes: readonly DecorationLinesLane[];
}

/** A host-created decoration source whose lifetime transfers to one editor part. */
export interface OwnedDecorationSource extends DecorationSource, IDisposable {}

export interface DecorationRectangle {
	readonly id: TextDecorationId;
	readonly presentation: DecorationPresentation;
	readonly lineIndex: number;
	readonly left: number;
	readonly width: number;
	readonly hoverText?: string;
}

export interface VisualDecorationRectangle {
	readonly id: TextDecorationId;
	readonly presentation: DecorationPresentation;
	readonly visualLineIndex: number;
	readonly left: number;
	readonly width: number;
	readonly hoverText?: string;
}

/**
 * Adapts one caller-owned decoration collection to browser presentation.
 *
 * The adapter and its consumers observe the collection but do not own it.
 * Returning `undefined` from `resolvePresentation` omits a decoration from
 * this renderer without changing the common collection.
 */
export function createStanzaDecorationSource<TMetadata>(
	collection: TextDecorationCollection<TMetadata>,
	resolvePresentation: (
		decoration: TextDecorationSnapshot<TMetadata>,
	) => DecorationPresentation | DecorationPresentationResolution | undefined,
	resolveHoverText?: (decoration: TextDecorationSnapshot<TMetadata>) => string | undefined,
	options: DecorationSourceOptions = {},
): DecorationSource {
	const glyphMarginLanes = normalizeGlyphMarginLanes(options.glyphMarginLanes);
	const linesDecorationLanes = normalizeLinesDecorationLanes(options.linesDecorationLanes);
	const onDidChange: Event<void> = listener => {
		return collection.onDidChange(() => listener());
	};
	return Object.freeze({
		onDidChange,
		glyphMarginLanes,
		linesDecorationLanes,
		get decorations(): readonly ResolvedDecoration[] {
			const resolved: ResolvedDecoration[] = [];
			for (const decoration of collection.decorations) {
				const resolution = resolvePresentation(decoration);
				if (resolution === undefined) continue;
				const details = isDecorationPresentationResolution(resolution) ? resolution : undefined;
				const presentation = isDecorationPresentationResolution(resolution)
					? resolution.presentation
					: resolution;
				validatePresentation(presentation);
				const hoverText = resolveHoverText?.(decoration);
				if (hoverText !== undefined && (typeof hoverText !== "string" || hoverText.trim().length === 0)) {
					throw new TypeError("Stanza decoration hover text must be non-empty text");
				}
				const color = normalizeColor(details?.color, presentation);
				const linesDecoration = normalizeLinesPresentation(details?.linesDecoration, linesDecorationLanes);
				const blockDecoration = normalizeBlockPresentation(details?.blockDecoration);
				const glyphMargin = normalizeGlyphMarginPresentation(details?.glyphMargin, glyphMarginLanes);
				const overviewRuler = normalizeOptionalBoolean(details?.overviewRuler, "overview ruler visibility");
				const minimap = normalizeOptionalBoolean(details?.minimap, "minimap visibility");
				if (blockDecoration?.isAfterEnd && !decoration.range.isEmpty()) {
					throw new TypeError("Stanza block decoration isAfterEnd requires an empty range");
				}
				resolved.push(Object.freeze({
					id: decoration.id,
					range: decoration.range,
					presentation,
					...(hoverText === undefined ? {} : { hoverText }),
					...(color === undefined ? {} : { color }),
					...(linesDecoration === undefined ? {} : { linesDecoration }),
					...(blockDecoration === undefined ? {} : { blockDecoration }),
					...(glyphMargin === undefined ? {} : { glyphMargin }),
					...(overviewRuler === undefined ? {} : { overviewRuler }),
					...(minimap === undefined ? {} : { minimap }),
				}));
			}
			return Object.freeze(resolved);
		},
	});
}

/** @internal */
export function createStanzaDecorationRectangles(
	model: TextModel,
	decorations: readonly ResolvedDecoration[],
	renderLines: EditorLineRange,
	textLeft: number,
	measurer: TextMeasurer,
): readonly DecorationRectangle[] {
	return Object.freeze(createStanzaRangeRectangles(
		model,
		decorations.map(decoration => ({
			range: decoration.range,
			value: decoration,
		})),
		renderLines,
		textLeft,
		measurer,
		EmptyRangeRendering.RenderAsSpace,
	).map(rectangle => Object.freeze({
		id: rectangle.value.id,
		presentation: rectangle.value.presentation,
		lineIndex: rectangle.lineIndex,
		left: rectangle.left,
		width: rectangle.width,
		...(rectangle.value.hoverText === undefined ? {} : { hoverText: rectangle.value.hoverText }),
	})));
}

/** @internal */
export function createStanzaVisualDecorationRectangles(model: TextModel, decorations: readonly ResolvedDecoration[], projection: EditorVisualLineProjection, renderLines: EditorLineRange, textLeft: number, measurer: TextMeasurer): readonly VisualDecorationRectangle[] {
	return Object.freeze(createStanzaVisualRangeRectangles(
		model,
		decorations.map(decoration => ({
			range: decoration.range,
			value: decoration,
		})),
		projection,
		renderLines,
		textLeft,
		measurer,
		EmptyRangeRendering.RenderAsSpace,
	).map(rectangle => Object.freeze({
		id: rectangle.value.id,
		presentation: rectangle.value.presentation,
		visualLineIndex: rectangle.visualLineIndex,
		left: rectangle.left,
		width: rectangle.width,
		...(rectangle.value.hoverText === undefined ? {} : { hoverText: rectangle.value.hoverText }),
	})));
}

function validatePresentation(
	presentation: DecorationPresentation,
): void {
	if (
		presentation !== DecorationPresentation.SearchMatch &&
		presentation !== DecorationPresentation.WordHighlight &&
		presentation !== DecorationPresentation.WordHighlightStrong &&
		presentation !== DecorationPresentation.WordHighlightText &&
		presentation !== DecorationPresentation.SelectionHighlight &&
		presentation !== DecorationPresentation.SelectionAnchor &&
		presentation !== DecorationPresentation.BracketMatch &&
		presentation !== DecorationPresentation.ErrorUnderline &&
		presentation !== DecorationPresentation.WarningUnderline &&
		presentation !== DecorationPresentation.InformationUnderline &&
		presentation !== DecorationPresentation.HintUnderline
		&& presentation !== DecorationPresentation.UnicodeHighlight
		&& presentation !== DecorationPresentation.UnusualLineTerminator
		&& presentation !== DecorationPresentation.DiffAdded
		&& presentation !== DecorationPresentation.DiffModified
		&& presentation !== DecorationPresentation.DiffDeleted
		&& presentation !== DecorationPresentation.ColorSwatch
		&& presentation !== DecorationPresentation.GlyphMargin
		&& presentation !== DecorationPresentation.LineDecoration
	) {
		throw new TypeError(`Unknown Stanza decoration presentation '${presentation}'`);
	}
}

function isDecorationPresentationResolution(
	value: DecorationPresentation | DecorationPresentationResolution,
): value is DecorationPresentationResolution {
	return typeof value === "object" && value !== null;
}

function normalizeLinesPresentation(presentation: DecorationLinesPresentation | undefined, lanes: readonly DecorationLinesLane[]): DecorationLinesPresentation | undefined {
	if (presentation === undefined) return undefined;
	if (!presentation || typeof presentation !== "object") {
		throw new TypeError("Stanza lines decoration presentation must be an object");
	}
	const owner = normalizeOptionalText(presentation.owner, "lines decoration owner");
	if (!owner) throw new TypeError("Stanza lines decoration owner must be non-empty text");
	if (!lanes.some(lane => lane.owner === owner)) throw new RangeError(`Lines decoration owner '${owner}' did not declare a lane`);
	const className = normalizeClassName(presentation.className, "lines decoration className");
	const firstLineClassName = normalizeClassName(presentation.firstLineClassName, "first-line decoration className");
	if (presentation.icon !== undefined && (typeof presentation.icon.id !== "string" || presentation.icon.id.length === 0)) throw new TypeError("Stanza lines decoration icon is invalid");
	if (className === undefined && firstLineClassName === undefined && presentation.icon === undefined) throw new TypeError("Stanza lines decoration presentation must provide a className or icon");
	const tooltip = normalizeOptionalText(presentation.tooltip, "lines decoration tooltip");
	const ariaLabel = normalizeOptionalText(presentation.ariaLabel, "lines decoration aria label");
	if (presentation.icon !== undefined && !ariaLabel) throw new TypeError("Stanza interactive lines decoration must provide an aria label");
	const expanded = normalizeOptionalBoolean(presentation.expanded, "lines decoration expanded state");
	return Object.freeze({
		owner,
		...(className === undefined ? {} : { className }),
		...(firstLineClassName === undefined ? {} : { firstLineClassName }),
		...(tooltip === undefined ? {} : { tooltip }),
		...(presentation.icon === undefined ? {} : { icon: presentation.icon }),
		...(ariaLabel === undefined ? {} : { ariaLabel }),
		...(expanded === undefined ? {} : { expanded }),
	});
}

function normalizeColor(value: string | undefined, presentation: DecorationPresentation): string | undefined {
	if (value === undefined) {
		if (presentation === DecorationPresentation.ColorSwatch) throw new TypeError("Stanza color-swatch decoration requires a color");
		return undefined;
	}
	if (presentation !== DecorationPresentation.ColorSwatch) throw new TypeError("Stanza decoration color is only valid for color swatches");
	if (!/^#[0-9a-f]{8}$/iu.test(value)) throw new TypeError("Stanza decoration color must be an eight-digit hexadecimal color");
	return value.toLowerCase();
}

function normalizeBlockPresentation(
	presentation: DecorationBlockPresentation | undefined,
): DecorationBlockPresentation | undefined {
	if (presentation === undefined) return undefined;
	if (!presentation || typeof presentation !== "object") {
		throw new TypeError("Stanza block decoration presentation must be an object");
	}
	const className = normalizeClassName(presentation.className, "block decoration className");
	if (className === undefined) throw new TypeError("Stanza block decoration presentation must provide a className");
	if (presentation.isAfterEnd !== undefined && typeof presentation.isAfterEnd !== "boolean") {
		throw new TypeError("Stanza block decoration isAfterEnd must be a boolean");
	}
	if (presentation.doesNotCollapse !== undefined && typeof presentation.doesNotCollapse !== "boolean") {
		throw new TypeError("Stanza block decoration doesNotCollapse must be a boolean");
	}
	const padding = normalizePadding(presentation.padding);
	return Object.freeze({
		className,
		...(presentation.isAfterEnd === undefined ? {} : { isAfterEnd: presentation.isAfterEnd }),
		...(presentation.doesNotCollapse === undefined ? {} : { doesNotCollapse: presentation.doesNotCollapse }),
		...(padding === undefined ? {} : { padding }),
	});
}

function normalizeClassName(value: string | undefined, name: string): string | undefined {
	if (value === undefined) return undefined;
	if (typeof value !== "string" || value.trim().length === 0 || !/^\S+(?:\s+\S+)*$/u.test(value.trim())) {
		throw new TypeError(`Stanza ${name} must be a non-empty CSS class list`);
	}
	return value.trim();
}

function normalizeOptionalText(value: string | undefined, name: string): string | undefined {
	if (value === undefined) return undefined;
	if (typeof value !== "string" || value.trim().length === 0) {
		throw new TypeError(`Stanza ${name} must be non-empty text`);
	}
	return value;
}

function normalizeGlyphMarginLanes(lanes: readonly DecorationGlyphMarginLane[] | undefined): readonly DecorationGlyphMarginLane[] {
	if (lanes === undefined) return Object.freeze([]);
	if (!Array.isArray(lanes)) throw new TypeError("Stanza glyph margin lanes must be an array");
	const seenOwners = new Set<string>();
	return Object.freeze(lanes.map(definition => {
		if (!definition || typeof definition !== "object") throw new TypeError("Stanza glyph margin lane must be an object");
		const owner = normalizeOptionalText(definition.owner, "glyph margin owner");
		if (!owner) throw new TypeError("Stanza glyph margin owner must be non-empty text");
		if (seenOwners.has(owner)) throw new RangeError(`Duplicate glyph margin owner '${owner}'`);
		seenOwners.add(owner);
		validateGlyphMarginLane(definition.lane);
		return Object.freeze({ owner, lane: definition.lane });
	}));
}

function normalizeLinesDecorationLanes(lanes: readonly DecorationLinesLane[] | undefined): readonly DecorationLinesLane[] {
	if (lanes === undefined) return Object.freeze([]);
	if (!Array.isArray(lanes)) throw new TypeError("Stanza lines decoration lanes must be an array");
	const seenOwners = new Set<string>();
	return Object.freeze(lanes.map(definition => {
		if (!definition || typeof definition !== "object") throw new TypeError("Stanza lines decoration lane must be an object");
		const owner = normalizeOptionalText(definition.owner, "lines decoration owner");
		if (!owner) throw new TypeError("Stanza lines decoration owner must be non-empty text");
		if (seenOwners.has(owner)) throw new RangeError(`Duplicate lines decoration owner '${owner}'`);
		if (!Number.isSafeInteger(definition.width) || definition.width <= 0) throw new RangeError("Stanza lines decoration lane width must be a positive safe integer");
		seenOwners.add(owner);
		return Object.freeze({ owner, width: definition.width });
	}));
}

function normalizeGlyphMarginPresentation(presentation: DecorationGlyphMarginPresentation | undefined, lanes: readonly DecorationGlyphMarginLane[]): DecorationGlyphMarginPresentation | undefined {
	if (presentation === undefined) return undefined;
	if (!presentation || typeof presentation !== "object") throw new TypeError("Stanza glyph margin presentation must be an object");
	const owner = normalizeOptionalText(presentation.owner, "glyph margin owner");
	if (!owner) throw new TypeError("Stanza glyph margin owner must be non-empty text");
	validateGlyphMarginLane(presentation.lane);
	if (!lanes.some(definition => definition.owner === owner && definition.lane === presentation.lane)) {
		throw new RangeError(`Glyph margin owner '${owner}' did not declare lane '${presentation.lane}'`);
	}
	if (presentation.icon !== undefined && (typeof presentation.icon.id !== "string" || presentation.icon.id.length === 0)) throw new TypeError("Stanza glyph margin icon is invalid");
	const className = normalizeClassName(presentation.className, "glyph margin className");
	const ariaLabel = normalizeOptionalText(presentation.ariaLabel, "glyph margin aria label");
	if (!ariaLabel) throw new TypeError("Stanza glyph margin aria label must be non-empty text");
	const title = normalizeOptionalText(presentation.title, "glyph margin title");
	const expanded = normalizeOptionalBoolean(presentation.expanded, "glyph margin expanded state");
	const pressed = normalizeOptionalBoolean(presentation.pressed, "glyph margin pressed state");
	if (presentation.zIndex !== undefined && !Number.isSafeInteger(presentation.zIndex)) throw new RangeError("Stanza glyph margin zIndex must be a safe integer");
	return Object.freeze({
		owner,
		lane: presentation.lane,
		...(presentation.icon === undefined ? {} : { icon: presentation.icon }),
		...(className === undefined ? {} : { className }),
		ariaLabel,
		...(title === undefined ? {} : { title }),
		...(expanded === undefined ? {} : { expanded }),
		...(pressed === undefined ? {} : { pressed }),
		...(presentation.zIndex === undefined ? {} : { zIndex: presentation.zIndex }),
	});
}

function validateGlyphMarginLane(lane: GlyphMarginLane): void {
	if (lane !== GlyphMarginLane.Left && lane !== GlyphMarginLane.Center && lane !== GlyphMarginLane.Right) throw new TypeError(`Unknown Stanza glyph margin lane '${lane}'`);
}

function normalizeOptionalBoolean(value: boolean | undefined, name: string): boolean | undefined {
	if (value === undefined) return undefined;
	if (typeof value !== "boolean") throw new TypeError(`Stanza ${name} must be a boolean`);
	return value;
}

function normalizePadding(
	padding: readonly [number, number, number, number] | undefined,
): readonly [number, number, number, number] | undefined {
	if (padding === undefined) return undefined;
	if (!Array.isArray(padding) || padding.length !== 4 || padding.some(value => !Number.isFinite(value) || value < 0)) {
		throw new TypeError("Stanza block decoration padding must contain four non-negative finite numbers");
	}
	return Object.freeze([padding[0]!, padding[1]!, padding[2]!, padding[3]!]);
}

interface DecorationInterval {
	readonly decoration: ResolvedDecoration;
	readonly startLineIndex: number;
	readonly endLineIndex: number;
	readonly order: number;
}

interface DecorationIntervalNode {
	readonly interval: DecorationInterval;
	readonly maximumEndLineIndex: number;
	readonly left: DecorationIntervalNode | undefined;
	readonly right: DecorationIntervalNode | undefined;
}

/**
 * Immutable interval index owned by the decorations view part.
 *
 * The index stores only the latest presentation snapshot. Decoration sources,
 * source invalidation, and DOM projection remain owned by DecorationsOverlay.
 */
export class DecorationLineIndex {
	private readonly root: DecorationIntervalNode | undefined;

	constructor(decorations: readonly ResolvedDecoration[]) {
		this.root = buildIntervalTree(decorations.map((decoration, order) => Object.freeze({
			decoration,
			startLineIndex: decoration.range.startLineNumber - 1,
			endLineIndex: lastCoveredLineIndex(decoration),
			order,
		})).sort(compareIntervals));
	}

	/** Returns decorations that can produce geometry on the inclusive line span. */
	getIntersectingLines(startLineIndex: number, endLineIndex: number): readonly ResolvedDecoration[] {
		if (!Number.isSafeInteger(startLineIndex) || !Number.isSafeInteger(endLineIndex) || startLineIndex < 0 || endLineIndex < startLineIndex) {
			throw new RangeError("Stanza decoration line queries require a non-negative ordered integer span");
		}
		const intervals: DecorationInterval[] = [];
		collectIntersecting(this.root, startLineIndex, endLineIndex, intervals);
		intervals.sort((left, right) => left.order - right.order);
		return Object.freeze(intervals.map(interval => interval.decoration));
	}
}

function buildIntervalTree(intervals: readonly DecorationInterval[]): DecorationIntervalNode | undefined {
	if (intervals.length === 0) return undefined;
	const middle = Math.floor(intervals.length / 2);
	const interval = intervals[middle]!;
	const left = buildIntervalTree(intervals.slice(0, middle));
	const right = buildIntervalTree(intervals.slice(middle + 1));
	return Object.freeze({
		interval,
		maximumEndLineIndex: Math.max(interval.endLineIndex, left?.maximumEndLineIndex ?? -1, right?.maximumEndLineIndex ?? -1),
		left,
		right,
	});
}

function collectIntersecting(node: DecorationIntervalNode | undefined, startLineIndex: number, endLineIndex: number, result: DecorationInterval[]): void {
	if (!node || node.maximumEndLineIndex < startLineIndex) return;
	if (node.interval.startLineIndex > endLineIndex) {
		collectIntersecting(node.left, startLineIndex, endLineIndex, result);
		return;
	}
	collectIntersecting(node.left, startLineIndex, endLineIndex, result);
	if (node.interval.endLineIndex >= startLineIndex) result.push(node.interval);
	collectIntersecting(node.right, startLineIndex, endLineIndex, result);
}

function compareIntervals(left: DecorationInterval, right: DecorationInterval): number {
	return left.startLineIndex - right.startLineIndex || left.endLineIndex - right.endLineIndex || left.order - right.order;
}

function lastCoveredLineIndex(decoration: ResolvedDecoration): number {
	const { startLineNumber, endLineNumber, endColumn } = decoration.range;
	if (decoration.range.isEmpty() || endColumn > 1 || endLineNumber === startLineNumber) return endLineNumber - 1;
	return endLineNumber - 2;
}

export class DecorationsOverlay extends DynamicViewOverlay {
	private _renderResult: string[] = [];
	private readonly model: TextModel;
	private readonly decorationSources: readonly DecorationSource[];
	private readonly decorationSnapshots = new Map<DecorationSource, readonly ResolvedDecoration[]>();
	private readonly changeEmitter = this._register(new Emitter<void>());
	private decorationLineIndex = new DecorationLineIndex([]);
	private markerRevision = 0;
	private readonly ownerDocument: Document;
	private readonly readVisualProjection: () => EditorVisualLineProjection;
	private readonly readTextLeft: () => number;
	private readonly textMeasurer: TextMeasurer;

	public readonly onDidChange: Event<void> = this.changeEmitter.event;

	constructor(private readonly _context: ViewContext, model: TextModel, decorationSources: readonly DecorationSource[], ownerDocument: Document, readVisualProjection: () => EditorVisualLineProjection, readTextLeft: () => number, textMeasurer: TextMeasurer) {
		super();
		this._context.addEventHandler(this);
		this.model = model;
		this.decorationSources = Object.freeze([...decorationSources]);
		this.ownerDocument = ownerDocument;
		this.readVisualProjection = readVisualProjection;
		this.readTextLeft = readTextLeft;
		this.textMeasurer = textMeasurer;
		for (const source of this.decorationSources) {
			this.decorationSnapshots.set(source, source.decorations);
			this._register(source.onDidChange(() => {
				this.decorationSnapshots.set(source, source.decorations);
				this.rebuildDecorationLineIndex();
				this.forceShouldRender();
				this.changeEmitter.fire();
			}));
		}
		this.rebuildDecorationLineIndex();
	}

	public override dispose(): void {
		this._context.removeEventHandler(this);
		super.dispose();
	}

	public override onConfigurationChanged(_event: viewEvents.ViewConfigurationChangedEvent): boolean { return true; }
	public override onDecorationsChanged(_event: viewEvents.ViewDecorationsChangedEvent): boolean { return true; }
	public override onFlushed(_event: viewEvents.ViewFlushedEvent): boolean { return true; }
	public override onLinesChanged(_event: viewEvents.ViewLinesChangedEvent): boolean { return true; }
	public override onLinesDeleted(_event: viewEvents.ViewLinesDeletedEvent): boolean { return true; }
	public override onLinesInserted(_event: viewEvents.ViewLinesInsertedEvent): boolean { return true; }
	public override onScrollChanged(_event: viewEvents.ViewScrollChangedEvent): boolean { return true; }
	public override onZonesChanged(_event: viewEvents.ViewZonesChangedEvent): boolean { return true; }

	public get markersRevision(): number {
		return this.markerRevision;
	}

	public prepareRender(context: RenderingContext): void {
		this._renderResult = renderViewPartRows(context, this.ownerDocument, rows => {
			projectStanzaDecorationOverlays(
				context,
				this.model,
				this.readVisualProjection(),
				this.readTextLeft(),
				this.textMeasurer,
				this.resolveVisibleDecorations(context),
				rows,
			);
		});
	}

	public render(startLineNumber: number, lineNumber: number): string {
		return this._renderResult[lineNumber - startLineNumber] ?? '';
	}

	public visibleDecorations(context: RenderingContext): readonly ResolvedDecoration[] {
		return this.resolveVisibleDecorations(context);
	}

	public overviewMarkers(): readonly DecorationsOverlayMarker[] {
		const decorations = this.allDecorations().filter(decoration => decoration.overviewRuler !== false);
		return markersForDecorations(decorations, this.model.lineCount);
	}

	public minimapMarkers(): readonly DecorationsOverlayMarker[] {
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

	private resolveVisibleDecorations(context: RenderingContext): readonly ResolvedDecoration[] {
		const projection = this.readVisualProjection();
		const renderLines = { startLineIndex: context.viewportData.startLineNumber - 1, endLineIndexExclusive: context.viewportData.endLineNumber };
		let minimumLogicalLineIndex = Number.POSITIVE_INFINITY;
		let maximumLogicalLineIndex = -1;
		for (let visualLineIndex = renderLines.startLineIndex; visualLineIndex < renderLines.endLineIndexExclusive; visualLineIndex += 1) {
			const visualLine = projection.lineAt(visualLineIndex);
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

const DIAGNOSTIC_PRESENTATION_PRIORITY = new Map<DecorationPresentation, number>([
	[DecorationPresentation.ErrorUnderline, 4],
	[DecorationPresentation.WarningUnderline, 3],
	[DecorationPresentation.InformationUnderline, 2],
	[DecorationPresentation.HintUnderline, 1],
]);

export function createStanzaDiagnosticOverviewMarkers(decorations: readonly ResolvedDecoration[], lineCount: number): readonly DiagnosticOverviewMarker[] {
	if (!Number.isSafeInteger(lineCount) || lineCount < 1) throw new RangeError('Diagnostic overview requires a positive line count');
	const byLine = new Map<number, ResolvedDecoration[]>();
	for (const decoration of decorations) {
		if (!DIAGNOSTIC_PRESENTATION_PRIORITY.has(decoration.presentation)) continue;
		const startLineIndex = decoration.range.startLineNumber - 1;
		const endLineIndex = decoration.range.endColumn === 1 && decoration.range.endLineNumber - 1 > startLineIndex ? decoration.range.endLineNumber - 2 : decoration.range.endLineNumber - 1;
		for (let lineIndex = startLineIndex; lineIndex <= endLineIndex; lineIndex += 1) {
			const line = byLine.get(lineIndex) ?? [];
			line.push(decoration);
			byLine.set(lineIndex, line);
		}
	}
	const markers: DiagnosticOverviewMarker[] = [];
	for (const lineIndex of [...byLine.keys()].sort((left, right) => left - right)) {
		const lineDecorations = byLine.get(lineIndex)!;
		const highest = lineDecorations.reduce((current, candidate) =>
			(DIAGNOSTIC_PRESENTATION_PRIORITY.get(candidate.presentation) ?? 0) > (DIAGNOSTIC_PRESENTATION_PRIORITY.get(current.presentation) ?? 0) ? candidate : current);
		const hoverText = uniqueHoverText(lineDecorations);
		const previous = markers.at(-1);
		if (previous && previous.endLineIndexExclusive === lineIndex && previous.presentation === highest.presentation && previous.hoverText === hoverText) {
			markers[markers.length - 1] = Object.freeze({ ...previous, endLineIndexExclusive: lineIndex + 1 });
			continue;
		}
		markers.push(Object.freeze({ startLineIndex: lineIndex, endLineIndexExclusive: lineIndex + 1, presentation: highest.presentation, hoverText }));
	}
	return Object.freeze(markers);
}

const DIFF_PRESENTATIONS = new Set<DecorationPresentation>([
	DecorationPresentation.DiffAdded,
	DecorationPresentation.DiffModified,
	DecorationPresentation.DiffDeleted,
]);

function createStanzaDiffOverviewMarkers(decorations: readonly ResolvedDecoration[], lineCount: number): readonly DiffOverviewMarker[] {
	if (!Number.isSafeInteger(lineCount) || lineCount < 1) throw new RangeError('Diff overview requires a positive line count');
	const markers: DiffOverviewMarker[] = [];
	for (const decoration of decorations) {
		if (!DIFF_PRESENTATIONS.has(decoration.presentation)) continue;
		const presentation = decoration.presentation as DiffOverviewMarker['presentation'];
		const lineIndex = decoration.range.startLineNumber - 1;
		const previous = markers.at(-1);
		if (previous && previous.endLineIndexExclusive === lineIndex && previous.presentation === presentation && previous.hoverText === decoration.hoverText) {
			markers[markers.length - 1] = Object.freeze({ ...previous, endLineIndexExclusive: lineIndex + 1 });
			continue;
		}
		markers.push(Object.freeze({ startLineIndex: lineIndex, endLineIndexExclusive: lineIndex + 1, presentation, hoverText: decoration.hoverText }));
	}
	return Object.freeze(markers);
}

function uniqueHoverText(decorations: readonly ResolvedDecoration[]): string | undefined {
	const values = [...new Set(decorations.flatMap(decoration => decoration.hoverText === undefined ? [] : [decoration.hoverText]))];
	return values.length > 0 ? values.join('\n') : undefined;
}

function projectStanzaDecorationOverlays(context: RenderingContext, model: TextModel, projection: EditorVisualLineProjection, textLeft: number, textMeasurer: TextMeasurer, decorations: readonly ResolvedDecoration[], rows: ReadonlyMap<number, HTMLElement>): void {
	const inlineDecorations = decorations.filter(decoration => (
		decoration.presentation !== DecorationPresentation.GlyphMargin
		&& decoration.presentation !== DecorationPresentation.LineDecoration
	));
	const renderLines = { startLineIndex: context.viewportData.startLineNumber - 1, endLineIndexExclusive: context.viewportData.endLineNumber };
	const rectangles = createStanzaVisualDecorationRectangles(model, inlineDecorations, projection, renderLines, textLeft, textMeasurer);
	const domRectangles = new Map(inlineDecorations.map(decoration => [decoration.id, context.linesVisibleRangesForRange(decoration.range, false)] as const));
	const decorationsById = new Map(inlineDecorations.map(decoration => [decoration.id, decoration] as const));
	for (const row of rows.values()) reset(row);
	const ownerDocument = rows.values().next().value?.ownerDocument;
	if (!ownerDocument) return;
	for (const rectangle of rectangles) {
		if (domRectangles.get(rectangle.id)) continue;
		const row = rows.get(rectangle.visualLineIndex);
		if (!row) continue;
		row.append(createDecorationElement(ownerDocument, decorationsById.get(rectangle.id)!, rectangle.left, rectangle.width));
	}
	for (const decoration of inlineDecorations) {
		const geometry = domRectangles.get(decoration.id);
		if (!geometry) continue;
		for (const line of geometry) {
			const row = rows.get(line.lineNumber - 1);
			if (!row) continue;
			for (const range of line.ranges) row.append(createDecorationElement(ownerDocument, decoration, range.left, range.width));
		}
	}
}

function createDecorationElement(ownerDocument: Document, decoration: ResolvedDecoration, left: number, width: number): HTMLElement {
	const element = h(ownerDocument, 'div');
	element.className = 'cdr';
	element.classList.add(decoration.presentation);
	element.classList.add('stanza-editor-decoration');
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

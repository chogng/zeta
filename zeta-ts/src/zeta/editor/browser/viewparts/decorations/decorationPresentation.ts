import { type Event } from "../../../../base/common/event.js";
import { type IDisposable } from "../../../../base/common/lifecycle.js";
import { type Icon } from "../../../../base/common/icon.js";
import { type TextDecorationCollection, type TextDecorationId, type TextDecorationSnapshot } from "../../../common/model/decorationCollection.js";
import { type TextRange } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type EditorVisualLineProjection } from "../../../common/viewModel/modelLineProjection.js";
import { type TextMeasurer } from "../../../common/viewModel/textMeasurer.js";
import { EmptyRangeRendering, createStanzaRangeRectangles } from "../../../common/viewModel/rangeGeometry.js";
import { createStanzaVisualRangeRectangles } from "../../../common/viewModel/visualRangeGeometry.js";
import { type EditorLineRange } from "../../../common/viewModel.js";

export enum DecorationPresentation {
	SearchMatch = "search-match",
	OccurrenceHighlight = "occurrence-highlight",
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
	GlyphMargin = "glyph-margin",
	LineDecoration = "line-decoration",
}

export enum GlyphMarginLane {
	Left = "left",
	Center = "center",
	Right = "right",
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
	readonly linesDecoration?: DecorationLinesPresentation;
	readonly blockDecoration?: DecorationBlockPresentation;
	readonly glyphMargin?: DecorationGlyphMarginPresentation;
	readonly overviewRuler?: boolean;
	readonly minimap?: boolean;
}

export interface ResolvedDecoration {
	readonly id: TextDecorationId;
	readonly range: TextRange;
	readonly presentation: DecorationPresentation;
	readonly hoverText?: string;
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
				const linesDecoration = normalizeLinesPresentation(details?.linesDecoration, linesDecorationLanes);
				const blockDecoration = normalizeBlockPresentation(details?.blockDecoration);
				const glyphMargin = normalizeGlyphMarginPresentation(details?.glyphMargin, glyphMarginLanes);
				const overviewRuler = normalizeOptionalBoolean(details?.overviewRuler, "overview ruler visibility");
				const minimap = normalizeOptionalBoolean(details?.minimap, "minimap visibility");
				if (blockDecoration?.isAfterEnd && !decoration.range.empty) {
					throw new TypeError("Stanza block decoration isAfterEnd requires an empty range");
				}
				resolved.push(Object.freeze({
					id: decoration.id,
					range: decoration.range,
					presentation,
					...(hoverText === undefined ? {} : { hoverText }),
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
		presentation !== DecorationPresentation.OccurrenceHighlight &&
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

import { type Event } from "../../../../base/common/event.js";
import { type IDisposable } from "../../../../base/common/lifecycle.js";
import { type TextDecorationCollection, type TextDecorationId, type TextDecorationSnapshot } from "../../../common/model/decorationCollection.js";
import { type TextRange } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type EditorVisualLineProjection } from "../../../common/viewModel/modelLineProjection.js";
import { type TextMeasurer } from "../../../common/viewModel/textMeasurer.js";
import { EmptyRangeRendering, createStanzaRangeRectangles } from "../../../common/viewModel/rangeGeometry.js";
import { createStanzaVisualRangeRectangles } from "../../../common/viewModel/visualRangeGeometry.js";
import { type EditorLineRange } from "../../../common/viewLayout/editorViewportModel.js";

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
}

/** Describes an optional class projected into the editor's line-side decoration lane. */
export interface DecorationLinesPresentation {
	readonly className?: string;
	readonly firstLineClassName?: string;
	readonly tooltip?: string;
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
}

export interface ResolvedDecoration {
	readonly id: TextDecorationId;
	readonly range: TextRange;
	readonly presentation: DecorationPresentation;
	readonly hoverText?: string;
	readonly linesDecoration?: DecorationLinesPresentation;
	readonly blockDecoration?: DecorationBlockPresentation;
}

export interface DecorationSource {
	readonly onDidChange: Event<void>;
	readonly decorations: readonly ResolvedDecoration[];
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
): DecorationSource {
	const onDidChange: Event<void> = listener => {
		return collection.onDidChange(() => listener());
	};
	return Object.freeze({
		onDidChange,
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
				const linesDecoration = normalizeLinesPresentation(details?.linesDecoration);
				const blockDecoration = normalizeBlockPresentation(details?.blockDecoration);
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
	) {
		throw new TypeError(`Unknown Stanza decoration presentation '${presentation}'`);
	}
}

function isDecorationPresentationResolution(
	value: DecorationPresentation | DecorationPresentationResolution,
): value is DecorationPresentationResolution {
	return typeof value === "object" && value !== null;
}

function normalizeLinesPresentation(
	presentation: DecorationLinesPresentation | undefined,
): DecorationLinesPresentation | undefined {
	if (presentation === undefined) return undefined;
	if (!presentation || typeof presentation !== "object") {
		throw new TypeError("Stanza lines decoration presentation must be an object");
	}
	const className = normalizeClassName(presentation.className, "lines decoration className");
	const firstLineClassName = normalizeClassName(presentation.firstLineClassName, "first-line decoration className");
	if (className === undefined && firstLineClassName === undefined) {
		throw new TypeError("Stanza lines decoration presentation must provide a className");
	}
	const tooltip = normalizeOptionalText(presentation.tooltip, "lines decoration tooltip");
	return Object.freeze({
		...(className === undefined ? {} : { className }),
		...(firstLineClassName === undefined ? {} : { firstLineClassName }),
		...(tooltip === undefined ? {} : { tooltip }),
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

function normalizePadding(
	padding: readonly [number, number, number, number] | undefined,
): readonly [number, number, number, number] | undefined {
	if (padding === undefined) return undefined;
	if (!Array.isArray(padding) || padding.length !== 4 || padding.some(value => !Number.isFinite(value) || value < 0)) {
		throw new TypeError("Stanza block decoration padding must contain four non-negative finite numbers");
	}
	return Object.freeze([padding[0]!, padding[1]!, padding[2]!, padding[3]!]);
}

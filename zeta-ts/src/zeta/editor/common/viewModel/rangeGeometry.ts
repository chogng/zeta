import { type Range } from "../core/range.js";
import { type TextModel } from "../model/textModel.js";
import { type EditorLineRange } from "./editorViewportContracts.js";
import { type TextMeasurer } from "./textMeasurer.js";

export interface RangeGeometryEntry<T> {
	readonly range: Range;
	readonly value: T;
}

export interface RangeRectangle<T> {
	readonly value: T;
	readonly lineIndex: number;
	readonly left: number;
	readonly width: number;
}

/** Controls whether a zero-length range produces a visible geometry rectangle. */
export enum EmptyRangeRendering {
	Ignore = "ignore",
	RenderAsSpace = "render-as-space",
}

/** @internal */
export function createStanzaRangeRectangles<T>(
	model: TextModel,
	entries: readonly RangeGeometryEntry<T>[],
	renderLines: EditorLineRange,
	textLeft: number,
	measurer: TextMeasurer,
	emptyRangeRendering = EmptyRangeRendering.Ignore,
): readonly RangeRectangle<T>[] {
	const rectangles: RangeRectangle<T>[] = [];
	const newlineWidth = measurer.measureLineWidth(" ");

	for (const entry of entries) {
		if (entry.range.isEmpty() && emptyRangeRendering === EmptyRangeRendering.RenderAsSpace) {
			const lineIndex = entry.range.startLineNumber - 1;
			if (!containsLine(renderLines, lineIndex)) continue;
			rectangles.push(Object.freeze({
				value: entry.value,
				lineIndex,
				left: textLeft + prefixWidth(model, lineIndex, entry.range.startColumn - 1, measurer),
				width: Math.max(1, newlineWidth),
			}));
			continue;
		}
		if (entry.range.isEmpty()) continue;
		for (
			let lineIndex = entry.range.startLineNumber - 1;
			lineIndex <= entry.range.endLineNumber - 1;
			lineIndex++
		) {
			if (!containsLine(renderLines, lineIndex)) continue;
			const startsOnLine = lineIndex === entry.range.startLineNumber - 1;
			const endsOnLine = lineIndex === entry.range.endLineNumber - 1;
			const startColumn = startsOnLine ? entry.range.startColumn - 1 : 0;
			const endColumn = endsOnLine
				? entry.range.endColumn - 1
				: model.getLineContent((lineIndex) + 1).length;
			if (endsOnLine && endColumn === 0 && !startsOnLine) continue;
			const left = textLeft + prefixWidth(
				model,
				lineIndex,
				startColumn,
				measurer,
			);
			let right = textLeft + prefixWidth(
				model,
				lineIndex,
				endColumn,
				measurer,
			);
			if (!endsOnLine) right += newlineWidth;
			if (right <= left) continue;
			rectangles.push(Object.freeze({
				value: entry.value,
				lineIndex,
				left,
				width: right - left,
			}));
		}
	}

	return Object.freeze(rectangles);
}

function prefixWidth(
	model: TextModel,
	lineIndex: number,
	columnIndex: number,
	measurer: TextMeasurer,
): number {
	return measurer.measureLineWidth(
		model.getLineContent((lineIndex) + 1).slice(0, columnIndex),
	);
}

function containsLine(range: EditorLineRange, lineIndex: number): boolean {
	return lineIndex >= range.startLineIndex &&
		lineIndex < range.endLineIndexExclusive;
}

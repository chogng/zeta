import { type TextSelectionSet } from "../core/selection.js";
import { type TextModel } from "../model/textModel.js";
import { type EditorLineRange } from "../viewLayout/editorViewportModel.js";
import { type TextMeasurer } from "./textMeasurer.js";
import { createAsterRangeRectangles } from "./rangeGeometry.js";

export interface SelectionRectangle {
	readonly selectionIndex: number;
	readonly lineIndex: number;
	readonly left: number;
	readonly width: number;
}

export interface CaretRectangle {
	readonly selectionIndex: number;
	readonly lineIndex: number;
	readonly left: number;
	readonly primary: boolean;
}

export interface SelectionGeometry {
	readonly selections: readonly SelectionRectangle[];
	readonly carets: readonly CaretRectangle[];
}

/** @internal */
export function createAsterSelectionGeometry(
	model: TextModel,
	selectionSet: TextSelectionSet,
	renderLines: EditorLineRange,
	textLeft: number,
	measurer: TextMeasurer,
): SelectionGeometry {
	const carets: CaretRectangle[] = [];

	for (
		let selectionIndex = 0;
		selectionIndex < selectionSet.selections.length;
		selectionIndex++
	) {
		const selection = selectionSet.selections[selectionIndex];
		if (!selection) continue;
		if (containsLine(renderLines, selection.active.lineIndex)) {
			carets.push(Object.freeze({
				selectionIndex,
				lineIndex: selection.active.lineIndex,
				left: textLeft + prefixWidth(
					model,
					selection.active.lineIndex,
					selection.active.columnIndex,
					measurer,
				),
				primary: selectionIndex === selectionSet.primaryIndex,
			}));
		}
	}
	const selections = createAsterRangeRectangles(
		model,
		selectionSet.selections.map((selection, selectionIndex) => ({
			range: selection.range,
			value: selectionIndex,
		})),
		renderLines,
		textLeft,
		measurer,
	).map(rectangle => Object.freeze({
		selectionIndex: rectangle.value,
		lineIndex: rectangle.lineIndex,
		left: rectangle.left,
		width: rectangle.width,
	}));

	return Object.freeze({
		selections: Object.freeze(selections),
		carets: Object.freeze(carets),
	});
}

function prefixWidth(
	model: TextModel,
	lineIndex: number,
	columnIndex: number,
	measurer: TextMeasurer,
): number {
	return measurer.measureLineWidth(
		model.getLineContent(lineIndex).slice(0, columnIndex),
	);
}

function containsLine(range: EditorLineRange, lineIndex: number): boolean {
	return lineIndex >= range.startLineIndex &&
		lineIndex < range.endLineIndexExclusive;
}

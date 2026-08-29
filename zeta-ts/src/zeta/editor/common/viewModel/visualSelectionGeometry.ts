import type { SelectionSet } from "../cursor/selectionSet.js";
import { type TextModel } from "../model/textModel.js";
import { type EditorVisualLineProjection } from "./modelLineProjection.js";
import { type EditorLineRange } from "../viewModel.js";
import { type TextMeasurer } from "./textMeasurer.js";
import { createStanzaVisualRangeRectangles } from "./visualRangeGeometry.js";

export interface VisualSelectionRectangle {
	readonly selectionIndex: number;
	readonly visualLineIndex: number;
	readonly left: number;
	readonly width: number;
}

export interface VisualCaretRectangle {
	readonly selectionIndex: number;
	readonly visualLineIndex: number;
	readonly left: number;
	readonly primary: boolean;
}

export interface VisualSelectionGeometry {
	readonly selections: readonly VisualSelectionRectangle[];
	readonly carets: readonly VisualCaretRectangle[];
}

/** @internal */
export function createStanzaVisualSelectionGeometry(model: TextModel, selectionSet: SelectionSet, projection: EditorVisualLineProjection, renderLines: EditorLineRange, textLeft: number, measurer: TextMeasurer): VisualSelectionGeometry {
	if (projection.modelVersion !== model.version) {
		throw new Error("Visual selection geometry requires the current text model projection");
	}
	const carets: VisualCaretRectangle[] = [];
	for (let selectionIndex = 0; selectionIndex < selectionSet.selections.length; selectionIndex += 1) {
		const selection = selectionSet.selections[selectionIndex];
		if (!selection) continue;
		const visualLineIndex = projection.visualLineIndexAt(selection.getPosition());
		if (visualLineIndex < renderLines.startLineIndex || visualLineIndex >= renderLines.endLineIndexExclusive) continue;
		const visualLine = projection.lineAt(visualLineIndex)!;
		const text = model.getLineContent((visualLine.logicalLineIndex) + 1);
		carets.push(Object.freeze({
			selectionIndex,
			visualLineIndex,
			left: textLeft + (visualLine.wrappedTextIndentWidth ?? 0) + measurer.measureLineWidth(text.slice(visualLine.startColumn, selection.getPosition().column - 1)),
			primary: selectionIndex === selectionSet.primaryIndex,
		}));
	}
	const selections = createStanzaVisualRangeRectangles(
		model,
		selectionSet.selections.map((selection, selectionIndex) => ({
			range: selection,
			value: selectionIndex,
		})),
		projection,
		renderLines,
		textLeft,
		measurer,
	).map(rectangle => Object.freeze({
		selectionIndex: rectangle.value,
		visualLineIndex: rectangle.visualLineIndex,
		left: rectangle.left,
		width: rectangle.width,
	}));
	return Object.freeze({
		selections: Object.freeze(selections),
		carets: Object.freeze(carets),
	});
}

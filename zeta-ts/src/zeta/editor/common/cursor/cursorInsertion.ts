import { TextSelection, TextSelectionSet } from "../core/selection.js";
import { TextPosition } from "../core/text.js";
import { type TextModel } from "../model/textModel.js";

/** Selects which adjacent physical line receives an additional caret. */
export enum EditorCursorInsertionDirection {
	Above = "above",
	Below = "below",
}

/**
 * Adds a caret adjacent to every existing selection's active position.
 *
 * The existing selections remain unchanged. New carets clamp to the target
 * line length and never duplicate or overlap a retained selection.
 */
export function addAdjacentLineCursors(model: TextModel, selections: TextSelectionSet, direction: EditorCursorInsertionDirection): TextSelectionSet {
	if (!Object.values(EditorCursorInsertionDirection).includes(direction)) {
		throw new TypeError("Unknown editor cursor insertion direction");
	}
	const nextSelections = [...selections.selections];
	let primaryIndex = selections.primaryIndex;
	for (const selection of selections.selections) {
		const target = adjacentLinePosition(model, selection.active, direction);
		if (!target || nextSelections.some(candidate => positionOverlapsSelection(target, candidate))) continue;
		nextSelections.push(TextSelection.collapsedAt(target));
		primaryIndex = nextSelections.length - 1;
	}
	return nextSelections.length === selections.selections.length
		? selections
		: TextSelectionSet.withPrimary(nextSelections, primaryIndex);
}

/** Replaces non-empty selections with carets at the selected physical line ends. */
export function addCursorsToSelectedLineEnds(model: TextModel, selections: TextSelectionSet): TextSelectionSet {
	const nextSelections: TextSelection[] = [];
	let primaryIndex: number | undefined;
	for (let selectionIndex = 0; selectionIndex < selections.selections.length; selectionIndex += 1) {
		const selection = selections.selections[selectionIndex]!;
		if (selection.collapsed) continue;
		const range = selection.range;
		const startIndex = nextSelections.length;
		for (let lineIndex = range.start.lineIndex; lineIndex < range.end.lineIndex; lineIndex += 1) {
			appendUniqueCaret(nextSelections, TextPosition.at(lineIndex, model.getLineContent(lineIndex).length));
		}
		if (range.end.columnIndex > 0) {
			appendUniqueCaret(nextSelections, range.end);
		}
		if (selectionIndex === selections.primaryIndex && nextSelections.length > startIndex) {
			primaryIndex = startIndex;
		}
	}
	if (nextSelections.length === 0) return selections;
	return TextSelectionSet.withPrimary(nextSelections, primaryIndex ?? 0);
}

function adjacentLinePosition(model: TextModel, position: TextPosition, direction: EditorCursorInsertionDirection): TextPosition | undefined {
	const lineIndex = position.lineIndex + (direction === EditorCursorInsertionDirection.Above ? -1 : 1);
	if (lineIndex < 0 || lineIndex >= model.lineCount) return undefined;
	return TextPosition.at(lineIndex, Math.min(position.columnIndex, model.getLineContent(lineIndex).length));
}

function positionOverlapsSelection(position: TextPosition, selection: TextSelection): boolean {
	if (selection.collapsed) return position.compareTo(selection.active) === 0;
	return position.compareTo(selection.range.start) >= 0 && position.compareTo(selection.range.end) < 0;
}

function appendUniqueCaret(target: TextSelection[], position: TextPosition): void {
	if (target.some(selection => selection.collapsed && selection.active.compareTo(position) === 0)) return;
	target.push(TextSelection.collapsedAt(position));
}

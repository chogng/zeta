import { TextSelection, TextSelectionSet } from "../core/selection.js";
import { TextPosition } from "../core/text.js";
import { type TextModel } from "../model/textModel.js";

/**
 * Adds a caret adjacent to every existing selection's active position.
 *
 * The existing selections remain unchanged. New carets clamp to the target
 * line length and never duplicate or overlap a retained selection.
 */
export class CursorMoveCommands {
	public static addCursorDown(model: TextModel, selections: TextSelectionSet): TextSelectionSet {
		return addAdjacentLineCursors(model, selections, 'below');
	}

	public static addCursorUp(model: TextModel, selections: TextSelectionSet): TextSelectionSet {
		return addAdjacentLineCursors(model, selections, 'above');
	}

	public static addCursorsToLineEnds(model: TextModel, selections: TextSelectionSet): TextSelectionSet {
		return addCursorsToSelectedLineEnds(model, selections);
	}

	public static readPointerMultiCursorModifier(value: PointerMultiCursorModifier | undefined): PointerMultiCursorModifier {
		const resolved = value ?? PointerMultiCursorModifier.Alt;
		if (resolved !== PointerMultiCursorModifier.Alt && resolved !== PointerMultiCursorModifier.ControlOrMeta) {
			throw new TypeError('Unknown Stanza pointer multi-cursor modifier');
		}
		return resolved;
	}

	public static isPointerMultiCursorGesture(state: PointerModifierState, modifier: PointerMultiCursorModifier): boolean {
		if (state.shiftKey) return false;
		if (modifier === PointerMultiCursorModifier.Alt) return state.altKey && !state.ctrlKey && !state.metaKey;
		return (state.ctrlKey || state.metaKey) && !state.altKey;
	}

	public static combinePointerSelection(base: TextSelectionSet, active: TextSelection, toggleCandidateIndex: number | undefined): TextSelectionSet {
		return combinePointerSelection(base, active, toggleCandidateIndex);
	}

	public static findPointerToggleCandidate(base: TextSelectionSet, selection: TextSelection): number | undefined {
		const index = base.selections.findIndex(candidate => selectionsHaveSameRange(candidate, selection));
		return index >= 0 ? index : undefined;
	}
}

function addAdjacentLineCursors(model: TextModel, selections: TextSelectionSet, direction: 'above' | 'below'): TextSelectionSet {
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
function addCursorsToSelectedLineEnds(model: TextModel, selections: TextSelectionSet): TextSelectionSet {
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

function adjacentLinePosition(model: TextModel, position: TextPosition, direction: 'above' | 'below'): TextPosition | undefined {
	const lineIndex = position.lineIndex + (direction === 'above' ? -1 : 1);
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

export enum PointerMultiCursorModifier {
	Alt = "alt",
	ControlOrMeta = "controlOrMeta",
}

export interface PointerModifierState {
	readonly altKey: boolean;
	readonly ctrlKey: boolean;
	readonly metaKey: boolean;
	readonly shiftKey: boolean;
}

function combinePointerSelection(base: TextSelectionSet, active: TextSelection, toggleCandidateIndex: number | undefined): TextSelectionSet {
	validateToggleCandidate(base, toggleCandidateIndex);
	if (toggleCandidateIndex !== undefined && selectionsHaveSameRange(base.selections[toggleCandidateIndex]!, active)) {
		if (base.selections.length === 1) return base;
		const selections = base.selections.filter((_, index) => index !== toggleCandidateIndex);
		return TextSelectionSet.withPrimary(selections, primaryAfterRemoval(base.primaryIndex, toggleCandidateIndex, selections.length));
	}

	const retained = toggleCandidateIndex === undefined
		? [...base.selections]
		: base.selections.filter((_, index) => index !== toggleCandidateIndex);
	const duplicateIndex = retained.findIndex(selection => selectionsHaveSameRange(selection, active));
	if (duplicateIndex >= 0) return TextSelectionSet.withPrimary(retained, duplicateIndex);
	const nonOverlapping = retained.filter(selection => !selectionRangesOverlap(selection, active));
	if (nonOverlapping.length === 0) return TextSelectionSet.single(active);
	return TextSelectionSet.withPrimary([...nonOverlapping, active], nonOverlapping.length);
}

function selectionsHaveSameRange(left: TextSelection, right: TextSelection): boolean {
	return left.range.start.compareTo(right.range.start) === 0 && left.range.end.compareTo(right.range.end) === 0;
}

function selectionRangesOverlap(left: TextSelection, right: TextSelection): boolean {
	if (left.collapsed) return pointOverlapsRange(left.range.start, right);
	if (right.collapsed) return pointOverlapsRange(right.range.start, left);
	return left.range.start.compareTo(right.range.end) < 0 && right.range.start.compareTo(left.range.end) < 0;
}

function pointOverlapsRange(point: TextPosition, selection: TextSelection): boolean {
	if (selection.collapsed) return point.compareTo(selection.range.start) === 0;
	return point.compareTo(selection.range.start) >= 0 && point.compareTo(selection.range.end) < 0;
}

function primaryAfterRemoval(primaryIndex: number, removedIndex: number, remainingCount: number): number {
	if (primaryIndex < removedIndex) return primaryIndex;
	if (primaryIndex > removedIndex) return primaryIndex - 1;
	return Math.min(removedIndex, remainingCount - 1);
}

function validateToggleCandidate(base: TextSelectionSet, index: number | undefined): void {
	if (index !== undefined && (!Number.isSafeInteger(index) || index < 0 || index >= base.selections.length)) {
		throw new RangeError("Pointer toggle candidate is outside the selection set");
	}
}

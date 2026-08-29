import { Selection } from "../core/selection.js";
import { SelectionSet } from "./selectionSet.js";
import { Position } from "../core/position.js";
import { type TextModel } from "../model/textModel.js";

/**
 * Adds a caret adjacent to every existing selection's active position.
 *
 * The existing selections remain unchanged. New carets clamp to the target
 * line length and never duplicate or overlap a retained selection.
 */
export class CursorMoveCommands {
	public static addCursorDown(model: TextModel, selections: SelectionSet): SelectionSet {
		return addAdjacentLineCursors(model, selections, 'below');
	}

	public static addCursorUp(model: TextModel, selections: SelectionSet): SelectionSet {
		return addAdjacentLineCursors(model, selections, 'above');
	}

	public static addCursorsToLineEnds(model: TextModel, selections: SelectionSet): SelectionSet {
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

	public static combinePointerSelection(base: SelectionSet, active: Selection, toggleCandidateIndex: number | undefined): SelectionSet {
		return combinePointerSelection(base, active, toggleCandidateIndex);
	}

	public static findPointerToggleCandidate(base: SelectionSet, selection: Selection): number | undefined {
		const index = base.selections.findIndex(candidate => selectionsHaveSameRange(candidate, selection));
		return index >= 0 ? index : undefined;
	}
}

function addAdjacentLineCursors(model: TextModel, selections: SelectionSet, direction: 'above' | 'below'): SelectionSet {
	const nextSelections = [...selections.selections];
	let primaryIndex = selections.primaryIndex;
	for (const selection of selections.selections) {
		const target = adjacentLinePosition(model, selection.getPosition(), direction);
		if (!target || nextSelections.some(candidate => positionOverlapsSelection(target, candidate))) continue;
		nextSelections.push(Selection.fromPositions(target));
		primaryIndex = nextSelections.length - 1;
	}
	return nextSelections.length === selections.selections.length
		? selections
		: SelectionSet.withPrimary(nextSelections, primaryIndex);
}

/** Replaces non-empty selections with carets at the selected physical line ends. */
function addCursorsToSelectedLineEnds(model: TextModel, selections: SelectionSet): SelectionSet {
	const nextSelections: Selection[] = [];
	let primaryIndex: number | undefined;
	for (let selectionIndex = 0; selectionIndex < selections.selections.length; selectionIndex += 1) {
		const selection = selections.selections[selectionIndex]!;
		if (selection.isEmpty()) continue;
		const range = selection;
		const startIndex = nextSelections.length;
		for (let lineNumber = range.startLineNumber; lineNumber < range.endLineNumber; lineNumber += 1) {
			appendUniqueCaret(nextSelections, new Position(lineNumber, model.getLineContent(lineNumber).length + 1));
		}
		if (range.endColumn > 1) {
			appendUniqueCaret(nextSelections, range.getEndPosition());
		}
		if (selectionIndex === selections.primaryIndex && nextSelections.length > startIndex) {
			primaryIndex = startIndex;
		}
	}
	if (nextSelections.length === 0) return selections;
	return SelectionSet.withPrimary(nextSelections, primaryIndex ?? 0);
}

function adjacentLinePosition(model: TextModel, position: Position, direction: 'above' | 'below'): Position | undefined {
	const lineNumber = position.lineNumber + (direction === 'above' ? -1 : 1);
	if (lineNumber < 1 || lineNumber > model.lineCount) return undefined;
	return new Position(lineNumber, Math.min(position.column, model.getLineContent(lineNumber).length + 1));
}

function positionOverlapsSelection(position: Position, selection: Selection): boolean {
	if (selection.isEmpty()) return Position.compare(position, selection.getPosition()) === 0;
	return Position.compare(position, selection.getStartPosition()) >= 0 && Position.compare(position, selection.getEndPosition()) < 0;
}

function appendUniqueCaret(target: Selection[], position: Position): void {
	if (target.some(selection => selection.isEmpty() && Position.compare(selection.getPosition(), position) === 0)) return;
	target.push(Selection.fromPositions(position));
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

function combinePointerSelection(base: SelectionSet, active: Selection, toggleCandidateIndex: number | undefined): SelectionSet {
	validateToggleCandidate(base, toggleCandidateIndex);
	if (toggleCandidateIndex !== undefined && selectionsHaveSameRange(base.selections[toggleCandidateIndex]!, active)) {
		if (base.selections.length === 1) return base;
		const selections = base.selections.filter((_, index) => index !== toggleCandidateIndex);
		return SelectionSet.withPrimary(selections, primaryAfterRemoval(base.primaryIndex, toggleCandidateIndex, selections.length));
	}

	const retained = toggleCandidateIndex === undefined
		? [...base.selections]
		: base.selections.filter((_, index) => index !== toggleCandidateIndex);
	const duplicateIndex = retained.findIndex(selection => selectionsHaveSameRange(selection, active));
	if (duplicateIndex >= 0) return SelectionSet.withPrimary(retained, duplicateIndex);
	const nonOverlapping = retained.filter(selection => !selectionRangesOverlap(selection, active));
	if (nonOverlapping.length === 0) return SelectionSet.single(active);
	return SelectionSet.withPrimary([...nonOverlapping, active], nonOverlapping.length);
}

function selectionsHaveSameRange(left: Selection, right: Selection): boolean {
	return Position.compare(left.getStartPosition(), right.getStartPosition()) === 0 && Position.compare(left.getEndPosition(), right.getEndPosition()) === 0;
}

function selectionRangesOverlap(left: Selection, right: Selection): boolean {
	if (left.isEmpty()) return pointOverlapsRange(left.getStartPosition(), right);
	if (right.isEmpty()) return pointOverlapsRange(right.getStartPosition(), left);
	return Position.compare(left.getStartPosition(), right.getEndPosition()) < 0 && Position.compare(right.getStartPosition(), left.getEndPosition()) < 0;
}

function pointOverlapsRange(point: Position, selection: Selection): boolean {
	if (selection.isEmpty()) return Position.compare(point, selection.getStartPosition()) === 0;
	return Position.compare(point, selection.getStartPosition()) >= 0 && Position.compare(point, selection.getEndPosition()) < 0;
}

function primaryAfterRemoval(primaryIndex: number, removedIndex: number, remainingCount: number): number {
	if (primaryIndex < removedIndex) return primaryIndex;
	if (primaryIndex > removedIndex) return primaryIndex - 1;
	return Math.min(removedIndex, remainingCount - 1);
}

function validateToggleCandidate(base: SelectionSet, index: number | undefined): void {
	if (index !== undefined && (!Number.isSafeInteger(index) || index < 0 || index >= base.selections.length)) {
		throw new RangeError("Pointer toggle candidate is outside the selection set");
	}
}

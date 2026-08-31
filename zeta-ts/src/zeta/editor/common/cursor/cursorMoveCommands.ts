import { Selection } from "../core/selection.js";
import { Position } from "../core/position.js";
import { type TextModel } from "../model/textModel.js";

/**
 * Adds a caret adjacent to every existing selection's active position.
 *
 * The existing selections remain unchanged. New carets clamp to the target
 * line length and never duplicate or overlap a retained selection.
 */
export class CursorMoveCommands {
	public static addCursorDown(model: TextModel, selections: readonly Selection[]): readonly Selection[] {
		return addAdjacentLineCursors(model, selections, 'below');
	}

	public static addCursorUp(model: TextModel, selections: readonly Selection[]): readonly Selection[] {
		return addAdjacentLineCursors(model, selections, 'above');
	}

	public static addCursorsToLineEnds(model: TextModel, selections: readonly Selection[]): readonly Selection[] {
		return addCursorsToSelectedLineEnds(model, selections);
	}

	public static expandLineSelection(model: TextModel, selections: readonly Selection[]): readonly Selection[] {
		return expandLineSelections(model, selections);
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

	public static combinePointerSelection(base: readonly Selection[], active: Selection, toggleCandidateIndex: number | undefined): readonly Selection[] {
		return combinePointerSelection(base, active, toggleCandidateIndex);
	}

	public static findPointerToggleCandidate(base: readonly Selection[], selection: Selection): number | undefined {
		const index = base.findIndex(candidate => selectionsHaveSameRange(candidate, selection));
		return index >= 0 ? index : undefined;
	}
}

function expandLineSelections(model: TextModel, selections: readonly Selection[]): readonly Selection[] {
	return Object.freeze(selections.map(selection => {
		const start = new Position(selection.startLineNumber, 1);
		const endLineNumber = selection.endLineNumber;
		const end = endLineNumber === model.lineCount
			? new Position(endLineNumber, model.getLineContent(endLineNumber).length + 1)
			: new Position(endLineNumber + 1, 1);
		return Selection.fromPositions(start, end);
	}));
}

function addAdjacentLineCursors(model: TextModel, selections: readonly Selection[], direction: 'above' | 'below'): readonly Selection[] {
	const nextSelections: Selection[] = [];
	let changed = false;
	for (const selection of selections) {
		nextSelections.push(selection);
		const target = adjacentLinePosition(model, selection.getPosition(), direction);
		if (!target || selections.some(candidate => positionOverlapsSelection(target, candidate)) || nextSelections.some(candidate => positionOverlapsSelection(target, candidate))) continue;
		nextSelections.push(Selection.fromPositions(target));
		changed = true;
	}
	return changed ? Object.freeze(nextSelections) : selections;
}

/** Replaces non-empty selections with carets at the selected physical line ends. */
function addCursorsToSelectedLineEnds(model: TextModel, selections: readonly Selection[]): readonly Selection[] {
	const nextSelections: Selection[] = [];
	for (let selectionIndex = 0; selectionIndex < selections.length; selectionIndex += 1) {
		const selection = selections[selectionIndex]!;
		if (selection.isEmpty()) continue;
		for (let lineNumber = selection.startLineNumber; lineNumber < selection.endLineNumber; lineNumber += 1) {
			appendUniqueCaret(nextSelections, new Position(lineNumber, model.getLineContent(lineNumber).length + 1));
		}
		if (selection.endColumn > 1) appendUniqueCaret(nextSelections, selection.getEndPosition());
	}
	if (nextSelections.length === 0) return selections;
	return Object.freeze(nextSelections);
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

function combinePointerSelection(base: readonly Selection[], active: Selection, toggleCandidateIndex: number | undefined): readonly Selection[] {
	validateToggleCandidate(base, toggleCandidateIndex);
	if (toggleCandidateIndex !== undefined && selectionsHaveSameRange(base[toggleCandidateIndex]!, active)) {
		if (base.length === 1) return base;
		return Object.freeze(base.filter((_, index) => index !== toggleCandidateIndex));
	}

	const retained = toggleCandidateIndex === undefined
		? [...base]
		: base.filter((_, index) => index !== toggleCandidateIndex);
	const duplicateIndex = retained.findIndex(selection => selectionsHaveSameRange(selection, active));
	if (duplicateIndex >= 0) return primaryFirst(retained, duplicateIndex);
	const nonOverlapping = retained.filter(selection => !selectionRangesOverlap(selection, active));
	return Object.freeze([active, ...nonOverlapping]);
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

function validateToggleCandidate(base: readonly Selection[], index: number | undefined): void {
	if (index !== undefined && (!Number.isSafeInteger(index) || index < 0 || index >= base.length)) {
		throw new RangeError("Pointer toggle candidate is outside the selection set");
	}
}

function primaryFirst(selections: readonly Selection[], primaryIndex: number): readonly Selection[] {
	if (primaryIndex === 0) return Object.freeze([...selections]);
	return Object.freeze([selections[primaryIndex]!, ...selections.slice(0, primaryIndex), ...selections.slice(primaryIndex + 1)]);
}

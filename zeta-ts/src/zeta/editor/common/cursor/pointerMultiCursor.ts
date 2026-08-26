import { TextSelection, TextSelectionSet } from '../core/selection.js';
import { type TextPosition } from '../core/text.js';

export enum PointerMultiCursorModifier {
	Alt = 'alt',
	ControlOrMeta = 'controlOrMeta',
}

export interface PointerModifierState {
	readonly altKey: boolean;
	readonly ctrlKey: boolean;
	readonly metaKey: boolean;
	readonly shiftKey: boolean;
}

export function readStanzaPointerMultiCursorModifier(value: PointerMultiCursorModifier | undefined): PointerMultiCursorModifier {
	const resolved = value ?? PointerMultiCursorModifier.Alt;
	if (
		resolved !== PointerMultiCursorModifier.Alt &&
		resolved !== PointerMultiCursorModifier.ControlOrMeta
	) {
		throw new TypeError('Unknown Stanza pointer multi-cursor modifier');
	}
	return resolved;
}

export function isStanzaPointerMultiCursorGesture(state: PointerModifierState, modifier: PointerMultiCursorModifier): boolean {
	if (state.shiftKey) return false;
	if (modifier === PointerMultiCursorModifier.Alt) {
		return state.altKey && !state.ctrlKey && !state.metaKey;
	}
	return (state.ctrlKey || state.metaKey) && !state.altKey;
}

/**
 * Adds one pointer-produced selection while preserving deterministic order.
 *
 * Clicking the selection where the gesture began toggles it off unless it is
 * the only selection. Dragging from that selection replaces it. Landing on
 * another identical range deduplicates and makes that range primary.
 */
export function combineStanzaPointerSelection(base: TextSelectionSet, active: TextSelection, toggleCandidateIndex: number | undefined): TextSelectionSet {
	validateToggleCandidate(base, toggleCandidateIndex);
	if (
		toggleCandidateIndex !== undefined &&
		selectionsHaveSameRange(base.selections[toggleCandidateIndex]!, active)
	) {
		if (base.selections.length === 1) return base;
		const selections = base.selections.filter((_, index) => index !== toggleCandidateIndex);
		return TextSelectionSet.withPrimary(
			selections,
			primaryAfterRemoval(base.primaryIndex, toggleCandidateIndex, selections.length),
		);
	}

	const retained = toggleCandidateIndex === undefined
		? [...base.selections]
		: base.selections.filter((_, index) => index !== toggleCandidateIndex);
	const duplicateIndex = retained.findIndex(selection => selectionsHaveSameRange(selection, active));
	if (duplicateIndex >= 0) return TextSelectionSet.withPrimary(retained, duplicateIndex);
	const nonOverlapping = retained.filter(selection => !selectionRangesOverlap(selection, active));
	if (nonOverlapping.length === 0) return TextSelectionSet.single(active);
	return TextSelectionSet.withPrimary(
		[...nonOverlapping, active],
		nonOverlapping.length,
	);
}

export function findStanzaPointerToggleCandidate(base: TextSelectionSet, selection: TextSelection): number | undefined {
	const index = base.selections.findIndex(candidate => selectionsHaveSameRange(candidate, selection));
	return index >= 0 ? index : undefined;
}

function selectionsHaveSameRange(left: TextSelection, right: TextSelection): boolean {
	return left.range.start.compareTo(right.range.start) === 0 &&
		left.range.end.compareTo(right.range.end) === 0;
}

function selectionRangesOverlap(left: TextSelection, right: TextSelection): boolean {
	if (left.collapsed) return pointOverlapsRange(left.range.start, right);
	if (right.collapsed) return pointOverlapsRange(right.range.start, left);
	return left.range.start.compareTo(right.range.end) < 0 &&
		right.range.start.compareTo(left.range.end) < 0;
}

function pointOverlapsRange(point: TextPosition, selection: TextSelection): boolean {
	if (selection.collapsed) return point.compareTo(selection.range.start) === 0;
	return point.compareTo(selection.range.start) >= 0 &&
		point.compareTo(selection.range.end) < 0;
}

function primaryAfterRemoval(primaryIndex: number, removedIndex: number, remainingCount: number): number {
	if (primaryIndex < removedIndex) return primaryIndex;
	if (primaryIndex > removedIndex) return primaryIndex - 1;
	return Math.min(removedIndex, remainingCount - 1);
}

function validateToggleCandidate(base: TextSelectionSet, index: number | undefined): void {
	if (
		index !== undefined &&
		(
			!Number.isSafeInteger(index) ||
			index < 0 ||
			index >= base.selections.length
		)
	) {
		throw new RangeError('Pointer toggle candidate is outside the selection set');
	}
}

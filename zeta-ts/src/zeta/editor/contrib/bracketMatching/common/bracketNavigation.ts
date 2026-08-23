import { TextSelection, TextSelectionSet } from "../../../common/core/selection.js";
import { type LanguageBracketMatcher } from "./bracketMatching.js";
import { type TextPosition } from "../../../common/core/text.js";

/** Moves every active cursor to its lexically valid matching bracket when present. */
export function jumpToMatchingBrackets(matcher: LanguageBracketMatcher, selections: TextSelectionSet): TextSelectionSet {
	let changed = false;
	const nextSelections = selections.selections.map(selection => {
		const match = matcher.findMatch(selection.active);
		if (!match) return selection;
		const next = TextSelection.collapsedAt(isAtOpening(match.opening.start, match.opening.end, selection.active)
			? match.closing.start
			: match.opening.start);
		changed ||= next.active.compareTo(selection.active) !== 0 || !selection.collapsed;
		return next;
	});
	return changed ? TextSelectionSet.withPrimary(nextSelections, selections.primaryIndex) : selections;
}

/** Selects the full configured bracket pair around every active cursor when present. */
export function selectToMatchingBrackets(matcher: LanguageBracketMatcher, selections: TextSelectionSet): TextSelectionSet {
	let changed = false;
	const nextSelections = selections.selections.map(selection => {
		const match = matcher.findMatch(selection.active);
		if (!match) return selection;
		const next = TextSelection.from(match.opening.start, match.closing.end);
		changed ||= next.range.start.compareTo(selection.range.start) !== 0 || next.range.end.compareTo(selection.range.end) !== 0;
		return next;
	});
	return changed ? TextSelectionSet.withPrimary(nextSelections, selections.primaryIndex) : selections;
}

function isAtOpening(start: TextPosition, end: TextPosition, position: TextPosition): boolean {
	return position.compareTo(start) >= 0 && position.compareTo(end) <= 0;
}

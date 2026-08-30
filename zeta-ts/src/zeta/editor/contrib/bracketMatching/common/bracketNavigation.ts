import { Selection } from "../../../common/core/selection.js";
import { type LanguageBracketPair, type LanguageBracketPairs } from "../../../common/languages/languageBracketPairs.js";
import { Position } from "../../../common/core/position.js";

/** Moves every active cursor to its lexically valid matching bracket when present. */
export function jumpToMatchingBrackets(bracketPairs: LanguageBracketPairs, selections: readonly Selection[]): readonly Selection[] {
	let changed = false;
	const nextSelections = selections.map(selection => {
		const match = matchAtOrAfter(bracketPairs, selection.getPosition());
		if (!match) return selection;
		const next = Selection.fromPositions(isAtOpening(match.opening.getStartPosition(), match.opening.getEndPosition(), selection.getPosition())
			? match.closing.getStartPosition()
			: match.opening.getStartPosition());
		changed ||= Position.compare(next.getPosition(), selection.getPosition()) !== 0 || !selection.isEmpty();
		return next;
	});
	return changed ? Object.freeze(nextSelections) : selections;
}

/** Selects the full configured bracket pair around every active cursor when present. */
export function selectToMatchingBrackets(bracketPairs: LanguageBracketPairs, selections: readonly Selection[]): readonly Selection[] {
	let changed = false;
	const nextSelections = selections.map(selection => {
		const match = matchAtOrAfter(bracketPairs, selection.getPosition());
		if (!match) return selection;
		const next = Selection.fromPositions(match.opening.getStartPosition(), match.closing.getEndPosition());
		changed ||= Position.compare(next.getStartPosition(), selection.getStartPosition()) !== 0 || Position.compare(next.getEndPosition(), selection.getEndPosition()) !== 0;
		return next;
	});
	return changed ? Object.freeze(nextSelections) : selections;
}

function matchAtOrAfter(bracketPairs: LanguageBracketPairs, position: Position): LanguageBracketPair | undefined {
	return bracketPairs.matchBracket(position)
		?? bracketPairs.findEnclosingBrackets(position)
		?? bracketPairs.findNextBracket(position)?.pair;
}

function isAtOpening(start: Position, end: Position, position: Position): boolean {
	return Position.compare(position, start) >= 0 && Position.compare(position, end) <= 0;
}

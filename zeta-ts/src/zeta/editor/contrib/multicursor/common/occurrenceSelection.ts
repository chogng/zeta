import { Selection } from "../../../common/core/selection.js";
import { SelectionSet } from "../../../common/cursor/selectionSet.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { findTextMatches, type TextSearchMatch } from "../../../common/model/textModelSearch.js";

const MAX_OCCURRENCE_SELECTIONS = 100_000;

/** Selects the direction in which one additional matching occurrence is found. */
export enum EditorOccurrenceDirection {
	Next = "next",
	Previous = "previous",
}

/**
 * Adds the next or previous exact occurrence of the primary selection.
 *
 * A collapsed primary cursor first selects its current word-like segment. The
 * model remains unchanged; callers own the resulting live selection state.
 */
export function addOccurrenceSelection(model: TextModel, selections: SelectionSet, direction: EditorOccurrenceDirection): SelectionSet {
	if (!Object.values(EditorOccurrenceDirection).includes(direction)) {
		throw new TypeError("Unknown editor occurrence direction");
	}
	const source = sourceSelection(model, selections);
	if (!source) return selections;
	if (selections.primary.isEmpty()) return replacePrimarySelection(selections, source);

	const matches = findOccurrences(model, source);
	const selectedRanges = new Set(selections.selections.map(selection => rangeKey(model, selection)));
	const startOffset = direction === EditorOccurrenceDirection.Next
		? model.offsetAt(source.getEndPosition())
		: model.offsetAt(source.getStartPosition());
	const candidate = orderedCandidates(model, matches, startOffset, direction)
		.find(match => !selectedRanges.has(rangeKey(model, Selection.fromPositions(match.range.getStartPosition(), match.range.getEndPosition()))));
	if (!candidate) return selections;
	const nextSelections = Object.freeze([
		...selections.selections,
		Selection.fromPositions(candidate.range.getStartPosition(), candidate.range.getEndPosition()),
	]);
	return SelectionSet.withPrimary(nextSelections, nextSelections.length - 1);
}

/** Selects every exact occurrence of the primary selection or cursor word. */
export function selectAllOccurrences(model: TextModel, selections: SelectionSet): SelectionSet {
	const source = sourceSelection(model, selections);
	if (!source) return selections;
	const matches = findOccurrences(model, source);
	if (matches.length === 0) return selections;
	const sourceKey = rangeKey(model, source);
	const nextSelections = Object.freeze(matches.map(match => Selection.fromPositions(match.range.getStartPosition(), match.range.getEndPosition())));
	const primaryIndex = nextSelections.findIndex(selection => rangeKey(model, selection) === sourceKey);
	return SelectionSet.withPrimary(nextSelections, primaryIndex < 0 ? 0 : primaryIndex);
}

function sourceSelection(model: TextModel, selections: SelectionSet): Selection | undefined {
	const primary = selections.primary;
	if (!primary.isEmpty()) return primary;
	const word = model.getWordAtPosition(primary.getPosition());
	return word ? new Selection(primary.positionLineNumber, word.startColumn, primary.positionLineNumber, word.endColumn) : undefined;
}

function findOccurrences(model: TextModel, source: Selection): readonly TextSearchMatch[] {
	return findTextMatches(model, {
		pattern: model.getTextInRange(source),
		matchCase: true,
	}, {
		resultLimit: MAX_OCCURRENCE_SELECTIONS,
	});
}

function orderedCandidates(model: TextModel, matches: readonly TextSearchMatch[], startOffset: number, direction: EditorOccurrenceDirection): readonly TextSearchMatch[] {
	const ordered = direction === EditorOccurrenceDirection.Next ? matches : [...matches].reverse();
	const beforeWrap = ordered.filter(match => direction === EditorOccurrenceDirection.Next
		? model.offsetAt(match.range.getStartPosition()) >= startOffset
		: model.offsetAt(match.range.getEndPosition()) <= startOffset);
	const afterWrap = ordered.filter(match => direction === EditorOccurrenceDirection.Next
		? model.offsetAt(match.range.getStartPosition()) < startOffset
		: model.offsetAt(match.range.getEndPosition()) > startOffset);
	return Object.freeze([...beforeWrap, ...afterWrap]);
}

function replacePrimarySelection(selections: SelectionSet, replacement: Selection): SelectionSet {
	const nextSelections = selections.selections.map((selection, index) => index === selections.primaryIndex ? replacement : selection);
	return SelectionSet.withPrimary(nextSelections, selections.primaryIndex);
}

function rangeKey(model: TextModel, selection: Selection): string {
	return `${model.offsetAt(selection.getStartPosition())}:${model.offsetAt(selection.getEndPosition())}`;
}

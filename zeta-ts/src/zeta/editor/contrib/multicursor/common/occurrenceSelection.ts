import { TextSelection, TextSelectionSet } from "../../../common/core/selection.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { findTextMatches, type TextSearchMatch } from "../../../common/model/textModelSearch.js";
import { WordOperations } from "../../../common/cursor/cursorWordOperations.js";

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
export function addOccurrenceSelection(model: TextModel, selections: TextSelectionSet, direction: EditorOccurrenceDirection, wordPattern?: RegExp): TextSelectionSet {
	if (!Object.values(EditorOccurrenceDirection).includes(direction)) {
		throw new TypeError("Unknown editor occurrence direction");
	}
	const source = sourceSelection(model, selections, wordPattern);
	if (!source) return selections;
	if (selections.primary.collapsed) return replacePrimarySelection(selections, source);

	const matches = findOccurrences(model, source);
	const selectedRanges = new Set(selections.selections.map(selection => rangeKey(model, selection)));
	const startOffset = direction === EditorOccurrenceDirection.Next
		? model.offsetAt(source.range.end)
		: model.offsetAt(source.range.start);
	const candidate = orderedCandidates(model, matches, startOffset, direction)
		.find(match => !selectedRanges.has(rangeKey(model, TextSelection.from(match.range.start, match.range.end))));
	if (!candidate) return selections;
	const nextSelections = Object.freeze([
		...selections.selections,
		TextSelection.from(candidate.range.start, candidate.range.end),
	]);
	return TextSelectionSet.withPrimary(nextSelections, nextSelections.length - 1);
}

/** Selects every exact occurrence of the primary selection or cursor word. */
export function selectAllOccurrences(model: TextModel, selections: TextSelectionSet, wordPattern?: RegExp): TextSelectionSet {
	const source = sourceSelection(model, selections, wordPattern);
	if (!source) return selections;
	const matches = findOccurrences(model, source);
	if (matches.length === 0) return selections;
	const sourceKey = rangeKey(model, source);
	const nextSelections = Object.freeze(matches.map(match => TextSelection.from(match.range.start, match.range.end)));
	const primaryIndex = nextSelections.findIndex(selection => rangeKey(model, selection) === sourceKey);
	return TextSelectionSet.withPrimary(nextSelections, primaryIndex < 0 ? 0 : primaryIndex);
}

function sourceSelection(model: TextModel, selections: TextSelectionSet, wordPattern: RegExp | undefined): TextSelection | undefined {
	const primary = selections.primary;
	if (!primary.collapsed) return primary;
	const range = WordOperations.getWordSelectionRange(model, primary.active, wordPattern);
	if (range.empty) return undefined;
	return TextSelection.from(range.start, range.end);
}

function findOccurrences(model: TextModel, source: TextSelection): readonly TextSearchMatch[] {
	return findTextMatches(model, {
		pattern: model.getTextInRange(source.range),
		matchCase: true,
	}, {
		resultLimit: MAX_OCCURRENCE_SELECTIONS,
	});
}

function orderedCandidates(model: TextModel, matches: readonly TextSearchMatch[], startOffset: number, direction: EditorOccurrenceDirection): readonly TextSearchMatch[] {
	const ordered = direction === EditorOccurrenceDirection.Next ? matches : [...matches].reverse();
	const beforeWrap = ordered.filter(match => direction === EditorOccurrenceDirection.Next
		? model.offsetAt(match.range.start) >= startOffset
		: model.offsetAt(match.range.end) <= startOffset);
	const afterWrap = ordered.filter(match => direction === EditorOccurrenceDirection.Next
		? model.offsetAt(match.range.start) < startOffset
		: model.offsetAt(match.range.end) > startOffset);
	return Object.freeze([...beforeWrap, ...afterWrap]);
}

function replacePrimarySelection(selections: TextSelectionSet, replacement: TextSelection): TextSelectionSet {
	const nextSelections = selections.selections.map((selection, index) => index === selections.primaryIndex ? replacement : selection);
	return TextSelectionSet.withPrimary(nextSelections, selections.primaryIndex);
}

function rangeKey(model: TextModel, selection: TextSelection): string {
	return `${model.offsetAt(selection.range.start)}:${model.offsetAt(selection.range.end)}`;
}

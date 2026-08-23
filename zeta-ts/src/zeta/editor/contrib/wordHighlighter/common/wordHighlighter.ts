import { TextRange } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { findTextMatches } from "../../../common/model/textModelSearch.js";
import { getTextWordSegments } from "../../../common/core/textSegmentation.js";
import { type TextSelectionSet } from "../../../common/core/selection.js";
import { getWordSelectionRange } from "../../../common/cursor/wordBoundary.js";

const MAX_OCCURRENCE_HIGHLIGHTS = 10_000;

/**
 * Returns exact occurrences for a primary word or a single-line selection.
 *
 * Collapsed cursors highlight only complete Unicode word segments, while an
 * explicit selection is treated as literal text. The common result owns no
 * presentation or selection mutation.
 */
export function getOccurrenceHighlightRanges(model: TextModel, selections: TextSelectionSet, wordPattern?: RegExp): readonly TextRange[] {
	const source = readOccurrenceSource(model, selections, wordPattern);
	if (!source) return Object.freeze([]);
	const matches = findTextMatches(model, {
		pattern: source.text,
		matchCase: true,
		wholeWord: source.wholeWord && !wordPattern,
	}, { resultLimit: MAX_OCCURRENCE_HIGHLIGHTS });
	return Object.freeze(matches.flatMap(match => wordPattern && source.wholeWord && !isPatternWord(model, match.range, wordPattern) ? [] : [match.range]));
}

function readOccurrenceSource(model: TextModel, selections: TextSelectionSet, wordPattern: RegExp | undefined): { readonly text: string; readonly wholeWord: boolean } | undefined {
	const selection = selections.primary;
	if (!selectionFitsModel(model, selection.range)) return undefined;
	if (!selection.collapsed) {
		if (selection.range.start.lineIndex !== selection.range.end.lineIndex) return undefined;
		const text = model.getTextInRange(selection.range);
		return text.length > 0 ? Object.freeze({ text, wholeWord: false }) : undefined;
	}
	const range = getWordSelectionRange(model, selection.active, wordPattern);
	if (range.empty) return undefined;
	const segment = wordPattern ? { wordLike: true } : getTextWordSegments(model.getLineContent(selection.active.lineIndex)).find(candidate =>
		candidate.start === range.start.columnIndex && candidate.end === range.end.columnIndex
	);
	if (!segment?.wordLike) return undefined;
	return Object.freeze({ text: model.getTextInRange(range), wholeWord: true });
}

/**
 * A model event can reach a presentation listener before its selection owner
 * installs the command's post-edit selection. Treat that transient snapshot as
 * having no occurrence source; the following selection event recomputes it.
 */
function selectionFitsModel(model: TextModel, range: TextRange): boolean {
	return positionFitsModel(model, range.start.lineIndex, range.start.columnIndex) &&
		positionFitsModel(model, range.end.lineIndex, range.end.columnIndex);
}

function positionFitsModel(model: TextModel, lineIndex: number, columnIndex: number): boolean {
	return Number.isSafeInteger(lineIndex) &&
		Number.isSafeInteger(columnIndex) &&
		lineIndex >= 0 &&
		columnIndex >= 0 &&
		lineIndex < model.lineCount &&
		columnIndex <= model.getLineLength(lineIndex);
}

function isPatternWord(model: TextModel, range: TextRange, wordPattern: RegExp): boolean {
	const selected = getWordSelectionRange(model, range.start, wordPattern);
	return selected.start.compareTo(range.start) === 0 && selected.end.compareTo(range.end) === 0;
}

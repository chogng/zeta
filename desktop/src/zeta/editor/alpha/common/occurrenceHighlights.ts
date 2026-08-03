import { TextRange } from "./text.js";
import { type TextModel } from "./textModel.js";
import { findTextMatches } from "./textModelSearch.js";
import { getTextWordSegments } from "./textSegmentation.js";
import { type TextSelectionSet } from "./selection.js";
import { getWordSelectionRange } from "./wordBoundary.js";

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

function isPatternWord(model: TextModel, range: TextRange, wordPattern: RegExp): boolean {
  const selected = getWordSelectionRange(model, range.start, wordPattern);
  return selected.start.compareTo(range.start) === 0 && selected.end.compareTo(range.end) === 0;
}

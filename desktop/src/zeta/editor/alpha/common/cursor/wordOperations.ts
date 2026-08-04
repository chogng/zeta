import { TextPosition, TextRange } from "../core/text.js";
import { type TextModel } from "../model/textModel.js";
import { getTextWordRanges, getWordSelectionRange } from "./wordBoundary.js";

/** Returns the word containing a position, or the nearest word to an empty line. */
export function getWordRangeAtPosition(model: TextModel, position: TextPosition, wordPattern?: RegExp): TextRange {
  return getWordSelectionRange(model, position, wordPattern);
}

/** Moves to the previous language word boundary without crossing the document start. */
export function previousWordPosition(model: TextModel, position: TextPosition, wordPattern?: RegExp): TextPosition {
  model.offsetAt(position);
  for (let lineIndex = position.lineIndex; lineIndex >= 0; lineIndex -= 1) {
    const limit = lineIndex === position.lineIndex ? position.columnIndex : Number.POSITIVE_INFINITY;
    const ranges = getTextWordRanges(model.getLineContent(lineIndex), wordPattern);
    for (let index = ranges.length - 1; index >= 0; index -= 1) {
      if (ranges[index]!.start < limit) return TextPosition.at(lineIndex, ranges[index]!.start);
    }
  }
  return TextPosition.at(0, 0);
}

/** Moves to the next language word boundary without crossing the document end. */
export function nextWordPosition(model: TextModel, position: TextPosition, wordPattern?: RegExp): TextPosition {
  model.offsetAt(position);
  for (let lineIndex = position.lineIndex; lineIndex < model.lineCount; lineIndex += 1) {
    const limit = lineIndex === position.lineIndex ? position.columnIndex : -1;
    const range = getTextWordRanges(model.getLineContent(lineIndex), wordPattern).find(candidate => candidate.start > limit);
    if (range) return TextPosition.at(lineIndex, range.start);
  }
  const lineIndex = model.lineCount - 1;
  return TextPosition.at(lineIndex, model.getLineContent(lineIndex).length);
}

/** Returns a word-selection range for every selection, preserving direction. */
export function selectWords(model: TextModel, positions: readonly TextPosition[], wordPattern?: RegExp): readonly TextRange[] {
  return Object.freeze(positions.map(position => getWordSelectionRange(model, position, wordPattern)));
}

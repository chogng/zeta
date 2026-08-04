import { TextSelection } from "../../../common/core/selection.js";
import { TextPosition, TextRange } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { getWordSelectionRange } from "../../../common/cursor/wordBoundary.js";

/** Expands one selection through word, enclosing-pair, line, and document scopes. */
export function expandSmartSelection(model: TextModel, selection: TextSelection, wordPattern?: RegExp): TextSelection {
  const current = selection.range;
  const next = expandRange(model, current, wordPattern);
  return TextSelection.fromRange(next, selection.direction);
}

function expandRange(model: TextModel, range: TextRange, wordPattern?: RegExp): TextRange {
  if (range.empty) return getWordSelectionRange(model, range.start, wordPattern);
  const enclosing = findEnclosingPair(model, range);
  if (enclosing && !enclosing.equals(range)) return enclosing;
  const lineStart = TextPosition.at(range.start.lineIndex, 0);
  const lineEnd = TextPosition.at(range.end.lineIndex, model.getLineContent(range.end.lineIndex).length);
  const lineRange = TextRange.from(lineStart, lineEnd);
  if (!lineRange.equals(range)) return lineRange;
  return TextRange.from(TextPosition.at(0, 0), TextPosition.at(model.lineCount - 1, model.getLineContent(model.lineCount - 1).length));
}

function findEnclosingPair(model: TextModel, range: TextRange): TextRange | undefined {
  const pairs: readonly [string, string][] = [["(", ")"], ["[", "]"], ["{", "}"], ["\"", "\""], ["'", "'"]];
  const startOffset = model.offsetAt(range.start);
  const endOffset = model.offsetAt(range.end);
  const text = model.getText();
  let best: TextRange | undefined;
  for (const [open, close] of pairs) {
    let openOffset = text.lastIndexOf(open, Math.max(0, startOffset - 1));
    while (openOffset >= 0) {
      const closeOffset = text.indexOf(close, Math.max(openOffset + open.length, endOffset));
      if (closeOffset < 0) break;
      const candidate = TextRange.from(model.positionAt(openOffset), model.positionAt(closeOffset + close.length));
      if (candidate.containsRange(range) && (!best || candidate.length.lineCount < best.length.lineCount || candidate.length.columnCount < best.length.columnCount)) best = candidate;
      openOffset = text.lastIndexOf(open, openOffset - 1);
    }
  }
  return best;
}

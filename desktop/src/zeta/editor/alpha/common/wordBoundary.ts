import { TextPosition, TextRange } from "./text.js";
import { type TextModel } from "./textModel.js";
import { getTextWordSegments } from "./textSegmentation.js";

/**
 * Returns the complete text segment selected by a word-selection gesture.
 *
 * Word-like text, whitespace, and punctuation are all selectable segments.
 * The range never crosses a line and its UTF-16 boundaries never split a
 * Unicode code point. At end of line, the preceding segment is selected.
 */
export function getWordSelectionRange(model: TextModel, position: TextPosition): TextRange {
  model.offsetAt(position);
  const line = model.getLineContent(position.lineIndex);
  if (line.length === 0) return TextRange.emptyAt(position);
  const probe = position.columnIndex === line.length
    ? line.length - 1
    : position.columnIndex;
  const segment = getTextWordSegments(line).find(candidate =>
    probe >= candidate.start && probe < candidate.end
  );
  if (!segment) {
    throw new RangeError("Word-selection probe is outside the line");
  }
  return TextRange.from(
    TextPosition.at(position.lineIndex, segment.start),
    TextPosition.at(position.lineIndex, segment.end),
  );
}

import { TextSelection, TextSelectionSet } from "./selection.js";
import { TextPosition } from "./text.js";
import { type TextModel } from "./textModel.js";

/**
 * Expands each selection through its next physical line, matching VS Code's
 * repeated line-selection action.
 *
 * Every result is forward from the first selected line's start. A complete
 * non-final line ends at the next line's start so it includes its line break.
 */
export function expandLineSelections(model: TextModel, selections: TextSelectionSet): TextSelectionSet {
  const expanded = selections.selections.map(selection => {
    const start = TextPosition.at(selection.range.start.lineIndex, 0);
    const selectedEndLineIndex = selection.range.end.lineIndex;
    const end = selectedEndLineIndex === model.lineCount - 1
      ? TextPosition.at(selectedEndLineIndex, model.getLineContent(selectedEndLineIndex).length)
      : TextPosition.at(selectedEndLineIndex + 1, 0);
    return TextSelection.from(start, end);
  });
  return TextSelectionSet.withPrimary(expanded, selections.primaryIndex);
}

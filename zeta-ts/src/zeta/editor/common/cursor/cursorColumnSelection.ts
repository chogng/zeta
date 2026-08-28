import { TextSelection, TextSelectionSet } from "../core/selection.js";
import { TextPosition } from "../core/text.js";
import { type TextModel } from "../model/textModel.js";

/**
 * Creates a same-column selection per physical line between two editor positions.
 *
 * Columns are clamped independently at every line end. The selection model does
 * not invent virtual whitespace, so a short line can contribute a collapsed
 * selection while preserving the rectangular operation's line membership.
 */
export function createEditorColumnSelectionSet(model: TextModel, anchor: TextPosition, active: TextPosition): TextSelectionSet {
	model.offsetAt(anchor);
	model.offsetAt(active);
	const firstLineIndex = Math.min(anchor.lineIndex, active.lineIndex);
	const lastLineIndex = Math.max(anchor.lineIndex, active.lineIndex);
	const selections = Array.from({ length: lastLineIndex - firstLineIndex + 1 }, (_, offset) => {
		const lineIndex = firstLineIndex + offset;
		const lineLength = model.getLineContent(lineIndex).length;
		return TextSelection.from(
			TextPosition.at(lineIndex, Math.min(anchor.columnIndex, lineLength)),
			TextPosition.at(lineIndex, Math.min(active.columnIndex, lineLength)),
		);
	});
	return TextSelectionSet.withPrimary(selections, active.lineIndex - firstLineIndex);
}

import { Selection } from "../../../common/core/selection.js";
import { Position } from "../../../common/core/position.js";
import { type TextModel } from "../../../common/model/textModel.js";

/**
 * Expands each selection through its next physical line, matching VS Code's
 * repeated line-selection action.
 *
 * Every result is forward from the first selected line's start. A complete
 * non-final line ends at the next line's start so it includes its line break.
 */
export function expandLineSelections(model: TextModel, selections: readonly Selection[]): readonly Selection[] {
	const expanded = selections.map(selection => {
		const start = new Position(selection.startLineNumber, 1);
		const selectedEndLineNumber = selection.endLineNumber;
		const end = selectedEndLineNumber === model.lineCount
			? new Position(selectedEndLineNumber, model.getLineContent(selectedEndLineNumber).length + 1)
			: new Position(selectedEndLineNumber + 1, 1);
		return Selection.fromPositions(start, end);
	});
	return Object.freeze(expanded);
}

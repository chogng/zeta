import { EditorCommandHistoryMode, type EditorEditCommand } from "../../../common/commands/editorEditCommand.js";
import type { SelectionSet } from "../../../common/cursor/selectionSet.js";
import { Range } from "../../../common/core/range.js";
import { type TextModel } from "../../../common/model/textModel.js";

/** Builds the optional save-boundary edit that terminates a non-empty document with LF. */
export function createInsertFinalNewLineCommand(model: TextModel, selections: SelectionSet): EditorEditCommand | undefined {
	const snapshot = model.createSnapshot();
	if (snapshot.length === 0 || snapshot.getText().endsWith("\n")) return undefined;
	const selectionsAfter = selections.selections.map(selection => Object.freeze({
		anchorOffset: model.offsetAt(selection.getSelectionStart()),
		activeOffset: model.offsetAt(selection.getPosition()),
	}));
	return Object.freeze({
		edits: Object.freeze([{ range: Range.fromPositions(model.positionAt(snapshot.length)), text: "\n" }]),
		selectionsAfter: Object.freeze(selectionsAfter),
		primarySelectionIndex: selections.primaryIndex,
		historyMode: EditorCommandHistoryMode.Isolated,
	});
}

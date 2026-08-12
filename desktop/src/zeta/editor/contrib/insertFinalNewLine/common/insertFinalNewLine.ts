import { EditorCommandHistoryMode, type EditorEditCommand } from "../../../common/commands/editorEditCommand.js";
import { type TextSelectionSet } from "../../../common/core/selection.js";
import { TextRange } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";

/** Builds the optional save-boundary edit that terminates a non-empty document with LF. */
export function createInsertFinalNewLineCommand(model: TextModel, selections: TextSelectionSet): EditorEditCommand | undefined {
  const snapshot = model.createSnapshot();
  if (snapshot.length === 0 || snapshot.getText().endsWith("\n")) return undefined;
  const selectionsAfter = selections.selections.map(selection => Object.freeze({
    anchorOffset: model.offsetAt(selection.anchor),
    activeOffset: model.offsetAt(selection.active),
  }));
  return Object.freeze({
    edits: Object.freeze([{ range: TextRange.emptyAt(model.positionAt(snapshot.length)), text: "\n" }]),
    selectionsAfter: Object.freeze(selectionsAfter),
    primarySelectionIndex: selections.primaryIndex,
    historyMode: EditorCommandHistoryMode.Isolated,
  });
}

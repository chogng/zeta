import { EditorCommandHistoryMode, type EditorEditCommand } from "./editorEditCommand.js";
import { type TextSelectionSet } from "../core/selection.js";
import { normalizeTextLineEndings, type TextEdit } from "../core/text.js";
import { type TextModel } from "../model/textModel.js";

/** Converts current-version text edits and selections into one editor command. */
export function createEditorEditCommand(model: TextModel, selections: TextSelectionSet, edits: readonly TextEdit[], historyMode = EditorCommandHistoryMode.Isolated): EditorEditCommand | undefined {
  if (edits.length === 0) return undefined;
  const normalized = edits.map(edit => Object.freeze({ range: edit.range, text: normalizeTextLineEndings(edit.text) }));
  const offsets = normalized.map(edit => ({ start: model.offsetAt(edit.range.start), end: model.offsetAt(edit.range.end), length: edit.text.length }));
  for (let index = 1; index < offsets.length; index += 1) {
    if (offsets[index - 1]!.end > offsets[index]!.start) throw new RangeError("Editor edits must be ordered and non-overlapping");
  }
  const mapOffset = (offset: number): number => {
    let delta = 0;
    for (const current of offsets) {
      if (offset < current.start) break;
      if (offset <= current.end) return current.start + delta + current.length;
      delta += current.length - (current.end - current.start);
    }
    return offset + delta;
  };
  return Object.freeze({
    edits: Object.freeze(normalized),
    selectionsAfter: Object.freeze(selections.selections.map(selection => Object.freeze({ anchorOffset: mapOffset(model.offsetAt(selection.anchor)), activeOffset: mapOffset(model.offsetAt(selection.active)) }))),
    primarySelectionIndex: selections.primaryIndex,
    historyMode,
  });
}

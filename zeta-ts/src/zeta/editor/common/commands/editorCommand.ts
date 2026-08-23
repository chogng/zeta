import { EditorCommandHistoryMode, type EditorEditCommand } from "./editorEditCommand.js";
import { type TextSelectionSet } from "../core/selection.js";
import { normalizeTextLineEndings, type TextEdit, type TextRange } from "../core/text.js";
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

/** Adds current-version edits to an existing command while preserving its post-command selections and history mode. */
export function extendEditorEditCommand(model: TextModel, command: EditorEditCommand, additionalEdits: readonly TextEdit[]): EditorEditCommand {
  if (additionalEdits.length === 0) return command;
  const original = command.edits.map(edit => offsetEdit(model, edit));
  const additional = additionalEdits.map(edit => offsetEdit(model, edit));
  const combined = [...original, ...additional].sort(compareOffsetEdits);
  for (let index = 1; index < combined.length; index += 1) {
    const previous = combined[index - 1]!;
    const current = combined[index]!;
    if (previous.end > current.start || (previous.start === previous.end && previous.start === current.start)) throw new RangeError("Extended editor edits must be non-overlapping");
  }
  const additionalInOriginalResult = additional.map(edit => {
    const precedingDelta = original.reduce((delta, candidate) => candidate.end <= edit.start ? delta + candidate.text.length - (candidate.end - candidate.start) : delta, 0);
    return { start: edit.start + precedingDelta, end: edit.end + precedingDelta, text: edit.text };
  }).sort(compareOffsetEdits);
  return Object.freeze({
    edits: Object.freeze(combined.map(edit => Object.freeze({ range: edit.range, text: edit.text }))),
    selectionsAfter: Object.freeze(command.selectionsAfter.map(selection => Object.freeze({ anchorOffset: mapOffsetThroughEdits(selection.anchorOffset, additionalInOriginalResult), activeOffset: mapOffsetThroughEdits(selection.activeOffset, additionalInOriginalResult) }))),
    primarySelectionIndex: command.primarySelectionIndex,
    historyMode: command.historyMode,
  });
}

interface OffsetEditorEdit {
  readonly range: TextRange;
  readonly start: number;
  readonly end: number;
  readonly text: string;
}

function offsetEdit(model: TextModel, edit: TextEdit): OffsetEditorEdit {
  return { range: edit.range, start: model.offsetAt(edit.range.start), end: model.offsetAt(edit.range.end), text: normalizeTextLineEndings(edit.text) };
}

function compareOffsetEdits(left: { readonly start: number; readonly end: number }, right: { readonly start: number; readonly end: number }): number {
  return left.start - right.start || left.end - right.end;
}

function mapOffsetThroughEdits(offset: number, edits: readonly { readonly start: number; readonly end: number; readonly text: string }[]): number {
  let delta = 0;
  for (const edit of edits) {
    if (offset < edit.start) break;
    if (offset <= edit.end) return edit.start + delta + edit.text.length;
    delta += edit.text.length - (edit.end - edit.start);
  }
  return offset + delta;
}

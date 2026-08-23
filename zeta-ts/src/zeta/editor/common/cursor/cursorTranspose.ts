import { EditorCommandHistoryMode, type EditorEditCommand } from "../commands/editorEditCommand.js";
import { type TextSelectionSet } from "../core/selection.js";
import { TextPosition, TextRange, type TextEdit } from "../core/text.js";
import { type TextModel } from "../model/textModel.js";
import { getTextGraphemeBoundaries } from "../core/textSegmentation.js";

interface TransposeOperation {
  readonly selectionIndex: number;
  readonly startOffset: number;
  readonly endOffset: number;
  readonly edit: TextEdit;
}

/**
 * Swaps the two grapheme units immediately around each collapsed cursor.
 *
 * A cursor in the middle of a line moves right by one grapheme; a cursor at a
 * line end swaps the two preceding graphemes. Crossing a physical-line start
 * treats its preceding line break as the left unit, matching VS Code.
 */
export function createTransposeCharactersCommand(model: TextModel, selections: TextSelectionSet): EditorEditCommand | undefined {
  const candidates = selections.selections.flatMap((selection, selectionIndex) => {
    if (!selection.collapsed) return [];
    const operation = createTransposeOperation(model, selection.active, selectionIndex);
    return operation ? [operation] : [];
  });
  const operations = selectNonOverlappingOperations(candidates, selections.primaryIndex);
  if (operations.length === 0) return undefined;
  const operationBySelection = new Map(operations.map(operation => [operation.selectionIndex, operation]));
  return Object.freeze({
    edits: Object.freeze(operations.map(operation => operation.edit)),
    selectionsAfter: Object.freeze(selections.selections.map((selection, selectionIndex) => {
      const operation = operationBySelection.get(selectionIndex);
      const activeOffset = operation?.endOffset ?? model.offsetAt(selection.active);
      return Object.freeze({
        anchorOffset: operation ? activeOffset : model.offsetAt(selection.anchor),
        activeOffset,
      });
    })),
    primarySelectionIndex: selections.primaryIndex,
    historyMode: EditorCommandHistoryMode.Isolated,
  });
}

function createTransposeOperation(model: TextModel, position: TextPosition, selectionIndex: number): TransposeOperation | undefined {
  const line = model.getLineContent(position.lineIndex);
  const end = position.columnIndex === line.length ? position : nextGraphemePosition(model, position);
  const middle = previousGraphemePosition(model, end);
  const begin = previousGraphemePosition(model, middle);
  if (begin.compareTo(middle) === 0 || middle.compareTo(end) === 0) return undefined;
  const range = TextRange.from(begin, end);
  const left = model.getTextInRange(TextRange.from(begin, middle));
  const right = model.getTextInRange(TextRange.from(middle, end));
  return Object.freeze({
    selectionIndex,
    startOffset: model.offsetAt(begin),
    endOffset: model.offsetAt(end),
    edit: Object.freeze({ range, text: `${right}${left}` }),
  });
}

function selectNonOverlappingOperations(candidates: readonly TransposeOperation[], primarySelectionIndex: number): readonly TransposeOperation[] {
  const selected: TransposeOperation[] = [];
  for (const candidate of [...candidates].sort((left, right) => left.startOffset - right.startOffset || left.endOffset - right.endOffset || left.selectionIndex - right.selectionIndex)) {
    const overlapIndex = selected.findIndex(existing =>
      candidate.startOffset < existing.endOffset && existing.startOffset < candidate.endOffset
    );
    if (overlapIndex < 0) {
      selected.push(candidate);
      continue;
    }
    if (candidate.selectionIndex === primarySelectionIndex && selected[overlapIndex]!.selectionIndex !== primarySelectionIndex) {
      selected[overlapIndex] = candidate;
    }
  }
  return Object.freeze(selected.sort((left, right) => left.startOffset - right.startOffset || left.endOffset - right.endOffset));
}

function previousGraphemePosition(model: TextModel, position: TextPosition): TextPosition {
  if (position.columnIndex === 0) {
    if (position.lineIndex === 0) return position;
    const previousLineIndex = position.lineIndex - 1;
    return TextPosition.at(previousLineIndex, model.getLineContent(previousLineIndex).length);
  }
  const boundaries = getTextGraphemeBoundaries(model.getLineContent(position.lineIndex));
  for (let index = boundaries.length - 1; index >= 0; index -= 1) {
    const boundary = boundaries[index]!;
    if (boundary < position.columnIndex) return TextPosition.at(position.lineIndex, boundary);
  }
  return TextPosition.at(position.lineIndex, 0);
}

function nextGraphemePosition(model: TextModel, position: TextPosition): TextPosition {
  const line = model.getLineContent(position.lineIndex);
  if (position.columnIndex === line.length) {
    return position.lineIndex + 1 < model.lineCount
      ? TextPosition.at(position.lineIndex + 1, 0)
      : position;
  }
  for (const boundary of getTextGraphemeBoundaries(line)) {
    if (boundary > position.columnIndex) return TextPosition.at(position.lineIndex, boundary);
  }
  return position;
}

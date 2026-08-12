import { EditorCommandHistoryMode, type EditorEditCommand, type TextSelectionOffsets } from "../../../common/commands/editorEditCommand.js";
import { TextSelectionSet } from "../../../common/core/selection.js";
import { TextPosition, TextRange, type TextEdit } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";

interface JoinSelection {
  readonly start: TextPosition;
  readonly end: TextPosition;
  readonly containsPrimary: boolean;
}

interface JoinOperation {
  readonly selection: JoinSelection;
  readonly range: TextRange;
  readonly startOffset: number;
  readonly endOffset: number;
  readonly replacement: string;
  readonly resultStartColumn: number;
  readonly resultEndColumn: number;
}

/**
 * Joins the physical lines covered by each cursor or selection in one isolated
 * edit. Leading indentation on subsequent non-empty lines is removed and one
 * separating space is retained when both adjacent fragments contain text.
 */
export function createJoinLinesCommand(model: TextModel, selections: TextSelectionSet): EditorEditCommand {
  const reduced = reduceJoinSelections(selections);
  const operations = reduced.map(selection => createJoinOperation(model, selection));
  if (operations.every(operation => operation.startOffset === operation.endOffset)) {
    return unchangedCommand(model, selections);
  }

  const edits: TextEdit[] = [];
  const selectionsAfter: TextSelectionOffsets[] = [];
  let primarySelectionIndex = 0;
  let cumulativeDelta = 0;
  for (const operation of operations) {
    const resultOffset = operation.startOffset + cumulativeDelta;
    selectionsAfter.push(Object.freeze({
      anchorOffset: resultOffset + operation.resultStartColumn,
      activeOffset: resultOffset + operation.resultEndColumn,
    }));
    if (operation.selection.containsPrimary) {
      primarySelectionIndex = selectionsAfter.length - 1;
    }
    if (operation.startOffset !== operation.endOffset) {
      edits.push(Object.freeze({
        range: operation.range,
        text: operation.replacement,
      }));
      cumulativeDelta += operation.replacement.length - (operation.endOffset - operation.startOffset);
    }
  }
  return Object.freeze({
    edits: Object.freeze(edits),
    selectionsAfter: Object.freeze(selectionsAfter),
    primarySelectionIndex,
    historyMode: EditorCommandHistoryMode.Isolated,
  });
}

function reduceJoinSelections(selections: TextSelectionSet): readonly JoinSelection[] {
  const ordered = selections.selections.map((selection, index) => Object.freeze({
    start: selection.range.start,
    end: selection.range.end,
    collapsed: selection.collapsed,
    containsPrimary: index === selections.primaryIndex,
  })).sort((left, right) => left.start.compareTo(right.start) || left.end.compareTo(right.end));
  const reduced: JoinSelection[] = [];
  for (const current of ordered) {
    const previous = reduced.at(-1);
    if (!previous) {
      reduced.push(Object.freeze(current));
      continue;
    }
    const previousCollapsed = previous.start.compareTo(previous.end) === 0;
    if (previousCollapsed && previous.end.lineIndex === current.start.lineIndex) {
      reduced[reduced.length - 1] = Object.freeze({
        start: current.start,
        end: current.end,
        containsPrimary: previous.containsPrimary || current.containsPrimary,
      });
      continue;
    }
    const separated = previousCollapsed
      ? current.start.lineIndex > previous.end.lineIndex + 1
      : current.start.lineIndex > previous.end.lineIndex;
    if (separated) {
      reduced.push(Object.freeze(current));
      continue;
    }
    reduced[reduced.length - 1] = Object.freeze({
      start: previous.start,
      end: current.end,
      containsPrimary: previous.containsPrimary || current.containsPrimary,
    });
  }
  return Object.freeze(reduced);
}

function createJoinOperation(model: TextModel, selection: JoinSelection): JoinOperation {
  const joinsFollowingLine = selection.start.lineIndex === selection.end.lineIndex;
  const endLineIndex = joinsFollowingLine
    ? Math.min(selection.start.lineIndex + 1, model.lineCount - 1)
    : selection.end.lineIndex;
  if (endLineIndex === selection.start.lineIndex) {
    const lineStart = TextPosition.at(selection.start.lineIndex, 0);
    const startOffset = model.offsetAt(lineStart);
    return Object.freeze({
      selection,
      range: TextRange.emptyAt(lineStart),
      startOffset,
      endOffset: startOffset,
      replacement: "",
      resultStartColumn: selection.start.columnIndex,
      resultEndColumn: selection.end.columnIndex,
    });
  }
  const end = TextPosition.at(endLineIndex, model.getLineContent(endLineIndex).length);
  const joined = joinLineContents(model, selection.start.lineIndex, endLineIndex);
  const selectionEndOffset = model.getLineContent(selection.end.lineIndex).length - selection.end.columnIndex;
  const endColumn = joinsFollowingLine
    ? joined.text.length - joined.finalSegmentLength
    : joined.text.length - selectionEndOffset;
  return Object.freeze({
    selection: Object.freeze({ ...selection, end }),
    range: TextRange.from(TextPosition.at(selection.start.lineIndex, 0), end),
    startOffset: model.offsetAt(TextPosition.at(selection.start.lineIndex, 0)),
    endOffset: model.offsetAt(end),
    replacement: joined.text,
    resultStartColumn: joinsFollowingLine ? endColumn : selection.start.columnIndex,
    resultEndColumn: endColumn,
  });
}

function joinLineContents(model: TextModel, startLineIndex: number, endLineIndex: number): { readonly text: string; readonly finalSegmentLength: number } {
  let text = model.getLineContent(startLineIndex);
  let finalSegmentLength = 0;
  for (let lineIndex = startLineIndex + 1; lineIndex <= endLineIndex; lineIndex += 1) {
    const nextLine = model.getLineContent(lineIndex);
    const trimmed = nextLine.replace(/^[\s\uFEFF\xA0]+/u, "");
    if (trimmed.length === 0) {
      finalSegmentLength = 0;
      continue;
    }
    let insertSpace = text.length > 0;
    if (insertSpace && /[\s\uFEFF\xA0]$/u.test(text)) {
      insertSpace = false;
      text = text.replace(/[\s\uFEFF\xA0]+$/u, " ");
    }
    text += `${insertSpace ? " " : ""}${trimmed}`;
    finalSegmentLength = trimmed.length + (insertSpace ? 1 : 0);
  }
  return Object.freeze({ text, finalSegmentLength });
}

function unchangedCommand(model: TextModel, selections: TextSelectionSet): EditorEditCommand {
  return Object.freeze({
    edits: Object.freeze([]),
    selectionsAfter: Object.freeze(selections.selections.map(selection => Object.freeze({
      anchorOffset: model.offsetAt(selection.anchor),
      activeOffset: model.offsetAt(selection.active),
    }))),
    primarySelectionIndex: selections.primaryIndex,
    historyMode: EditorCommandHistoryMode.Isolated,
  });
}

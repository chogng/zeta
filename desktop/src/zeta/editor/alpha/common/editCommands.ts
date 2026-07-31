import { EditorEmptySelectionClipboardPolicy, getEditorClipboardEntries } from "./clipboard.js";
import { EditorCommandHistoryMode, type EditorEditCommand, type TextSelectionOffsets } from "./editorSelectionController.js";
import { type TextSelectionSet } from "./selection.js";
import { normalizeTextLineEndings, TextPosition, TextRange, type TextEdit } from "./text.js";
import { type TextModel } from "./textModel.js";
import { getTextGraphemeBoundaries } from "./textSegmentation.js";

interface SelectionReplacement {
  readonly selectionIndex: number;
  readonly range: TextRange;
  readonly startOffset: number;
  readonly endOffset: number;
  readonly text: string;
  readonly anchorOffsetInText: number;
  readonly activeOffsetInText: number;
}

export interface EditorSelectionEdit {
  readonly range: TextRange;
  readonly text: string;
  readonly anchorOffsetInText: number;
  readonly activeOffsetInText: number;
}

/** Replaces every selection with text and places each caret after its insert. */
export function createTypeTextCommand(model: TextModel, selections: TextSelectionSet, text: string): EditorEditCommand {
  if (typeof text !== "string") {
    throw new TypeError("Typed text must be a string");
  }
  const normalized = normalizeTextLineEndings(text);
  return buildSelectionEditCommand(
    model,
    selections,
    selections.selections.map((selection, selectionIndex) => replacement(
      model,
      selectionIndex,
      selection.range,
      normalized,
      normalized.length,
    )),
    EditorCommandHistoryMode.CoalesceTyping,
  );
}

/** Replaces every selection with the same pasted text as an isolated undo step. */
export function createPasteTextCommand(model: TextModel, selections: TextSelectionSet, text: string): EditorEditCommand {
  if (typeof text !== "string") {
    throw new TypeError("Pasted text must be a string");
  }
  const normalized = normalizeTextLineEndings(text);
  return buildSelectionEditCommand(
    model,
    selections,
    selections.selections.map((selection, selectionIndex) => replacement(
      model,
      selectionIndex,
      selection.range,
      normalized,
      normalized.length,
    )),
    EditorCommandHistoryMode.Isolated,
  );
}

/** Replaces each selection with its corresponding pasted text. */
export function createDistributedPasteTextCommand(model: TextModel, selections: TextSelectionSet, texts: readonly string[]): EditorEditCommand {
  if (texts.length !== selections.selections.length) {
    throw new RangeError("Distributed paste text must match the selection count");
  }
  const normalized = texts.map(text => {
    if (typeof text !== "string") {
      throw new TypeError("Distributed paste text must contain only strings");
    }
    return normalizeTextLineEndings(text);
  });
  return buildSelectionEditCommand(
    model,
    selections,
    selections.selections.map((selection, selectionIndex) => replacement(
      model,
      selectionIndex,
      selection.range,
      normalized[selectionIndex]!,
      normalized[selectionIndex]!.length,
    )),
    EditorCommandHistoryMode.Isolated,
  );
}

/** Deletes only non-empty selections as one isolated cut transaction. */
export function createCutCommand(model: TextModel, selections: TextSelectionSet): EditorEditCommand {
  return buildSelectionEditCommand(
    model,
    selections,
    selections.selections.map((selection, selectionIndex) => replacement(
      model,
      selectionIndex,
      selection.range,
      "",
      0,
    )),
    EditorCommandHistoryMode.Isolated,
  );
}

/** Cuts selected text and optional complete lines as one isolated transaction. */
export function createClipboardCutCommand(model: TextModel, selections: TextSelectionSet, emptySelectionPolicy: EditorEmptySelectionClipboardPolicy): EditorEditCommand {
  const entries = getEditorClipboardEntries(
    model,
    selections,
    emptySelectionPolicy,
  );
  const sourceRanges = mergeDeletionRanges(
    model,
    entries.map(entry => entry.sourceRange),
  );
  const selectionsAfter = normalizeSelectionsAfter(
    entries.map(entry => {
      const targetOffset = mapOffsetThroughDeletions(
        model.offsetAt(entry.sourceRange.start),
        sourceRanges,
      );
      return {
        anchorOffset: targetOffset,
        activeOffset: targetOffset,
      };
    }),
    selections.primaryIndex,
  );
  return {
    edits: Object.freeze(sourceRanges.map(range => ({ range: range.range, text: "" }))),
    selectionsAfter: selectionsAfter.selections,
    primarySelectionIndex: selectionsAfter.primaryIndex,
    historyMode: EditorCommandHistoryMode.Isolated,
  };
}

/** Inserts one complete-line clipboard text at every collapsed target line. */
export function createLinePasteCommand(model: TextModel, selections: TextSelectionSet, texts: readonly string[]): EditorEditCommand {
  if (texts.length !== selections.selections.length) {
    throw new RangeError("Line paste text must match the selection count");
  }
  const normalized = texts.map(text => {
    if (typeof text !== "string") {
      throw new TypeError("Line paste text must contain only strings");
    }
    const value = normalizeTextLineEndings(text);
    if (!value.endsWith("\n")) {
      throw new RangeError("Line paste text must end with a line break");
    }
    return value;
  });
  const groups = new Map<number, {
    readonly lineIndex: number;
    readonly selectionIndices: number[];
    text: string;
  }>();
  for (let selectionIndex = 0; selectionIndex < selections.selections.length; selectionIndex += 1) {
    const selection = selections.selections[selectionIndex]!;
    if (!selection.collapsed) {
      throw new RangeError("Line paste requires collapsed selections");
    }
    const lineIndex = selection.active.lineIndex;
    let group = groups.get(lineIndex);
    if (!group) {
      group = { lineIndex, selectionIndices: [], text: "" };
      groups.set(lineIndex, group);
    }
    group.selectionIndices.push(selectionIndex);
    group.text += normalized[selectionIndex]!;
  }
  const sorted = [...groups.values()].sort((left, right) =>
    left.lineIndex - right.lineIndex
  );
  const selectionsAfter = new Array<TextSelectionOffsets>(
    selections.selections.length,
  );
  const edits: TextEdit[] = [];
  let cumulativeDelta = 0;
  for (const group of sorted) {
    const position = TextPosition.at(group.lineIndex, 0);
    const startOffset = model.offsetAt(position);
    edits.push({ range: TextRange.emptyAt(position), text: group.text });
    for (const selectionIndex of group.selectionIndices) {
      const column = selections.selections[selectionIndex]!.active.columnIndex;
      const caretOffset = startOffset +
        cumulativeDelta +
        group.text.length +
        column;
      selectionsAfter[selectionIndex] = {
        anchorOffset: caretOffset,
        activeOffset: caretOffset,
      };
    }
    cumulativeDelta += group.text.length;
  }
  const normalizedSelections = normalizeSelectionsAfter(
    selectionsAfter,
    selections.primaryIndex,
  );
  return {
    edits: Object.freeze(edits),
    selectionsAfter: normalizedSelections.selections,
    primarySelectionIndex: normalizedSelections.primaryIndex,
    historyMode: EditorCommandHistoryMode.Isolated,
  };
}

/** Deletes each selection or the preceding grapheme/newline. */
export function createBackspaceCommand(model: TextModel, selections: TextSelectionSet): EditorEditCommand {
  return buildSelectionEditCommand(
    model,
    selections,
    selections.selections.map((selection, selectionIndex) => {
      const range = selection.collapsed
        ? getPreviousDeleteRange(model, selection.active)
        : selection.range;
      return replacement(model, selectionIndex, range, "", 0);
    }),
    EditorCommandHistoryMode.CoalesceBackspace,
  );
}

/** Deletes each selection or the following grapheme/newline. */
export function createDeleteForwardCommand(model: TextModel, selections: TextSelectionSet): EditorEditCommand {
  return buildSelectionEditCommand(
    model,
    selections,
    selections.selections.map((selection, selectionIndex) => {
      const range = selection.collapsed
        ? nextDeleteRange(model, selection.active)
        : selection.range;
      return replacement(model, selectionIndex, range, "", 0);
    }),
    EditorCommandHistoryMode.CoalesceDelete,
  );
}

/** Builds one validated multi-selection command from pre-change replacement scripts. */
export function createSelectionEditCommand(model: TextModel, selections: TextSelectionSet, edits: readonly EditorSelectionEdit[], historyMode: EditorCommandHistoryMode): EditorEditCommand {
  if (!Array.isArray(edits) || edits.length !== selections.selections.length) {
    throw new RangeError("Selection edits must match the selection count");
  }
  return buildSelectionEditCommand(
    model,
    selections,
    edits.map((edit, selectionIndex) => {
      if (typeof edit !== "object" || edit === null || typeof edit.text !== "string") {
        throw new TypeError("Selection edit must contain replacement text");
      }
      if (!Number.isSafeInteger(edit.anchorOffsetInText) || edit.anchorOffsetInText < 0 || !Number.isSafeInteger(edit.activeOffsetInText) || edit.activeOffsetInText < 0) {
        throw new RangeError("Selection edit result offsets must be non-negative safe integers");
      }
      return {
        selectionIndex,
        range: edit.range,
        startOffset: model.offsetAt(edit.range.start),
        endOffset: model.offsetAt(edit.range.end),
        text: edit.text,
        anchorOffsetInText: edit.anchorOffsetInText,
        activeOffsetInText: edit.activeOffsetInText,
      };
    }),
    historyMode,
  );
}

function buildSelectionEditCommand(
  model: TextModel,
  selections: TextSelectionSet,
  replacements: readonly SelectionReplacement[],
  historyMode: EditorCommandHistoryMode,
): EditorEditCommand {
  const sorted = [...replacements].sort((left, right) =>
    left.startOffset - right.startOffset ||
    left.endOffset - right.endOffset ||
    left.selectionIndex - right.selectionIndex
  );
  validateNonOverlapping(sorted);
  const selectionsAfter = new Array<{
    readonly anchorOffset: number;
    readonly activeOffset: number;
  }>(selections.selections.length);
  const edits: TextEdit[] = [];
  let cumulativeDelta = 0;
  for (const item of sorted) {
    selectionsAfter[item.selectionIndex] = {
      anchorOffset: item.startOffset + cumulativeDelta + item.anchorOffsetInText,
      activeOffset: item.startOffset + cumulativeDelta + item.activeOffsetInText,
    };
    if (item.startOffset !== item.endOffset || item.text.length > 0) {
      edits.push({ range: item.range, text: item.text });
    }
    cumulativeDelta +=
      item.text.length -
      (item.endOffset - item.startOffset);
  }
  const normalizedSelections = normalizeSelectionsAfter(
    selectionsAfter,
    selections.primaryIndex,
  );
  return {
    edits: Object.freeze(edits),
    selectionsAfter: normalizedSelections.selections,
    primarySelectionIndex: normalizedSelections.primaryIndex,
    historyMode,
  };
}

function normalizeSelectionsAfter(selections: readonly TextSelectionOffsets[], primaryIndex: number): {
  readonly selections: readonly TextSelectionOffsets[];
  readonly primaryIndex: number;
} {
  const normalized: TextSelectionOffsets[] = [];
  const sourceToNormalized: number[] = [];
  for (const selection of selections) {
    let targetIndex = normalized.findIndex(candidate =>
      candidate.anchorOffset === selection.anchorOffset &&
      candidate.activeOffset === selection.activeOffset
    );
    if (targetIndex < 0) {
      targetIndex = normalized.length;
      normalized.push(selection);
    }
    sourceToNormalized.push(targetIndex);
  }
  return {
    selections: Object.freeze(normalized),
    primaryIndex: sourceToNormalized[primaryIndex]!,
  };
}

function replacement(
  model: TextModel,
  selectionIndex: number,
  range: TextRange,
  text: string,
  caretOffsetInText: number,
): SelectionReplacement {
  return {
    selectionIndex,
    range,
    startOffset: model.offsetAt(range.start),
    endOffset: model.offsetAt(range.end),
    text,
    anchorOffsetInText: caretOffsetInText,
    activeOffsetInText: caretOffsetInText,
  };
}

export function getPreviousDeleteRange(model: TextModel, position: TextPosition): TextRange {
  if (position.columnIndex > 0) {
    const boundaries = getTextGraphemeBoundaries(
      model.getLineContent(position.lineIndex),
    );
    return TextRange.from(
      TextPosition.at(
        position.lineIndex,
        previousBoundary(boundaries, position.columnIndex),
      ),
      position,
    );
  }
  if (position.lineIndex === 0) return TextRange.emptyAt(position);
  const previousLineIndex = position.lineIndex - 1;
  return TextRange.from(
    TextPosition.at(
      previousLineIndex,
      model.getLineContent(previousLineIndex).length,
    ),
    position,
  );
}

function nextDeleteRange(model: TextModel, position: TextPosition): TextRange {
  const line = model.getLineContent(position.lineIndex);
  if (position.columnIndex < line.length) {
    const boundaries = getTextGraphemeBoundaries(line);
    return TextRange.from(
      position,
      TextPosition.at(
        position.lineIndex,
        nextBoundary(boundaries, position.columnIndex),
      ),
    );
  }
  if (position.lineIndex + 1 >= model.lineCount) {
    return TextRange.emptyAt(position);
  }
  return TextRange.from(
    position,
    TextPosition.at(position.lineIndex + 1, 0),
  );
}

function validateNonOverlapping(replacements: readonly SelectionReplacement[]): void {
  for (let index = 1; index < replacements.length; index += 1) {
    const previous = replacements[index - 1]!;
    const current = replacements[index]!;
    const ambiguousSharedStart =
      current.startOffset === previous.startOffset &&
      (
        current.startOffset === current.endOffset ||
        previous.startOffset === previous.endOffset
      );
    if (
      current.startOffset < previous.endOffset ||
      ambiguousSharedStart
    ) {
      throw new RangeError(
        "Selections must not overlap when creating an edit command",
      );
    }
  }
}

function previousBoundary(boundaries: readonly number[], column: number): number {
  for (let index = boundaries.length - 1; index >= 0; index -= 1) {
    if (boundaries[index]! < column) return boundaries[index]!;
  }
  return 0;
}

function nextBoundary(boundaries: readonly number[], column: number): number {
  return boundaries.find(boundary => boundary > column) ??
    boundaries[boundaries.length - 1]!;
}

interface OffsetDeletionRange {
  readonly range: TextRange;
  readonly startOffset: number;
  readonly endOffset: number;
}

function mergeDeletionRanges(model: TextModel, ranges: readonly TextRange[]): readonly OffsetDeletionRange[] {
  const sorted = ranges.map(range => ({
    startOffset: model.offsetAt(range.start),
    endOffset: model.offsetAt(range.end),
  })).filter(range => range.startOffset !== range.endOffset).sort((left, right) =>
    left.startOffset - right.startOffset ||
    left.endOffset - right.endOffset
  );
  const merged: Array<{ startOffset: number; endOffset: number }> = [];
  for (const range of sorted) {
    const previous = merged[merged.length - 1];
    if (previous && range.startOffset <= previous.endOffset) {
      previous.endOffset = Math.max(previous.endOffset, range.endOffset);
    } else {
      merged.push({ ...range });
    }
  }
  return Object.freeze(merged.map(range => Object.freeze({
    ...range,
    range: TextRange.from(
      model.positionAt(range.startOffset),
      model.positionAt(range.endOffset),
    ),
  })));
}

function mapOffsetThroughDeletions(offset: number, ranges: readonly OffsetDeletionRange[]): number {
  let delta = 0;
  for (const range of ranges) {
    if (offset < range.startOffset) break;
    if (offset <= range.endOffset) return range.startOffset + delta;
    delta -= range.endOffset - range.startOffset;
  }
  return offset + delta;
}

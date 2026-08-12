import { EditorCommandHistoryMode, type EditorEditCommand } from "../../../common/commands/editorEditCommand.js";
import { type TextSelectionSet } from "../../../common/core/selection.js";
import { TextPosition, TextRange, type TextEdit } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";

export interface EditorLineCommentOptions {
  readonly lineComment: string;
  readonly insertSpace?: boolean;
}

interface OffsetEdit {
  readonly startOffset: number;
  readonly endOffset: number;
  readonly text: string;
  readonly edit: TextEdit;
}

/** Toggles one language line-comment token over every selected physical line. */
export function createToggleLineCommentCommand(model: TextModel, selections: TextSelectionSet, options: EditorLineCommentOptions): EditorEditCommand {
  const lineComment = readLineComment(options);
  const lineIndices = selectedLineIndices(selections);
  const remove = shouldRemoveLineComments(model, lineIndices, lineComment);
  const edits = lineIndices.flatMap<OffsetEdit>(lineIndex => {
    const line = model.getLineContent(lineIndex);
    const leadingWhitespaceLength = leadingWhitespace(line).length;
    const position = TextPosition.at(lineIndex, leadingWhitespaceLength);
    const startOffset = model.offsetAt(position);
    if (remove) {
      if (!line.startsWith(lineComment, leadingWhitespaceLength)) return [];
      const followingSpace = line.startsWith(" ", leadingWhitespaceLength + lineComment.length) ? 1 : 0;
      const endColumn = leadingWhitespaceLength + lineComment.length + followingSpace;
      return [{
        startOffset,
        endOffset: model.offsetAt(TextPosition.at(lineIndex, endColumn)),
        text: "",
        edit: Object.freeze({
          range: TextRange.from(position, TextPosition.at(lineIndex, endColumn)),
          text: "",
        }),
      }];
    }
    const hasContent = line.length > leadingWhitespaceLength;
    const text = lineComment + (options.insertSpace !== false && hasContent ? " " : "");
    return [{
      startOffset,
      endOffset: startOffset,
      text,
      edit: Object.freeze({ range: TextRange.emptyAt(position), text }),
    }];
  });
  return Object.freeze({
    edits: Object.freeze(edits.map(edit => edit.edit)),
    selectionsAfter: Object.freeze(selections.selections.map(selection => Object.freeze({
      anchorOffset: mapOffsetThroughEdits(model.offsetAt(selection.anchor), edits),
      activeOffset: mapOffsetThroughEdits(model.offsetAt(selection.active), edits),
    }))),
    primarySelectionIndex: selections.primaryIndex,
    historyMode: EditorCommandHistoryMode.Isolated,
  });
}

function shouldRemoveLineComments(model: TextModel, lineIndices: readonly number[], lineComment: string): boolean {
  const contentLines = lineIndices.filter(lineIndex => {
    const content = model.getLineContent(lineIndex);
    return content.trim().length > 0;
  });
  const candidates = contentLines.length > 0 ? contentLines : lineIndices;
  return candidates.length > 0 && candidates.every(lineIndex => {
    const content = model.getLineContent(lineIndex);
    return content.startsWith(lineComment, leadingWhitespace(content).length);
  });
}

function selectedLineIndices(selections: TextSelectionSet): readonly number[] {
  const indices = new Set<number>();
  for (const selection of selections.selections) {
    const range = selection.range;
    let endLineIndex = range.end.lineIndex;
    if (!selection.collapsed && range.end.columnIndex === 0 && endLineIndex > range.start.lineIndex) {
      endLineIndex -= 1;
    }
    for (let lineIndex = range.start.lineIndex; lineIndex <= endLineIndex; lineIndex += 1) indices.add(lineIndex);
  }
  return Object.freeze([...indices].sort((left, right) => left - right));
}

function leadingWhitespace(text: string): string {
  return /^[ \t]*/.exec(text)![0];
}

function mapOffsetThroughEdits(offset: number, edits: readonly OffsetEdit[]): number {
  let delta = 0;
  for (const edit of edits) {
    if (offset < edit.startOffset) break;
    if (edit.startOffset === edit.endOffset && offset === edit.startOffset) {
      return offset + delta + edit.text.length;
    }
    if (offset <= edit.endOffset) {
      return edit.startOffset + delta + Math.min(offset - edit.startOffset, edit.text.length);
    }
    delta += edit.text.length - (edit.endOffset - edit.startOffset);
  }
  return offset + delta;
}

function readLineComment(options: EditorLineCommentOptions): string {
  if (!options || typeof options !== "object" || typeof options.lineComment !== "string") {
    throw new TypeError("Line comment command requires a line comment token");
  }
  if (options.lineComment.length === 0 || /[\r\n]/.test(options.lineComment)) {
    throw new RangeError("Line comment token must be a non-empty single-line string");
  }
  if (options.insertSpace !== undefined && typeof options.insertSpace !== "boolean") {
    throw new TypeError("Line comment insertSpace must be a boolean");
  }
  return options.lineComment;
}

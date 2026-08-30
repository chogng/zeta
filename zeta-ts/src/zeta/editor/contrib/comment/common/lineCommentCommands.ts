import { EditorCommandHistoryMode, type EditorEditCommand } from "../../../common/commands/editorEditCommand.js";
import { type Selection } from "../../../common/core/selection.js";
import { Position } from "../../../common/core/position.js";
import { Range } from "../../../common/core/range.js";

import { type TextModel } from "../../../common/model/textModel.js";
import { type TextEdit } from '../../../common/languages.js';

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
export function createToggleLineCommentCommand(model: TextModel, selections: readonly Selection[], options: EditorLineCommentOptions): EditorEditCommand {
	const lineComment = readLineComment(options);
	const lineIndices = selectedLineIndices(selections);
	const remove = shouldRemoveLineComments(model, lineIndices, lineComment);
	const edits = lineIndices.flatMap<OffsetEdit>(lineIndex => {
		const line = model.getLineContent((lineIndex) + 1);
		const leadingWhitespaceLength = leadingWhitespace(line).length;
		const position = new Position((lineIndex) + 1, (leadingWhitespaceLength) + 1);
		const startOffset = model.offsetAt(position);
		if (remove) {
			if (!line.startsWith(lineComment, leadingWhitespaceLength)) return [];
			const followingSpace = line.startsWith(" ", leadingWhitespaceLength + lineComment.length) ? 1 : 0;
			const endColumn = leadingWhitespaceLength + lineComment.length + followingSpace;
			return [{
				startOffset,
				endOffset: model.offsetAt(new Position((lineIndex) + 1, (endColumn) + 1)),
				text: "",
				edit: Object.freeze({
					range: Range.fromPositions(position, new Position((lineIndex) + 1, (endColumn) + 1)),
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
			edit: Object.freeze({ range: Range.fromPositions(position), text }),
		}];
	});
	return Object.freeze({
		edits: Object.freeze(edits.map(edit => edit.edit)),
		selectionsAfter: Object.freeze(selections.map(selection => Object.freeze({
			anchorOffset: mapOffsetThroughEdits(model.offsetAt(selection.getSelectionStart()), edits),
			activeOffset: mapOffsetThroughEdits(model.offsetAt(selection.getPosition()), edits),
		}))),
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.Isolated,
	});
}

function shouldRemoveLineComments(model: TextModel, lineIndices: readonly number[], lineComment: string): boolean {
	const contentLines = lineIndices.filter(lineIndex => {
		const content = model.getLineContent((lineIndex) + 1);
		return content.trim().length > 0;
	});
	const candidates = contentLines.length > 0 ? contentLines : lineIndices;
	return candidates.length > 0 && candidates.every(lineIndex => {
		const content = model.getLineContent((lineIndex) + 1);
		return content.startsWith(lineComment, leadingWhitespace(content).length);
	});
}

function selectedLineIndices(selections: readonly Selection[]): readonly number[] {
	const indices = new Set<number>();
	for (const selection of selections) {
		const range = selection;
		let endLineIndex = range.endLineNumber - 1;
		if (!selection.isEmpty() && range.endColumn === 1 && endLineIndex > range.startLineNumber - 1) {
			endLineIndex -= 1;
		}
		for (let lineIndex = range.startLineNumber - 1; lineIndex <= endLineIndex; lineIndex += 1) indices.add(lineIndex);
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

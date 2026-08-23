import { getEditorIndentationUnit, resolveEditorIndentationOptions, type EditorIndentationOptions } from "../../../common/editorIndentation.js";
import { EditorCommandHistoryMode, type EditorEditCommand } from "../../../common/commands/editorEditCommand.js";
import { type TextSelectionSet } from "../../../common/core/selection.js";
import { TextPosition, TextRange, type TextEdit } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";

export enum EditorLineIndentDirection {
	Indent = "indent",
	Outdent = "outdent",
}

interface OffsetEdit {
	readonly startOffset: number;
	readonly endOffset: number;
	readonly text: string;
	readonly edit: TextEdit;
}

/** Indents or outdents the union of physical lines touched by the current selections. */
export function createLineIndentCommand(model: TextModel, selections: TextSelectionSet, direction: EditorLineIndentDirection, options: EditorIndentationOptions = {}): EditorEditCommand {
	if (!Object.values(EditorLineIndentDirection).includes(direction)) {
		throw new TypeError("Unknown editor line indentation direction");
	}
	const indentation = resolveEditorIndentationOptions(options);
	const lineIndices = selectedLineIndices(selections);
	const edits = lineIndices.flatMap<OffsetEdit>(lineIndex => {
		const lineStart = TextPosition.at(lineIndex, 0);
		const startOffset = model.offsetAt(lineStart);
		if (direction === EditorLineIndentDirection.Indent) {
			const text = getEditorIndentationUnit(indentation);
			return [{
				startOffset,
				endOffset: startOffset,
				text,
				edit: Object.freeze({ range: TextRange.emptyAt(lineStart), text }),
			}];
		}
		const content = model.getLineContent(lineIndex);
		const removableLength = content.startsWith("\t")
			? 1
			: Math.min(indentation.tabSize, /^[ ]*/.exec(content)![0].length);
		if (removableLength === 0) return [];
		return [{
			startOffset,
			endOffset: startOffset + removableLength,
			text: "",
			edit: Object.freeze({
				range: TextRange.from(lineStart, TextPosition.at(lineIndex, removableLength)),
				text: "",
			}),
		}];
	});
	const selectionsAfter = selections.selections.map(selection => Object.freeze({
		anchorOffset: mapOffsetThroughEdits(model.offsetAt(selection.anchor), edits),
		activeOffset: mapOffsetThroughEdits(model.offsetAt(selection.active), edits),
	}));
	return Object.freeze({
		edits: Object.freeze(edits.map(edit => edit.edit)),
		selectionsAfter: Object.freeze(selectionsAfter),
		primarySelectionIndex: selections.primaryIndex,
		historyMode: EditorCommandHistoryMode.Isolated,
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

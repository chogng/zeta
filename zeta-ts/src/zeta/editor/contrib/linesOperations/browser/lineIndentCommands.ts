import { getEditorIndentationUnit, resolveEditorIndentationOptions, type EditorIndentationOptions } from "../../../common/core/misc/indentation.js";
import { EditorCommandHistoryMode, type EditorEditCommand } from "../../../common/commands/editorEditCommand.js";
import type { SelectionSet } from "../../../common/cursor/selectionSet.js";
import { Position } from "../../../common/core/position.js";
import { Range } from "../../../common/core/range.js";

import { type TextModel } from "../../../common/model/textModel.js";
import { type TextEdit } from '../../../common/languages.js';

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
export function createLineIndentCommand(model: TextModel, selections: SelectionSet, direction: EditorLineIndentDirection, options: EditorIndentationOptions = {}): EditorEditCommand {
	if (!Object.values(EditorLineIndentDirection).includes(direction)) {
		throw new TypeError("Unknown editor line indentation direction");
	}
	const indentation = resolveEditorIndentationOptions(options);
	const lineIndices = selectedLineIndices(selections);
	const edits = lineIndices.flatMap<OffsetEdit>(lineIndex => {
		const lineStart = new Position((lineIndex) + 1, (0) + 1);
		const startOffset = model.offsetAt(lineStart);
		if (direction === EditorLineIndentDirection.Indent) {
			const text = getEditorIndentationUnit(indentation);
			return [{
				startOffset,
				endOffset: startOffset,
				text,
				edit: Object.freeze({ range: Range.fromPositions(lineStart), text }),
			}];
		}
		const content = model.getLineContent((lineIndex) + 1);
		const removableLength = content.startsWith("\t")
			? 1
			: Math.min(indentation.tabSize, /^[ ]*/.exec(content)![0].length);
		if (removableLength === 0) return [];
		return [{
			startOffset,
			endOffset: startOffset + removableLength,
			text: "",
			edit: Object.freeze({
				range: Range.fromPositions(lineStart, new Position((lineIndex) + 1, (removableLength) + 1)),
				text: "",
			}),
		}];
	});
	const selectionsAfter = selections.selections.map(selection => Object.freeze({
		anchorOffset: mapOffsetThroughEdits(model.offsetAt(selection.getSelectionStart()), edits),
		activeOffset: mapOffsetThroughEdits(model.offsetAt(selection.getPosition()), edits),
	}));
	return Object.freeze({
		edits: Object.freeze(edits.map(edit => edit.edit)),
		selectionsAfter: Object.freeze(selectionsAfter),
		primarySelectionIndex: selections.primaryIndex,
		historyMode: EditorCommandHistoryMode.Isolated,
	});
}

function selectedLineIndices(selections: SelectionSet): readonly number[] {
	const indices = new Set<number>();
	for (const selection of selections.selections) {
		const range = selection;
		let endLineIndex = range.endLineNumber - 1;
		if (!selection.isEmpty() && range.endColumn === 1 && endLineIndex > range.startLineNumber - 1) {
			endLineIndex -= 1;
		}
		for (let lineIndex = range.startLineNumber - 1; lineIndex <= endLineIndex; lineIndex += 1) indices.add(lineIndex);
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

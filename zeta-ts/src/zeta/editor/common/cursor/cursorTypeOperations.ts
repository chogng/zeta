import { getOrSet } from '../../../base/common/map.js';
import { EditorCommandHistoryMode, type EditorEditCommand, type TextSelectionOffsets } from '../commands/editorEditCommand.js';
import type { SelectionSet } from './selectionSet.js';
import { Position } from '../core/position.js';
import { Range } from '../core/range.js';
import { normalizeTextLineEndings } from '../core/textChange.js';
import { type TextEdit } from '../core/editOperation.js';
import { type TextModel } from '../model/textModel.js';
import { TypeWithoutInterceptorsOperation, type SelectionEdit } from './cursorTypeEditOperations.js';

export class TypeOperations {
	public static typeWithoutInterceptors(model: TextModel, selections: SelectionSet, text: string): EditorEditCommand {
		if (typeof text !== 'string') throw new TypeError('Typed text must be a string');
		const normalized = normalizeTextLineEndings(text);
		return TypeWithoutInterceptorsOperation.getEdits(
			model,
			selections,
			selections.selections.map(selection => textEdit(selection, normalized, normalized.length)),
			EditorCommandHistoryMode.CoalesceTyping,
		);
	}

	public static paste(model: TextModel, selections: SelectionSet, text: string): EditorEditCommand {
		if (typeof text !== 'string') throw new TypeError('Pasted text must be a string');
		const normalized = normalizeTextLineEndings(text);
		return TypeWithoutInterceptorsOperation.getEdits(
			model,
			selections,
			selections.selections.map(selection => textEdit(selection, normalized, normalized.length)),
			EditorCommandHistoryMode.Isolated,
		);
	}

	public static distributedPaste(model: TextModel, selections: SelectionSet, texts: readonly string[]): EditorEditCommand {
		if (texts.length !== selections.selections.length) throw new RangeError('Distributed paste text must match the selection count');
		const normalized = texts.map(text => {
			if (typeof text !== 'string') throw new TypeError('Distributed paste text must contain only strings');
			return normalizeTextLineEndings(text);
		});
		return TypeWithoutInterceptorsOperation.getEdits(
			model,
			selections,
			selections.selections.map((selection, selectionIndex) => textEdit(
				selection,
				normalized[selectionIndex]!,
				normalized[selectionIndex]!.length,
			)),
			EditorCommandHistoryMode.Isolated,
		);
	}

	public static linePaste(model: TextModel, selections: SelectionSet, texts: readonly string[]): EditorEditCommand {
		if (texts.length !== selections.selections.length) throw new RangeError('Line paste text must match the selection count');
		const normalized = texts.map(text => {
			if (typeof text !== 'string') throw new TypeError('Line paste text must contain only strings');
			const value = normalizeTextLineEndings(text);
			if (!value.endsWith('\n')) throw new RangeError('Line paste text must end with a line break');
			return value;
		});
		const groups = new Map<number, { readonly lineNumber: number; readonly selectionIndices: number[]; text: string }>();
		for (let selectionIndex = 0; selectionIndex < selections.selections.length; selectionIndex += 1) {
			const selection = selections.selections[selectionIndex]!;
			if (!selection.isEmpty()) throw new RangeError('Line paste requires collapsed selections');
			const lineNumber = selection.getPosition().lineNumber;
			const group = getOrSet(groups, lineNumber, { lineNumber, selectionIndices: [], text: '' });
			group.selectionIndices.push(selectionIndex);
			group.text += normalized[selectionIndex]!;
		}
		const sorted = [...groups.values()].sort((left, right) => left.lineNumber - right.lineNumber);
		const selectionsAfter = new Array<TextSelectionOffsets>(selections.selections.length);
		const edits: TextEdit[] = [];
		let cumulativeDelta = 0;
		for (const group of sorted) {
			const position = new Position(group.lineNumber, 1);
			const startOffset = model.offsetAt(position);
			edits.push({ range: Range.fromPositions(position), text: group.text });
			for (const selectionIndex of group.selectionIndices) {
				const columnIndex = selections.selections[selectionIndex]!.getPosition().column - 1;
				const caretOffset = startOffset + cumulativeDelta + group.text.length + columnIndex;
				selectionsAfter[selectionIndex] = { anchorOffset: caretOffset, activeOffset: caretOffset };
			}
			cumulativeDelta += group.text.length;
		}
		const normalizedSelections = TypeWithoutInterceptorsOperation.normalizeSelectionOffsets(selectionsAfter, selections.primaryIndex);
		return {
			edits: Object.freeze(edits),
			selectionsAfter: normalizedSelections.selections,
			primarySelectionIndex: normalizedSelections.primaryIndex,
			historyMode: EditorCommandHistoryMode.Isolated,
		};
	}
}

function textEdit(range: Range, text: string, caretOffsetInText: number): SelectionEdit {
	return { range, text, anchorOffsetInText: caretOffsetInText, activeOffsetInText: caretOffsetInText };
}

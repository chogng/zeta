import { getOrSet } from '../../../base/common/map.js';
import { EditorCommandHistoryMode, type EditorEditCommand, type TextSelectionOffsets } from '../commands/editorEditCommand.js';
import { type TextSelectionSet } from '../core/selection.js';
import { normalizeTextLineEndings, TextPosition, TextRange, type TextEdit } from '../core/text.js';
import { type TextModel } from '../model/textModel.js';
import { TypeWithoutInterceptorsOperation, type SelectionEdit } from './cursorTypeEditOperations.js';

export class TypeOperations {
	public static typeWithoutInterceptors(model: TextModel, selections: TextSelectionSet, text: string): EditorEditCommand {
		if (typeof text !== 'string') throw new TypeError('Typed text must be a string');
		const normalized = normalizeTextLineEndings(text);
		return TypeWithoutInterceptorsOperation.getEdits(
			model,
			selections,
			selections.selections.map(selection => textEdit(selection.range, normalized, normalized.length)),
			EditorCommandHistoryMode.CoalesceTyping,
		);
	}

	public static paste(model: TextModel, selections: TextSelectionSet, text: string): EditorEditCommand {
		if (typeof text !== 'string') throw new TypeError('Pasted text must be a string');
		const normalized = normalizeTextLineEndings(text);
		return TypeWithoutInterceptorsOperation.getEdits(
			model,
			selections,
			selections.selections.map(selection => textEdit(selection.range, normalized, normalized.length)),
			EditorCommandHistoryMode.Isolated,
		);
	}

	public static distributedPaste(model: TextModel, selections: TextSelectionSet, texts: readonly string[]): EditorEditCommand {
		if (texts.length !== selections.selections.length) throw new RangeError('Distributed paste text must match the selection count');
		const normalized = texts.map(text => {
			if (typeof text !== 'string') throw new TypeError('Distributed paste text must contain only strings');
			return normalizeTextLineEndings(text);
		});
		return TypeWithoutInterceptorsOperation.getEdits(
			model,
			selections,
			selections.selections.map((selection, selectionIndex) => textEdit(
				selection.range,
				normalized[selectionIndex]!,
				normalized[selectionIndex]!.length,
			)),
			EditorCommandHistoryMode.Isolated,
		);
	}

	public static linePaste(model: TextModel, selections: TextSelectionSet, texts: readonly string[]): EditorEditCommand {
		if (texts.length !== selections.selections.length) throw new RangeError('Line paste text must match the selection count');
		const normalized = texts.map(text => {
			if (typeof text !== 'string') throw new TypeError('Line paste text must contain only strings');
			const value = normalizeTextLineEndings(text);
			if (!value.endsWith('\n')) throw new RangeError('Line paste text must end with a line break');
			return value;
		});
		const groups = new Map<number, { readonly lineIndex: number; readonly selectionIndices: number[]; text: string }>();
		for (let selectionIndex = 0; selectionIndex < selections.selections.length; selectionIndex += 1) {
			const selection = selections.selections[selectionIndex]!;
			if (!selection.collapsed) throw new RangeError('Line paste requires collapsed selections');
			const lineIndex = selection.active.lineIndex;
			const group = getOrSet(groups, lineIndex, { lineIndex, selectionIndices: [], text: '' });
			group.selectionIndices.push(selectionIndex);
			group.text += normalized[selectionIndex]!;
		}
		const sorted = [...groups.values()].sort((left, right) => left.lineIndex - right.lineIndex);
		const selectionsAfter = new Array<TextSelectionOffsets>(selections.selections.length);
		const edits: TextEdit[] = [];
		let cumulativeDelta = 0;
		for (const group of sorted) {
			const position = TextPosition.at(group.lineIndex, 0);
			const startOffset = model.offsetAt(position);
			edits.push({ range: TextRange.emptyAt(position), text: group.text });
			for (const selectionIndex of group.selectionIndices) {
				const column = selections.selections[selectionIndex]!.active.columnIndex;
				const caretOffset = startOffset + cumulativeDelta + group.text.length + column;
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

function textEdit(range: TextRange, text: string, caretOffsetInText: number): SelectionEdit {
	return { range, text, anchorOffsetInText: caretOffsetInText, activeOffsetInText: caretOffsetInText };
}

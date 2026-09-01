import { EditorCommandHistoryMode, normalizeEditorSelections, type EditorEditCommand, type TextSelectionOffsets } from '../commands/editorEditCommand.js';
import { type Selection } from '../core/selection.js';
import { type Position } from '../core/position.js';
import { Range } from '../core/range.js';
import { normalizeTextLineEndings } from '../core/textChange.js';

import { type TextModel } from '../model/textModel.js';
import { getTextGraphemeBoundaries } from '../core/textSegmentation.js';
import { MoveOperations } from './cursorMoveOperations.js';
import { type TextEdit } from '../languages.js';

interface SelectionReplacement {
	readonly selectionIndex: number;
	readonly range: Range;
	readonly startOffset: number;
	readonly endOffset: number;
	readonly text: string;
	readonly anchorOffsetInText: number;
	readonly activeOffsetInText: number;
}

export interface SelectionEdit {
	readonly range: Range;
	readonly text: string;
	readonly anchorOffsetInText: number;
	readonly activeOffsetInText: number;
}

export class TypeWithoutInterceptorsOperation {
	public static getEdits(model: TextModel, selections: readonly Selection[], edits: readonly SelectionEdit[], historyMode: EditorCommandHistoryMode): EditorEditCommand {
		if (!Array.isArray(edits) || edits.length !== selections.length) {
			throw new RangeError('Selection edits must match the selection count');
		}
		return buildSelectionEditCommand(
			model,
			selections,
			edits.map((edit, selectionIndex) => {
				if (typeof edit !== 'object' || edit === null || typeof edit.text !== 'string') {
					throw new TypeError('Selection edit must contain replacement text');
				}
				if (!Number.isSafeInteger(edit.anchorOffsetInText) || edit.anchorOffsetInText < 0 || !Number.isSafeInteger(edit.activeOffsetInText) || edit.activeOffsetInText < 0) {
					throw new RangeError('Selection edit result offsets must be non-negative safe integers');
				}
				return {
					selectionIndex,
					range: edit.range,
					startOffset: model.offsetAt(edit.range.getStartPosition()),
					endOffset: model.offsetAt(edit.range.getEndPosition()),
					text: edit.text,
					anchorOffsetInText: edit.anchorOffsetInText,
					activeOffsetInText: edit.activeOffsetInText,
				};
			}),
			historyMode,
		);
	}

}

export class AutoClosingOvertypeOperation {
	public static getEdits(model: TextModel, selections: readonly Selection[], text: string): EditorEditCommand {
		if (typeof text !== 'string') throw new TypeError('Overtype text must be a string');
		const normalized = normalizeTextLineEndings(text);
		return this._runAutoClosingOvertype(model, selections, normalized);
	}

	private static _runAutoClosingOvertype(model: TextModel, selections: readonly Selection[], text: string): EditorEditCommand {
		const graphemeCount = text.includes('\n') ? 0 : getTextGraphemeBoundaries(text).length - 1;
		return TypeWithoutInterceptorsOperation.getEdits(model, selections, selections.map(selection => {
			const range = selection.isEmpty() && graphemeCount > 0
				? Range.fromPositions(selection.getPosition(), advancePositionInLine(model, selection.getPosition(), graphemeCount))
				: selection;
			return { range, text, anchorOffsetInText: text.length, activeOffsetInText: text.length };
		}), EditorCommandHistoryMode.CoalesceTyping);
	}
}

function buildSelectionEditCommand(model: TextModel, selections: readonly Selection[], replacements: readonly SelectionReplacement[], historyMode: EditorCommandHistoryMode): EditorEditCommand {
	const sorted = [...replacements].sort((left, right) => left.startOffset - right.startOffset || left.endOffset - right.endOffset || left.selectionIndex - right.selectionIndex);
	validateNonOverlapping(sorted);
	const selectionsAfter = new Array<TextSelectionOffsets>(selections.length);
	const edits: TextEdit[] = [];
	let cumulativeDelta = 0;
	for (const item of sorted) {
		selectionsAfter[item.selectionIndex] = {
			anchorOffset: item.startOffset + cumulativeDelta + item.anchorOffsetInText,
			activeOffset: item.startOffset + cumulativeDelta + item.activeOffsetInText,
		};
		if (item.startOffset !== item.endOffset || item.text.length > 0) edits.push({ range: item.range, text: item.text });
		cumulativeDelta += item.text.length - (item.endOffset - item.startOffset);
	}
	const normalizedSelections = normalizeEditorSelections(selectionsAfter, 0);
	return {
		edits: Object.freeze(edits),
		selectionsAfter: normalizedSelections.selections,
		primarySelectionIndex: normalizedSelections.primaryIndex,
		historyMode,
	};
}

function validateNonOverlapping(replacements: readonly SelectionReplacement[]): void {
	for (let index = 1; index < replacements.length; index += 1) {
		const previous = replacements[index - 1]!;
		const current = replacements[index]!;
		const ambiguousSharedStart = current.startOffset === previous.startOffset && (current.startOffset === current.endOffset || previous.startOffset === previous.endOffset);
		if (current.startOffset < previous.endOffset || ambiguousSharedStart) {
			throw new RangeError('Selections must not overlap when creating an edit command');
		}
	}
}

function advancePositionInLine(model: TextModel, position: Position, count: number): Position {
	let current = position;
	for (let index = 0; index < count; index += 1) {
		const next = MoveOperations.rightPosition(model, current);
		if (next.lineNumber !== position.lineNumber) break;
		current = next;
	}
	return current;
}

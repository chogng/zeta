import { EditorCommandHistoryMode, type EditorEditCommand } from '../commands/editorEditCommand.js';
import { type TextSelectionSet } from '../core/selection.js';
import { TextPosition, TextRange } from '../core/text.js';
import { type TextModel } from '../model/textModel.js';
import { MoveOperations } from './cursorMoveOperations.js';
import { TypeWithoutInterceptorsOperation, type SelectionEdit } from './cursorTypeEditOperations.js';

export class DeleteOperations {
	public static cut(model: TextModel, selections: TextSelectionSet, cutRanges: readonly TextRange[]): EditorEditCommand {
		if (cutRanges.length !== selections.selections.length) throw new TypeError('Cut ranges must match the editor selections');
		const sourceRanges = mergeDeletionRanges(model, cutRanges);
		const selectionsAfter = TypeWithoutInterceptorsOperation.normalizeSelectionOffsets(cutRanges.map(range => {
			const targetOffset = mapOffsetThroughDeletions(model.offsetAt(range.start), sourceRanges);
			return { anchorOffset: targetOffset, activeOffset: targetOffset };
		}), selections.primaryIndex);
		return {
			edits: Object.freeze(sourceRanges.map(range => ({ range: range.range, text: '' }))),
			selectionsAfter: selectionsAfter.selections,
			primarySelectionIndex: selectionsAfter.primaryIndex,
			historyMode: EditorCommandHistoryMode.Isolated,
		};
	}

	public static deleteLeft(model: TextModel, selections: TextSelectionSet): EditorEditCommand {
		return TypeWithoutInterceptorsOperation.getEdits(
			model,
			selections,
			selections.selections.map(selection => emptySelectionEdit(
				selection.collapsed ? this.getPreviousDeleteRange(model, selection.active) : selection.range,
			)),
			EditorCommandHistoryMode.CoalesceBackspace,
		);
	}

	public static deleteRight(model: TextModel, selections: TextSelectionSet): EditorEditCommand {
		return TypeWithoutInterceptorsOperation.getEdits(
			model,
			selections,
			selections.selections.map(selection => emptySelectionEdit(
				selection.collapsed ? nextDeleteRange(model, selection.active) : selection.range,
			)),
			EditorCommandHistoryMode.CoalesceDelete,
		);
	}

	public static deleteToBeginningOfLine(model: TextModel, selections: TextSelectionSet): EditorEditCommand {
		return createDeleteToLineBoundaryCommand(model, selections, 'start');
	}

	public static deleteToEndOfLine(model: TextModel, selections: TextSelectionSet): EditorEditCommand {
		return createDeleteToLineBoundaryCommand(model, selections, 'end');
	}

	public static getPreviousDeleteRange(model: TextModel, position: TextPosition): TextRange {
		return TextRange.from(MoveOperations.leftPosition(model, position), position);
	}
}

function createDeleteToLineBoundaryCommand(model: TextModel, selections: TextSelectionSet, boundary: 'start' | 'end'): EditorEditCommand {
	return TypeWithoutInterceptorsOperation.getEdits(
		model,
		selections,
		selections.selections.map(selection => {
			const range = selection.collapsed
				? boundary === 'start'
					? TextRange.from(TextPosition.at(selection.active.lineIndex, 0), selection.active)
					: TextRange.from(selection.active, TextPosition.at(
						selection.active.lineIndex,
						model.getLineContent(selection.active.lineIndex).length,
					))
				: selection.range;
			return emptySelectionEdit(range);
		}),
		EditorCommandHistoryMode.Isolated,
	);
}

function emptySelectionEdit(range: TextRange): SelectionEdit {
	return { range, text: '', anchorOffsetInText: 0, activeOffsetInText: 0 };
}

function nextDeleteRange(model: TextModel, position: TextPosition): TextRange {
	return TextRange.from(position, MoveOperations.rightPosition(model, position));
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
	})).filter(range => range.startOffset !== range.endOffset).sort((left, right) => left.startOffset - right.startOffset || left.endOffset - right.endOffset);
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
		range: TextRange.from(model.positionAt(range.startOffset), model.positionAt(range.endOffset)),
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

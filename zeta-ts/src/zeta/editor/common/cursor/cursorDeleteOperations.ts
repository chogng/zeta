import { EditorCommandHistoryMode, type EditorEditCommand } from '../commands/editorEditCommand.js';
import type { SelectionSet } from './selectionSet.js';
import { Position } from '../core/position.js';
import { Range } from '../core/range.js';
import { type TextModel } from '../model/textModel.js';
import { MoveOperations } from './cursorMoveOperations.js';
import { TypeWithoutInterceptorsOperation, type SelectionEdit } from './cursorTypeEditOperations.js';

export class DeleteOperations {
	public static cut(model: TextModel, selections: SelectionSet, cutRanges: readonly Range[]): EditorEditCommand {
		if (cutRanges.length !== selections.selections.length) throw new TypeError('Cut ranges must match the editor selections');
		const sourceRanges = mergeDeletionRanges(model, cutRanges);
		const selectionsAfter = TypeWithoutInterceptorsOperation.normalizeSelectionOffsets(cutRanges.map(range => {
			const targetOffset = mapOffsetThroughDeletions(model.offsetAt(range.getStartPosition()), sourceRanges);
			return { anchorOffset: targetOffset, activeOffset: targetOffset };
		}), selections.primaryIndex);
		return {
			edits: Object.freeze(sourceRanges.map(range => ({ range: range.range, text: '' }))),
			selectionsAfter: selectionsAfter.selections,
			primarySelectionIndex: selectionsAfter.primaryIndex,
			historyMode: EditorCommandHistoryMode.Isolated,
		};
	}

	public static deleteLeft(model: TextModel, selections: SelectionSet): EditorEditCommand {
		return TypeWithoutInterceptorsOperation.getEdits(
			model,
			selections,
			selections.selections.map(selection => emptySelectionEdit(
				selection.isEmpty() ? this.getPreviousDeleteRange(model, selection.getPosition()) : selection,
			)),
			EditorCommandHistoryMode.CoalesceBackspace,
		);
	}

	public static deleteRight(model: TextModel, selections: SelectionSet): EditorEditCommand {
		return TypeWithoutInterceptorsOperation.getEdits(
			model,
			selections,
			selections.selections.map(selection => emptySelectionEdit(
				selection.isEmpty() ? nextDeleteRange(model, selection.getPosition()) : selection,
			)),
			EditorCommandHistoryMode.CoalesceDelete,
		);
	}

	public static deleteToBeginningOfLine(model: TextModel, selections: SelectionSet): EditorEditCommand {
		return createDeleteToLineBoundaryCommand(model, selections, 'start');
	}

	public static deleteToEndOfLine(model: TextModel, selections: SelectionSet): EditorEditCommand {
		return createDeleteToLineBoundaryCommand(model, selections, 'end');
	}

	public static getPreviousDeleteRange(model: TextModel, position: Position): Range {
		return Range.fromPositions(MoveOperations.leftPosition(model, position), position);
	}
}

function createDeleteToLineBoundaryCommand(model: TextModel, selections: SelectionSet, boundary: 'start' | 'end'): EditorEditCommand {
	return TypeWithoutInterceptorsOperation.getEdits(
		model,
		selections,
		selections.selections.map(selection => {
			const range = selection.isEmpty()
				? boundary === 'start'
					? Range.fromPositions(new Position(selection.getPosition().lineNumber, 1), selection.getPosition())
					: Range.fromPositions(selection.getPosition(), new Position(selection.getPosition().lineNumber, model.getLineContent(selection.getPosition().lineNumber).length + 1))
				: selection;
			return emptySelectionEdit(range);
		}),
		EditorCommandHistoryMode.Isolated,
	);
}

function emptySelectionEdit(range: Range): SelectionEdit {
	return { range, text: '', anchorOffsetInText: 0, activeOffsetInText: 0 };
}

function nextDeleteRange(model: TextModel, position: Position): Range {
	return Range.fromPositions(position, MoveOperations.rightPosition(model, position));
}

interface OffsetDeletionRange {
	readonly range: Range;
	readonly startOffset: number;
	readonly endOffset: number;
}

function mergeDeletionRanges(model: TextModel, ranges: readonly Range[]): readonly OffsetDeletionRange[] {
	const sorted = ranges.map(range => ({
		startOffset: model.offsetAt(range.getStartPosition()),
		endOffset: model.offsetAt(range.getEndPosition()),
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
		range: Range.fromPositions(model.positionAt(range.startOffset), model.positionAt(range.endOffset)),
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

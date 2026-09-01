import { EditorCommandHistoryMode, normalizeEditorSelections, type EditorEditCommand } from '../commands/editorEditCommand.js';
import { type Selection } from '../core/selection.js';
import { Position } from '../core/position.js';
import { Range } from '../core/range.js';
import { type TextModel } from '../model/textModel.js';
import { MoveOperations } from './cursorMoveOperations.js';
import { TypeWithoutInterceptorsOperation, type SelectionEdit } from './cursorTypeEditOperations.js';
import { type ResolvedLanguageConfiguration } from '../languages/languageConfigurationRegistry.js';

export class DeleteOperations {
	public static cut(model: TextModel, selections: readonly Selection[], cutRanges: readonly Range[]): EditorEditCommand {
		if (cutRanges.length !== selections.length) throw new TypeError('Cut ranges must match the editor selections');
		const sourceRanges = mergeDeletionRanges(model, cutRanges);
		const selectionsAfter = normalizeEditorSelections(cutRanges.map(range => {
			const targetOffset = mapOffsetThroughDeletions(model.offsetAt(range.getStartPosition()), sourceRanges);
			return { anchorOffset: targetOffset, activeOffset: targetOffset };
		}), 0);
		return {
			edits: Object.freeze(sourceRanges.map(range => ({ range: range.range, text: '' }))),
			selectionsAfter: selectionsAfter.selections,
			primarySelectionIndex: selectionsAfter.primaryIndex,
			historyMode: EditorCommandHistoryMode.Isolated,
		};
	}

	public static deleteLeft(
		model: TextModel,
		selections: readonly Selection[],
		configuration?: ResolvedLanguageConfiguration,
		autoClosedCharacters: readonly Range[] = [],
	): EditorEditCommand {
		return TypeWithoutInterceptorsOperation.getEdits(
			model,
			selections,
			selections.map(selection => emptySelectionEdit(
				selection.isEmpty()
					? autoClosingDeleteRange(model, selection.getPosition(), configuration, autoClosedCharacters)
						?? this.getPreviousDeleteRange(model, selection.getPosition())
					: selection,
			)),
			EditorCommandHistoryMode.CoalesceBackspace,
		);
	}

	public static deleteRight(model: TextModel, selections: readonly Selection[]): EditorEditCommand {
		return TypeWithoutInterceptorsOperation.getEdits(
			model,
			selections,
			selections.map(selection => emptySelectionEdit(
				selection.isEmpty() ? nextDeleteRange(model, selection.getPosition()) : selection,
			)),
			EditorCommandHistoryMode.CoalesceDelete,
		);
	}

	public static deleteToBeginningOfLine(model: TextModel, selections: readonly Selection[]): EditorEditCommand {
		return createDeleteToLineBoundaryCommand(model, selections, 'start');
	}

	public static deleteToEndOfLine(model: TextModel, selections: readonly Selection[]): EditorEditCommand {
		return createDeleteToLineBoundaryCommand(model, selections, 'end');
	}

	public static getPreviousDeleteRange(model: TextModel, position: Position): Range {
		return Range.fromPositions(MoveOperations.leftPosition(model, position), position);
	}
}

function autoClosingDeleteRange(
	model: TextModel,
	position: Position,
	configuration: ResolvedLanguageConfiguration | undefined,
	autoClosedCharacters: readonly Range[],
): Range | undefined {
	if (!configuration) return undefined;
	const closer = autoClosedCharacters.find(range => Position.equals(range.getStartPosition(), position));
	if (!closer) return undefined;
	const close = model.getTextInRange(closer);
	const line = model.getLineContent(position.lineNumber);
	const columnIndex = position.column - 1;
	const pair = [...configuration.characterPair.getAutoClosingPairs()]
		.sort((left, right) => right.open.length - left.open.length)
		.find(candidate => candidate.close === close
			&& columnIndex >= candidate.open.length
			&& line.slice(columnIndex - candidate.open.length, columnIndex) === candidate.open);
	if (!pair) return undefined;
	return Range.fromPositions(
		new Position(position.lineNumber, position.column - pair.open.length),
		closer.getEndPosition(),
	);
}

function createDeleteToLineBoundaryCommand(model: TextModel, selections: readonly Selection[], boundary: 'start' | 'end'): EditorEditCommand {
	return TypeWithoutInterceptorsOperation.getEdits(
		model,
		selections,
		selections.map(selection => {
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
	return Range.fromPositions(position, MoveOperations.rightPosition(model, position.lineNumber, position.column));
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

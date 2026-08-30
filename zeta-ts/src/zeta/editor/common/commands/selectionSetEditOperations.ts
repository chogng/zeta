import { Position } from '../core/position.js';
import { Range } from '../core/range.js';
import { Selection } from '../core/selection.js';
import { normalizeTextLineEndings } from '../core/textChange.js';
import { SelectionSet } from '../cursor/selectionSet.js';
import { type TextEdit } from '../languages.js';
import { type TextModel } from '../model/textModel.js';
import { type TextSelectionOffsets } from './editorEditCommand.js';

export function calculateResultLength(model: TextModel, edits: readonly TextEdit[]): number {
	let length = model.createVersionedSnapshot().length;
	for (const edit of edits) {
		const range = Range.lift(edit.range);
		const startOffset = model.offsetAt(range.getStartPosition());
		const endOffset = model.offsetAt(range.getEndPosition());
		length += normalizeTextLineEndings(edit.text).length - (endOffset - startOffset);
	}
	return length;
}

export function selectionSetFromOffsets(model: TextModel, selections: readonly TextSelectionOffsets[], primarySelectionIndex: number): SelectionSet {
	return SelectionSet.withPrimary(
		selections.map(selection => Selection.fromPositions(
			model.positionAt(selection.anchorOffset),
			model.positionAt(selection.activeOffset),
		)),
		primarySelectionIndex,
	);
}

export function validateSelectionOffsets(selections: readonly TextSelectionOffsets[], primarySelectionIndex: number, documentLength: number): void {
	if (selections.length === 0) throw new RangeError('selectionsAfter must not be empty');
	if (!Number.isSafeInteger(primarySelectionIndex) || primarySelectionIndex < 0 || primarySelectionIndex >= selections.length) {
		throw new RangeError('primarySelectionIndex must identify selectionsAfter');
	}
	for (const selection of selections) {
		assertOffset(selection.anchorOffset, documentLength, 'anchorOffset');
		assertOffset(selection.activeOffset, documentLength, 'activeOffset');
	}
}

export function selectionSetsEqual(left: SelectionSet, right: SelectionSet): boolean {
	return left.primaryIndex === right.primaryIndex
		&& left.selections.length === right.selections.length
		&& left.selections.every((selection, index) => {
			const other = right.selections[index]!;
			return Position.compare(selection.getSelectionStart(), other.getSelectionStart()) === 0
				&& Position.compare(selection.getPosition(), other.getPosition()) === 0;
		});
}

function assertOffset(offset: number, documentLength: number, name: string): void {
	if (!Number.isSafeInteger(offset) || offset < 0 || offset > documentLength) {
		throw new RangeError(`${name} must be between 0 and ${documentLength}`);
	}
}

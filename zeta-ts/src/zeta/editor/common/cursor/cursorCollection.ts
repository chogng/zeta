import { type TextSelectionOffsets } from '../commands/editorEditCommand.js';
import { Disposable, DisposableStore } from '../../../base/common/lifecycle.js';
import { TextSelection, TextSelectionSet } from '../core/selection.js';
import { normalizeTextLineEndings, type TextEdit } from '../core/text.js';
import { TextModel } from '../model/textModel.js';
import { Cursor } from './oneCursor.js';

export class CursorCollection extends Disposable {
	private readonly resources = this._register(new DisposableStore());
	private cursors: Cursor[] = [];
	private primaryIndex = 0;

	constructor(private readonly model: TextModel, selections: TextSelectionSet) {
		super();
		this.setSelections(selections);
	}

	public getSelections(): TextSelectionSet {
		return TextSelectionSet.withPrimary(this.cursors.map(cursor => cursor.selection), this.primaryIndex);
	}

	public setSelections(selections: TextSelectionSet): void {
		CursorCollection.validateSelectionSet(this.model, selections);
		this.resources.clear();
		this.cursors = selections.selections.map(selection => {
			const cursor = new Cursor(this.model, selection);
			this.resources.add(cursor);
			return cursor;
		});
		this.primaryIndex = selections.primaryIndex;
	}

	public static calculateResultLength(model: TextModel, edits: readonly TextEdit[]): number {
		let length = model.createSnapshot().length;
		for (const edit of edits) {
			const startOffset = model.offsetAt(edit.range.start);
			const endOffset = model.offsetAt(edit.range.end);
			length += normalizeTextLineEndings(edit.text).length - (endOffset - startOffset);
		}
		return length;
	}

	public static selectionSetFromOffsets(model: TextModel, selections: readonly TextSelectionOffsets[], primarySelectionIndex: number): TextSelectionSet {
		return TextSelectionSet.withPrimary(
			selections.map(selection => TextSelection.from(
				model.positionAt(selection.anchorOffset),
				model.positionAt(selection.activeOffset),
			)),
			primarySelectionIndex,
		);
	}

	public static validateSelectionOffsets(selections: readonly TextSelectionOffsets[], primarySelectionIndex: number, documentLength: number): void {
		if (selections.length === 0) throw new RangeError('selectionsAfter must not be empty');
		if (!Number.isSafeInteger(primarySelectionIndex) || primarySelectionIndex < 0 || primarySelectionIndex >= selections.length) {
			throw new RangeError('primarySelectionIndex must identify selectionsAfter');
		}
		for (const selection of selections) {
			assertOffset(selection.anchorOffset, documentLength, 'anchorOffset');
			assertOffset(selection.activeOffset, documentLength, 'activeOffset');
		}
	}

	public static validateSelectionSet(model: TextModel, selections: TextSelectionSet): void {
		for (const selection of selections.selections) {
			model.offsetAt(selection.anchor);
			model.offsetAt(selection.active);
		}
	}

	public static selectionsEqual(left: TextSelectionSet, right: TextSelectionSet): boolean {
		return left.primaryIndex === right.primaryIndex &&
			left.selections.length === right.selections.length &&
			left.selections.every((selection, index) => {
				const other = right.selections[index]!;
				return selection.anchor.compareTo(other.anchor) === 0 && selection.active.compareTo(other.active) === 0;
			});
	}
}

function assertOffset(offset: number, documentLength: number, name: string): void {
	if (!Number.isSafeInteger(offset) || offset < 0 || offset > documentLength) {
		throw new RangeError(`${name} must be between 0 and ${documentLength}`);
	}
}

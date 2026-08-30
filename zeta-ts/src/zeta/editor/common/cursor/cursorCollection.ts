import { Position } from "../core/position.js";
import { Range } from '../core/range.js';
import { type TextSelectionOffsets } from '../commands/editorEditCommand.js';
import { Disposable, DisposableStore } from '../../../base/common/lifecycle.js';
import { Selection } from '../core/selection.js';
import { normalizeTextLineEndings } from '../core/textChange.js';

import { TextModel } from '../model/textModel.js';
import { Cursor } from './oneCursor.js';
import { type TextEdit } from '../languages.js';

export class CursorCollection extends Disposable {
	private readonly resources = this._register(new DisposableStore());
	private cursors: Cursor[] = [];

	constructor(private readonly model: TextModel, selections: readonly Selection[]) {
		super();
		this.setSelections(selections);
	}

	public getSelections(): readonly Selection[] {
		return Object.freeze(this.cursors.map(cursor => cursor.selection));
	}

	public setSelections(selections: readonly Selection[]): void {
		CursorCollection.validateSelections(this.model, selections);
		this.resources.clear();
		this.cursors = selections.map(selection => {
			const cursor = new Cursor(this.model, selection);
			this.resources.add(cursor);
			return cursor;
		});
	}

	public static calculateResultLength(model: TextModel, edits: readonly TextEdit[]): number {
		let length = model.length;
		for (const edit of edits) {
			const range = Range.lift(edit.range);
			const startOffset = model.offsetAt(range.getStartPosition());
			const endOffset = model.offsetAt(range.getEndPosition());
			length += normalizeTextLineEndings(edit.text).length - (endOffset - startOffset);
		}
		return length;
	}

	public static selectionsFromOffsets(model: TextModel, selections: readonly TextSelectionOffsets[], primarySelectionIndex: number): readonly Selection[] {
		return primaryFirst(
			selections.map(selection => Selection.fromPositions(
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

	public static validateSelections(model: TextModel, selections: readonly Selection[]): void {
		if (selections.length === 0) throw new RangeError('Selections must not be empty');
		for (const selection of selections) {
			model.offsetAt(selection.getSelectionStart());
			model.offsetAt(selection.getPosition());
		}
	}

	public static selectionsEqual(left: readonly Selection[], right: readonly Selection[]): boolean {
		return left.length === right.length &&
			left.every((selection, index) => {
				const other = right[index]!;
				return Position.compare(selection.getSelectionStart(), other.getSelectionStart()) === 0 && Position.compare(selection.getPosition(), other.getPosition()) === 0;
			});
	}
}

function primaryFirst<T>(items: readonly T[], primaryIndex: number): readonly T[] {
	if (!Number.isSafeInteger(primaryIndex) || primaryIndex < 0 || primaryIndex >= items.length) {
		throw new RangeError('Primary selection index must identify a selection');
	}
	if (primaryIndex === 0) return Object.freeze([...items]);
	return Object.freeze([items[primaryIndex]!, ...items.slice(0, primaryIndex), ...items.slice(primaryIndex + 1)]);
}

function assertOffset(offset: number, documentLength: number, name: string): void {
	if (!Number.isSafeInteger(offset) || offset < 0 || offset > documentLength) {
		throw new RangeError(`${name} must be between 0 and ${documentLength}`);
	}
}

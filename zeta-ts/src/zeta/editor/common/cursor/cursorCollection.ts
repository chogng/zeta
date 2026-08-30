import { compareBy } from '../../../base/common/arrays.js';
import { findFirstMin, findLastMax } from '../../../base/common/arraysFind.js';
import { CursorState, type PartialCursorState } from '../cursorCommon.js';
import { Position } from '../core/position.js';
import { Range } from '../core/range.js';
import { type ISelection, Selection } from '../core/selection.js';
import { CursorContext } from './cursorContext.js';
import { Cursor } from './oneCursor.js';

export class CursorCollection {
	private context: CursorContext;
	/** The primary cursor is always at index zero. */
	private cursors: Cursor[];
	private lastAddedCursorIndex: number;

	constructor(context: CursorContext) {
		this.context = context;
		this.cursors = [new Cursor(context)];
		this.lastAddedCursorIndex = 0;
	}

	dispose(): void {
		for (const cursor of this.cursors) cursor.dispose(this.context);
	}

	startTrackingSelections(): void {
		for (const cursor of this.cursors) cursor.startTrackingSelection(this.context);
	}

	stopTrackingSelections(): void {
		for (const cursor of this.cursors) cursor.stopTrackingSelection(this.context);
	}

	updateContext(context: CursorContext): void {
		this.context = context;
	}

	ensureValidState(): void {
		for (const cursor of this.cursors) cursor.ensureValidState(this.context);
	}

	readSelectionFromMarkers(): Selection[] {
		return this.cursors.map(cursor => cursor.readSelectionFromMarkers(this.context));
	}

	getAll(): CursorState[] {
		return this.cursors.map(cursor => cursor.asCursorState());
	}

	getViewPositions(): Position[] {
		return this.cursors.map(cursor => cursor.viewState.position);
	}

	getTopMostViewPosition(): Position {
		return findFirstMin(this.cursors, compareBy(cursor => cursor.viewState.position, Position.compare))!.viewState.position;
	}

	getBottomMostViewPosition(): Position {
		return findLastMax(this.cursors, compareBy(cursor => cursor.viewState.position, Position.compare))!.viewState.position;
	}

	getSelections(): Selection[] {
		return this.cursors.map(cursor => cursor.modelState.selection);
	}

	getViewSelections(): Selection[] {
		return this.cursors.map(cursor => cursor.viewState.selection);
	}

	setSelections(selections: ISelection[]): void {
		this.setStates(CursorState.fromModelSelections(selections));
	}

	getPrimaryCursor(): CursorState {
		return this.cursors[0].asCursorState();
	}

	setStates(states: PartialCursorState[] | null): void {
		if (states === null) return;
		this.cursors[0].setState(this.context, states[0].modelState, states[0].viewState);
		this._setSecondaryStates(states.slice(1));
	}

	private _setSecondaryStates(secondaryStates: PartialCursorState[]): void {
		const secondaryCursorCount = this.cursors.length - 1;
		if (secondaryCursorCount < secondaryStates.length) {
			for (let index = secondaryCursorCount; index < secondaryStates.length; index++) this._addSecondaryCursor();
		} else if (secondaryCursorCount > secondaryStates.length) {
			for (let index = secondaryCursorCount; index > secondaryStates.length; index--) this._removeSecondaryCursor(this.cursors.length - 2);
		}

		for (let index = 0; index < secondaryStates.length; index++) {
			this.cursors[index + 1].setState(this.context, secondaryStates[index].modelState, secondaryStates[index].viewState);
		}
	}

	killSecondaryCursors(): void {
		this._setSecondaryStates([]);
	}

	private _addSecondaryCursor(): void {
		this.cursors.push(new Cursor(this.context));
		this.lastAddedCursorIndex = this.cursors.length - 1;
	}

	getLastAddedCursorIndex(): number {
		if (this.cursors.length === 1 || this.lastAddedCursorIndex === 0) return 0;
		return this.lastAddedCursorIndex;
	}

	private _removeSecondaryCursor(removeIndex: number): void {
		if (this.lastAddedCursorIndex >= removeIndex + 1) this.lastAddedCursorIndex--;
		this.cursors[removeIndex + 1].dispose(this.context);
		this.cursors.splice(removeIndex + 1, 1);
	}

	normalize(): void {
		if (this.cursors.length === 1) return;
		const cursors = this.cursors.slice();
		const sortedCursors: Array<{ index: number; selection: Selection }> = cursors.map((cursor, index) => ({
			index,
			selection: cursor.modelState.selection,
		}));
		sortedCursors.sort(compareBy(item => item.selection, Range.compareRangesUsingStarts));

		for (let sortedIndex = 0; sortedIndex < sortedCursors.length - 1; sortedIndex++) {
			const current = sortedCursors[sortedIndex];
			const next = sortedCursors[sortedIndex + 1];
			if (!this.context.cursorConfig.multiCursorMergeOverlapping) continue;

			const shouldMerge = next.selection.isEmpty() || current.selection.isEmpty()
				? next.selection.getStartPosition().isBeforeOrEqual(current.selection.getEndPosition())
				: next.selection.getStartPosition().isBefore(current.selection.getEndPosition());
			if (!shouldMerge) continue;

			const winnerSortedIndex = current.index < next.index ? sortedIndex : sortedIndex + 1;
			const loserSortedIndex = current.index < next.index ? sortedIndex + 1 : sortedIndex;
			const loserIndex = sortedCursors[loserSortedIndex].index;
			const winnerIndex = sortedCursors[winnerSortedIndex].index;
			const loserSelection = sortedCursors[loserSortedIndex].selection;
			const winnerSelection = sortedCursors[winnerSortedIndex].selection;

			if (!loserSelection.equalsSelection(winnerSelection)) {
				const resultingRange = loserSelection.plusRange(winnerSelection);
				const loserIsLeftToRight = loserSelection.selectionStartLineNumber === loserSelection.startLineNumber
					&& loserSelection.selectionStartColumn === loserSelection.startColumn;
				const winnerIsLeftToRight = winnerSelection.selectionStartLineNumber === winnerSelection.startLineNumber
					&& winnerSelection.selectionStartColumn === winnerSelection.startColumn;
				let resultIsLeftToRight: boolean;
				if (loserIndex === this.lastAddedCursorIndex) {
					resultIsLeftToRight = loserIsLeftToRight;
					this.lastAddedCursorIndex = winnerIndex;
				} else {
					resultIsLeftToRight = winnerIsLeftToRight;
				}
				const resultingSelection = resultIsLeftToRight
					? new Selection(resultingRange.startLineNumber, resultingRange.startColumn, resultingRange.endLineNumber, resultingRange.endColumn)
					: new Selection(resultingRange.endLineNumber, resultingRange.endColumn, resultingRange.startLineNumber, resultingRange.startColumn);
				sortedCursors[winnerSortedIndex].selection = resultingSelection;
				const resultingState = CursorState.fromModelSelection(resultingSelection);
				cursors[winnerIndex].setState(this.context, resultingState.modelState, resultingState.viewState);
			}

			for (const sortedCursor of sortedCursors) {
				if (sortedCursor.index > loserIndex) sortedCursor.index--;
			}
			cursors.splice(loserIndex, 1);
			sortedCursors.splice(loserSortedIndex, 1);
			this._removeSecondaryCursor(loserIndex - 1);
			sortedIndex--;
		}
	}
}

import { CursorState, type ICursorSimpleModel, SelectionStartKind, SingleCursorState } from '../cursorCommon.js';
import { Position } from '../core/position.js';
import { Range } from '../core/range.js';
import { Selection } from '../core/selection.js';
import { PositionAffinity, TrackedRangeStickiness } from '../model.js';
import { CursorContext } from './cursorContext.js';

/** Represents a single model/view cursor. */
export class Cursor {
	public modelState!: SingleCursorState;
	public viewState!: SingleCursorState;

	private _selTrackedRange: string | null;
	private _trackSelection: boolean;

	constructor(context: CursorContext) {
		this._selTrackedRange = null;
		this._trackSelection = true;
		this._setState(
			context,
			new SingleCursorState(new Range(1, 1, 1, 1), SelectionStartKind.Simple, 0, new Position(1, 1), 0),
			new SingleCursorState(new Range(1, 1, 1, 1), SelectionStartKind.Simple, 0, new Position(1, 1), 0),
		);
	}

	dispose(context: CursorContext): void {
		this._removeTrackedRange(context);
	}

	startTrackingSelection(context: CursorContext): void {
		this._trackSelection = true;
		this._updateTrackedRange(context);
	}

	stopTrackingSelection(context: CursorContext): void {
		this._trackSelection = false;
		this._removeTrackedRange(context);
	}

	private _updateTrackedRange(context: CursorContext): void {
		if (!this._trackSelection) return;
		this._selTrackedRange = context.model._setTrackedRange(
			this._selTrackedRange,
			this.modelState.selection,
			TrackedRangeStickiness.AlwaysGrowsWhenTypingAtEdges,
		);
	}

	private _removeTrackedRange(context: CursorContext): void {
		this._selTrackedRange = context.model._setTrackedRange(
			this._selTrackedRange,
			null,
			TrackedRangeStickiness.AlwaysGrowsWhenTypingAtEdges,
		);
	}

	asCursorState(): CursorState {
		return new CursorState(this.modelState, this.viewState);
	}

	readSelectionFromMarkers(context: CursorContext): Selection {
		const range = context.model._getTrackedRange(this._selTrackedRange!)!;
		if (this.modelState.selection.isEmpty() && !range.isEmpty()) {
			return Selection.fromRange(range.collapseToEnd(), this.modelState.selection.getDirection());
		}
		return Selection.fromRange(range, this.modelState.selection.getDirection());
	}

	ensureValidState(context: CursorContext): void {
		this._setState(context, this.modelState, this.viewState);
	}

	setState(context: CursorContext, modelState: SingleCursorState | null, viewState: SingleCursorState | null): void {
		this._setState(context, modelState, viewState);
	}

	private static _validatePositionWithCache(viewModel: ICursorSimpleModel, position: Position, cacheInput: Position, cacheOutput: Position): Position {
		return position.equals(cacheInput)
			? cacheOutput
			: viewModel.normalizePosition(position, PositionAffinity.None);
	}

	private static _validateViewState(viewModel: ICursorSimpleModel, viewState: SingleCursorState): SingleCursorState {
		const position = viewState.position;
		const selectionStartPosition = viewState.selectionStart.getStartPosition();
		const selectionEndPosition = viewState.selectionStart.getEndPosition();
		const validPosition = viewModel.normalizePosition(position, PositionAffinity.None);
		const validSelectionStartPosition = this._validatePositionWithCache(viewModel, selectionStartPosition, position, validPosition);
		const validSelectionEndPosition = this._validatePositionWithCache(viewModel, selectionEndPosition, selectionStartPosition, validSelectionStartPosition);

		if (position.equals(validPosition)
			&& selectionStartPosition.equals(validSelectionStartPosition)
			&& selectionEndPosition.equals(validSelectionEndPosition)) {
			return viewState;
		}

		return new SingleCursorState(
			Range.fromPositions(validSelectionStartPosition, validSelectionEndPosition),
			viewState.selectionStartKind,
			viewState.selectionStartLeftoverVisibleColumns + selectionStartPosition.column - validSelectionStartPosition.column,
			validPosition,
			viewState.leftoverVisibleColumns + position.column - validPosition.column,
		);
	}

	private _setState(context: CursorContext, modelState: SingleCursorState | null, viewState: SingleCursorState | null): void {
		if (viewState) viewState = Cursor._validateViewState(context.viewModel, viewState);

		if (!modelState) {
			if (!viewState) return;
			const selectionStart = context.model.validateRange(
				context.coordinatesConverter.convertViewRangeToModelRange(viewState.selectionStart),
			);
			const position = context.model.validatePosition(
				context.coordinatesConverter.convertViewPositionToModelPosition(viewState.position),
			);
			modelState = new SingleCursorState(
				selectionStart,
				viewState.selectionStartKind,
				viewState.selectionStartLeftoverVisibleColumns,
				position,
				viewState.leftoverVisibleColumns,
			);
		} else {
			const selectionStart = context.model.validateRange(modelState.selectionStart);
			const selectionStartLeftoverVisibleColumns = modelState.selectionStart.equalsRange(selectionStart)
				? modelState.selectionStartLeftoverVisibleColumns
				: 0;
			const position = context.model.validatePosition(modelState.position);
			const leftoverVisibleColumns = modelState.position.equals(position) ? modelState.leftoverVisibleColumns : 0;
			modelState = new SingleCursorState(
				selectionStart,
				modelState.selectionStartKind,
				selectionStartLeftoverVisibleColumns,
				position,
				leftoverVisibleColumns,
			);
		}

		if (!viewState) {
			const viewSelectionStart = Range.fromPositions(
				context.coordinatesConverter.convertModelPositionToViewPosition(modelState.selectionStart.getStartPosition()),
				context.coordinatesConverter.convertModelPositionToViewPosition(modelState.selectionStart.getEndPosition()),
			);
			const viewPosition = context.coordinatesConverter.convertModelPositionToViewPosition(modelState.position);
			viewState = new SingleCursorState(
				viewSelectionStart,
				modelState.selectionStartKind,
				modelState.selectionStartLeftoverVisibleColumns,
				viewPosition,
				modelState.leftoverVisibleColumns,
			);
		} else {
			const viewSelectionStart = context.coordinatesConverter.validateViewRange(viewState.selectionStart, modelState.selectionStart);
			const viewPosition = context.coordinatesConverter.validateViewPosition(viewState.position, modelState.position);
			viewState = new SingleCursorState(
				viewSelectionStart,
				modelState.selectionStartKind,
				modelState.selectionStartLeftoverVisibleColumns,
				viewPosition,
				modelState.leftoverVisibleColumns,
			);
		}

		this.modelState = modelState;
		this.viewState = viewState;
		this._updateTrackedRange(context);
	}
}

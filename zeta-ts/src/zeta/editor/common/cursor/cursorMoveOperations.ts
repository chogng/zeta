import * as strings from '../../../base/common/strings.js';
import { Constants } from '../../../base/common/uint.js';
import { CursorColumns } from '../core/cursorColumns.js';
import { Position } from '../core/position.js';
import { Range } from '../core/range.js';
import { PositionAffinity } from '../model.js';
import { CursorConfiguration, type ICursorSimpleModel, SelectionStartKind, SingleCursorState } from '../cursorCommon.js';
import { AtomicTabMoveOperations, Direction } from './cursorAtomicMoveOperations.js';

export class CursorPosition {
	_cursorPositionBrand: void = undefined;

	constructor(
		public readonly lineNumber: number,
		public readonly column: number,
		public readonly leftoverVisibleColumns: number,
	) {}
}

export class MoveOperations {
	public static leftPosition(model: ICursorSimpleModel, position: Position): Position {
		if (position.column > model.getLineMinColumn(position.lineNumber)) {
			return position.delta(undefined, -strings.prevCharLength(model.getLineContent(position.lineNumber), position.column - 1));
		}
		if (position.lineNumber > 1) {
			const lineNumber = position.lineNumber - 1;
			return new Position(lineNumber, model.getLineMaxColumn(lineNumber));
		}
		return position;
	}

	private static leftPositionAtomicSoftTabs(model: ICursorSimpleModel, position: Position, tabSize: number): Position {
		if (position.column <= model.getLineIndentColumn(position.lineNumber)) {
			const column = AtomicTabMoveOperations.atomicPosition(model.getLineContent(position.lineNumber), position.column - 1, tabSize, Direction.Left);
			if (column !== -1 && column + 1 >= model.getLineMinColumn(position.lineNumber)) return new Position(position.lineNumber, column + 1);
		}
		return this.leftPosition(model, position);
	}

	private static left(config: CursorConfiguration, model: ICursorSimpleModel, position: Position): CursorPosition {
		const result = config.stickyTabStops
			? this.leftPositionAtomicSoftTabs(model, position, config.tabSize)
			: this.leftPosition(model, position);
		return new CursorPosition(result.lineNumber, result.column, 0);
	}

	public static moveLeft(config: CursorConfiguration, model: ICursorSimpleModel, cursor: SingleCursorState, inSelectionMode: boolean, noOfColumns: number): SingleCursorState {
		let lineNumber: number;
		let column: number;
		if (cursor.hasSelection() && !inSelectionMode) {
			lineNumber = cursor.selection.startLineNumber;
			column = cursor.selection.startColumn;
		} else {
			const position = cursor.position.delta(undefined, -(noOfColumns - 1));
			const normalized = model.normalizePosition(this.clipPositionColumn(position, model), PositionAffinity.Left);
			const result = this.left(config, model, normalized);
			lineNumber = result.lineNumber;
			column = result.column;
		}
		return cursor.move(inSelectionMode, lineNumber, column, 0);
	}

	private static clipPositionColumn(position: Position, model: ICursorSimpleModel): Position {
		return new Position(position.lineNumber, this.clipRange(position.column, model.getLineMinColumn(position.lineNumber), model.getLineMaxColumn(position.lineNumber)));
	}

	private static clipRange(value: number, min: number, max: number): number {
		return Math.max(min, Math.min(max, value));
	}

	public static rightPosition(model: ICursorSimpleModel, lineNumber: number, column: number): Position {
		if (column < model.getLineMaxColumn(lineNumber)) {
			column += strings.nextCharLength(model.getLineContent(lineNumber), column - 1);
		} else if (lineNumber < model.getLineCount()) {
			lineNumber++;
			column = model.getLineMinColumn(lineNumber);
		}
		return model.normalizePosition(new Position(lineNumber, column), PositionAffinity.Right);
	}

	public static rightPositionAtomicSoftTabs(model: ICursorSimpleModel, lineNumber: number, column: number, tabSize: number, _indentSize: number): Position {
		if (column < model.getLineIndentColumn(lineNumber)) {
			const next = AtomicTabMoveOperations.atomicPosition(model.getLineContent(lineNumber), column - 1, tabSize, Direction.Right);
			if (next !== -1) return new Position(lineNumber, next + 1);
		}
		return this.rightPosition(model, lineNumber, column);
	}

	public static right(config: CursorConfiguration, model: ICursorSimpleModel, position: Position): CursorPosition {
		const result = config.stickyTabStops
			? this.rightPositionAtomicSoftTabs(model, position.lineNumber, position.column, config.tabSize, config.indentSize)
			: this.rightPosition(model, position.lineNumber, position.column);
		return new CursorPosition(result.lineNumber, result.column, 0);
	}

	public static moveRight(config: CursorConfiguration, model: ICursorSimpleModel, cursor: SingleCursorState, inSelectionMode: boolean, noOfColumns: number): SingleCursorState {
		let lineNumber: number;
		let column: number;
		if (cursor.hasSelection() && !inSelectionMode) {
			lineNumber = cursor.selection.endLineNumber;
			column = cursor.selection.endColumn;
		} else {
			const position = cursor.position.delta(undefined, noOfColumns - 1);
			const normalized = model.normalizePosition(this.clipPositionColumn(position, model), PositionAffinity.Right);
			const result = this.right(config, model, normalized);
			lineNumber = result.lineNumber;
			column = result.column;
		}
		return cursor.move(inSelectionMode, lineNumber, column, 0);
	}

	public static vertical(config: CursorConfiguration, model: ICursorSimpleModel, lineNumber: number, column: number, leftoverVisibleColumns: number, newLineNumber: number, allowMoveOnEdgeLine: boolean, normalizationAffinity?: PositionAffinity): CursorPosition {
		const currentVisibleColumn = CursorColumns.visibleColumnFromColumn(model.getLineContent(lineNumber), column, config.tabSize) + leftoverVisibleColumns;
		const lineCount = model.getLineCount();
		const wasOnFirstPosition = lineNumber === 1 && column === 1;
		const wasOnLastPosition = lineNumber === lineCount && column === model.getLineMaxColumn(lineNumber);
		const wasAtEdgePosition = newLineNumber < lineNumber ? wasOnFirstPosition : wasOnLastPosition;
		lineNumber = newLineNumber;
		if (lineNumber < 1) {
			lineNumber = 1;
			column = allowMoveOnEdgeLine ? model.getLineMinColumn(lineNumber) : Math.min(model.getLineMaxColumn(lineNumber), column);
		} else if (lineNumber > lineCount) {
			lineNumber = lineCount;
			column = allowMoveOnEdgeLine ? model.getLineMaxColumn(lineNumber) : Math.min(model.getLineMaxColumn(lineNumber), column);
		} else {
			column = config.columnFromVisibleColumn(model, lineNumber, currentVisibleColumn);
		}
		leftoverVisibleColumns = wasAtEdgePosition ? 0 : currentVisibleColumn - CursorColumns.visibleColumnFromColumn(model.getLineContent(lineNumber), column, config.tabSize);
		if (normalizationAffinity !== undefined) {
			const normalized = model.normalizePosition(new Position(lineNumber, column), normalizationAffinity);
			leftoverVisibleColumns += column - normalized.column;
			lineNumber = normalized.lineNumber;
			column = normalized.column;
		}
		return new CursorPosition(lineNumber, column, leftoverVisibleColumns);
	}

	public static down(config: CursorConfiguration, model: ICursorSimpleModel, lineNumber: number, column: number, leftoverVisibleColumns: number, count: number, allowMoveOnLastLine: boolean): CursorPosition {
		return this.vertical(config, model, lineNumber, column, leftoverVisibleColumns, lineNumber + count, allowMoveOnLastLine, PositionAffinity.RightOfInjectedText);
	}

	public static moveDown(config: CursorConfiguration, model: ICursorSimpleModel, cursor: SingleCursorState, inSelectionMode: boolean, linesCount: number): SingleCursorState {
		let lineNumber = cursor.hasSelection() && !inSelectionMode ? cursor.selection.endLineNumber : cursor.position.lineNumber;
		const column = cursor.hasSelection() && !inSelectionMode ? cursor.selection.endColumn : cursor.position.column;
		let index = 0;
		let result: CursorPosition;
		do {
			result = this.down(config, model, lineNumber + index, column, cursor.leftoverVisibleColumns, linesCount, true);
			if (model.normalizePosition(new Position(result.lineNumber, result.column), PositionAffinity.None).lineNumber > lineNumber) break;
		} while (index++ < 10 && lineNumber + index < model.getLineCount());
		return cursor.move(inSelectionMode, result.lineNumber, result.column, result.leftoverVisibleColumns);
	}

	public static translateDown(config: CursorConfiguration, model: ICursorSimpleModel, cursor: SingleCursorState): SingleCursorState {
		const selection = cursor.selection;
		const start = this.down(config, model, selection.selectionStartLineNumber, selection.selectionStartColumn, cursor.selectionStartLeftoverVisibleColumns, 1, false);
		const position = this.down(config, model, selection.positionLineNumber, selection.positionColumn, cursor.leftoverVisibleColumns, 1, false);
		return new SingleCursorState(new Range(start.lineNumber, start.column, start.lineNumber, start.column), SelectionStartKind.Simple, start.leftoverVisibleColumns, new Position(position.lineNumber, position.column), position.leftoverVisibleColumns);
	}

	public static up(config: CursorConfiguration, model: ICursorSimpleModel, lineNumber: number, column: number, leftoverVisibleColumns: number, count: number, allowMoveOnFirstLine: boolean): CursorPosition {
		return this.vertical(config, model, lineNumber, column, leftoverVisibleColumns, lineNumber - count, allowMoveOnFirstLine, PositionAffinity.LeftOfInjectedText);
	}

	public static moveUp(config: CursorConfiguration, model: ICursorSimpleModel, cursor: SingleCursorState, inSelectionMode: boolean, linesCount: number): SingleCursorState {
		const lineNumber = cursor.hasSelection() && !inSelectionMode ? cursor.selection.startLineNumber : cursor.position.lineNumber;
		const column = cursor.hasSelection() && !inSelectionMode ? cursor.selection.startColumn : cursor.position.column;
		const result = this.up(config, model, lineNumber, column, cursor.leftoverVisibleColumns, linesCount, true);
		return cursor.move(inSelectionMode, result.lineNumber, result.column, result.leftoverVisibleColumns);
	}

	public static translateUp(config: CursorConfiguration, model: ICursorSimpleModel, cursor: SingleCursorState): SingleCursorState {
		const selection = cursor.selection;
		const start = this.up(config, model, selection.selectionStartLineNumber, selection.selectionStartColumn, cursor.selectionStartLeftoverVisibleColumns, 1, false);
		const position = this.up(config, model, selection.positionLineNumber, selection.positionColumn, cursor.leftoverVisibleColumns, 1, false);
		return new SingleCursorState(new Range(start.lineNumber, start.column, start.lineNumber, start.column), SelectionStartKind.Simple, start.leftoverVisibleColumns, new Position(position.lineNumber, position.column), position.leftoverVisibleColumns);
	}

	private static _isBlankLine(model: ICursorSimpleModel, lineNumber: number): boolean {
		return model.getLineFirstNonWhitespaceColumn(lineNumber) === 0;
	}

	public static moveToPrevBlankLine(_config: CursorConfiguration, model: ICursorSimpleModel, cursor: SingleCursorState, inSelectionMode: boolean): SingleCursorState {
		let lineNumber = cursor.position.lineNumber;
		while (lineNumber > 1 && this._isBlankLine(model, lineNumber)) lineNumber--;
		while (lineNumber > 1 && !this._isBlankLine(model, lineNumber)) lineNumber--;
		return cursor.move(inSelectionMode, lineNumber, model.getLineMinColumn(lineNumber), 0);
	}

	public static moveToNextBlankLine(_config: CursorConfiguration, model: ICursorSimpleModel, cursor: SingleCursorState, inSelectionMode: boolean): SingleCursorState {
		const lineCount = model.getLineCount();
		let lineNumber = cursor.position.lineNumber;
		while (lineNumber < lineCount && this._isBlankLine(model, lineNumber)) lineNumber++;
		while (lineNumber < lineCount && !this._isBlankLine(model, lineNumber)) lineNumber++;
		return cursor.move(inSelectionMode, lineNumber, model.getLineMinColumn(lineNumber), 0);
	}

	public static moveToBeginningOfLine(_config: CursorConfiguration, model: ICursorSimpleModel, cursor: SingleCursorState, inSelectionMode: boolean): SingleCursorState {
		const lineNumber = cursor.position.lineNumber;
		const minColumn = model.getLineMinColumn(lineNumber);
		const firstNonBlankColumn = model.getLineFirstNonWhitespaceColumn(lineNumber) || minColumn;
		return cursor.move(inSelectionMode, lineNumber, cursor.position.column === firstNonBlankColumn ? minColumn : firstNonBlankColumn, 0);
	}

	public static moveToEndOfLine(_config: CursorConfiguration, model: ICursorSimpleModel, cursor: SingleCursorState, inSelectionMode: boolean, sticky: boolean): SingleCursorState {
		const lineNumber = cursor.position.lineNumber;
		const maxColumn = model.getLineMaxColumn(lineNumber);
		return cursor.move(inSelectionMode, lineNumber, maxColumn, sticky ? Constants.MAX_SAFE_SMALL_INTEGER - maxColumn : 0);
	}

	public static moveToBeginningOfBuffer(_config: CursorConfiguration, _model: ICursorSimpleModel, cursor: SingleCursorState, inSelectionMode: boolean): SingleCursorState {
		return cursor.move(inSelectionMode, 1, 1, 0);
	}

	public static moveToEndOfBuffer(_config: CursorConfiguration, model: ICursorSimpleModel, cursor: SingleCursorState, inSelectionMode: boolean): SingleCursorState {
		const lineNumber = model.getLineCount();
		return cursor.move(inSelectionMode, lineNumber, model.getLineMaxColumn(lineNumber), 0);
	}
}

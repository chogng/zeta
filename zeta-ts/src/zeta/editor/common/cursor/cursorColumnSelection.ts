import { CursorConfiguration, type IColumnSelectData, type ICursorSimpleModel, SelectionStartKind, SingleCursorState } from '../cursorCommon.js';
import { Position } from '../core/position.js';
import { Range } from '../core/range.js';

export class ColumnSelection {
	public static columnSelect(config: CursorConfiguration, model: ICursorSimpleModel, fromLineNumber: number, fromVisibleColumn: number, toLineNumber: number, toVisibleColumn: number): IColumnSelectResult {
		const lineCount = Math.abs(toLineNumber - fromLineNumber) + 1;
		const reversed = fromLineNumber > toLineNumber;
		const isRTL = fromVisibleColumn > toVisibleColumn;
		const isLTR = fromVisibleColumn < toVisibleColumn;
		const result: SingleCursorState[] = [];

		for (let index = 0; index < lineCount; index += 1) {
			const lineNumber = fromLineNumber + (reversed ? -index : index);
			const startColumn = config.columnFromVisibleColumn(model, lineNumber, fromVisibleColumn);
			const endColumn = config.columnFromVisibleColumn(model, lineNumber, toVisibleColumn);
			const visibleStartColumn = config.visibleColumnFromColumn(model, new Position(lineNumber, startColumn));
			const visibleEndColumn = config.visibleColumnFromColumn(model, new Position(lineNumber, endColumn));
			if (isLTR && (visibleStartColumn > toVisibleColumn || visibleEndColumn < fromVisibleColumn)) continue;
			if (isRTL && (visibleEndColumn > fromVisibleColumn || visibleStartColumn < toVisibleColumn)) continue;
			result.push(new SingleCursorState(new Range(lineNumber, startColumn, lineNumber, startColumn), SelectionStartKind.Simple, 0, new Position(lineNumber, endColumn), 0));
		}

		if (result.length === 0) {
			for (let index = 0; index < lineCount; index += 1) {
				const lineNumber = fromLineNumber + (reversed ? -index : index);
				const maxColumn = model.getLineMaxColumn(lineNumber);
				result.push(new SingleCursorState(new Range(lineNumber, maxColumn, lineNumber, maxColumn), SelectionStartKind.Simple, 0, new Position(lineNumber, maxColumn), 0));
			}
		}

		return { viewStates: result, reversed, fromLineNumber, fromVisualColumn: fromVisibleColumn, toLineNumber, toVisualColumn: toVisibleColumn };
	}

	public static columnSelectLeft(config: CursorConfiguration, model: ICursorSimpleModel, previous: IColumnSelectData): IColumnSelectResult {
		return ColumnSelection.columnSelect(config, model, previous.fromViewLineNumber, previous.fromViewVisualColumn, previous.toViewLineNumber, Math.max(0, previous.toViewVisualColumn - 1));
	}

	public static columnSelectRight(config: CursorConfiguration, model: ICursorSimpleModel, previous: IColumnSelectData): IColumnSelectResult {
		let maxVisualColumn = 0;
		const minLineNumber = Math.min(previous.fromViewLineNumber, previous.toViewLineNumber);
		const maxLineNumber = Math.max(previous.fromViewLineNumber, previous.toViewLineNumber);
		for (let lineNumber = minLineNumber; lineNumber <= maxLineNumber; lineNumber += 1) {
			maxVisualColumn = Math.max(maxVisualColumn, config.visibleColumnFromColumn(model, new Position(lineNumber, model.getLineMaxColumn(lineNumber))));
		}
		return ColumnSelection.columnSelect(config, model, previous.fromViewLineNumber, previous.fromViewVisualColumn, previous.toViewLineNumber, Math.min(maxVisualColumn, previous.toViewVisualColumn + 1));
	}

	public static columnSelectUp(config: CursorConfiguration, model: ICursorSimpleModel, previous: IColumnSelectData, isPaged: boolean): IColumnSelectResult {
		const lineCount = isPaged ? config.pageSize : 1;
		return ColumnSelection.columnSelect(config, model, previous.fromViewLineNumber, previous.fromViewVisualColumn, Math.max(1, previous.toViewLineNumber - lineCount), previous.toViewVisualColumn);
	}

	public static columnSelectDown(config: CursorConfiguration, model: ICursorSimpleModel, previous: IColumnSelectData, isPaged: boolean): IColumnSelectResult {
		const lineCount = isPaged ? config.pageSize : 1;
		return ColumnSelection.columnSelect(config, model, previous.fromViewLineNumber, previous.fromViewVisualColumn, Math.min(model.getLineCount(), previous.toViewLineNumber + lineCount), previous.toViewVisualColumn);
	}
}

export interface IColumnSelectResult {
	viewStates: SingleCursorState[];
	reversed: boolean;
	fromLineNumber: number;
	fromVisualColumn: number;
	toLineNumber: number;
	toVisualColumn: number;
}

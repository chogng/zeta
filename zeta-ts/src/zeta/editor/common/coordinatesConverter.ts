import { Position } from './core/position.js';
import { Range } from './core/range.js';
import { PositionAffinity, type ITextModel } from './model.js';

export interface ICoordinatesConverter {
	convertViewPositionToModelPosition(viewPosition: Position): Position;
	convertViewRangeToModelRange(viewRange: Range): Range;
	validateViewPosition(viewPosition: Position, expectedModelPosition: Position): Position;
	validateViewRange(viewRange: Range, expectedModelRange: Range): Range;
	convertModelPositionToViewPosition(modelPosition: Position, affinity?: PositionAffinity, allowZeroLineNumber?: boolean, belowHiddenRanges?: boolean): Position;
	convertModelRangeToViewRange(modelRange: Range, affinity?: PositionAffinity): Range;
	modelPositionIsVisible(modelPosition: Position): boolean;
	getModelLineViewLineCount(modelLineNumber: number): number;
	getViewLineNumberOfModelPosition(modelLineNumber: number, modelColumn: number): number;
}

export class IdentityCoordinatesConverter implements ICoordinatesConverter {
	public constructor(private readonly model: ITextModel) {}

	public convertViewPositionToModelPosition(viewPosition: Position): Position {
		return this.model.validatePosition(viewPosition);
	}

	public convertViewRangeToModelRange(viewRange: Range): Range {
		return this.model.validateRange(viewRange);
	}

	public validateViewPosition(_viewPosition: Position, expectedModelPosition: Position): Position {
		return this.model.validatePosition(expectedModelPosition);
	}

	public validateViewRange(_viewRange: Range, expectedModelRange: Range): Range {
		return this.model.validateRange(expectedModelRange);
	}

	public convertModelPositionToViewPosition(modelPosition: Position): Position {
		return this.model.validatePosition(modelPosition);
	}

	public convertModelRangeToViewRange(modelRange: Range): Range {
		return this.model.validateRange(modelRange);
	}

	public modelPositionIsVisible(modelPosition: Position): boolean {
		return modelPosition.lineNumber >= 1 && modelPosition.lineNumber <= this.model.getLineCount();
	}

	public modelRangeIsVisible(modelRange: Range): boolean {
		return modelRange.startLineNumber >= 1
			&& modelRange.startLineNumber <= this.model.getLineCount()
			&& modelRange.endLineNumber >= 1
			&& modelRange.endLineNumber <= this.model.getLineCount();
	}

	public getModelLineViewLineCount(_modelLineNumber: number): number {
		return 1;
	}

	public getViewLineNumberOfModelPosition(modelLineNumber: number, _modelColumn: number): number {
		return modelLineNumber;
	}
}

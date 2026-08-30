import { Range } from '../core/range.js';
import { Selection } from '../core/selection.js';
import type { ICommand, ICursorStateComputerData, IEditOperationBuilder } from '../editorCommon.js';
import type { ITextModel } from '../model.js';

export class ReplaceCommand implements ICommand {
	private readonly range: Range;
	private readonly text: string;
	public readonly insertsAutoWhitespace: boolean;

	constructor(range: Range, text: string, insertsAutoWhitespace = false) {
		this.range = range;
		this.text = text;
		this.insertsAutoWhitespace = insertsAutoWhitespace;
	}

	public getEditOperations(_model: ITextModel, builder: IEditOperationBuilder): void {
		builder.addTrackedEditOperation(this.range, this.text);
	}

	public computeCursorState(_model: ITextModel, helper: ICursorStateComputerData): Selection {
		const sourceRange = helper.getInverseEditOperations()[0]!.range;
		return Selection.fromPositions(sourceRange.getEndPosition());
	}
}

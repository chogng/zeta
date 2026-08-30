import type { IRange } from './core/range.js';
import type { ISelection, Selection } from './core/selection.js';
import type { ITextModel, IValidEditOperation } from './model.js';

/** Collects edit operations and tracked selections for one editor command. */
export interface IEditOperationBuilder {
	addEditOperation(range: IRange, text: string | null, forceMoveMarkers?: boolean): void;
	addTrackedEditOperation(range: IRange, text: string | null, forceMoveMarkers?: boolean): void;
	trackSelection(selection: ISelection, trackPreviousOnEmpty?: boolean): string;
}

/** Supplies inverse edits and tracked selections after a command has run. */
export interface ICursorStateComputerData {
	getInverseEditOperations(): IValidEditOperation[];
	getTrackedSelection(id: string): Selection;
}

/** A model edit whose resulting selection is computed after the edits apply. */
export interface ICommand {
	readonly insertsAutoWhitespace?: boolean;
	getEditOperations(model: ITextModel, builder: IEditOperationBuilder): void;
	computeCursorState(model: ITextModel, helper: ICursorStateComputerData): Selection;
}

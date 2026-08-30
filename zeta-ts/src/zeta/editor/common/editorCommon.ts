import type { IRange } from './core/range.js';
import type { IPosition } from './core/position.js';
import type { ISelection, Selection } from './core/selection.js';
import type { ITextModel, IValidEditOperation } from './model.js';

export interface INewScrollPosition {
	scrollLeft?: number;
	scrollTop?: number;
}

export interface IScrollEvent {
	readonly scrollTop: number;
	readonly scrollLeft: number;
	readonly scrollWidth: number;
	readonly scrollHeight: number;
	readonly scrollTopChanged: boolean;
	readonly scrollLeftChanged: boolean;
	readonly scrollWidthChanged: boolean;
	readonly scrollHeightChanged: boolean;
}

export interface IContentSizeChangedEvent {
	readonly contentWidth: number;
	readonly contentHeight: number;
	readonly contentWidthChanged: boolean;
	readonly contentHeightChanged: boolean;
}


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

/** A serializable state of one cursor. */
export interface ICursorState {
	inSelectionMode: boolean;
	selectionStart: IPosition;
	position: IPosition;
}

/** A serializable state of the editor view. */
export interface IViewState {
	scrollTop?: number;
	scrollTopWithoutViewZones?: number;
	scrollLeft: number;
	firstPosition: IPosition;
	firstPositionDeltaTop: number;
}

export const enum ScrollType {
	Smooth = 0,
	Immediate = 1,
}

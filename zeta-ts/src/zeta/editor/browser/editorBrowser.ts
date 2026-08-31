import { type Event } from '../../base/common/event.js';
import { type IDimension } from '../common/core/2d/dimension.js';
import { type IPosition } from '../common/core/position.js';
import { type Range } from '../common/core/range.js';
import { type ISelection, type Selection } from '../common/core/selection.js';
import { type IModelDecorationsChangeAccessor, type PositionAffinity } from '../common/model.js';
import { type IModelDeltaDecoration } from '../common/model.js';
import { type EditorLayoutInfo, type EditorOption, type FindComputedEditorOptionValueById } from '../common/config/editorOptions.js';
import { type ICommand, type IEditorContribution, type IEditorDecorationsCollection, type ScrollType } from '../common/editorCommon.js';
import { type ITextModel } from '../common/model.js';
import { type ServicesAccessor } from '../../platform/instantiation/common/instantiation.js';
import { type IClipboardPasteEvent } from './controller/editContext/clipboardUtils.js';
import { type EditorViewMouseTarget } from './view/viewUserInputEvents.js';

export interface IEditorMouseEvent {
	readonly event: MouseEvent | PointerEvent;
	readonly target?: EditorViewMouseTarget;
}

/** Browser-facing contract shared by editor services and contributions. */
export interface ICodeEditor {
	readonly onDidDispose: Event<void>;
	readonly onDidAttemptReadOnlyEdit: Event<void>;
	readonly onDidLayoutChange: Event<EditorLayoutInfo>;
	readonly onDidChangeCursorSelection: Event<void>;
	readonly onDidCompositionStart: Event<void>;
	readonly onDidCompositionEnd: Event<void>;
	readonly onDidType: Event<string>;
	readonly onDidPaste: Event<IClipboardPasteEvent>;
	readonly onMouseMove: Event<IEditorMouseEvent>;
	readonly onMouseLeave: Event<IEditorMouseEvent>;
	readonly inComposition: boolean;
	getId(): string;
	focus(): void;
	hasTextFocus(): boolean;
	hasWidgetFocus(): boolean;
	getModel(): ITextModel | null;
	hasModel(): boolean;
	getPosition(): IPosition | null;
	getScrollTop(): number;
	getScrollLeft(): number;
	getContentHeight(): number;
	getContentWidth(): number;
	hasPendingScrollAnimation(): boolean;
	getVisibleRanges(): Range[];
	getTopForPosition(lineNumber: number, column: number): number;
	getTopForLineNumber(lineNumber: number): number;
	getBottomForLineNumber(lineNumber: number): number;
	setScrollTop(newScrollTop: number, scrollType?: ScrollType): void;
	getSelection(): Selection | null;
	getSelections(): Selection[] | null;
	setSelection(selection: ISelection, source?: string): void;
	setSelections(selections: readonly ISelection[], source?: string): void;
	executeCommand(source: string | null | undefined, command: ICommand): void;
	executeCommands(source: string | null | undefined, commands: ICommand[]): void;
	pushUndoStop(): boolean;
	getContribution<T extends IEditorContribution>(id: string): T | null;
	invokeWithinContext<T>(fn: (accessor: ServicesAccessor) => T): T;
	getContainerDomNode(): HTMLElement;
	getDomNode(): HTMLElement | null;
	getLayoutInfo(): EditorLayoutInfo;
	getOption<T extends EditorOption>(id: T): FindComputedEditorOptionValueById<T>;
	getScrolledVisiblePosition(position: IPosition): { top: number; left: number; height: number } | null;
	getWidthOfLine(lineNumber: number): number;
	applyFontInfo(target: HTMLElement): void;
	createDecorationsCollection(decorations?: IModelDeltaDecoration[]): IEditorDecorationsCollection;
	changeDecorations<T>(callback: (changeAccessor: IModelDecorationsChangeAccessor) => T): T | null;
	removeDecorations(decorationIds: string[]): void;
	removeDecorationsByType(key: string): void;
	addContentWidget(widget: IContentWidget): void;
	layoutContentWidget(widget: IContentWidget): void;
	removeContentWidget(widget: IContentWidget): void;
	addOverlayWidget(widget: IOverlayWidget): void;
	layoutOverlayWidget(widget: IOverlayWidget): void;
	removeOverlayWidget(widget: IOverlayWidget): void;
	changeViewZones(callback: (accessor: IViewZoneChangeAccessor) => void): void;
	revealRange(range: Range, scrollType?: ScrollType): void;
}

export interface IDiffEditor {
	getId(): string;
}

export interface IViewZone {
	afterLineNumber: number;
	afterColumn?: number;
	afterColumnAffinity?: PositionAffinity;
	showInHiddenAreas?: boolean;
	heightInLines?: number;
	heightInPx?: number;
	ordinal?: number;
	minWidthInPx?: number;
	suppressMouseDown?: boolean;
	readonly domNode: HTMLElement;
	readonly marginDomNode?: HTMLElement | null;
	onDomNodeTop?: (top: number) => void;
	onComputedHeight?: (height: number) => void;
}

export interface IViewZoneChangeAccessor {
	addZone(zone: IViewZone): string;
	removeZone(id: string): void;
	layoutZone(id: string): void;
}

export const enum ContentWidgetPositionPreference {
	EXACT,
	ABOVE,
	BELOW,
}

export interface IContentWidgetPosition {
	readonly position: IPosition | null;
	readonly secondaryPosition?: IPosition | null;
	readonly preference: ContentWidgetPositionPreference[];
	readonly positionAffinity?: PositionAffinity;
}

export interface IContentWidget {
	readonly allowEditorOverflow?: boolean;
	readonly useDisplayNone?: boolean;
	readonly suppressMouseDown?: boolean;
	getId(): string;
	getDomNode(): HTMLElement;
	getPosition(): IContentWidgetPosition | null;
	beforeRender?(): IDimension | null;
	afterRender?(position: ContentWidgetPositionPreference | null, coordinate: IContentWidgetRenderedCoordinate | null): void;
}

export interface IContentWidgetRenderedCoordinate {
	readonly top: number;
	readonly left: number;
}

export const enum OverlayWidgetPositionPreference {
	TOP_RIGHT_CORNER,
	BOTTOM_RIGHT_CORNER,
	TOP_CENTER,
}

export interface IOverlayWidgetPositionCoordinates {
	readonly top: number;
	readonly left: number;
}

export interface IOverlayWidgetPosition {
	readonly preference: OverlayWidgetPositionPreference | IOverlayWidgetPositionCoordinates | null;
	readonly stackOrdinal?: number;
}

export interface IOverlayWidget {
	readonly onDidLayout?: Event<void>;
	readonly allowEditorOverflow?: boolean;
	getId(): string;
	getDomNode(): HTMLElement;
	getPosition(): IOverlayWidgetPosition | null;
	getMinContentWidthInPx?(): number;
}

export interface IEditorAriaOptions {
	activeDescendant: string | undefined;
	role?: string;
}

import { type Event } from '../../base/common/event.js';
import { type IDimension } from '../common/core/2d/dimension.js';
import { type IPosition } from '../common/core/position.js';
import { type PositionAffinity } from '../common/model.js';
import { type IEditorContribution } from '../common/editorCommon.js';
import { type ITextModel } from '../common/model.js';
import { type ServicesAccessor } from '../../platform/instantiation/common/instantiation.js';

/** Browser-facing contract shared by editor services and contributions. */
export interface ICodeEditor {
	getId(): string;
	focus(): void;
	hasTextFocus(): boolean;
	hasWidgetFocus(): boolean;
	getModel(): ITextModel | null;
	getContribution<T extends IEditorContribution>(id: string): T | null;
	invokeWithinContext<T>(fn: (accessor: ServicesAccessor) => T): T;
	getContainerDomNode(): HTMLElement;
	applyFontInfo(target: HTMLElement): void;
	removeDecorationsByType(key: string): void;
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

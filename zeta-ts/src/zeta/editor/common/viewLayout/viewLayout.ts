import { Emitter, type Event } from '../../../base/common/event.js';
import { type ISize } from '../../../base/common/layout.js';
import { Disposable } from '../../../base/common/lifecycle.js';
import { clamp, isFiniteNumber, isNonNegativeSafeInteger, isPositiveSafeInteger } from '../../../base/common/numbers.js';
import { type TextModelChange } from '../core/textChange.js';
import { type EditorLineHeightChangeAccessor, type EditorLineRange, type EditorScrollPosition, type EditorViewZoneLayout, type EditorViewportLineSource, type EditorViewportModelSource } from '../viewModel/editorViewportContracts.js';
import { EditorViewportLinesLayout, type EditorViewportVerticalPadding } from './editorViewportLinesLayout.js';
import { type CustomLineHeightData } from './lineHeights.js';

export type { EditorLineHeightChangeAccessor, EditorLineRange, EditorScrollPosition, EditorViewportLineSource, EditorViewportModelSource } from '../viewModel/editorViewportContracts.js';
export type { EditorViewportVerticalPadding } from './editorViewportLinesLayout.js';

export interface EditorViewportLayout {
	readonly modelVersion: number;
	readonly lineHeight: number;
	readonly viewportSize: ISize;
	readonly contentSize: ISize;
	readonly scrollPosition: EditorScrollPosition;
	readonly maximumScrollPosition: EditorScrollPosition;
	readonly visibleLines: EditorLineRange;
	readonly renderLines: EditorLineRange;
	readonly renderTop: number;
	readonly relativeVerticalOffset?: readonly number[];
	readonly viewZones?: readonly EditorViewZoneLayout[];
}

export enum EditorViewportChangeReason {
	Model = 'model',
	LineProjection = 'lineProjection',
	ViewportSize = 'viewportSize',
	ContentWidth = 'contentWidth',
	LineHeight = 'lineHeight',
	Scroll = 'scroll',
	ViewZones = 'viewZones',
}

export interface EditorViewportChange {
	readonly reason: EditorViewportChangeReason;
	readonly layout: EditorViewportLayout;
	readonly modelChange?: TextModelChange;
}

export interface EditorViewportOptions {
	readonly lineHeight: number;
	readonly overscanLineCount?: number;
	readonly lineSource?: EditorViewportLineSource;
	readonly padding?: EditorViewportVerticalPadding;
	readonly customLineHeightData?: readonly CustomLineHeightData[];
}

/**
 * Owns the immutable layout snapshot shared by the browser view and view-model.
 *
 * Horizontal measurement and scroll state stay here, while
 * `EditorViewportLinesLayout` owns line heights, padding, and visible/render
 * line projection for Zeta's immutable viewport snapshots.
 */
export class ViewLayout extends Disposable {
	private readonly changeEmitter = this._register(new Emitter<EditorViewportChange>());
	private readonly lineSource: EditorViewportLineSource;
	private readonly linesLayout: EditorViewportLinesLayout;
	private viewportSize: ISize = Object.freeze({ width: 0, height: 0 });
	private measuredContentWidth = 0;
	private requestedScrollPosition: EditorScrollPosition = Object.freeze({ left: 0, top: 0 });
	private currentLayout: EditorViewportLayout;

	public readonly onDidChange: Event<EditorViewportChange> = this.changeEmitter.event;

	public constructor(private readonly model: EditorViewportModelSource, options: EditorViewportOptions) {
		super();
		validateModelSource(model);
		if (!options || typeof options !== 'object') throw new TypeError('View layout requires options');
		const lineHeight = positiveFinite(options.lineHeight, 'lineHeight');
		const overscanLineCount = nonNegativeSafeInteger(options.overscanLineCount ?? 2, 'overscanLineCount');
		this.lineSource = options.lineSource ?? createTextModelLineSource(model);
		const padding = readPadding(options.padding);
		this.linesLayout = new EditorViewportLinesLayout(
			this.lineSource,
			lineHeight,
			padding.top,
			padding.bottom,
			overscanLineCount,
			options.customLineHeightData,
		);
		this.currentLayout = this.createLayout();
		this._register(model.onDidChange(change => this.publish(EditorViewportChangeReason.Model, change)));
		if (options.lineSource) {
			this._register(this.lineSource.onDidChange(() => this.publish(EditorViewportChangeReason.LineProjection)));
		}
	}

	public get layout(): EditorViewportLayout {
		return this.currentLayout;
	}

	public get lineCount(): number {
		return this.linesLayout.lineCount;
	}

	public setViewportSize(size: ISize): EditorViewportLayout {
		const next = readSize(size, 'viewportSize');
		if (sizesEqual(this.viewportSize, next)) return this.currentLayout;
		this.viewportSize = next;
		this.publish(EditorViewportChangeReason.ViewportSize);
		return this.currentLayout;
	}

	public setContentWidth(width: number): EditorViewportLayout {
		const next = nonNegativeFinite(width, 'contentWidth');
		if (this.measuredContentWidth === next) return this.currentLayout;
		this.measuredContentWidth = next;
		this.publish(EditorViewportChangeReason.ContentWidth);
		return this.currentLayout;
	}

	public setLineHeight(lineHeight: number): EditorViewportLayout {
		const next = positiveFinite(lineHeight, 'lineHeight');
		if (this.linesLayout.lineHeight === next) return this.currentLayout;
		const currentTop = this.currentLayout.scrollPosition.top;
		const paddingTop = this.linesLayout.padding.top;
		const nextTop = currentTop <= paddingTop
			? currentTop
			: paddingTop + (currentTop - paddingTop) / this.linesLayout.lineHeight * next;
		this.linesLayout.setDefaultLineHeight(next);
		this.requestedScrollPosition = Object.freeze({
			left: this.currentLayout.scrollPosition.left,
			top: nextTop,
		});
		this.publish(EditorViewportChangeReason.LineHeight);
		return this.currentLayout;
	}

	public changeLineHeights(callback: (accessor: EditorLineHeightChangeAccessor) => void): EditorViewportLayout {
		if (!this.linesLayout.changeLineHeights(callback)) return this.currentLayout;
		this.publish(EditorViewportChangeReason.LineHeight);
		return this.currentLayout;
	}

	public addViewZone(afterLineIndex: number, heightInPixels: number, ordinal?: number): string {
		const id = this.linesLayout.addViewZone(afterLineIndex, heightInPixels, ordinal);
		this.publish(EditorViewportChangeReason.ViewZones);
		return id;
	}

	public changeViewZone(id: string, afterLineIndex: number, heightInPixels: number, ordinal?: number): EditorViewportLayout {
		if (!this.linesLayout.changeViewZone(id, afterLineIndex, heightInPixels, ordinal)) return this.currentLayout;
		this.publish(EditorViewportChangeReason.ViewZones);
		return this.currentLayout;
	}

	public removeViewZone(id: string): EditorViewportLayout {
		if (!this.linesLayout.removeViewZone(id)) return this.currentLayout;
		this.publish(EditorViewportChangeReason.ViewZones);
		return this.currentLayout;
	}

	public getVerticalOffsetForLineIndex(lineIndex: number): number {
		return this.linesLayout.getVerticalOffsetForLineIndex(lineIndex);
	}

	public getLineIndexAtVerticalOffset(verticalOffset: number): number {
		return this.linesLayout.getLineNumberAtVerticalOffset(verticalOffset);
	}

	public getViewZoneLayout(id: string): EditorViewZoneLayout | undefined {
		return this.linesLayout.getViewZoneLayout(id);
	}

	public setScrollPosition(position: EditorScrollPosition): EditorViewportLayout {
		const next = readScrollPosition(position);
		if (scrollPositionsEqual(this.requestedScrollPosition, next)) return this.currentLayout;
		this.requestedScrollPosition = next;
		this.publish(EditorViewportChangeReason.Scroll);
		return this.currentLayout;
	}

	private publish(reason: EditorViewportChangeReason, modelChange?: TextModelChange): void {
		const next = this.createLayout();
		this.requestedScrollPosition = next.scrollPosition;
		if (layoutsEqual(this.currentLayout, next)) return;
		this.currentLayout = next;
		this.changeEmitter.fire(Object.freeze({ reason, layout: next, modelChange }));
	}

	private createLayout(): EditorViewportLayout {
		const contentSize = Object.freeze({
			width: Math.max(this.viewportSize.width, this.measuredContentWidth),
			height: Math.max(this.viewportSize.height, this.linesLayout.getLinesTotalHeight()),
		});
		const maximumScrollPosition = Object.freeze({
			left: Math.max(0, contentSize.width - this.viewportSize.width),
			top: Math.max(0, contentSize.height - this.viewportSize.height),
		});
		const scrollPosition = Object.freeze({
			left: clamp(this.requestedScrollPosition.left, 0, maximumScrollPosition.left),
			top: clamp(this.requestedScrollPosition.top, 0, maximumScrollPosition.top),
		});
		const viewportData = this.linesLayout.getLinesViewportData(scrollPosition.top, this.viewportSize.height);
		const viewZones = this.linesLayout.getViewZoneLayouts();
		return Object.freeze({
			modelVersion: this.model.version,
			lineHeight: this.linesLayout.lineHeight,
			viewportSize: this.viewportSize,
			contentSize,
			scrollPosition,
			maximumScrollPosition,
			visibleLines: viewportData.visibleLines,
			renderLines: viewportData.renderLines,
			renderTop: viewportData.renderTop,
			...(viewZones.length > 0 ? {
				relativeVerticalOffset: viewportData.relativeVerticalOffset,
				viewZones,
			} : {}),
		});
	}
}

/** Creates the default one-row-per-model-line source used by an unwrapped view. */
function createTextModelLineSource(model: EditorViewportModelSource): EditorViewportLineSource {
	validateModelSource(model);
	return Object.freeze({
		get lineCount(): number {
			return model.lineCount;
		},
		onDidChange: (listener: () => void) => model.onDidChange(() => listener()),
	});
}

function validateModelSource(model: EditorViewportModelSource): void {
	if (!model || typeof model !== 'object' || typeof model.onDidChange !== 'function') throw new TypeError('View layout requires a model source');
	if (!isPositiveSafeInteger(model.lineCount)) throw new RangeError('View layout model line count must be a positive safe integer');
	if (!isNonNegativeSafeInteger(model.version)) throw new RangeError('View layout model version must be a non-negative safe integer');
}

function readPadding(padding: EditorViewportVerticalPadding | undefined): EditorViewportVerticalPadding {
	return Object.freeze({
		top: nonNegativeFinite(padding?.top ?? 0, 'padding.top'),
		bottom: nonNegativeFinite(padding?.bottom ?? 0, 'padding.bottom'),
	});
}

function readSize(size: ISize, name: string): ISize {
	if (!size || typeof size !== 'object') throw new TypeError(`${name} must be a size`);
	return Object.freeze({
		width: nonNegativeFinite(size.width, `${name}.width`),
		height: nonNegativeFinite(size.height, `${name}.height`),
	});
}

function readScrollPosition(position: EditorScrollPosition): EditorScrollPosition {
	if (!position || typeof position !== 'object') throw new TypeError('scrollPosition must be an object');
	return Object.freeze({
		left: finite(position.left, 'scrollPosition.left'),
		top: finite(position.top, 'scrollPosition.top'),
	});
}

function positiveFinite(value: number, name: string): number {
	const result = finite(value, name);
	if (result <= 0) throw new RangeError(`${name} must be positive`);
	return result;
}

function nonNegativeFinite(value: number, name: string): number {
	const result = finite(value, name);
	if (result < 0) throw new RangeError(`${name} must be non-negative`);
	return result;
}

function finite(value: number, name: string): number {
	if (!isFiniteNumber(value)) throw new RangeError(`${name} must be finite`);
	return value;
}

function nonNegativeSafeInteger(value: number, name: string): number {
	if (!isNonNegativeSafeInteger(value)) throw new RangeError(`${name} must be a non-negative safe integer`);
	return value;
}

function sizesEqual(left: ISize, right: ISize): boolean {
	return left.width === right.width && left.height === right.height;
}

function scrollPositionsEqual(left: EditorScrollPosition, right: EditorScrollPosition): boolean {
	return left.left === right.left && left.top === right.top;
}

function layoutsEqual(left: EditorViewportLayout, right: EditorViewportLayout): boolean {
	return left.modelVersion === right.modelVersion &&
		left.lineHeight === right.lineHeight &&
		sizesEqual(left.viewportSize, right.viewportSize) &&
		sizesEqual(left.contentSize, right.contentSize) &&
		scrollPositionsEqual(left.scrollPosition, right.scrollPosition) &&
		scrollPositionsEqual(left.maximumScrollPosition, right.maximumScrollPosition) &&
		lineRangesEqual(left.visibleLines, right.visibleLines) &&
		lineRangesEqual(left.renderLines, right.renderLines) &&
		left.renderTop === right.renderTop &&
		numberArraysEqual(left.relativeVerticalOffset, right.relativeVerticalOffset) &&
		viewZonesEqual(left.viewZones, right.viewZones);
}

function numberArraysEqual(left: readonly number[] | undefined, right: readonly number[] | undefined): boolean {
	if (left === right) return true;
	if (!left || !right || left.length !== right.length) return false;
	return left.every((value, index) => value === right[index]);
}

function viewZonesEqual(left: readonly EditorViewZoneLayout[] | undefined, right: readonly EditorViewZoneLayout[] | undefined): boolean {
	if (left === right) return true;
	if (!left || !right || left.length !== right.length) return false;
	return left.every((zone, index) => {
		const candidate = right[index];
		return candidate !== undefined && zone.id === candidate.id && zone.afterLineIndex === candidate.afterLineIndex && zone.top === candidate.top && zone.heightInPixels === candidate.heightInPixels;
	});
}

function lineRangesEqual(left: EditorLineRange, right: EditorLineRange): boolean {
	return left.startLineIndex === right.startLineIndex && left.endLineIndexExclusive === right.endLineIndexExclusive;
}

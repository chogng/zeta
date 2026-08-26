import { Emitter, type Event } from '../../../base/common/event.js';
import { type ISize } from '../../../base/common/layout.js';
import { DisposableOwner } from '../../../base/common/lifecycle.js';
import { clamp, isFiniteNumber, isNonNegativeSafeInteger } from '../../../base/common/numbers.js';
import { type TextModelChange } from '../core/text.js';
import { type TextModel } from '../model/textModel.js';
import { createTextModelLineSource, LinesLayout, type EditorLineRange, type EditorViewportLineSource, type EditorViewportVerticalPadding } from './linesLayout.js';
import { ViewportData } from './viewLinesViewportData.js';

export type { EditorLineRange, EditorViewportLineSource, EditorViewportVerticalPadding } from './linesLayout.js';

export interface EditorScrollPosition {
	readonly left: number;
	readonly top: number;
}

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
}

export enum EditorViewportChangeReason {
	Model = 'model',
	LineProjection = 'lineProjection',
	ViewportSize = 'viewportSize',
	ContentWidth = 'contentWidth',
	LineHeight = 'lineHeight',
	Scroll = 'scroll',
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
}

/**
 * Owns the immutable layout snapshot shared by the browser view and view-model.
 *
 * Horizontal measurement and scroll state stay here, while `LinesLayout` owns
 * line heights, padding, and visible/render line projection.
 */
export class ViewLayout extends DisposableOwner {
	private readonly changeEmitter = this.own(new Emitter<EditorViewportChange>());
	private readonly lineSource: EditorViewportLineSource;
	private readonly linesLayout: LinesLayout;
	private viewportSize: ISize = Object.freeze({ width: 0, height: 0 });
	private measuredContentWidth = 0;
	private requestedScrollPosition: EditorScrollPosition = Object.freeze({ left: 0, top: 0 });
	private currentLayout: EditorViewportLayout;

	public readonly onDidChange: Event<EditorViewportChange> = this.changeEmitter.event;

	public constructor(private readonly model: TextModel, options: EditorViewportOptions) {
		super();
		if (!model || typeof model !== 'object') throw new TypeError('View layout requires a text model');
		if (!options || typeof options !== 'object') throw new TypeError('View layout requires options');
		const lineHeight = positiveFinite(options.lineHeight, 'lineHeight');
		const overscanLineCount = nonNegativeSafeInteger(options.overscanLineCount ?? 2, 'overscanLineCount');
		this.lineSource = options.lineSource ?? createTextModelLineSource(model);
		const padding = readPadding(options.padding);
		this.linesLayout = new LinesLayout(
			this.lineSource,
			lineHeight,
			padding.top,
			padding.bottom,
			overscanLineCount,
		);
		this.currentLayout = this.createLayout();
		this.own(model.onDidChange(change => this.publish(EditorViewportChangeReason.Model, change)));
		if (options.lineSource) {
			this.own(this.lineSource.onDidChange(() => this.publish(EditorViewportChangeReason.LineProjection)));
		}
	}

	public get layout(): EditorViewportLayout {
		return this.currentLayout;
	}

	public get lineCount(): number {
		return this.linesLayout.lineCount;
	}

	public getViewportData(): ViewportData {
		return new ViewportData(this.currentLayout);
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

	public setScrollPosition(position: EditorScrollPosition): EditorViewportLayout {
		const next = readScrollPosition(position);
		if (scrollPositionsEqual(this.requestedScrollPosition, next)) return this.currentLayout;
		this.requestedScrollPosition = next;
		this.publish(EditorViewportChangeReason.Scroll);
		return this.currentLayout;
	}

	public getLinesLayout(): LinesLayout {
		return this.linesLayout;
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
		});
	}
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
		left.renderTop === right.renderTop;
}

function lineRangesEqual(left: EditorLineRange, right: EditorLineRange): boolean {
	return left.startLineIndex === right.startLineIndex && left.endLineIndexExclusive === right.endLineIndexExclusive;
}

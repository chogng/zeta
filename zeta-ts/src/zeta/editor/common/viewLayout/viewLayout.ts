import { Emitter, type Event } from '../../../base/common/event.js';
import { type ISize } from '../../../base/common/layout.js';
import { Disposable } from '../../../base/common/lifecycle.js';
import { clamp, isFiniteNumber, isNonNegativeSafeInteger, isPositiveSafeInteger } from '../../../base/common/numbers.js';
import { type TextModelChange } from '../core/textChange.js';
import { type EditorLineHeightChangeAccessor, type EditorLineRange, type EditorScrollPosition, type EditorViewZoneLayout, type EditorViewportLineSource, type EditorViewportModelSource } from '../viewModel/editorViewportContracts.js';
import { type CustomLineHeightData } from './lineHeights.js';
import { LinesLayout } from './linesLayout.js';

export type { EditorLineHeightChangeAccessor, EditorLineRange, EditorScrollPosition, EditorViewportLineSource, EditorViewportModelSource } from '../viewModel/editorViewportContracts.js';

/** Vertical space reserved around the projected line collection. */
export interface EditorViewportVerticalPadding {
	readonly top: number;
	readonly bottom: number;
}
interface LinesLayoutViewport {
	readonly lineCount: number;
	readonly contentHeight: number;
	readonly visibleLines: EditorLineRange;
	readonly renderLines: EditorLineRange;
	readonly renderTop: number;
	readonly relativeVerticalOffset: readonly number[];
}
interface ViewZoneData {
	readonly id: string;
	readonly afterLineIndex: number;
	readonly heightInPixels: number;
	readonly ordinal: number;
	readonly whitespaceId: string;
}
const DefaultViewZoneOrdinal = 10_000;
/**
 * Owns vertical line geometry, view zones, and rendered ranges.
 *
 * The outer layout owner keeps horizontal dimensions and scroll state. This
 * helper keeps the vertical line collection, including overscan and padding.
 */
class VerticalLayout {
	private readonly lineSource: EditorViewportLineSource | undefined;
	private readonly fixedLineCount: number | undefined;
	private readonly linesLayout: LinesLayout;
	private readonly customLineHeights = new Map<string, CustomLineHeightData>();
	private synchronizedLineCount: number;
	private defaultLineHeight: number;
	private overscanLineCount: number;
	private paddingTop: number;
	private paddingBottom: number;
	private readonly viewZones = new Map<string, ViewZoneData>();
	private nextViewZoneId = 0;
	public constructor(
		lineSourceOrCount: EditorViewportLineSource | number,
		lineHeight: number,
		paddingTop = 0,
		paddingBottom = 0,
		overscanLineCount = 2,
		customLineHeightData: readonly CustomLineHeightData[] = [],
	) {
		let initialLineCount: number;
		if (typeof lineSourceOrCount === 'number') {
			validateLineCount(lineSourceOrCount);
			this.fixedLineCount = lineSourceOrCount;
			initialLineCount = lineSourceOrCount;
		} else {
			validateLineSource(lineSourceOrCount);
			this.lineSource = lineSourceOrCount;
			initialLineCount = lineSourceOrCount.lineCount;
		}
		this.paddingTop = nonNegativeFinite(paddingTop, 'paddingTop');
		this.paddingBottom = nonNegativeFinite(paddingBottom, 'paddingBottom');
		this.overscanLineCount = nonNegativeSafeInteger(overscanLineCount, 'overscanLineCount');
		this.defaultLineHeight = lineHeight;
		for (const data of customLineHeightData) this.customLineHeights.set(data.decorationId, data);
		this.synchronizedLineCount = initialLineCount;
		this.linesLayout = new LinesLayout(initialLineCount, lineHeight, this.paddingTop, this.paddingBottom, [...this.customLineHeights.values()]);
	}
	public get lineCount(): number {
		const lineCount = this.lineSource?.lineCount ?? this.fixedLineCount;
		validateLineCount(lineCount);
		if (lineCount !== this.synchronizedLineCount) {
			this.linesLayout.onFlushed(lineCount, [...this.customLineHeights.values()]);
			this.synchronizeViewZones(lineCount);
			this.synchronizedLineCount = lineCount;
		}
		return lineCount;
	}
	public get lineHeight(): number {
		return this.defaultLineHeight;
	}
	public get padding(): EditorViewportVerticalPadding {
		return Object.freeze({ top: this.paddingTop, bottom: this.paddingBottom });
	}
	public setDefaultLineHeight(lineHeight: number): void {
		this.defaultLineHeight = lineHeight;
		this.linesLayout.setDefaultLineHeight(lineHeight);
	}
	public setPadding(paddingTop: number, paddingBottom: number): void {
		this.paddingTop = nonNegativeFinite(paddingTop, 'paddingTop');
		this.paddingBottom = nonNegativeFinite(paddingBottom, 'paddingBottom');
		this.linesLayout.setPadding(this.paddingTop, this.paddingBottom);
	}
	public setOverscanLineCount(overscanLineCount: number): void {
		this.overscanLineCount = nonNegativeSafeInteger(overscanLineCount, 'overscanLineCount');
	}
	public changeLineHeights(callback: (accessor: EditorLineHeightChangeAccessor) => void): boolean {
		if (typeof callback !== 'function') throw new TypeError('Line-height changes require a callback');
		void this.lineCount;
		let hadAChange = false;
		this.linesLayout.changeLineHeights(accessor => {
			callback({
				insertOrChangeCustomLineHeight: (decorationId, startLineNumber, endLineNumber, lineHeight) => {
					hadAChange = true;
					this.customLineHeights.set(decorationId, { decorationId, startLineNumber, endLineNumber, lineHeight });
					accessor.insertOrChangeCustomLineHeight(decorationId, startLineNumber, endLineNumber, lineHeight);
				},
				removeCustomLineHeight: decorationId => {
					hadAChange = true;
					this.customLineHeights.delete(decorationId);
					accessor.removeCustomLineHeight(decorationId);
				},
			});
		});
		return hadAChange;
	}
	public addViewZone(afterLineIndex: number, heightInPixels: number, ordinal?: number): string {
		validateViewZone(afterLineIndex, heightInPixels, ordinal, this.lineCount);
		const id = `view-zone-${++this.nextViewZoneId}`;
		const normalizedOrdinal = ordinal ?? DefaultViewZoneOrdinal;
		let whitespaceId = '';
		this.linesLayout.changeWhitespace(accessor => {
			whitespaceId = accessor.insertWhitespace(afterLineIndex + 1, normalizedOrdinal, heightInPixels, 0);
		});
		this.viewZones.set(id, Object.freeze({
			id,
			afterLineIndex,
			heightInPixels,
			ordinal: normalizedOrdinal,
			whitespaceId,
		}));
		return id;
	}
	public changeViewZone(id: string, afterLineIndex: number, heightInPixels: number, ordinal?: number): boolean {
		validateViewZone(afterLineIndex, heightInPixels, ordinal, this.lineCount);
		const current = this.viewZones.get(id);
		if (!current) throw new Error(`Unknown editor view zone: ${id}`);
		const nextOrdinal = ordinal ?? DefaultViewZoneOrdinal;
		if (current.afterLineIndex === afterLineIndex && current.heightInPixels === heightInPixels && current.ordinal === nextOrdinal) return false;
		let whitespaceId = current.whitespaceId;
		this.linesLayout.changeWhitespace(accessor => {
			if (current.ordinal === nextOrdinal) {
				accessor.changeOneWhitespace(whitespaceId, afterLineIndex + 1, heightInPixels);
				return;
			}
			accessor.removeWhitespace(whitespaceId);
			whitespaceId = accessor.insertWhitespace(afterLineIndex + 1, nextOrdinal, heightInPixels, 0);
		});
		this.viewZones.set(id, Object.freeze({ ...current, afterLineIndex, heightInPixels, ordinal: nextOrdinal, whitespaceId }));
		return true;
	}
	public removeViewZone(id: string): boolean {
		const current = this.viewZones.get(id);
		if (!current) return false;
		this.linesLayout.changeWhitespace(accessor => accessor.removeWhitespace(current.whitespaceId));
		this.viewZones.delete(id);
		return true;
	}
	public getViewZoneLayouts(): readonly EditorViewZoneLayout[] {
		const lineCount = this.lineCount;
		const zonesByWhitespaceId = new Map([...this.viewZones.values()].map(zone => [zone.whitespaceId, zone]));
		return Object.freeze(this.linesLayout.getWhitespaces().map((whitespace, index) => {
			const zone = zonesByWhitespaceId.get(whitespace.id);
			if (!zone) throw new Error(`Unknown editor whitespace: ${whitespace.id}`);
			return Object.freeze({
				id: zone.id,
				afterLineIndex: Math.min(zone.afterLineIndex, lineCount - 1),
				top: this.linesLayout.getVerticalOffsetForWhitespaceIndex(index),
				heightInPixels: whitespace.height,
			});
		}));
	}
	public getViewZoneLayout(id: string): EditorViewZoneLayout | undefined {
		return this.getViewZoneLayouts().find(zone => zone.id === id);
	}
	public getLinesTotalHeight(): number {
		void this.lineCount;
		return this.linesLayout.getLinesTotalHeight();
	}
	public getVerticalOffsetForLineIndex(lineIndex: number): number {
		void this.lineCount;
		return this.linesLayout.getVerticalOffsetForLineNumber(lineIndex + 1);
	}
	public getLineHeightForLineIndex(lineIndex: number): number {
		if (!Number.isSafeInteger(lineIndex) || lineIndex < 0 || lineIndex >= this.lineCount) {
			throw new RangeError('Line index is outside the line collection');
		}
		return this.linesLayout.getLineHeightForLineNumber(lineIndex + 1);
	}
	public getLinesViewportData(verticalOffset: number, viewportHeight: number): LinesLayoutViewport {
		if (!isFiniteNumber(verticalOffset) || !isFiniteNumber(viewportHeight) || viewportHeight < 0) {
			throw new RangeError('Line viewport coordinates must be finite and non-negative');
		}
		const lineCount = this.lineCount;
		const visibleLines = this.viewZones.size === 0
			? this.getVisibleLineRange(verticalOffset, viewportHeight)
			: this.getVisibleLineRangeWithViewZones(verticalOffset, viewportHeight);
		const hasVisibleLines = visibleLines.startLineIndex < visibleLines.endLineIndexExclusive;
		const renderLines = Object.freeze(hasVisibleLines
			? {
				startLineIndex: Math.max(0, visibleLines.startLineIndex - this.overscanLineCount),
				endLineIndexExclusive: Math.min(lineCount, visibleLines.endLineIndexExclusive + this.overscanLineCount),
			}
			: visibleLines);
		return Object.freeze({
			lineCount,
			contentHeight: this.linesLayout.getLinesTotalHeight(),
			visibleLines,
			renderLines,
			renderTop: this.getVerticalOffsetForLineIndex(renderLines.startLineIndex),
			relativeVerticalOffset: Object.freeze(Array.from(
				{ length: renderLines.endLineIndexExclusive - renderLines.startLineIndex },
				(_, index) => this.getVerticalOffsetForLineIndex(renderLines.startLineIndex + index),
			)),
		});
	}
	public getLineNumberAtVerticalOffset(verticalOffset: number): number {
		if (!isFiniteNumber(verticalOffset)) throw new RangeError('Vertical offset must be finite');
		const lineCount = this.lineCount;
		if (verticalOffset < this.paddingTop) return 0;
		if (verticalOffset >= this.linesLayout.getLinesTotalHeight() - this.paddingBottom) return lineCount;
		return this.linesLayout.getLineNumberAtOrAfterVerticalOffset(verticalOffset) - 1;
	}
	private getVisibleLineRange(verticalOffset: number, viewportHeight: number): EditorLineRange {
		const lineCount = this.lineCount;
		if (viewportHeight === 0) return lineRange(0, 0);
		const visibleBottom = verticalOffset + viewportHeight;
		const linesBottom = this.linesLayout.getLinesTotalHeight() - this.paddingBottom;
		if (visibleBottom <= this.paddingTop) return lineRange(0, 0);
		if (verticalOffset >= linesBottom) return lineRange(lineCount, lineCount);
		const viewport = this.linesLayout.getLinesViewportData(verticalOffset, visibleBottom);
		return lineRange(viewport.startLineNumber - 1, viewport.endLineNumber);
	}
	private getVisibleLineRangeWithViewZones(verticalOffset: number, viewportHeight: number): EditorLineRange {
		if (viewportHeight === 0) return lineRange(0, 0);
		const visibleBottom = verticalOffset + viewportHeight;
		const startLineIndex = this.getLineNumberAtVerticalOffset(verticalOffset);
		if (startLineIndex >= this.lineCount || this.getVerticalOffsetForLineIndex(startLineIndex) >= visibleBottom) return lineRange(startLineIndex, startLineIndex);
		let endLineIndexExclusive = startLineIndex;
		while (endLineIndexExclusive < this.lineCount && this.getVerticalOffsetForLineIndex(endLineIndexExclusive) < visibleBottom) endLineIndexExclusive += 1;
		return lineRange(startLineIndex, endLineIndexExclusive);
	}
	private synchronizeViewZones(lineCount: number): void {
		this.linesLayout.changeWhitespace(accessor => {
			for (const zone of this.viewZones.values()) {
				accessor.changeOneWhitespace(zone.whitespaceId, Math.min(zone.afterLineIndex, lineCount - 1) + 1, zone.heightInPixels);
			}
		});
	}
}
function validateViewZone(afterLineIndex: number, heightInPixels: number, ordinal: number | undefined, lineCount: number): void {
	if (!Number.isSafeInteger(afterLineIndex) || afterLineIndex < -1 || afterLineIndex >= lineCount) throw new RangeError('View zone line index is outside the line collection');
	if (!isFiniteNumber(heightInPixels) || heightInPixels <= 0) throw new RangeError('View zone height must be finite and positive');
	if (ordinal !== undefined && !isFiniteNumber(ordinal)) throw new RangeError('View zone ordinal must be finite');
}
function lineRange(startLineIndex: number, endLineIndexExclusive: number): EditorLineRange {
	return Object.freeze({ startLineIndex, endLineIndexExclusive });
}
function validateLineSource(source: EditorViewportLineSource): void {
	if (!source || typeof source !== 'object' || typeof source.onDidChange !== 'function') {
		throw new TypeError('Editor viewport line source must expose a line count and change event');
	}
	validateLineCount(source.lineCount);
}
function validateLineCount(lineCount: number | undefined): asserts lineCount is number {
	if (!isPositiveSafeInteger(lineCount)) throw new RangeError('Line count must be a positive safe integer');
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
	EditorViewZones = 'viewZones',
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
 * Horizontal measurement and scroll state stay here, while `VerticalLayout`
 * owns line heights, padding, view zones, and rendered line ranges.
 */
export class EditorViewportLayoutManager extends Disposable {
	private readonly changeEmitter = this._register(new Emitter<EditorViewportChange>());
	private readonly lineSource: EditorViewportLineSource;
	private readonly linesLayout: VerticalLayout;
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
		this.linesLayout = new VerticalLayout(
			this.lineSource,
			lineHeight,
			padding.top,
			padding.bottom,
			overscanLineCount,
			options.customLineHeightData,
		);
		this.currentLayout = this.createLayout();
		this._register(model.onDidChangeContent(change => this.publish(EditorViewportChangeReason.Model, change)));
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
		this.publish(EditorViewportChangeReason.EditorViewZones);
		return id;
	}

	public changeViewZone(id: string, afterLineIndex: number, heightInPixels: number, ordinal?: number): EditorViewportLayout {
		if (!this.linesLayout.changeViewZone(id, afterLineIndex, heightInPixels, ordinal)) return this.currentLayout;
		this.publish(EditorViewportChangeReason.EditorViewZones);
		return this.currentLayout;
	}

	public removeViewZone(id: string): EditorViewportLayout {
		if (!this.linesLayout.removeViewZone(id)) return this.currentLayout;
		this.publish(EditorViewportChangeReason.EditorViewZones);
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
		onDidChange: (listener: () => void) => model.onDidChangeContent(() => listener()),
	});
}

function validateModelSource(model: EditorViewportModelSource): void {
	if (!model || typeof model !== 'object' || typeof model.onDidChangeContent !== 'function') throw new TypeError('View layout requires a model source');
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

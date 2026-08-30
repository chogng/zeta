import { isFiniteNumber, isNonNegativeSafeInteger, isPositiveSafeInteger } from '../../../base/common/numbers.js';
import { type EditorLineHeightChangeAccessor, type EditorLineRange, type EditorViewZoneLayout, type EditorViewportLineSource } from '../viewModel/editorViewportContracts.js';
import { CustomLineHeightData, LineHeightsManager } from './lineHeights.js';

export type { EditorLineRange, EditorViewportLineSource } from '../viewModel/editorViewportContracts.js';

/** Vertical space reserved around the projected line collection. */
export interface EditorViewportVerticalPadding {
	readonly top: number;
	readonly bottom: number;
}

export interface LinesLayoutViewport {
	readonly lineCount: number;
	readonly contentHeight: number;
	readonly visibleLines: EditorLineRange;
	readonly renderLines: EditorLineRange;
	readonly renderTop: number;
	readonly relativeVerticalOffset: readonly number[];
}

interface ViewZoneData {
	readonly id: string;
	readonly ordinal: number;
	readonly creationOrder: number;
	readonly afterLineIndex: number;
	readonly heightInPixels: number;
}

const DefaultViewZoneOrdinal = 10_000;

/**
 * Calculates line positions and virtualized line ranges.
 *
 * `ViewLayout` owns horizontal dimensions and scroll state. This class owns
 * only the vertical line collection, including overscan and padding.
 */
export class LinesLayout {
	private readonly lineSource: EditorViewportLineSource | undefined;
	private readonly fixedLineCount: number | undefined;
	private readonly lineHeights: LineHeightsManager;
	private overscanLineCount: number;
	private paddingTop: number;
	private paddingBottom: number;
	private readonly viewZones = new Map<string, ViewZoneData>();
	private nextViewZoneId = 0;
	private nextViewZoneOrdinal = 0;

	public constructor(
		lineSourceOrCount: EditorViewportLineSource | number,
		lineHeight: number,
		paddingTop = 0,
		paddingBottom = 0,
		overscanLineCount = 2,
		customLineHeightData: readonly CustomLineHeightData[] = [],
	) {
		if (typeof lineSourceOrCount === 'number') {
			validateLineCount(lineSourceOrCount);
			this.fixedLineCount = lineSourceOrCount;
		} else {
			validateLineSource(lineSourceOrCount);
			this.lineSource = lineSourceOrCount;
		}
		this.lineHeights = new LineHeightsManager(lineHeight, customLineHeightData);
		this.paddingTop = nonNegativeFinite(paddingTop, 'paddingTop');
		this.paddingBottom = nonNegativeFinite(paddingBottom, 'paddingBottom');
		this.overscanLineCount = nonNegativeSafeInteger(overscanLineCount, 'overscanLineCount');
	}

	public get lineCount(): number {
		const lineCount = this.lineSource?.lineCount ?? this.fixedLineCount;
		validateLineCount(lineCount);
		return lineCount;
	}

	public get lineHeight(): number {
		return this.lineHeights.defaultLineHeight;
	}

	public get padding(): EditorViewportVerticalPadding {
		return Object.freeze({ top: this.paddingTop, bottom: this.paddingBottom });
	}

	public setDefaultLineHeight(lineHeight: number): void {
		this.lineHeights.defaultLineHeight = lineHeight;
	}

	public setPadding(paddingTop: number, paddingBottom: number): void {
		this.paddingTop = nonNegativeFinite(paddingTop, 'paddingTop');
		this.paddingBottom = nonNegativeFinite(paddingBottom, 'paddingBottom');
	}

	public setOverscanLineCount(overscanLineCount: number): void {
		this.overscanLineCount = nonNegativeSafeInteger(overscanLineCount, 'overscanLineCount');
	}

	public changeLineHeights(callback: (accessor: EditorLineHeightChangeAccessor) => void): boolean {
		if (typeof callback !== 'function') throw new TypeError('Line-height changes require a callback');
		let hadAChange = false;
		callback({
			insertOrChangeCustomLineHeight: (decorationId, startLineNumber, endLineNumber, lineHeight) => {
				hadAChange = true;
				this.lineHeights.insertOrChangeCustomLineHeight(decorationId, startLineNumber, endLineNumber, lineHeight);
			},
			removeCustomLineHeight: decorationId => {
				hadAChange = true;
				this.lineHeights.removeCustomLineHeight(decorationId);
			},
		});
		return hadAChange;
	}

	public addViewZone(afterLineIndex: number, heightInPixels: number, ordinal?: number): string {
		validateViewZone(afterLineIndex, heightInPixels, ordinal, this.lineCount);
		const id = `view-zone-${++this.nextViewZoneId}`;
		const creationOrder = ++this.nextViewZoneOrdinal;
		this.viewZones.set(id, Object.freeze({
			id,
			ordinal: ordinal ?? DefaultViewZoneOrdinal,
			creationOrder,
			afterLineIndex,
			heightInPixels,
		}));
		return id;
	}

	public changeViewZone(id: string, afterLineIndex: number, heightInPixels: number, ordinal?: number): boolean {
		validateViewZone(afterLineIndex, heightInPixels, ordinal, this.lineCount);
		const current = this.viewZones.get(id);
		if (!current) throw new Error(`Unknown editor view zone: ${id}`);
		const nextOrdinal = ordinal ?? DefaultViewZoneOrdinal;
		if (current.afterLineIndex === afterLineIndex && current.heightInPixels === heightInPixels && current.ordinal === nextOrdinal) return false;
		this.viewZones.set(id, Object.freeze({ ...current, afterLineIndex, heightInPixels, ordinal: nextOrdinal }));
		return true;
	}

	public removeViewZone(id: string): boolean {
		return this.viewZones.delete(id);
	}

	public getViewZoneLayouts(): readonly EditorViewZoneLayout[] {
		let accumulatedHeight = 0;
		return Object.freeze(this.sortedViewZones.map(zone => {
			const afterLineIndex = Math.min(zone.afterLineIndex, this.lineCount - 1);
			const top = this.paddingTop + this.lineHeights.getVerticalOffsetForLineIndex(afterLineIndex + 1, this.lineCount) + accumulatedHeight;
			accumulatedHeight += zone.heightInPixels;
			return Object.freeze({ id: zone.id, afterLineIndex, top, heightInPixels: zone.heightInPixels });
		}));
	}

	public getViewZoneLayout(id: string): EditorViewZoneLayout | undefined {
		return this.getViewZoneLayouts().find(zone => zone.id === id);
	}

	public getLinesTotalHeight(): number {
		return this.paddingTop + this.lineHeights.getTotalHeight(this.lineCount) + this.viewZonesTotalHeight + this.paddingBottom;
	}

	public getVerticalOffsetForLineIndex(lineIndex: number): number {
		return this.paddingTop + this.lineHeights.getVerticalOffsetForLineIndex(lineIndex, this.lineCount) + this.getViewZonesHeightBeforeLineIndex(lineIndex);
	}

	public getLineHeightForLineIndex(lineIndex: number): number {
		if (!Number.isSafeInteger(lineIndex) || lineIndex < 0 || lineIndex >= this.lineCount) {
			throw new RangeError('Line index is outside the line collection');
		}
		return this.lineHeights.heightForLineIndex(lineIndex);
	}

	public getLinesViewportData(verticalOffset: number, viewportHeight: number): LinesLayoutViewport {
		if (!isFiniteNumber(verticalOffset) || !isFiniteNumber(viewportHeight) || viewportHeight < 0) {
			throw new RangeError('Line viewport coordinates must be finite and non-negative');
		}
		const lineCount = this.lineCount;
		const totalLineHeight = this.lineHeights.getTotalHeight(lineCount) + this.viewZonesTotalHeight;
		const visibleLines = this.viewZones.size === 0
			? getVisibleLineRange(lineCount, this.lineHeights, viewportHeight, verticalOffset, this.paddingTop)
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
			contentHeight: this.paddingTop + totalLineHeight + this.paddingBottom,
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
		if (this.viewZones.size === 0) return this.lineHeights.getLineIndexAtVerticalOffset(verticalOffset - this.paddingTop, this.lineCount);
		let low = 0;
		let high = this.lineCount;
		while (low < high) {
			const middle = Math.floor((low + high) / 2);
			const bottom = this.getVerticalOffsetForLineIndex(middle) + this.lineHeights.heightForLineIndex(middle);
			if (bottom <= verticalOffset) low = middle + 1;
			else high = middle;
		}
		return low;
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

	private getViewZonesHeightBeforeLineIndex(lineIndex: number): number {
		let height = 0;
		for (const zone of this.viewZones.values()) {
			if (Math.min(zone.afterLineIndex, this.lineCount - 1) < lineIndex) height += zone.heightInPixels;
		}
		return height;
	}

	private get viewZonesTotalHeight(): number {
		let height = 0;
		for (const zone of this.viewZones.values()) height += zone.heightInPixels;
		return height;
	}

	private get sortedViewZones(): readonly ViewZoneData[] {
		return [...this.viewZones.values()].sort((left, right) => left.afterLineIndex - right.afterLineIndex || left.ordinal - right.ordinal || left.creationOrder - right.creationOrder);
	}
}

function validateViewZone(afterLineIndex: number, heightInPixels: number, ordinal: number | undefined, lineCount: number): void {
	if (!Number.isSafeInteger(afterLineIndex) || afterLineIndex < -1 || afterLineIndex >= lineCount) throw new RangeError('View zone line index is outside the line collection');
	if (!isFiniteNumber(heightInPixels) || heightInPixels <= 0) throw new RangeError('View zone height must be finite and positive');
	if (ordinal !== undefined && !isFiniteNumber(ordinal)) throw new RangeError('View zone ordinal must be finite');
}

export function getVisibleLineRange(
	lineCount: number,
	lineHeights: LineHeightsManager,
	viewportHeight: number,
	scrollTop: number,
	paddingTop: number,
): EditorLineRange {
	validateLineCount(lineCount);
	if (!isFiniteNumber(viewportHeight) || viewportHeight < 0) throw new RangeError('Viewport height must be finite and non-negative');
	if (!isFiniteNumber(scrollTop)) throw new RangeError('Scroll top must be finite');
	if (!isFiniteNumber(paddingTop) || paddingTop < 0) throw new RangeError('Top padding must be finite and non-negative');
	if (viewportHeight === 0) return lineRange(0, 0);
	const visibleTop = scrollTop - paddingTop;
	const visibleBottom = scrollTop + viewportHeight - paddingTop;
	if (visibleBottom <= 0) return lineRange(0, 0);
	const totalLineHeight = lineHeights.getTotalHeight(lineCount);
	if (visibleTop >= totalLineHeight) return lineRange(lineCount, lineCount);
	const startLineIndex = visibleTop <= 0 ? 0 : lineHeights.getLineIndexAtVerticalOffset(visibleTop, lineCount);
	const candidateEndLineIndex = visibleBottom >= totalLineHeight
		? lineCount
		: lineHeights.getLineIndexAtVerticalOffset(visibleBottom, lineCount);
	const candidateEndLineTop = candidateEndLineIndex >= lineCount
		? totalLineHeight
		: lineHeights.getVerticalOffsetForLineIndex(candidateEndLineIndex, lineCount);
	const endLineIndexExclusive = Math.min(
		lineCount,
		candidateEndLineIndex + (candidateEndLineTop < visibleBottom ? 1 : 0),
	);
	return lineRange(startLineIndex, Math.min(lineCount, endLineIndexExclusive));
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

function nonNegativeFinite(value: number, name: string): number {
	if (!isFiniteNumber(value) || value < 0) throw new RangeError(`${name} must be finite and non-negative`);
	return value;
}

function nonNegativeSafeInteger(value: number, name: string): number {
	if (!isNonNegativeSafeInteger(value)) throw new RangeError(`${name} must be a non-negative safe integer`);
	return value;
}

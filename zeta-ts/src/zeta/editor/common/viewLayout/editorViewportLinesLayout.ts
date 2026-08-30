import { isFiniteNumber, isNonNegativeSafeInteger, isPositiveSafeInteger } from '../../../base/common/numbers.js';
import { type EditorLineHeightChangeAccessor, type EditorLineRange, type EditorViewZoneLayout, type EditorViewportLineSource } from '../viewModel/editorViewportContracts.js';
import { type CustomLineHeightData } from './lineHeights.js';
import { LinesLayout } from './linesLayout.js';

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
	readonly afterLineIndex: number;
	readonly heightInPixels: number;
	readonly ordinal: number;
	readonly whitespaceId: string;
}

const DefaultViewZoneOrdinal = 10_000;

/**
 * Calculates line positions and virtualized line ranges.
 *
 * `ViewLayout` owns horizontal dimensions and scroll state. This class owns
 * only the vertical line collection, including overscan and padding.
 */
export class EditorViewportLinesLayout {
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

function nonNegativeFinite(value: number, name: string): number {
	if (!isFiniteNumber(value) || value < 0) throw new RangeError(`${name} must be finite and non-negative`);
	return value;
}

function nonNegativeSafeInteger(value: number, name: string): number {
	if (!isNonNegativeSafeInteger(value)) throw new RangeError(`${name} must be a non-negative safe integer`);
	return value;
}

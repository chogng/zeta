import { isFiniteNumber, isNonNegativeSafeInteger, isPositiveSafeInteger } from '../../../base/common/numbers.js';
import { type EditorLineHeightChangeAccessor, type EditorLineRange, type EditorViewportLineSource } from '../viewModel.js';
import { CustomLineHeightData, LineHeightsManager } from './lineHeights.js';

export type { EditorLineRange, EditorViewportLineSource } from '../viewModel.js';

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

	public getLinesTotalHeight(): number {
		return this.paddingTop + this.lineHeights.getTotalHeight(this.lineCount) + this.paddingBottom;
	}

	public getVerticalOffsetForLineIndex(lineIndex: number): number {
		return this.paddingTop + this.lineHeights.getVerticalOffsetForLineIndex(lineIndex, this.lineCount);
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
		const totalLineHeight = this.lineHeights.getTotalHeight(lineCount);
		const visibleLines = getVisibleLineRange(
			lineCount,
			this.lineHeights,
			viewportHeight,
			verticalOffset,
			this.paddingTop,
		);

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
		const lineCount = this.lineCount;
		const lineOffset = verticalOffset - this.paddingTop;
		return this.lineHeights.getLineIndexAtVerticalOffset(lineOffset, lineCount);
	}
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

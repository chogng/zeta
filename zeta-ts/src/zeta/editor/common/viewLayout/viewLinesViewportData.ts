import { type EditorLineRange } from '../viewModel.js';

export interface ViewportDataOptions {
	readonly modelVersion: number;
	readonly lineHeight: number;
	readonly visibleLines: EditorLineRange;
	readonly renderLines: EditorLineRange;
	readonly renderTop: number;
	readonly relativeVerticalOffset?: readonly number[];
}

/** Immutable render snapshot for one viewport window. */
export class ViewportData {
	public readonly modelVersion: number;
	public readonly lineHeight: number;
	public readonly startLineIndex: number;
	public readonly endLineIndexExclusive: number;
	public readonly visibleLines: EditorLineRange;
	public readonly renderLines: EditorLineRange;
	public readonly renderTop: number;
	public readonly relativeVerticalOffset: readonly number[];

	public constructor(options: ViewportDataOptions) {
		validateViewportDataOptions(options);
		this.modelVersion = options.modelVersion;
		this.lineHeight = options.lineHeight;
		this.startLineIndex = options.renderLines.startLineIndex;
		this.endLineIndexExclusive = options.renderLines.endLineIndexExclusive;
		this.visibleLines = options.visibleLines;
		this.renderLines = options.renderLines;
		this.renderTop = options.renderTop;
		this.relativeVerticalOffset = Object.freeze([...(options.relativeVerticalOffset ?? Array.from(
			{ length: this.endLineIndexExclusive - this.startLineIndex },
			(_, index) => this.renderTop + index * this.lineHeight,
		))]);
		if (this.relativeVerticalOffset.length !== this.endLineIndexExclusive - this.startLineIndex) {
			throw new RangeError('Viewport relative vertical offsets must cover the render range');
		}
		Object.freeze(this);
	}

	public getLineTop(lineIndex: number): number {
		if (!Number.isSafeInteger(lineIndex) || lineIndex < this.startLineIndex || lineIndex >= this.endLineIndexExclusive) {
			throw new RangeError('Viewport line index is outside the render window');
		}
		return this.relativeVerticalOffset[lineIndex - this.startLineIndex]!;
	}
}

function validateViewportDataOptions(options: ViewportDataOptions): void {
	if (!options || typeof options !== 'object') throw new TypeError('Viewport data requires options');
	if (!Number.isSafeInteger(options.modelVersion) || options.modelVersion < 0) throw new RangeError('Viewport model version must be a non-negative safe integer');
	if (!Number.isFinite(options.lineHeight) || options.lineHeight <= 0) throw new RangeError('Viewport line height must be finite and positive');
	if (!Number.isFinite(options.renderTop)) throw new RangeError('Viewport render top must be finite');
	validateLineRange(options.visibleLines, 'visible');
	validateLineRange(options.renderLines, 'render');
	if (options.relativeVerticalOffset !== undefined && (!Array.isArray(options.relativeVerticalOffset) || options.relativeVerticalOffset.some(offset => !Number.isFinite(offset)))) throw new RangeError('Viewport relative vertical offsets must be finite');
}

function validateLineRange(range: EditorLineRange, name: string): void {
	if (!range || !Number.isSafeInteger(range.startLineIndex) || !Number.isSafeInteger(range.endLineIndexExclusive) || range.startLineIndex < 0 || range.endLineIndexExclusive < range.startLineIndex) {
		throw new RangeError(`Viewport ${name} line range is invalid`);
	}
}

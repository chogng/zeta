import { type EditorLineRange } from './linesLayout.js';
import { type EditorViewportLayout } from './viewLayout.js';

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
	public readonly layout: EditorViewportLayout;

	public constructor(layout: EditorViewportLayout) {
		this.layout = layout;
		this.modelVersion = layout.modelVersion;
		this.lineHeight = layout.lineHeight;
		this.startLineIndex = layout.renderLines.startLineIndex;
		this.endLineIndexExclusive = layout.renderLines.endLineIndexExclusive;
		this.visibleLines = layout.visibleLines;
		this.renderLines = layout.renderLines;
		this.renderTop = layout.renderTop;
		this.relativeVerticalOffset = Object.freeze(Array.from(
			{ length: this.endLineIndexExclusive - this.startLineIndex },
			(_, index) => this.renderTop + index * this.lineHeight,
		));
		Object.freeze(this);
	}

	public getLineTop(lineIndex: number): number {
		if (!Number.isSafeInteger(lineIndex) || lineIndex < this.startLineIndex || lineIndex >= this.endLineIndexExclusive) {
			throw new RangeError('Viewport line index is outside the render window');
		}
		return this.relativeVerticalOffset[lineIndex - this.startLineIndex]!;
	}
}

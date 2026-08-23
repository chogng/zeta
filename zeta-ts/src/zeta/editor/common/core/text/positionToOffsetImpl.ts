import { TextPosition } from "../position.js";
import { TextRange } from "../range.js";
import { OffsetRange } from "../ranges/offsetRange.js";
import { TextLength } from "./textLength.js";

/** Shared coordinate conversion contract for strings and line-based models. */
export abstract class PositionOffsetTransformerBase {
	abstract getOffset(position: TextPosition): number;
	abstract getPosition(offset: number): TextPosition;
	abstract getLineLength(lineIndex: number): number;
	abstract readonly textLength: TextLength;

	getOffsetRange(range: TextRange): OffsetRange { return new OffsetRange(this.getOffset(range.start), this.getOffset(range.end)); }
	getRange(range: OffsetRange): TextRange { return TextRange.from(this.getPosition(range.start), this.getPosition(range.endExclusive)); }
	getTextLength(range: OffsetRange): TextLength { return TextLength.ofRange(this.getRange(range)); }
}

/** Converts zero-based positions and UTF-16 offsets in normalized text. */
export class PositionOffsetTransformer extends PositionOffsetTransformerBase {
	private readonly lineStarts: readonly number[];
	private readonly lineEnds: readonly number[];

	constructor(public readonly text: string) {
		super();
		const starts = [0];
		const ends: number[] = [];
		for (let index = 0; index < text.length; index += 1) {
			if (text.charCodeAt(index) === 10) {
				ends.push(index);
				starts.push(index + 1);
			}
		}
		ends.push(text.length);
		this.lineStarts = starts;
		this.lineEnds = ends;
	}

	get textLength(): TextLength {
		const lastLine = this.lineStarts.length - 1;
		return new TextLength(lastLine, this.text.length - this.lineStarts[lastLine]);
	}

	getOffset(position: TextPosition): number {
		const lineIndex = Math.min(Math.max(position.lineIndex, 0), this.lineStarts.length - 1);
		const columnIndex = Math.min(Math.max(position.columnIndex, 0), this.getLineLength(lineIndex));
		return this.lineStarts[lineIndex] + columnIndex;
	}

	getPosition(offset: number): TextPosition {
		const clampedOffset = Math.min(Math.max(Math.trunc(offset), 0), this.text.length);
		let low = 0;
		let high = this.lineStarts.length - 1;
		while (low <= high) {
			const middle = (low + high) >> 1;
			if (this.lineStarts[middle] <= clampedOffset) low = middle + 1;
			else high = middle - 1;
		}
		const lineIndex = Math.max(0, high);
		return TextPosition.at(lineIndex, Math.min(clampedOffset - this.lineStarts[lineIndex], this.getLineLength(lineIndex)));
	}

	getLineLength(lineIndex: number): number {
		if (!Number.isSafeInteger(lineIndex) || lineIndex < 0 || lineIndex >= this.lineStarts.length) throw new RangeError("Invalid line index");
		return this.lineEnds[lineIndex] - this.lineStarts[lineIndex];
	}
}

/** A transformer backed by a model that exposes logical line contents. */
export class LineBasedPositionOffsetTransformer extends PositionOffsetTransformerBase {
	private readonly lineStarts: readonly number[];
	private readonly lineLengths: readonly number[];

	constructor(lines: readonly string[]) {
		super();
		if (lines.length === 0) throw new RangeError("A text source must contain at least one line");
		const starts: number[] = [];
		const lengths: number[] = [];
		let offset = 0;
		for (const line of lines) {
			starts.push(offset);
			lengths.push(line.length);
			offset += line.length + 1;
		}
		this.lineStarts = starts;
		this.lineLengths = lengths;
	}

	get textLength(): TextLength {
		const lastLineIndex = this.lineLengths.length - 1;
		return new TextLength(lastLineIndex, this.lineLengths[lastLineIndex]);
	}

	getOffset(position: TextPosition): number {
		const lineIndex = Math.min(Math.max(position.lineIndex, 0), this.lineLengths.length - 1);
		const columnIndex = Math.min(Math.max(position.columnIndex, 0), this.lineLengths[lineIndex]);
		return this.lineStarts[lineIndex] + columnIndex;
	}

	getPosition(offset: number): TextPosition {
		const clampedOffset = Math.min(Math.max(Math.trunc(offset), 0), this.textLengthToOffset());
		let low = 0;
		let high = this.lineStarts.length - 1;
		while (low <= high) {
			const middle = (low + high) >> 1;
			if (this.lineStarts[middle] <= clampedOffset) low = middle + 1;
			else high = middle - 1;
		}
		const lineIndex = Math.max(0, high);
		return TextPosition.at(lineIndex, Math.min(clampedOffset - this.lineStarts[lineIndex], this.lineLengths[lineIndex]));
	}

	getLineLength(lineIndex: number): number {
		if (!Number.isSafeInteger(lineIndex) || lineIndex < 0 || lineIndex >= this.lineLengths.length) throw new RangeError("Invalid line index");
		return this.lineLengths[lineIndex];
	}

	private textLengthToOffset(): number { return this.lineStarts.at(-1)! + this.lineLengths.at(-1)!; }
}

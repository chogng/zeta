import { findLastIdxMonotonous } from "../../../../base/common/arraysFind.js";
import { CharCode } from "../../../../base/common/charCode.js";
import { Position } from "../position.js";
import { Range } from "../range.js";
import { OffsetRange } from "../ranges/offsetRange.js";
import { TextLength } from "./textLength.js";

/** Shared coordinate conversion contract for strings and line-based models. */
export abstract class PositionOffsetTransformerBase {
	abstract getOffset(position: Position): number;
	abstract getPosition(offset: number): Position;
	abstract getLineLength(lineNumber: number): number;
	abstract readonly textLength: TextLength;

	getOffsetRange(range: Range): OffsetRange { return new OffsetRange(this.getOffset(range.getStartPosition()), this.getOffset(range.getEndPosition())); }
	getRange(range: OffsetRange): Range { return Range.fromPositions(this.getPosition(range.start), this.getPosition(range.endExclusive)); }
	getTextLength(range: OffsetRange): TextLength { return TextLength.ofRange(this.getRange(range)); }
}

/** Converts one-based editor positions and zero-based UTF-16 offsets in normalized text. */
export class PositionOffsetTransformer extends PositionOffsetTransformerBase {
	private readonly lineStarts: readonly number[];
	private readonly lineEnds: readonly number[];

	constructor(public readonly text: string) {
		super();
		const starts = [0];
		const ends: number[] = [];
		for (let index = 0; index < text.length; index += 1) {
			if (text.charCodeAt(index) === CharCode.LineFeed) {
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

	getOffset(position: Position): number {
		const lineIndex = Math.min(Math.max(position.lineNumber - 1, 0), this.lineStarts.length - 1);
		const columnIndex = Math.min(Math.max(position.column - 1, 0), this.getLineLength(lineIndex + 1));
		return this.lineStarts[lineIndex] + columnIndex;
	}

	getPosition(offset: number): Position {
		const clampedOffset = Math.min(Math.max(Math.trunc(offset), 0), this.text.length);
		const lineIndex = Math.max(0, findLastIdxMonotonous(this.lineStarts, start => start <= clampedOffset));
		return new Position(lineIndex + 1, Math.min(clampedOffset - this.lineStarts[lineIndex], this.getLineLength(lineIndex + 1)) + 1);
	}

	getLineLength(lineNumber: number): number {
		if (!Number.isSafeInteger(lineNumber) || lineNumber < 1 || lineNumber > this.lineStarts.length) throw new RangeError("Invalid line number");
		return this.lineEnds[lineNumber - 1] - this.lineStarts[lineNumber - 1];
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

	getOffset(position: Position): number {
		const lineIndex = Math.min(Math.max(position.lineNumber - 1, 0), this.lineLengths.length - 1);
		const columnIndex = Math.min(Math.max(position.column - 1, 0), this.lineLengths[lineIndex]);
		return this.lineStarts[lineIndex] + columnIndex;
	}

	getPosition(offset: number): Position {
		const clampedOffset = Math.min(Math.max(Math.trunc(offset), 0), this.textLengthToOffset());
		const lineIndex = Math.max(0, findLastIdxMonotonous(this.lineStarts, start => start <= clampedOffset));
		return new Position((lineIndex) + 1, (Math.min(clampedOffset - this.lineStarts[lineIndex], this.lineLengths[lineIndex])) + 1);
	}

	getLineLength(lineNumber: number): number {
		if (!Number.isSafeInteger(lineNumber) || lineNumber < 1 || lineNumber > this.lineLengths.length) throw new RangeError("Invalid line number");
		return this.lineLengths[lineNumber - 1];
	}

	private textLengthToOffset(): number { return this.lineStarts.at(-1)! + this.lineLengths.at(-1)!; }
}

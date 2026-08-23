import { LineRange } from "../ranges/lineRange.js";
import { OffsetRange } from "../ranges/offsetRange.js";
import { TextPosition } from "../position.js";
import { TextRange } from "../range.js";

/**
 * A non-negative text length represented as line breaks plus the final column.
 *
 * The representation is the same as the length of a text fragment: `lineCount`
 * is the number of `\n` characters and `columnCount` is the UTF-16 length after
 * the final line break. It is therefore safe to add lengths without knowing
 * the original line contents.
 */
export class TextLength {
	static readonly zero = new TextLength(0, 0);

	static lengthDiffNonNegative(start: TextLength, end: TextLength): TextLength { return end.isLessThan(start) ? TextLength.zero : TextLength.betweenLengths(start, end); }

	static betweenPositions(start: TextPosition, end: TextPosition): TextLength {
		if (end.isBefore(start)) throw new RangeError("TextLength end must not precede its start");
		return start.lineIndex === end.lineIndex
			? new TextLength(0, end.columnIndex - start.columnIndex)
			: new TextLength(end.lineIndex - start.lineIndex, end.columnIndex);
	}

	static ofPosition(position: TextPosition): TextLength { return new TextLength(position.lineIndex, position.columnIndex); }
	static fromPosition(position: TextPosition): TextLength { return TextLength.ofPosition(position); }
	static ofRange(range: TextRange): TextLength { return TextLength.betweenPositions(range.start, range.end); }
	static ofText(text: string): TextLength {
		let lineCount = 0;
		let columnCount = 0;
		for (let index = 0; index < text.length; index += 1) {
			if (text.charCodeAt(index) === 10) {
				lineCount += 1;
				columnCount = 0;
			} else {
				columnCount += 1;
			}
		}
		return new TextLength(lineCount, columnCount);
	}

	static ofOffsetRange(text: string, range: OffsetRange): TextLength { return TextLength.ofText(range.substring(text)); }
	static ofSubstr(text: string, range: OffsetRange): TextLength { return TextLength.ofOffsetRange(text, range); }
	static sum<T>(values: readonly T[], getLength: (value: T) => TextLength): TextLength {
		return values.reduce((length, value) => length.add(getLength(value)), TextLength.zero);
	}

	private static betweenLengths(start: TextLength, end: TextLength): TextLength {
		return start.lineCount === end.lineCount
			? new TextLength(0, end.columnCount - start.columnCount)
			: new TextLength(end.lineCount - start.lineCount, end.columnCount);
	}

	constructor(readonly lineCount: number, readonly columnCount: number) {
		if (!Number.isSafeInteger(lineCount) || !Number.isSafeInteger(columnCount) || lineCount < 0 || columnCount < 0) {
			throw new RangeError("TextLength values must be non-negative safe integers");
		}
	}

	isZero(): boolean { return this.lineCount === 0 && this.columnCount === 0; }
	isLessThan(other: TextLength): boolean { return this.compare(other) < 0; }
	isGreaterThan(other: TextLength): boolean { return this.compare(other) > 0; }
	isGreaterThanOrEqualTo(other: TextLength): boolean { return this.compare(other) >= 0; }
	equals(other: TextLength): boolean { return this.lineCount === other.lineCount && this.columnCount === other.columnCount; }
	compare(other: TextLength): number { return this.lineCount - other.lineCount || this.columnCount - other.columnCount; }

	add(other: TextLength): TextLength {
		return other.lineCount === 0
			? new TextLength(this.lineCount, this.columnCount + other.columnCount)
			: new TextLength(this.lineCount + other.lineCount, other.columnCount);
	}

	addToPosition(position: TextPosition): TextPosition {
		return this.lineCount === 0
			? TextPosition.at(position.lineIndex, position.columnIndex + this.columnCount)
			: TextPosition.at(position.lineIndex + this.lineCount, this.columnCount);
	}

	addToRange(range: TextRange): TextRange { return TextRange.from(this.addToPosition(range.start), this.addToPosition(range.end)); }
	createRange(start: TextPosition): TextRange { return TextRange.from(start, this.addToPosition(start)); }
	toRange(): TextRange { return this.createRange(TextPosition.at(0, 0)); }
	toLineRange(): LineRange { return LineRange.ofLength(0, this.lineCount + 1); }
	toString(): string { return `${this.lineCount},${this.columnCount}`; }
}

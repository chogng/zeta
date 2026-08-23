import { IPosition } from "./position.js";
import { TextPosition } from "./position.js";

export interface ITextRange {
	readonly start: IPosition;
	readonly end: IPosition;
}

export interface IRange extends ITextRange {}

/**
 * An ordered, end-exclusive text range.
 */
export class TextRange {
	private constructor(
		readonly start: TextPosition,
		readonly end: TextPosition,
	) {
		Object.freeze(this);
	}

	static from(start: IPosition, end: IPosition): TextRange {
		const startPosition = TextPosition.lift(start);
		const endPosition = TextPosition.lift(end);
		if (TextPosition.compare(startPosition, endPosition) > 0) {
			throw new RangeError("TextRange end must not precede its start");
		}
		return new TextRange(startPosition, endPosition);
	}

	static fromPositions(start: IPosition, end = start): TextRange {
		return TextRange.from(start, end);
	}

	static emptyAt(position: IPosition): TextRange {
		const lifted = TextPosition.lift(position);
		return new TextRange(lifted, lifted);
	}

	static isEmpty(range: ITextRange): boolean {
		return TextPosition.equals(range.start, range.end);
	}

	static equals(left: ITextRange | undefined, right: ITextRange | undefined): boolean {
		return left === right || Boolean(left && right && TextPosition.equals(left.start, right.start) && TextPosition.equals(left.end, right.end));
	}

	static join(left: ITextRange, right: ITextRange): TextRange {
		return TextRange.from(
			comparePositions(left.start, right.start) <= 0 ? left.start : right.start,
			comparePositions(left.end, right.end) >= 0 ? left.end : right.end,
		);
	}

	static intersect(left: ITextRange, right: ITextRange): TextRange | undefined {
		const start = comparePositions(left.start, right.start) >= 0 ? left.start : right.start;
		const end = comparePositions(left.end, right.end) <= 0 ? left.end : right.end;
		return comparePositions(start, end) <= 0 ? TextRange.from(start, end) : undefined;
	}

	static intersectRanges(left: ITextRange, right: ITextRange): TextRange | undefined { return TextRange.intersect(left, right); }
	static plusRange(left: ITextRange, right: ITextRange): TextRange { return TextRange.join(left, right); }
	static containsPosition(range: ITextRange, position: IPosition): boolean { return comparePositions(range.start, position) <= 0 && comparePositions(position, range.end) <= 0; }
	static strictContainsPosition(range: ITextRange, position: IPosition): boolean { return comparePositions(range.start, position) < 0 && comparePositions(position, range.end) < 0; }
	static containsRange(range: ITextRange, other: ITextRange): boolean { return comparePositions(range.start, other.start) <= 0 && comparePositions(other.end, range.end) <= 0; }
	static strictContainsRange(range: ITextRange, other: ITextRange): boolean { return comparePositions(range.start, other.start) < 0 && comparePositions(other.end, range.end) < 0; }
	static equalsRange(left: ITextRange | undefined | null, right: ITextRange | undefined | null): boolean { return TextRange.equals(left ?? undefined, right ?? undefined); }
	static compareRangesUsingStarts(left: ITextRange | undefined | null, right: ITextRange | undefined | null): number { return compareRanges(left, right, false); }
	static compareRangesUsingEnds(left: ITextRange, right: ITextRange): number { return compareRanges(left, right, true); }
	static areIntersectingOrTouching(left: ITextRange, right: ITextRange): boolean { return comparePositions(left.start, right.end) <= 0 && comparePositions(right.start, left.end) <= 0; }
	static areIntersecting(left: ITextRange, right: ITextRange): boolean { return comparePositions(left.start, right.end) < 0 && comparePositions(right.start, left.end) < 0; }
	static areOnlyIntersecting(left: ITextRange, right: ITextRange): boolean { return TextRange.areIntersecting(left, right); }
	static lift(range: ITextRange | undefined | null): TextRange | null { return range ? TextRange.from(range.start, range.end) : null; }
	static isIRange(value: unknown): value is ITextRange { return Boolean(value && typeof value === "object" && TextPosition.isIPosition((value as ITextRange).start) && TextPosition.isIPosition((value as ITextRange).end)); }
	static spansMultipleLines(range: ITextRange): boolean { return range.start.lineIndex < range.end.lineIndex; }

	get empty(): boolean {
		return this.start.compareTo(this.end) === 0;
	}

	get isEmpty(): boolean { return this.empty; }

	get length(): { readonly lineCount: number; readonly columnCount: number } {
		if (this.start.lineIndex === this.end.lineIndex) {
			return { lineCount: 0, columnCount: this.end.columnIndex - this.start.columnIndex };
		}
		return { lineCount: this.end.lineIndex - this.start.lineIndex, columnCount: this.end.columnIndex };
	}

	containsPosition(position: IPosition): boolean {
		return comparePositions(this.start, position) <= 0 && comparePositions(position, this.end) <= 0;
	}

	strictContainsPosition(position: IPosition): boolean {
		return comparePositions(this.start, position) < 0 && comparePositions(position, this.end) < 0;
	}

	containsRange(range: ITextRange): boolean {
		return comparePositions(this.start, range.start) <= 0 && comparePositions(range.end, this.end) <= 0;
	}

	strictContainsRange(range: ITextRange): boolean { return comparePositions(this.start, range.start) < 0 && comparePositions(range.end, this.end) < 0; }

	intersects(range: ITextRange): boolean {
		return comparePositions(this.start, range.end) < 0 && comparePositions(range.start, this.end) < 0;
	}

	intersectsOrTouches(range: ITextRange): boolean {
		return comparePositions(this.start, range.end) <= 0 && comparePositions(range.start, this.end) <= 0;
	}

	plusRange(range: ITextRange): TextRange {
		return TextRange.join(this, range);
	}

	intersect(range: ITextRange): TextRange | undefined {
		return TextRange.intersect(this, range);
	}

	equals(other: ITextRange): boolean {
		return TextPosition.equals(this.start, other.start) && TextPosition.equals(this.end, other.end);
	}

	equalsRange(other: ITextRange | undefined | null): boolean { return TextRange.equals(this, other ?? undefined); }
	getStartPosition(): TextPosition { return this.start; }
	getEndPosition(): TextPosition { return this.end; }
	setStartPosition(lineIndex: number, columnIndex: number): TextRange { return TextRange.from(TextPosition.at(lineIndex, columnIndex), this.end); }
	setEndPosition(lineIndex: number, columnIndex: number): TextRange { return TextRange.from(this.start, TextPosition.at(lineIndex, columnIndex)); }
	collapseToStart(): TextRange { return TextRange.emptyAt(this.start); }
	collapseToEnd(): TextRange { return TextRange.emptyAt(this.end); }
	delta(lineDelta: number): TextRange { return TextRange.from(this.start.delta(lineDelta), this.end.delta(lineDelta)); }
	isSingleLine(): boolean { return this.start.lineIndex === this.end.lineIndex; }
	toJSON(): ITextRange { return this; }

	toString(): string {
		return `[${this.start.toString()},${this.end.toString()})`;
	}
}

function comparePositions(left: IPosition, right: IPosition): number { return TextPosition.compare(left, right); }

function compareRanges(left: ITextRange | undefined | null, right: ITextRange | undefined | null, byEnd: boolean): number {
	if (!left || !right) return Number(Boolean(right)) - Number(Boolean(left));
	const primary = byEnd ? comparePositions(left.end, right.end) : comparePositions(left.start, right.start);
	if (primary !== 0) return primary;
	return byEnd ? comparePositions(left.start, right.start) : comparePositions(left.end, right.end);
}

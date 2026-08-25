import { isNonNegativeSafeInteger } from "../../../base/common/numbers.js";

export interface IPosition {
	readonly lineIndex: number;
	readonly columnIndex: number;
}

/**
 * A zero-based position in normalized UTF-16 text.
 *
 * `lineIndex` and `columnIndex` are explicit about their indexing convention.
 * Columns count UTF-16 code units, matching JavaScript string offsets and DOM
 * selection APIs.
 */
export class TextPosition {
	private constructor(
		readonly lineIndex: number,
		readonly columnIndex: number,
	) {
		Object.freeze(this);
	}

	static at(lineIndex: number, columnIndex: number): TextPosition {
		assertIndex(lineIndex, "lineIndex");
		assertIndex(columnIndex, "columnIndex");
		return new TextPosition(lineIndex, columnIndex);
	}

	static lift(position: IPosition): TextPosition { return position instanceof TextPosition ? position : TextPosition.at(position.lineIndex, position.columnIndex); }

	static isIPosition(value: unknown): value is IPosition {
		if (!value || typeof value !== "object") return false;
		const position = value as Partial<IPosition>;
		return typeof position.lineIndex === "number" && typeof position.columnIndex === "number";
	}

	static equals(left: IPosition | undefined, right: IPosition | undefined): boolean {
		return left === right || Boolean(left && right && TextPosition.compare(left, right) === 0);
	}

	static compare(left: IPosition, right: IPosition): number {
		return left.lineIndex - right.lineIndex || left.columnIndex - right.columnIndex;
	}

	compareTo(other: IPosition): number {
		return this.lineIndex - other.lineIndex ||
			this.columnIndex - other.columnIndex;
	}

	with(lineIndex = this.lineIndex, columnIndex = this.columnIndex): TextPosition {
		if (lineIndex === this.lineIndex && columnIndex === this.columnIndex) return this;
		return TextPosition.at(lineIndex, columnIndex);
	}

	delta(lineDelta = 0, columnDelta = 0): TextPosition {
		return TextPosition.at(
			Math.max(0, this.lineIndex + lineDelta),
			Math.max(0, this.columnIndex + columnDelta),
		);
	}

	clone(): TextPosition { return this; }

	equals(other: IPosition): boolean {
		return this.compareTo(other) === 0;
	}

	isBefore(other: IPosition): boolean {
		return this.compareTo(other) < 0;
	}

	isBeforeOrEqual(other: IPosition): boolean {
		return this.compareTo(other) <= 0;
	}

	isAfter(other: IPosition): boolean {
		return this.compareTo(other) > 0;
	}

	isAfterOrEqual(other: IPosition): boolean {
		return this.compareTo(other) >= 0;
	}

	toString(): string {
		return `(${this.lineIndex},${this.columnIndex})`;
	}

	toJSON(): IPosition { return { lineIndex: this.lineIndex, columnIndex: this.columnIndex }; }
}

function assertIndex(value: number, name: string): void {
	if (!isNonNegativeSafeInteger(value)) {
		throw new RangeError(`${name} must be a non-negative safe integer`);
	}
}

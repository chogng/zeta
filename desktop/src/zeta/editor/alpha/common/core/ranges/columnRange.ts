import { TextPosition } from "../position.js";
import { TextRange } from "../range.js";
import { OffsetRange } from "./offsetRange.js";

/** A zero-based half-open range of columns on one physical line. */
export class ColumnRange {
  static fromOffsetRange(range: OffsetRange): ColumnRange { return new ColumnRange(range.start, range.endExclusive); }

  constructor(readonly startColumnIndex: number, readonly endColumnIndexExclusive: number) {
    if (startColumnIndex < 0 || startColumnIndex > endColumnIndexExclusive) throw new RangeError("Invalid column range");
  }

  get length(): number { return this.endColumnIndexExclusive - this.startColumnIndex; }
  get empty(): boolean { return this.length === 0; }
  toRange(lineIndex: number): TextRange { return TextRange.from(TextPosition.at(lineIndex, this.startColumnIndex), TextPosition.at(lineIndex, this.endColumnIndexExclusive)); }
  toOffsetRange(): OffsetRange { return new OffsetRange(this.startColumnIndex, this.endColumnIndexExclusive); }
  equals(other: ColumnRange): boolean { return this.startColumnIndex === other.startColumnIndex && this.endColumnIndexExclusive === other.endColumnIndexExclusive; }
}

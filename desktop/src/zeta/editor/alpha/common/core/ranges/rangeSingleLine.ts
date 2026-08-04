import { TextPosition } from "../position.js";
import { TextRange } from "../range.js";
import { ColumnRange } from "./columnRange.js";

export class RangeSingleLine {
  static fromRange(range: TextRange): RangeSingleLine | undefined {
    if (range.start.lineIndex !== range.end.lineIndex) return undefined;
    return new RangeSingleLine(range.start.lineIndex, new ColumnRange(range.start.columnIndex, range.end.columnIndex));
  }

  constructor(readonly lineIndex: number, readonly columnRange: ColumnRange) {}

  toRange(): TextRange {
    const start = rangePosition(this.lineIndex, this.columnRange.startColumnIndex);
    const end = rangePosition(this.lineIndex, this.columnRange.endColumnIndexExclusive);
    return TextRange.from(start, end);
  }
}

function rangePosition(lineIndex: number, columnIndex: number) {
  return TextPosition.at(lineIndex, columnIndex);
}

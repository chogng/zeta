import { LineRange } from "../ranges/lineRange.js";
import { OffsetRange } from "../ranges/offsetRange.js";
import { TextPosition } from "../position.js";
import { TextRange } from "../range.js";
import { normalizeTextLineEndings } from "../textChange.js";
import { PositionOffsetTransformer } from "./positionToOffsetImpl.js";
import { TextLength } from "./textLength.js";

/** A DOM-free text value that can expose slices in editor coordinates. */
export abstract class AbstractText {
  abstract readonly length: TextLength;
  abstract getValueOfRange(range: TextRange): string;

  get endPositionExclusive(): TextPosition { return this.length.addToPosition(TextPosition.at(0, 0)); }
  get lineRange(): LineRange { return this.length.toLineRange(); }
  getValue(): string { return this.getValueOfRange(this.length.toRange()); }
  getValueOfOffsetRange(range: OffsetRange): string { return this.getValueOfRange(this.getTransformer().getRange(range)); }
  getLineLength(lineIndex: number): number { return this.getTransformer().getLineLength(lineIndex); }
  getLineAt(lineIndex: number): string { return this.getValueOfRange(TextRange.from(TextPosition.at(lineIndex, 0), TextPosition.at(lineIndex, this.getLineLength(lineIndex)))); }
  getLines(): string[] { return this.getValue().split("\n"); }
  getLinesOfRange(range: LineRange): string[] { return range.mapToLineArray(lineIndex => this.getLineAt(lineIndex)); }
  getTransformer(): PositionOffsetTransformer { return new PositionOffsetTransformer(this.getValue()); }
  equals(other: AbstractText): boolean { return this === other || this.getValue() === other.getValue(); }
}

/** An AbstractText view backed by a model-like line source. */
export class LineBasedText extends AbstractText {
  constructor(private readonly getLineContent: (lineIndex: number) => string, readonly lineCount: number) {
    super();
    if (!Number.isSafeInteger(lineCount) || lineCount < 1) throw new RangeError("A text source must contain at least one line");
  }

  get length(): TextLength {
    return new TextLength(this.lineCount - 1, this.getLineContent(this.lineCount - 1).length);
  }

  getValueOfRange(range: TextRange): string {
    validateLineRange(range, this.lineCount);
    if (range.start.lineIndex === range.end.lineIndex) return this.getLineContent(range.start.lineIndex).slice(range.start.columnIndex, range.end.columnIndex);
    let value = this.getLineContent(range.start.lineIndex).slice(range.start.columnIndex);
    for (let lineIndex = range.start.lineIndex + 1; lineIndex < range.end.lineIndex; lineIndex += 1) value += `\n${this.getLineContent(lineIndex)}`;
    return `${value}\n${this.getLineContent(range.end.lineIndex).slice(0, range.end.columnIndex)}`;
  }

  override getLineLength(lineIndex: number): number { return this.getLineContent(lineIndex).length; }
}

export class ArrayText extends LineBasedText {
  constructor(lines: readonly string[]) {
    super(lineIndex => lines[lineIndex], lines.length);
  }
}

/** A text view backed by one normalized JavaScript string. */
export class StringText extends AbstractText {
  readonly value: string;
  private readonly transformer: PositionOffsetTransformer;

  constructor(value: string) {
    super();
    this.value = normalizeTextLineEndings(value);
    this.transformer = new PositionOffsetTransformer(this.value);
  }

  get length(): TextLength { return this.transformer.textLength; }
  getValueOfRange(range: TextRange): string { return this.transformer.getOffsetRange(range).substring(this.value); }
  override getTransformer(): PositionOffsetTransformer { return this.transformer; }
}

function validateLineRange(range: TextRange, lineCount: number): void {
  if (range.start.lineIndex >= lineCount || range.end.lineIndex >= lineCount) throw new RangeError("Text range is outside the text source");
}

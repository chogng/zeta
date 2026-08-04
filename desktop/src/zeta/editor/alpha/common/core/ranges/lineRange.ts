import { TextRange } from "../range.js";
import { TextPosition } from "../position.js";
import { OffsetRange } from "./offsetRange.js";

/** A zero-based half-open range of physical line indexes. */
export class LineRange {
  static readonly compareByStart = (left: LineRange, right: LineRange): number => left.startLineIndex - right.startLineIndex;
  static ofLength(startLineIndex: number, length: number): LineRange { return new LineRange(startLineIndex, startLineIndex + length); }
  static fromRange(range: TextRange): LineRange { return new LineRange(range.start.lineIndex, range.end.lineIndex + (range.end.columnIndex > 0 ? 1 : 0)); }
  static fromRangeInclusive(range: TextRange): LineRange { return new LineRange(range.start.lineIndex, range.end.lineIndex + 1); }
  static joinMany(lineRanges: readonly (readonly LineRange[])[]): readonly LineRange[] { return lineRanges.reduce<LineRangeSet>((set, ranges) => set.getUnion(new LineRangeSet(ranges)), new LineRangeSet()).ranges; }
  static join(ranges: readonly LineRange[]): LineRange { if (ranges.length === 0) throw new RangeError("Cannot join an empty line range list"); return new LineRange(Math.min(...ranges.map(range => range.startLineIndex)), Math.max(...ranges.map(range => range.endLineIndexExclusive))); }
  static deserialize(value: readonly [number, number]): LineRange { return new LineRange(value[0], value[1]); }
  static subtract(left: LineRange, right: LineRange | undefined): LineRange[] { if (!right) return [left]; if (right.endLineIndexExclusive <= left.startLineIndex || left.endLineIndexExclusive <= right.startLineIndex) return [left]; const result: LineRange[] = []; if (left.startLineIndex < right.startLineIndex) result.push(new LineRange(left.startLineIndex, right.startLineIndex)); if (right.endLineIndexExclusive < left.endLineIndexExclusive) result.push(new LineRange(right.endLineIndexExclusive, left.endLineIndexExclusive)); return result; }

  constructor(readonly startLineIndex: number, readonly endLineIndexExclusive: number) {
    if (!Number.isSafeInteger(startLineIndex) || !Number.isSafeInteger(endLineIndexExclusive) || startLineIndex < 0 || startLineIndex > endLineIndexExclusive) throw new RangeError("Invalid line range");
  }

  get length(): number { return this.endLineIndexExclusive - this.startLineIndex; }
  get empty(): boolean { return this.length === 0; }
  get isEmpty(): boolean { return this.empty; }
  contains(lineIndex: number): boolean { return this.startLineIndex <= lineIndex && lineIndex < this.endLineIndexExclusive; }
  containsRange(range: LineRange): boolean { return this.startLineIndex <= range.startLineIndex && range.endLineIndexExclusive <= this.endLineIndexExclusive; }
  delta(offset: number): LineRange { return new LineRange(this.startLineIndex + offset, this.endLineIndexExclusive + offset); }
  deltaLength(offset: number): LineRange { return new LineRange(this.startLineIndex, this.endLineIndexExclusive + offset); }
  join(other: LineRange): LineRange { return new LineRange(Math.min(this.startLineIndex, other.startLineIndex), Math.max(this.endLineIndexExclusive, other.endLineIndexExclusive)); }
  intersect(other: LineRange): LineRange | undefined { const start = Math.max(this.startLineIndex, other.startLineIndex); const end = Math.min(this.endLineIndexExclusive, other.endLineIndexExclusive); return start <= end ? new LineRange(start, end) : undefined; }
  intersectsStrict(other: LineRange): boolean { return this.startLineIndex < other.endLineIndexExclusive && other.startLineIndex < this.endLineIndexExclusive; }
  intersectsOrTouches(other: LineRange): boolean { return this.startLineIndex <= other.endLineIndexExclusive && other.startLineIndex <= this.endLineIndexExclusive; }
  equals(other: LineRange): boolean { return this.startLineIndex === other.startLineIndex && this.endLineIndexExclusive === other.endLineIndexExclusive; }
  toOffsetRange(): OffsetRange { return new OffsetRange(this.startLineIndex, this.endLineIndexExclusive); }
  toRange(): TextRange | undefined { return this.empty ? undefined : TextRange.from(TextPosition.at(this.startLineIndex, 0), TextPosition.at(this.endLineIndexExclusive - 1, Number.MAX_SAFE_INTEGER)); }
  toInclusiveRange(): TextRange | undefined { return this.toRange(); }
  toExclusiveRange(): TextRange { return TextRange.from(TextPosition.at(this.startLineIndex, 0), TextPosition.at(this.endLineIndexExclusive, 0)); }
  serialize(): readonly [number, number] { return [this.startLineIndex, this.endLineIndexExclusive]; }
  mapToLineArray<T>(map: (lineIndex: number) => T): T[] { const result: T[] = []; this.forEach(lineIndex => result.push(map(lineIndex))); return result; }
  forEach(callback: (lineIndex: number) => void): void { for (let lineIndex = this.startLineIndex; lineIndex < this.endLineIndexExclusive; lineIndex += 1) callback(lineIndex); }
  distanceToRange(other: LineRange): number { if (this.endLineIndexExclusive <= other.startLineIndex) return other.startLineIndex - this.endLineIndexExclusive; if (other.endLineIndexExclusive <= this.startLineIndex) return this.startLineIndex - other.endLineIndexExclusive; return 0; }
  distanceToLine(lineIndex: number): number { if (this.contains(lineIndex)) return 0; return lineIndex < this.startLineIndex ? this.startLineIndex - lineIndex : lineIndex - this.endLineIndexExclusive; }
  addMargin(top: number, bottom = top): LineRange { return new LineRange(Math.max(0, this.startLineIndex - top), this.endLineIndexExclusive + bottom); }
  toString(): string { return `[${this.startLineIndex},${this.endLineIndexExclusive})`; }
}

export class LineRangeSet {
  private readonly normalizedRanges: LineRange[];

  constructor(normalizedRanges: readonly LineRange[] = []) {
    this.normalizedRanges = [...normalizedRanges];
    this.normalizedRanges.sort((left, right) => left.startLineIndex - right.startLineIndex);
  }

  get ranges(): readonly LineRange[] { return [...this.normalizedRanges]; }

  addRange(range: LineRange): void {
    if (range.empty) return;
    const ranges = [...this.normalizedRanges, range].sort((left, right) => left.startLineIndex - right.startLineIndex);
    this.normalizedRanges.length = 0;
    for (const candidate of ranges) {
      const previous = this.normalizedRanges.at(-1);
      if (previous && previous.endLineIndexExclusive >= candidate.startLineIndex) this.normalizedRanges[this.normalizedRanges.length - 1] = previous.join(candidate);
      else this.normalizedRanges.push(candidate);
    }
  }

  contains(lineIndex: number): boolean { return this.normalizedRanges.some(range => range.contains(lineIndex)); }
  intersects(range: LineRange): boolean { return this.normalizedRanges.some(candidate => candidate.intersectsStrict(range)); }
  getUnion(other: LineRangeSet): LineRangeSet { const result = new LineRangeSet(this.ranges); for (const range of other.ranges) result.addRange(range); return result; }
  subtractFrom(range: LineRange): LineRangeSet { let result = [range]; for (const candidate of this.normalizedRanges) result = result.flatMap(current => LineRange.subtract(current, candidate)); return new LineRangeSet(result); }
  getIntersection(other: LineRangeSet): LineRangeSet { const result = new LineRangeSet(); for (const left of this.normalizedRanges) for (const right of other.normalizedRanges) { const intersection = left.intersect(right); if (intersection && !intersection.empty) result.addRange(intersection); } return result; }
  getWithDelta(delta: number): LineRangeSet { return new LineRangeSet(this.normalizedRanges.map(range => range.delta(delta))); }
  toString(): string { return this.normalizedRanges.map(range => range.toString()).join(", "); }
}

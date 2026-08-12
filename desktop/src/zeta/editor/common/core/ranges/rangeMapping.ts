import { TextPosition } from "../position.js";
import { TextRange } from "../range.js";
import { TextLength } from "../text/textLength.js";

export class RangeMapping {
  constructor(readonly mappings: readonly SingleRangeMapping[]) {}

  mapPosition(position: TextPosition): PositionOrRange {
    const mapping = [...this.mappings].reverse().find(candidate => candidate.original.start.isBeforeOrEqual(position));
    if (!mapping) return PositionOrRange.position(position);
    if (mapping.original.containsPosition(position)) return PositionOrRange.range(mapping.modified);
    return PositionOrRange.position(TextLength.betweenPositions(mapping.original.end, position).addToPosition(mapping.modified.end));
  }

  mapRange(range: TextRange): TextRange {
    const start = this.mapPosition(range.start);
    const end = this.mapPosition(range.end);
    return TextRange.from(start.range?.start ?? start.position!, end.range?.end ?? end.position!);
  }

  reverse(): RangeMapping { return new RangeMapping(this.mappings.map(mapping => mapping.reverse())); }
}

export class SingleRangeMapping {
  constructor(readonly original: TextRange, readonly modified: TextRange) {}
  reverse(): SingleRangeMapping { return new SingleRangeMapping(this.modified, this.original); }
  toString(): string { return `${this.original.toString()} -> ${this.modified.toString()}`; }
}

export class PositionOrRange {
  static position(position: TextPosition): PositionOrRange { return new PositionOrRange(position, undefined); }
  static range(range: TextRange): PositionOrRange { return new PositionOrRange(undefined, range); }
  private constructor(readonly position: TextPosition | undefined, readonly range: TextRange | undefined) {}
}

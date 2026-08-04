import { OffsetRange } from "../ranges/offsetRange.js";

/** Common contract for a replacement whose input and output have measurable length. */
export abstract class BaseReplacement<Self extends BaseReplacement<Self>> {
  constructor(readonly replaceRange: OffsetRange) {}

  abstract getNewLength(): number;
  abstract tryJoinTouching(other: Self): Self | undefined;
  abstract slice(newReplaceRange: OffsetRange, rangeInReplacement?: OffsetRange): Self;
  abstract equals(other: Self): boolean;

  delta(offset: number): Self { return this.slice(this.replaceRange.delta(offset), new OffsetRange(0, this.getNewLength())); }
  getLengthDelta(): number { return this.getNewLength() - this.replaceRange.length; }
  get isEmpty(): boolean { return this.replaceRange.isEmpty && this.getNewLength() === 0; }
  getRangeAfterReplace(): OffsetRange { return new OffsetRange(this.replaceRange.start, this.replaceRange.start + this.getNewLength()); }
  toString(): string { return `{ ${this.replaceRange.toString()} -> ${this.getNewLength()} }`; }
}

/** A sorted, disjoint set of replacements applied simultaneously. */
export abstract class BaseEdit<Replacement extends BaseReplacement<Replacement>, Self extends BaseEdit<Replacement, Self>> {
  constructor(readonly replacements: readonly Replacement[]) {
    let previousEnd = -1;
    for (const replacement of replacements) {
      if (replacement.replaceRange.start < previousEnd) throw new RangeError("Edits must be sorted and disjoint");
      previousEnd = replacement.replaceRange.endExclusive;
    }
  }

  protected abstract createNew(replacements: readonly Replacement[]): Self;

  equals(other: Self): boolean {
    return this.replacements.length === other.replacements.length && this.replacements.every((replacement, index) => replacement.equals(other.replacements[index]!));
  }

  isEmpty(): boolean { return this.replacements.length === 0; }
  getLengthDelta(): number { return this.replacements.reduce((sum, replacement) => sum + replacement.getLengthDelta(), 0); }
  getNewDataLength(dataLength: number): number { return dataLength + this.getLengthDelta(); }
  toString(): string { return `[${this.replacements.map(replacement => replacement.toString()).join(", ")}]`; }

  normalize(): Self {
    const normalized: Replacement[] = [];
    let previous: Replacement | undefined;
    for (const replacement of this.replacements) {
      if (replacement.isEmpty) continue;
      if (previous && previous.replaceRange.endExclusive === replacement.replaceRange.start) {
        const joined = previous.tryJoinTouching(replacement);
        if (joined) {
          previous = joined;
          continue;
        }
      }
      if (previous) normalized.push(previous);
      previous = replacement;
    }
    if (previous) normalized.push(previous);
    return this.createNew(normalized);
  }

  /** Composes this edit with an edit expressed in the result coordinate space. */
  compose(other: Self): Self {
    const first = this.normalize();
    const second = other.normalize();
    if (first.isEmpty()) return second;
    if (second.isEmpty()) return first;

    const pending = [...first.replacements];
    const result: Replacement[] = [];
    let delta = 0;

    for (const secondReplacement of second.replacements) {
      while (true) {
        const firstReplacement = pending[0];
        if (!firstReplacement || firstReplacement.replaceRange.start + delta + firstReplacement.getNewLength() >= secondReplacement.replaceRange.start) break;
        pending.shift();
        result.push(firstReplacement);
        delta += firstReplacement.getLengthDelta();
      }

      const deltaBeforeIntersecting = delta;
      let firstIntersecting: Replacement | undefined;
      let lastIntersecting: Replacement | undefined;
      while (true) {
        const firstReplacement = pending[0];
        if (!firstReplacement || firstReplacement.replaceRange.start + delta > secondReplacement.replaceRange.endExclusive) break;
        firstIntersecting ??= firstReplacement;
        lastIntersecting = firstReplacement;
        pending.shift();
        delta += firstReplacement.getLengthDelta();
      }

      if (!firstIntersecting) {
        result.push(secondReplacement.delta(-delta));
        continue;
      }

      const newReplaceRangeStart = Math.min(firstIntersecting.replaceRange.start, secondReplacement.replaceRange.start - deltaBeforeIntersecting);
      const prefixLength = secondReplacement.replaceRange.start - (firstIntersecting.replaceRange.start + deltaBeforeIntersecting);
      if (prefixLength > 0) result.push(firstIntersecting.slice(OffsetRange.emptyAt(newReplaceRangeStart), new OffsetRange(0, prefixLength)));
      if (!lastIntersecting) throw new Error("Edit composition invariant violated");

      const suffixLength = lastIntersecting.replaceRange.endExclusive + delta - secondReplacement.replaceRange.endExclusive;
      if (suffixLength > 0) {
        const suffix = lastIntersecting.slice(OffsetRange.ofStartAndLength(lastIntersecting.replaceRange.endExclusive, 0), new OffsetRange(lastIntersecting.getNewLength() - suffixLength, lastIntersecting.getNewLength()));
        pending.unshift(suffix);
        delta -= suffix.getLengthDelta();
      }

      const newReplaceRange = new OffsetRange(newReplaceRangeStart, secondReplacement.replaceRange.endExclusive - delta);
      result.push(secondReplacement.slice(newReplaceRange, new OffsetRange(0, secondReplacement.getNewLength())));
    }

    result.push(...pending);
    return this.createNew(result).normalize();
  }

  decomposeSplit(shouldBeInFirst: (replacement: Replacement) => boolean): { first: Self; second: Self } {
    const first: Replacement[] = [];
    const second: Replacement[] = [];
    let secondDelta = 0;
    for (const replacement of this.replacements) {
      if (shouldBeInFirst(replacement)) {
        first.push(replacement);
        secondDelta += replacement.getLengthDelta();
      } else {
        second.push(replacement.slice(replacement.replaceRange.delta(secondDelta), new OffsetRange(0, replacement.getNewLength())));
      }
    }
    return { first: this.createNew(first), second: this.createNew(second) };
  }

  getNewRanges(): OffsetRange[] {
    const result: OffsetRange[] = [];
    let delta = 0;
    for (const replacement of this.replacements) {
      result.push(OffsetRange.ofStartAndLength(replacement.replaceRange.start + delta, replacement.getNewLength()));
      delta += replacement.getLengthDelta();
    }
    return result;
  }

  getJoinedReplaceRange(): OffsetRange | undefined {
    if (this.replacements.length === 0) return undefined;
    return this.replacements[0]!.replaceRange.join(this.replacements.at(-1)!.replaceRange);
  }

  applyToOffset(originalOffset: number): number {
    let delta = 0;
    for (const replacement of this.replacements) {
      if (replacement.replaceRange.start > originalOffset) break;
      if (originalOffset < replacement.replaceRange.endExclusive) return replacement.replaceRange.start + delta;
      delta += replacement.getLengthDelta();
    }
    return originalOffset + delta;
  }

  applyToOffsetRange(originalRange: OffsetRange): OffsetRange { return new OffsetRange(this.applyToOffset(originalRange.start), this.applyToOffset(originalRange.endExclusive)); }

  applyInverseToOffset(newOffset: number): number {
    let delta = 0;
    for (const replacement of this.replacements) {
      const newStart = replacement.replaceRange.start + delta;
      const newEnd = newStart + replacement.getNewLength();
      if (newOffset < newStart) break;
      if (newOffset < newEnd) return replacement.replaceRange.start;
      delta += replacement.getLengthDelta();
    }
    return newOffset - delta;
  }

  applyToOffsetOrUndefined(originalOffset: number): number | undefined {
    let delta = 0;
    for (const replacement of this.replacements) {
      if (replacement.replaceRange.start > originalOffset) break;
      if (originalOffset < replacement.replaceRange.endExclusive) return undefined;
      delta += replacement.getLengthDelta();
    }
    return originalOffset + delta;
  }

  applyToOffsetRangeOrUndefined(originalRange: OffsetRange): OffsetRange | undefined {
    const start = this.applyToOffsetOrUndefined(originalRange.start);
    const end = this.applyToOffsetOrUndefined(originalRange.endExclusive);
    return start === undefined || end === undefined ? undefined : new OffsetRange(start, end);
  }
}

export type AnyReplacement = BaseReplacement<any>;
export type AnyEdit = BaseEdit<any, any>;

export class Edit<Replacement extends BaseReplacement<Replacement>> extends BaseEdit<Replacement, Edit<Replacement>> {
  static readonly empty = new Edit<never>([]);
  static create<Replacement extends BaseReplacement<Replacement>>(replacements: readonly Replacement[]): Edit<Replacement> { return new Edit(replacements); }
  static single<Replacement extends BaseReplacement<Replacement>>(replacement: Replacement): Edit<Replacement> { return new Edit([replacement]); }
  protected createNew(replacements: readonly Replacement[]): Edit<Replacement> { return new Edit(replacements); }
}

/** A length-only replacement carrying an annotation through edit algebra. */
export class AnnotationReplacement<Annotation> extends BaseReplacement<AnnotationReplacement<Annotation>> {
  constructor(range: OffsetRange, readonly newLength: number, readonly annotation: Annotation) {
    super(range);
    if (!Number.isSafeInteger(newLength) || newLength < 0) throw new RangeError("Replacement length must be non-negative");
  }

  getNewLength(): number { return this.newLength; }
  equals(other: AnnotationReplacement<Annotation>): boolean { return this.replaceRange.equals(other.replaceRange) && this.newLength === other.newLength && this.annotation === other.annotation; }
  tryJoinTouching(other: AnnotationReplacement<Annotation>): AnnotationReplacement<Annotation> | undefined {
    return this.annotation === other.annotation ? new AnnotationReplacement(this.replaceRange.joinRightTouching(other.replaceRange), this.newLength + other.newLength, this.annotation) : undefined;
  }
  slice(range: OffsetRange, rangeInReplacement?: OffsetRange): AnnotationReplacement<Annotation> { return new AnnotationReplacement(range, rangeInReplacement?.length ?? this.newLength, this.annotation); }
}

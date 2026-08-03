import { getTextGraphemeBoundaries } from "../../textSegmentation.js";

export enum LineDiffKind {
  Unchanged = "unchanged",
  Modified = "modified",
  Removed = "removed",
  Added = "added",
}

export interface DiffRange {
  readonly startColumn: number;
  readonly endColumn: number;
}

/** One aligned visual row in a side-by-side line diff. */
export interface LineDiffRow {
  readonly kind: LineDiffKind;
  readonly originalLineIndex?: number;
  readonly modifiedLineIndex?: number;
  readonly originalChanges: readonly DiffRange[];
  readonly modifiedChanges: readonly DiffRange[];
}

export interface LineDiff {
  readonly rows: readonly LineDiffRow[];
  /** True only when the bounded exact algorithm had to retain a coarse hunk. */
  readonly approximate: boolean;
}

export interface LineDiffOptions {
  /** Maximum path exploration work before the model produces one conservative hunk. */
  readonly maximumComputationSteps?: number;
}

type Edit = EqualEdit | RemovedEdit | AddedEdit;

interface EqualEdit {
  readonly kind: "equal";
  readonly originalLineIndex: number;
  readonly modifiedLineIndex: number;
}

interface RemovedEdit {
  readonly kind: "removed";
  readonly originalLineIndex: number;
}

interface AddedEdit {
  readonly kind: "added";
  readonly modifiedLineIndex: number;
}

const DEFAULT_MAXIMUM_COMPUTATION_STEPS = 2_000_000;

/**
 * Computes a bounded, stable line diff without depending on DOM or Monaco.
 *
 * Equal lines retain their original and modified positions. Adjacent remove/add
 * edits align into modified rows, and each such pair contains grapheme-safe
 * inline ranges for presentation. If a pathological input exceeds the stated
 * work budget, the result is conservative rather than pretending unrelated
 * lines are equal.
 */
export function computeLineDiff(originalText: string, modifiedText: string, options: LineDiffOptions = {}): LineDiff {
  if (typeof originalText !== "string" || typeof modifiedText !== "string") {
    throw new TypeError("Line diff requires string inputs");
  }
  const maximumComputationSteps = options.maximumComputationSteps ?? DEFAULT_MAXIMUM_COMPUTATION_STEPS;
  if (!Number.isSafeInteger(maximumComputationSteps) || maximumComputationSteps <= 0) {
    throw new RangeError("Line diff computation budget must be a positive safe integer");
  }
  const originalLines = originalText.split("\n");
  const modifiedLines = modifiedText.split("\n");
  const edits = computeEdits(originalLines, modifiedLines, maximumComputationSteps);
  if (!edits) return coarseDiff(originalLines, modifiedLines);
  return Object.freeze({
    rows: Object.freeze(createRows(edits, originalLines, modifiedLines)),
    approximate: false,
  });
}

function computeEdits(originalLines: readonly string[], modifiedLines: readonly string[], maximumSteps: number): readonly Edit[] | undefined {
  const originalLength = originalLines.length;
  const modifiedLength = modifiedLines.length;
  const maximumDistance = originalLength + modifiedLength;
  const offset = maximumDistance;
  let vector = new Int32Array(maximumDistance * 2 + 3);
  vector.fill(-1);
  vector[offset + 1] = 0;
  const trace: DiffVectorSnapshot[] = [];
  let steps = 0;

  for (let distance = 0; distance <= maximumDistance; distance += 1) {
    const next = new Int32Array(vector);
    for (let diagonal = -distance; diagonal <= distance; diagonal += 2) {
      if (++steps > maximumSteps) return undefined;
      const vectorIndex = offset + diagonal;
      let originalIndex: number;
      if (diagonal === -distance || (diagonal !== distance && vector[vectorIndex - 1]! < vector[vectorIndex + 1]!)) {
        originalIndex = vector[vectorIndex + 1]!;
      } else {
        originalIndex = vector[vectorIndex - 1]! + 1;
      }
      let modifiedIndex = originalIndex - diagonal;
      while (
        originalIndex < originalLength &&
        modifiedIndex < modifiedLength &&
        originalLines[originalIndex] === modifiedLines[modifiedIndex]
      ) {
        if (++steps > maximumSteps) return undefined;
        originalIndex += 1;
        modifiedIndex += 1;
      }
      next[vectorIndex] = originalIndex;
      if (originalIndex >= originalLength && modifiedIndex >= modifiedLength) {
        trace.push(captureVector(next, offset, distance));
        return backtrack(trace, originalLines, modifiedLines);
      }
    }
    trace.push(captureVector(next, offset, distance));
    vector = next;
  }
  throw new Error("Line diff did not find an edit path");
}

function backtrack(trace: readonly DiffVectorSnapshot[], originalLines: readonly string[], modifiedLines: readonly string[]): readonly Edit[] {
  let originalIndex = originalLines.length;
  let modifiedIndex = modifiedLines.length;
  const result: Edit[] = [];
  for (let distance = trace.length - 1; distance > 0; distance -= 1) {
    const previous = trace[distance - 1]!;
    const diagonal = originalIndex - modifiedIndex;
    const previousDiagonal = diagonal === -distance || (diagonal !== distance && vectorValue(previous, diagonal - 1) < vectorValue(previous, diagonal + 1))
      ? diagonal + 1
      : diagonal - 1;
    const previousOriginalIndex = vectorValue(previous, previousDiagonal);
    const previousModifiedIndex = previousOriginalIndex - previousDiagonal;
    while (originalIndex > previousOriginalIndex && modifiedIndex > previousModifiedIndex) {
      originalIndex -= 1;
      modifiedIndex -= 1;
      result.push({ kind: "equal", originalLineIndex: originalIndex, modifiedLineIndex: modifiedIndex });
    }
    if (originalIndex === previousOriginalIndex) {
      modifiedIndex -= 1;
      result.push({ kind: "added", modifiedLineIndex: modifiedIndex });
    } else {
      originalIndex -= 1;
      result.push({ kind: "removed", originalLineIndex: originalIndex });
    }
  }
  while (originalIndex > 0 && modifiedIndex > 0) {
    originalIndex -= 1;
    modifiedIndex -= 1;
    result.push({ kind: "equal", originalLineIndex: originalIndex, modifiedLineIndex: modifiedIndex });
  }
  while (originalIndex > 0) {
    originalIndex -= 1;
    result.push({ kind: "removed", originalLineIndex: originalIndex });
  }
  while (modifiedIndex > 0) {
    modifiedIndex -= 1;
    result.push({ kind: "added", modifiedLineIndex: modifiedIndex });
  }
  result.reverse();
  return result;
}

interface DiffVectorSnapshot {
  readonly distance: number;
  readonly values: Int32Array;
}

function captureVector(vector: Int32Array, offset: number, distance: number): DiffVectorSnapshot {
  return Object.freeze({
    distance,
    values: vector.slice(offset - distance - 1, offset + distance + 2),
  });
}

function vectorValue(snapshot: DiffVectorSnapshot, diagonal: number): number {
  const value = snapshot.values[diagonal + snapshot.distance + 1];
  return value === undefined ? -1 : value;
}

function createRows(edits: readonly Edit[], originalLines: readonly string[], modifiedLines: readonly string[]): readonly LineDiffRow[] {
  const rows: LineDiffRow[] = [];
  for (let index = 0; index < edits.length;) {
    const edit = edits[index]!;
    if (edit.kind === "equal") {
      rows.push(row(LineDiffKind.Unchanged, edit.originalLineIndex, edit.modifiedLineIndex));
      index += 1;
      continue;
    }
    const removals: RemovedEdit[] = [];
    const additions: AddedEdit[] = [];
    while (index < edits.length && edits[index]!.kind !== "equal") {
      const changed = edits[index]!;
      if (changed.kind === "removed") removals.push(changed);
      else additions.push(changed as AddedEdit);
      index += 1;
    }
    const alignedCount = Math.min(removals.length, additions.length);
    for (let changedIndex = 0; changedIndex < alignedCount; changedIndex += 1) {
      const removed = removals[changedIndex]!;
      const added = additions[changedIndex]!;
      const ranges = inlineDiffRanges(originalLines[removed.originalLineIndex]!, modifiedLines[added.modifiedLineIndex]!);
      rows.push(row(LineDiffKind.Modified, removed.originalLineIndex, added.modifiedLineIndex, ranges.original, ranges.modified));
    }
    for (let changedIndex = alignedCount; changedIndex < removals.length; changedIndex += 1) {
      rows.push(row(LineDiffKind.Removed, removals[changedIndex]!.originalLineIndex));
    }
    for (let changedIndex = alignedCount; changedIndex < additions.length; changedIndex += 1) {
      rows.push(row(LineDiffKind.Added, undefined, additions[changedIndex]!.modifiedLineIndex));
    }
  }
  return rows;
}

function coarseDiff(originalLines: readonly string[], modifiedLines: readonly string[]): LineDiff {
  const rows: LineDiffRow[] = [];
  const alignedCount = Math.min(originalLines.length, modifiedLines.length);
  for (let index = 0; index < alignedCount; index += 1) {
    const ranges = inlineDiffRanges(originalLines[index]!, modifiedLines[index]!);
    rows.push(row(LineDiffKind.Modified, index, index, ranges.original, ranges.modified));
  }
  for (let index = alignedCount; index < originalLines.length; index += 1) {
    rows.push(row(LineDiffKind.Removed, index));
  }
  for (let index = alignedCount; index < modifiedLines.length; index += 1) {
    rows.push(row(LineDiffKind.Added, undefined, index));
  }
  return Object.freeze({ rows: Object.freeze(rows), approximate: true });
}

function row(kind: LineDiffKind, originalLineIndex?: number, modifiedLineIndex?: number, originalChanges: readonly DiffRange[] = [], modifiedChanges: readonly DiffRange[] = []): LineDiffRow {
  return Object.freeze({
    kind,
    ...(originalLineIndex === undefined ? {} : { originalLineIndex }),
    ...(modifiedLineIndex === undefined ? {} : { modifiedLineIndex }),
    originalChanges: Object.freeze(originalChanges.map(range => Object.freeze(range))),
    modifiedChanges: Object.freeze(modifiedChanges.map(range => Object.freeze(range))),
  });
}

function inlineDiffRanges(original: string, modified: string): { readonly original: readonly DiffRange[]; readonly modified: readonly DiffRange[] } {
  const originalBoundaries = getTextGraphemeBoundaries(original);
  const modifiedBoundaries = getTextGraphemeBoundaries(modified);
  let commonPrefixLength = 0;
  const maximumPrefix = Math.min(originalBoundaries.length, modifiedBoundaries.length) - 1;
  while (
    commonPrefixLength < maximumPrefix &&
    graphemeAt(original, originalBoundaries, commonPrefixLength) === graphemeAt(modified, modifiedBoundaries, commonPrefixLength)
  ) commonPrefixLength += 1;
  let commonSuffixLength = 0;
  const maximumSuffix = Math.min(originalBoundaries.length - 1 - commonPrefixLength, modifiedBoundaries.length - 1 - commonPrefixLength);
  while (
    commonSuffixLength < maximumSuffix &&
    graphemeAt(original, originalBoundaries, originalBoundaries.length - 2 - commonSuffixLength) === graphemeAt(modified, modifiedBoundaries, modifiedBoundaries.length - 2 - commonSuffixLength)
  ) commonSuffixLength += 1;
  const originalStart = originalBoundaries[commonPrefixLength]!;
  const originalEnd = originalBoundaries[originalBoundaries.length - 1 - commonSuffixLength]!;
  const modifiedStart = modifiedBoundaries[commonPrefixLength]!;
  const modifiedEnd = modifiedBoundaries[modifiedBoundaries.length - 1 - commonSuffixLength]!;
  return Object.freeze({
    original: originalStart === originalEnd ? [] : [{ startColumn: originalStart, endColumn: originalEnd }],
    modified: modifiedStart === modifiedEnd ? [] : [{ startColumn: modifiedStart, endColumn: modifiedEnd }],
  });
}

function graphemeAt(text: string, boundaries: readonly number[], index: number): string {
  return text.slice(boundaries[index]!, boundaries[index + 1]!);
}

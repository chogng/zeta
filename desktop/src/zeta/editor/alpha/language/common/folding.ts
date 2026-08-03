import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { TextPosition, TextRange } from "../../common/text.js";
import { type TextModel } from "../../common/textModel.js";
import { TrackedRangeStickiness, type TrackedRange } from "../../common/trackedRange.js";

/** Identifies whether a folding range was manually created or supplied by a language provider. */
export enum EditorFoldingRangeSource {
  Manual = "manual",
  Provider = "provider",
}

/** One inclusive physical-line range that can be collapsed without changing document text. */
export interface EditorFoldingRange {
  readonly startLineIndex: number;
  readonly endLineIndex: number;
  readonly collapsed?: boolean;
  readonly source?: EditorFoldingRangeSource;
}

/** One current folding range whose line boundaries follow edits in its owning TextModel. */
export interface EditorFoldingRegion {
  readonly startLineIndex: number;
  readonly endLineIndex: number;
  readonly collapsed: boolean;
  readonly source: EditorFoldingRangeSource;
}

interface EditorFoldingRegionRecord {
  readonly range: TrackedRange;
  collapsed: boolean;
  readonly source: EditorFoldingRangeSource;
}

/**
 * Owns one editor's fold state independently from text and browser presentation.
 *
 * Ranges include their header and final physical lines. Collapsing a range hides
 * only the lines after its header. Callers replace provider ranges when language
 * analysis changes; the model retains manual ranges until their tracked text span
 * is deleted.
 */
export class EditorFoldingModel extends DisposableOwner {
  private readonly changeEmitter = this.own(new Emitter<void>());
  private records: readonly EditorFoldingRegionRecord[] = Object.freeze([]);

  readonly onDidChange: Event<void> = this.changeEmitter.event;

  constructor(private readonly textModel: TextModel) {
    super();
    this.defer(() => disposeRecords(this.records));
    this.own(textModel.onDidChange(() => this.reconcileTrackedRanges()));
  }

  get model(): TextModel {
    return this.textModel;
  }

  /** Returns ordered, immutable current regions. */
  get regions(): readonly EditorFoldingRegion[] {
    return Object.freeze(this.records.map(record => this.toRegion(record)));
  }

  /** Replaces all known ranges after validating non-crossing physical-line spans. */
  setRanges(ranges: readonly EditorFoldingRange[]): void {
    const normalized = normalizeRanges(this.textModel, ranges);
    const next = normalized.map(range => ({
      range: this.textModel.trackRange(
        TextRange.from(
          TextPosition.at(range.startLineIndex, 0),
          TextPosition.at(range.endLineIndex, this.textModel.getLineContent(range.endLineIndex).length),
        ),
        TrackedRangeStickiness.NeverGrowsAtEdges,
      ),
      collapsed: range.collapsed,
      source: range.source,
    }));
    const previous = this.records;
    this.records = Object.freeze(next);
    disposeRecords(previous);
    this.changeEmitter.fire();
  }

  /** Replaces provider ranges while retaining manual ranges and matching collapsed state. */
  setProviderRanges(ranges: readonly EditorFoldingRange[]): void {
    if (!Array.isArray(ranges)) throw new TypeError("Folding ranges must be an array");
    const current = this.regions;
    const collapsedProviders = new Map(current
      .filter(region => region.source === EditorFoldingRangeSource.Provider)
      .map(region => [rangeKey(region), region.collapsed]));
    const manualRanges = current
      .filter(region => region.source === EditorFoldingRangeSource.Manual)
      .map(region => ({ ...region }));
    const providerRanges = ranges.map(range => ({
      ...range,
      collapsed: collapsedProviders.get(rangeKey(range)) ?? range.collapsed ?? false,
      source: EditorFoldingRangeSource.Provider,
    }));
    this.setRanges([...manualRanges, ...providerRanges]);
  }

  /** Toggles the innermost fold whose header is exactly `lineIndex`. */
  toggleAtLine(lineIndex: number): EditorFoldingRegion | undefined {
    validateLineIndex(this.textModel, lineIndex);
    const record = this.findRecord(candidate => candidate.range.range.start.lineIndex === lineIndex);
    if (!record) return undefined;
    record.collapsed = !record.collapsed;
    const region = this.toRegion(record);
    this.changeEmitter.fire();
    return region;
  }

  /** Toggles the innermost fold containing `lineIndex`, as used by editor fold chords. */
  toggleContainingLine(lineIndex: number): EditorFoldingRegion | undefined {
    validateLineIndex(this.textModel, lineIndex);
    const record = this.findRecord(candidate => {
        const range = candidate.range.range;
        return lineIndex >= range.start.lineIndex && lineIndex <= range.end.lineIndex;
      });
    if (!record) return undefined;
    record.collapsed = !record.collapsed;
    const region = this.toRegion(record);
    this.changeEmitter.fire();
    return region;
  }

  /** Sets the innermost fold containing `lineIndex` to a specific collapse state. */
  setContainingLineCollapsed(lineIndex: number, collapsed: boolean): EditorFoldingRegion | undefined {
    validateLineIndex(this.textModel, lineIndex);
    if (typeof collapsed !== "boolean") throw new TypeError("Folding collapse state must be boolean");
    const record = this.findRecord(candidate => {
      const range = candidate.range.range;
      return lineIndex >= range.start.lineIndex && lineIndex <= range.end.lineIndex;
    });
    if (!record) return undefined;
    if (record.collapsed === collapsed) return this.toRegion(record);
    record.collapsed = collapsed;
    const region = this.toRegion(record);
    this.changeEmitter.fire();
    return region;
  }

  /** Collapses the innermost containing fold together with every nested descendant. */
  collapseContainingRegionRecursively(lineIndex: number): EditorFoldingRegion | undefined {
    return this.setContainingRegionRecursively(lineIndex, true);
  }

  /** Expands the innermost containing fold together with every nested descendant. */
  expandContainingRegionRecursively(lineIndex: number): EditorFoldingRegion | undefined {
    return this.setContainingRegionRecursively(lineIndex, false);
  }

  /** Creates one persistent manual range when it is nested within or disjoint from existing folds. */
  addManualRange(startLineIndex: number, endLineIndex: number): EditorFoldingRegion | undefined {
    validateLineIndex(this.textModel, startLineIndex);
    validateLineIndex(this.textModel, endLineIndex);
    if (endLineIndex <= startLineIndex || !this.canAddManualRange(startLineIndex, endLineIndex)) return undefined;
    const existingManual = this.records.find(record => {
      const range = record.range.range;
      return record.source === EditorFoldingRangeSource.Manual && range.start.lineIndex === startLineIndex && range.end.lineIndex === endLineIndex;
    });
    if (existingManual) return this.toRegion(existingManual);
    const ranges = this.regions
      .filter(region => region.startLineIndex !== startLineIndex || region.endLineIndex !== endLineIndex)
      .map(region => ({ ...region }));
    ranges.push({ startLineIndex, endLineIndex, collapsed: false, source: EditorFoldingRangeSource.Manual });
    this.setRanges(ranges);
    return this.regions.find(region => region.source === EditorFoldingRangeSource.Manual && region.startLineIndex === startLineIndex && region.endLineIndex === endLineIndex);
  }

  /** Removes the innermost manual fold containing `lineIndex`. */
  removeContainingManualRange(lineIndex: number): EditorFoldingRegion | undefined {
    validateLineIndex(this.textModel, lineIndex);
    const target = this.findRecord(candidate => {
      const range = candidate.range.range;
      return candidate.source === EditorFoldingRangeSource.Manual && lineIndex >= range.start.lineIndex && lineIndex <= range.end.lineIndex;
    });
    if (!target) return undefined;
    const region = this.toRegion(target);
    this.setRanges(this.records.filter(record => record !== target).map(record => this.toRegion(record)));
    return region;
  }

  /** Sets the collapse state of every current range. */
  setAllCollapsed(collapsed: boolean): boolean {
    if (typeof collapsed !== "boolean") throw new TypeError("Folding collapse state must be boolean");
    let changed = false;
    for (const record of this.records) {
      if (record.collapsed === collapsed) continue;
      record.collapsed = collapsed;
      changed = true;
    }
    if (changed) this.changeEmitter.fire();
    return changed;
  }

  /** Collapses every fold at or below one one-based nesting level and expands shallower levels. */
  collapseToLevel(level: number): boolean {
    if (!Number.isSafeInteger(level) || level < 1) throw new RangeError("Folding level must be a positive safe integer");
    let changed = false;
    for (const record of this.records) {
      const collapsed = this.getNestingDepth(record) >= level;
      if (record.collapsed === collapsed) continue;
      record.collapsed = collapsed;
      changed = true;
    }
    if (changed) this.changeEmitter.fire();
    return changed;
  }

  /** Returns whether a physical line is suppressed by a collapsed ancestor range. */
  isLineHidden(lineIndex: number): boolean {
    validateLineIndex(this.textModel, lineIndex);
    return this.records.some(record => {
      const range = record.range.range;
      return record.collapsed && lineIndex > range.start.lineIndex && lineIndex <= range.end.lineIndex;
    });
  }

  /** Returns current physical lines that remain visible after all collapsed ranges apply. */
  getVisibleLineIndexes(): readonly number[] {
    const visible: number[] = [];
    for (let lineIndex = 0; lineIndex < this.textModel.lineCount; lineIndex += 1) {
      if (!this.isLineHidden(lineIndex)) visible.push(lineIndex);
    }
    return Object.freeze(visible);
  }

  private reconcileTrackedRanges(): void {
    const retained = this.records.filter(record => {
      const range = record.range.range;
      return range.start.lineIndex < range.end.lineIndex;
    });
    const normalized = normalizeRecords(retained);
    if (recordsEqual(this.records, normalized)) return;
    const removed = this.records.filter(record => !normalized.includes(record));
    this.records = Object.freeze(normalized);
    disposeRecords(removed);
    this.changeEmitter.fire();
  }

  private setContainingRegionRecursively(lineIndex: number, collapsed: boolean): EditorFoldingRegion | undefined {
    validateLineIndex(this.textModel, lineIndex);
    const target = this.findRecord(candidate => {
      const range = candidate.range.range;
      return lineIndex >= range.start.lineIndex && lineIndex <= range.end.lineIndex;
    });
    if (!target) return undefined;
    const targetRange = target.range.range;
    let changed = false;
    for (const candidate of this.records) {
      const range = candidate.range.range;
      if (range.start.lineIndex < targetRange.start.lineIndex || range.end.lineIndex > targetRange.end.lineIndex) continue;
      if (candidate.collapsed === collapsed) continue;
      candidate.collapsed = collapsed;
      changed = true;
    }
    if (changed) this.changeEmitter.fire();
    return this.toRegion(target);
  }

  private canAddManualRange(startLineIndex: number, endLineIndex: number): boolean {
    return this.records.every(record => {
      const range = record.range.range;
      const disjoint = endLineIndex < range.start.lineIndex || startLineIndex > range.end.lineIndex;
      const contains = startLineIndex <= range.start.lineIndex && endLineIndex >= range.end.lineIndex;
      const containedBy = startLineIndex >= range.start.lineIndex && endLineIndex <= range.end.lineIndex;
      return disjoint || contains || containedBy;
    });
  }

  private getNestingDepth(record: EditorFoldingRegionRecord): number {
    const range = record.range.range;
    return 1 + this.records.filter(candidate => {
      if (candidate === record) return false;
      const candidateRange = candidate.range.range;
      return candidateRange.start.lineIndex <= range.start.lineIndex && candidateRange.end.lineIndex >= range.end.lineIndex;
    }).length;
  }

  private toRegion(record: EditorFoldingRegionRecord): EditorFoldingRegion {
    const range = record.range.range;
    return Object.freeze({
      startLineIndex: range.start.lineIndex,
      endLineIndex: range.end.lineIndex,
      collapsed: record.collapsed,
      source: record.source,
    });
  }

  private findRecord(predicate: (candidate: EditorFoldingRegionRecord) => boolean): EditorFoldingRegionRecord | undefined {
    return this.records
      .filter(predicate)
      .sort((left, right) => {
        const leftRange = left.range.range;
        const rightRange = right.range.range;
        return leftRange.end.lineIndex - leftRange.start.lineIndex - (rightRange.end.lineIndex - rightRange.start.lineIndex);
      })[0];
  }
}

function normalizeRanges(model: TextModel, ranges: readonly EditorFoldingRange[]): readonly Required<EditorFoldingRange>[] {
  if (!Array.isArray(ranges)) throw new TypeError("Folding ranges must be an array");
  const normalized = ranges.map(range => {
    if (!range || typeof range !== "object") throw new TypeError("Folding range must be an object");
    validateLineIndex(model, range.startLineIndex);
    validateLineIndex(model, range.endLineIndex);
    if (range.endLineIndex <= range.startLineIndex) throw new RangeError("Folding ranges must span at least two lines");
    if (range.collapsed !== undefined && typeof range.collapsed !== "boolean") throw new TypeError("Folding range collapse state must be boolean");
    if (range.source !== undefined && !Object.values(EditorFoldingRangeSource).includes(range.source)) {
      throw new TypeError("Unknown folding range source");
    }
    return Object.freeze({
      startLineIndex: range.startLineIndex,
      endLineIndex: range.endLineIndex,
      collapsed: range.collapsed ?? false,
      source: range.source ?? EditorFoldingRangeSource.Provider,
    });
  });
  normalized.sort((left, right) => left.startLineIndex - right.startLineIndex || right.endLineIndex - left.endLineIndex);
  for (let index = 1; index < normalized.length; index += 1) {
    const previous = normalized[index - 1]!;
    const current = normalized[index]!;
    if (current.startLineIndex <= previous.endLineIndex && current.endLineIndex > previous.endLineIndex) {
      throw new RangeError("Folding ranges must be nested or disjoint");
    }
  }
  return Object.freeze(deduplicateRanges(normalized));
}

function normalizeRecords(records: readonly EditorFoldingRegionRecord[]): readonly EditorFoldingRegionRecord[] {
  const sorted = [...records].sort((left, right) => {
    const leftRange = left.range.range;
    const rightRange = right.range.range;
    return leftRange.start.lineIndex - rightRange.start.lineIndex || rightRange.end.lineIndex - leftRange.end.lineIndex;
  });
  const normalized: EditorFoldingRegionRecord[] = [];
  for (const record of sorted) {
    const previous = normalized.at(-1);
    if (previous) {
      const previousRange = previous.range.range;
      const range = record.range.range;
      if (range.start.lineIndex <= previousRange.end.lineIndex && range.end.lineIndex > previousRange.end.lineIndex) {
        record.range.dispose();
        continue;
      }
      if (range.start.lineIndex === previousRange.start.lineIndex && range.end.lineIndex === previousRange.end.lineIndex) {
        record.range.dispose();
        continue;
      }
    }
    normalized.push(record);
  }
  return normalized;
}

function deduplicateRanges(ranges: readonly Required<EditorFoldingRange>[]): readonly Required<EditorFoldingRange>[] {
  return ranges.filter((range, index) => {
    const previous = ranges[index - 1];
    return !previous || range.startLineIndex !== previous.startLineIndex || range.endLineIndex !== previous.endLineIndex;
  });
}

function recordsEqual(left: readonly EditorFoldingRegionRecord[], right: readonly EditorFoldingRegionRecord[]): boolean {
  return left.length === right.length && left.every((record, index) => record === right[index]);
}

function disposeRecords(records: readonly EditorFoldingRegionRecord[]): void {
  for (const record of records) record.range.dispose();
}

function rangeKey(range: Pick<EditorFoldingRange, "startLineIndex" | "endLineIndex">): string {
  return `${range.startLineIndex}:${range.endLineIndex}`;
}

function validateLineIndex(model: TextModel, lineIndex: number): void {
  if (!Number.isSafeInteger(lineIndex) || lineIndex < 0 || lineIndex >= model.lineCount) {
    throw new RangeError("Folding line index is outside the text model");
  }
}

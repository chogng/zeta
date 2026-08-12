import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner, DisposableSlot, type IDisposable } from "../../../base/common/lifecycle.js";
import { type TextModel } from "../../common/model/textModel.js";
import { EditorVisualLineProjection } from "../../common/viewModel/modelLineProjection.js";
import { getTextGraphemeBoundaries } from "../../common/core/textSegmentation.js";
import { type EditorViewportLineSource } from "../../common/viewLayout/editorViewportModel.js";
import { type TextMeasurer } from "./fontMetrics.js";

export enum EditorLineWrapping {
  Off = "off",
  On = "on",
}

export interface VisualLineProjectionOptions {
  readonly wrapping?: EditorLineWrapping;
  readonly wrapWidth?: number;
  /**
   * Defers expensive initial soft-wrap measurement while preserving a usable
   * one-row-per-logical-line projection until the complete result is ready.
   */
  readonly initialWrappingMeasurement?: VisualLineInitialMeasurementOptions;
}

/** Schedules a later, cancellable slice of initial soft-wrap measurement. */
export type VisualLineMeasurementScheduler = (callback: () => void) => IDisposable;

/** Controls non-blocking initial measurement for a large wrapped document. */
export interface VisualLineInitialMeasurementOptions {
  readonly initialLineCount?: number;
  readonly linesPerSlice?: number;
  readonly schedule: VisualLineMeasurementScheduler;
}

interface ResolvedInitialMeasurement {
  readonly initialLineCount: number;
  readonly linesPerSlice: number;
  readonly schedule: VisualLineMeasurementScheduler;
}

/** Browser-measured, DOM-free visual-line projection for one Aster TextModel. */
export class VisualLineProjection extends DisposableOwner {
  private readonly changeEmitter = this.own(new Emitter<void>());
  private readonly lineCountChangeEmitter = this.own(new Emitter<void>());
  private wrapping: EditorLineWrapping;
  private wrapWidth: number;
  private readonly initialMeasurement: ResolvedInitialMeasurement | undefined;
  private readonly pendingMeasurement = this.own(new DisposableSlot<IDisposable>());
  private projectionRevision = 0;
  private currentProjection: EditorVisualLineProjection;
  private pendingBreakColumns: number[][] | undefined;
  private nextLineIndex = 0;
  private scanVersion = -1;

  readonly onDidChange: Event<void> = this.changeEmitter.event;
  readonly onDidChangeLineCount: Event<void> = this.lineCountChangeEmitter.event;
  readonly lineSource: EditorViewportLineSource;

  constructor(
    private readonly model: TextModel,
    private readonly textMeasurer: TextMeasurer,
    options: VisualLineProjectionOptions = {},
  ) {
    super();
    this.wrapping = readWrapping(options.wrapping);
    this.wrapWidth = readWrapWidth(options.wrapWidth);
    this.initialMeasurement = readInitialMeasurement(options.initialWrappingMeasurement);
    this.currentProjection = this.usesInitialMeasurement()
      ? this.createUnwrappedProjection()
      : this.createProjection();
    const projection = this;
    this.lineSource = Object.freeze({
      get lineCount(): number {
        return projection.currentProjection.visualLineCount;
      },
      onDidChange: this.onDidChangeLineCount,
    });
    if (this.usesInitialMeasurement()) this.startInitialMeasurement();
    this.own(model.onDidChange(() => this.refresh()));
  }

  get textModel(): TextModel {
    return this.model;
  }

  get projection(): EditorVisualLineProjection {
    return this.currentProjection;
  }

  get lineCount(): number {
    return this.currentProjection.visualLineCount;
  }

  get revision(): number {
    return this.projectionRevision;
  }

  /** Whether the current soft-wrap projection includes every current model line. */
  get complete(): boolean {
    return !this.pendingBreakColumns || this.nextLineIndex >= this.model.lineCount;
  }

  ensureCurrent(): EditorVisualLineProjection {
    if (this.currentProjection.modelVersion !== this.model.version) this.refresh();
    return this.currentProjection;
  }

  setWrapping(wrapping: EditorLineWrapping): void {
    const next = readWrapping(wrapping);
    if (next === this.wrapping) return;
    this.wrapping = next;
    this.refresh();
  }

  setWrapWidth(width: number): void {
    const next = readWrapWidth(width);
    if (next === this.wrapWidth) return;
    this.wrapWidth = next;
    this.refresh();
  }

  refresh(): void {
    if (this.usesInitialMeasurement()) this.startInitialMeasurement();
    else this.rebuild();
  }

  private rebuild(): void {
    this.pendingMeasurement.clear();
    this.pendingBreakColumns = undefined;
    this.nextLineIndex = this.model.lineCount;
    this.scanVersion = this.model.version;
    this.replaceProjection(this.createProjection());
  }

  private startInitialMeasurement(): void {
    const options = this.initialMeasurement;
    if (!options) return;
    this.pendingMeasurement.clear();
    this.scanVersion = this.model.version;
    this.nextLineIndex = 0;
    this.pendingBreakColumns = Array.from(
      { length: this.model.lineCount },
      (_, lineIndex) => [this.model.getLineContent(lineIndex).length],
    );
    this.measureNextSlice(options.initialLineCount);
    this.replaceProjection(this.createProjectionFromPendingBreaks());
    this.scheduleNextSlice();
  }

  private scheduleNextSlice(): void {
    const options = this.initialMeasurement;
    if (!options || this.complete) return;
    this.pendingMeasurement.replace(options.schedule(() => {
      this.pendingMeasurement.clear();
      if (this.scanVersion !== this.model.version) {
        this.startInitialMeasurement();
        return;
      }
      this.measureNextSlice(options.linesPerSlice);
      if (this.complete) this.replaceProjection(this.createProjectionFromPendingBreaks());
      this.scheduleNextSlice();
    }));
  }

  private measureNextSlice(lineCount: number): void {
    const breaks = this.pendingBreakColumns;
    if (!breaks) return;
    const endLineIndex = Math.min(this.model.lineCount, this.nextLineIndex + lineCount);
    for (; this.nextLineIndex < endLineIndex; this.nextLineIndex += 1) {
      breaks[this.nextLineIndex] = this.breakColumnsForLine(
        this.model.getLineContent(this.nextLineIndex),
      );
    }
  }

  private replaceProjection(next: EditorVisualLineProjection): void {
    const previousLineCount = this.currentProjection.visualLineCount;
    this.currentProjection = next;
    this.projectionRevision += 1;
    if (this.currentProjection.visualLineCount !== previousLineCount) {
      this.lineCountChangeEmitter.fire();
    }
    this.changeEmitter.fire();
  }

  private createProjection(): EditorVisualLineProjection {
    const breakColumnsByLine: number[][] = [];
    for (let lineIndex = 0; lineIndex < this.model.lineCount; lineIndex += 1) {
      const text = this.model.getLineContent(lineIndex);
      breakColumnsByLine.push(this.breakColumnsForLine(text));
    }
    return EditorVisualLineProjection.fromBreakColumns(this.model, breakColumnsByLine);
  }

  private createUnwrappedProjection(): EditorVisualLineProjection {
    return EditorVisualLineProjection.fromBreakColumns(
      this.model,
      Array.from(
        { length: this.model.lineCount },
        (_, lineIndex) => [this.model.getLineContent(lineIndex).length],
      ),
    );
  }

  private createProjectionFromPendingBreaks(): EditorVisualLineProjection {
    const breaks = this.pendingBreakColumns;
    if (!breaks) throw new Error("Aster visual-line measurement is not active");
    return EditorVisualLineProjection.fromBreakColumns(this.model, breaks);
  }

  private usesInitialMeasurement(): boolean {
    return this.initialMeasurement !== undefined &&
      this.wrapping === EditorLineWrapping.On &&
      this.wrapWidth > 0;
  }

  private breakColumnsForLine(text: string): number[] {
    if (this.wrapping === EditorLineWrapping.Off || this.wrapWidth === 0 || text.length === 0) {
      return [text.length];
    }
    const breaks: number[] = [];
    const boundaries = getTextGraphemeBoundaries(text);
    let startColumn = 0;
    let previousColumn = 0;
    for (let index = 1; index < boundaries.length; index += 1) {
      const column = boundaries[index]!;
      const width = this.textMeasurer.measureLineWidth(text.slice(startColumn, column));
      if (!Number.isFinite(width) || width < 0) {
        throw new RangeError("Aster wrapped line measurement must be finite and non-negative");
      }
      if (width > this.wrapWidth && previousColumn > startColumn) {
        breaks.push(previousColumn);
        startColumn = previousColumn;
      }
      previousColumn = column;
    }
    breaks.push(text.length);
    return breaks;
  }
}

function readWrapping(value: EditorLineWrapping | undefined): EditorLineWrapping {
  const wrapping = value ?? EditorLineWrapping.Off;
  if (!Object.values(EditorLineWrapping).includes(wrapping)) {
    throw new TypeError("Unknown Aster editor line wrapping mode");
  }
  return wrapping;
}

function readWrapWidth(value: number | undefined): number {
  const width = value ?? 0;
  if (!Number.isFinite(width) || width < 0) {
    throw new RangeError("Aster editor wrap width must be finite and non-negative");
  }
  return width;
}

function readInitialMeasurement(value: VisualLineInitialMeasurementOptions | undefined): ResolvedInitialMeasurement | undefined {
  if (value === undefined) return undefined;
  if (!value || typeof value.schedule !== "function") {
    throw new TypeError("Aster initial visual-line measurement requires a scheduler");
  }
  const initialLineCount = value.initialLineCount ?? 512;
  const linesPerSlice = value.linesPerSlice ?? initialLineCount;
  if (!Number.isSafeInteger(initialLineCount) || initialLineCount <= 0) {
    throw new RangeError("Aster initial visual-line measurement count must be a positive safe integer");
  }
  if (!Number.isSafeInteger(linesPerSlice) || linesPerSlice <= 0) {
    throw new RangeError("Aster visual-line measurement slice size must be a positive safe integer");
  }
  return Object.freeze({ initialLineCount, linesPerSlice, schedule: value.schedule });
}

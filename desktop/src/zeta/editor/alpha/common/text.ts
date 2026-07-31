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

  compareTo(other: TextPosition): number {
    return this.lineIndex - other.lineIndex ||
      this.columnIndex - other.columnIndex;
  }
}

/**
 * An ordered, end-exclusive text range.
 */
export class TextRange {
  private constructor(
    readonly start: TextPosition,
    readonly end: TextPosition,
  ) {
    Object.freeze(this);
  }

  static from(start: TextPosition, end: TextPosition): TextRange {
    if (start.compareTo(end) > 0) {
      throw new RangeError("TextRange end must not precede its start");
    }
    return new TextRange(start, end);
  }

  static emptyAt(position: TextPosition): TextRange {
    return new TextRange(position, position);
  }

  get empty(): boolean {
    return this.start.compareTo(this.end) === 0;
  }
}

/** One replacement in the pre-transaction document. */
export interface TextEdit {
  readonly range: TextRange;
  readonly text: string;
}

/**
 * Nominal identity used only while consecutive compatible edits may share one
 * undo step.
 */
export class TextEditHistoryGroup {
  private readonly identity = undefined;

  private constructor() {
    Object.freeze(this);
  }

  static create(): TextEditHistoryGroup {
    return new TextEditHistoryGroup();
  }
}

/** Selects how a compatible edit updates the latest grouped undo step. */
export enum TextEditHistoryMergeMode {
  Sequential = "sequential",
  ReplacePrevious = "replacePrevious",
}

/** The operation that committed one text-model version. */
export enum TextModelChangeReason {
  Edit = "edit",
  Undo = "undo",
  Redo = "redo",
  HistoryCancellation = "historyCancellation",
}

/** One normalized replacement reported after a transaction commits. */
export interface TextModelContentChange {
  readonly range: TextRange;
  readonly rangeOffset: number;
  readonly rangeLength: number;
  readonly text: string;
}

/**
 * Immutable description of one committed text-model transaction.
 *
 * `transactionId` identifies one undo step and remains stable across grouped
 * edits and their undo/redo commits. `version` identifies each commit.
 */
export interface TextModelChange {
  readonly version: number;
  readonly transactionId: number;
  readonly reason: TextModelChangeReason;
  readonly changes: readonly TextModelContentChange[];
}

/**
 * An immutable, versioned view of normalized model text.
 *
 * Offset ranges are end-exclusive and use UTF-16 code units. A snapshot
 * remains readable after later model edits or disposal.
 */
export interface TextSnapshot {
  readonly version: number;
  readonly length: number;
  readonly lineCount: number;
  getText(): string;
  getTextBetweenOffsets(startOffset: number, endOffset: number): string;
}

export function normalizeTextLineEndings(text: string): string {
  return text.replace(/\r\n?|\u2028|\u2029/g, "\n");
}

function assertIndex(value: number, name: string): void {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new RangeError(`${name} must be a non-negative safe integer`);
  }
}

import { PieceTreeTextBuffer } from "./pieceTreeTextBuffer.js";
import { normalizeTextLineEndings, type TextSnapshot } from "./text.js";

export interface LanguageWorkerDocumentChange {
  readonly rangeOffset: number;
  readonly rangeLength: number;
  readonly text: string;
}

export interface LanguageWorkerDocumentSynchronization {
  readonly previousVersion: number;
  readonly modelVersion: number;
  readonly changes: readonly LanguageWorkerDocumentChange[];
  readonly snapshot: TextSnapshot;
}

export interface LanguageWorkerDocumentSynchronizationObserver {
  synchronizeDocument(synchronization: LanguageWorkerDocumentSynchronization): void;
}

/** Single-document Piece Tree mirror owned by one language-worker server. */
export class LanguageWorkerDocumentMirror {
  private readonly buffer: PieceTreeTextBuffer;
  private _version: number;

  constructor(snapshot: TextSnapshot) {
    assertPositiveSafeInteger(snapshot.version, "Language worker mirror version");
    const text = snapshot.getText();
    if (text.length !== snapshot.length || countLines(text) !== snapshot.lineCount || normalizeTextLineEndings(text) !== text) {
      throw new Error("Language worker mirror snapshot metadata is inconsistent");
    }
    this._version = snapshot.version;
    this.buffer = new PieceTreeTextBuffer(text);
  }

  get version(): number {
    return this._version;
  }

  get length(): number {
    return this.buffer.length;
  }

  get lineCount(): number {
    return this.buffer.lineCount;
  }

  createSnapshot(): TextSnapshot {
    const version = this._version;
    const snapshot = this.buffer.createSnapshot();
    return Object.freeze({
      version,
      length: snapshot.length,
      lineCount: snapshot.lineCount,
      getText: () => snapshot.getText(),
      getTextBetweenOffsets: (startOffset: number, endOffset: number) => snapshot.getTextBetweenOffsets(startOffset, endOffset),
    });
  }

  synchronize(previousVersion: number, modelVersion: number, changes: readonly LanguageWorkerDocumentChange[]): void {
    if (previousVersion !== this._version || modelVersion !== this._version + 1) {
      throw new Error("Language worker sync version does not follow its document mirror");
    }
    if (!Array.isArray(changes) || changes.length === 0) {
      throw new RangeError("Language worker sync must contain changes");
    }
    let previousStart = -1;
    let previousEnd = 0;
    for (const change of changes) {
      assertNonNegativeSafeInteger(change.rangeOffset, "Language worker sync range offset");
      assertNonNegativeSafeInteger(change.rangeLength, "Language worker sync range length");
      if (typeof change.text !== "string" || normalizeTextLineEndings(change.text) !== change.text) {
        throw new TypeError("Language worker sync text must use normalized LF line endings");
      }
      const end = change.rangeOffset + change.rangeLength;
      const ambiguousSharedStart = change.rangeOffset === previousStart && (change.rangeLength === 0 || previousEnd === previousStart);
      if (change.rangeOffset < previousEnd || ambiguousSharedStart || end > this.buffer.length) {
        throw new RangeError("Language worker sync ranges must be ordered, non-overlapping, and inside the mirror");
      }
      previousStart = change.rangeOffset;
      previousEnd = end;
    }
    for (let index = changes.length - 1; index >= 0; index -= 1) {
      const change = changes[index]!;
      this.buffer.replace(change.rangeOffset, change.rangeOffset + change.rangeLength, change.text);
    }
    this._version = modelVersion;
  }
}

function assertPositiveSafeInteger(value: unknown, owner: string): asserts value is number {
  if (!Number.isSafeInteger(value) || (value as number) <= 0) {
    throw new RangeError(`${owner} must be a positive safe integer`);
  }
}

function assertNonNegativeSafeInteger(value: unknown, owner: string): asserts value is number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) {
    throw new RangeError(`${owner} must be a non-negative safe integer`);
  }
}

function countLines(text: string): number {
  let result = 1;
  for (let index = 0; index < text.length; index += 1) {
    if (text.charCodeAt(index) === 10) result += 1;
  }
  return result;
}

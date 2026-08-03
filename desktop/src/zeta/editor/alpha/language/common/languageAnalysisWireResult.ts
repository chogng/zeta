import { LANGUAGE_DIAGNOSTIC_LANE, LANGUAGE_TOKEN_LANE, type LanguageAnalysisLane, type LanguageAnalysisResult } from "./languageAnalysisService.js";
import { createLanguageAnalysisItemSplices, type LanguageAnalysisItem } from "./languageAnalysisItemDelta.js";
import { attachLanguageTokenResultDelta, createLanguageDiagnosticSnapshotNormalizer, createLanguageTokenSnapshotNormalizer, type LanguageDiagnostic, type LanguageDiagnosticCode, type LanguageDiagnosticResult, type LanguageToken, type LanguageTokenResult } from "./languageResults.js";
import { type LanguageWorkerWireResultState } from "./languageWorkerWireProtocol.js";
import { TextPosition, TextRange, type TextSnapshot } from "../../common/text.js";

export function encodeLanguageAnalysisWireResult(lane: LanguageAnalysisLane, result: LanguageAnalysisResult, snapshot: TextSnapshot, base: LanguageWorkerWireResultState<LanguageAnalysisResult> | undefined): unknown {
  assertResultLane(lane, result);
  const items = lane === LANGUAGE_TOKEN_LANE
    ? (result.value as LanguageTokenResult).tokens
    : (result.value as LanguageDiagnosticResult).diagnostics;
  const baseItems = readBaseItems(lane, base);
  if (!base || !baseItems) return encodeFull(lane, items);
  const splices = createLanguageAnalysisItemSplices(lane, baseItems, items, base.snapshot, snapshot);
  const insertedItemCount = splices.reduce((count, splice) => count + splice.items.length, 0);
  if (insertedItemCount >= items.length) return encodeFull(lane, items);
  return Object.freeze({
    kind: "delta",
    baseRequestId: base.requestId,
    splices: Object.freeze(splices.map(splice => Object.freeze({
      startItemIndex: splice.startItemIndex,
      deleteItemCount: splice.deleteItemCount,
      lineDelta: splice.lineDeltaAfter,
      items: Object.freeze(splice.items.map(item => encodeItem(lane, item))),
    }))),
  });
}

export function decodeLanguageAnalysisWireResult(lane: LanguageAnalysisLane, value: unknown, snapshot: TextSnapshot, base: LanguageWorkerWireResultState<LanguageAnalysisResult> | undefined): LanguageAnalysisResult {
  assertRecord(value, "Language analysis wire result");
  if (value.kind === "full") {
    if (!Array.isArray(value.items)) throw new TypeError("Full language analysis wire result must contain items");
    return resultFromItems(lane, value.items.map(item => decodeItem(lane, item)), snapshot);
  }
  if (value.kind !== "delta") {
    throw new TypeError(`Unknown language analysis wire result kind '${String(value.kind)}'`);
  }
  if (!Array.isArray(value.splices)) {
    throw new TypeError("Language analysis delta must contain splices");
  }
  const baseRequestId = decodePositiveSafeInteger(value.baseRequestId, "Language analysis delta base request ID");
  if (!base || base.requestId !== baseRequestId) {
    throw new Error("Language analysis delta base result is unavailable");
  }
  const baseItems = readBaseItems(lane, base);
  if (!baseItems) {
    throw new Error("Language analysis delta base lane does not match");
  }
  const items: LanguageAnalysisItem[] = [];
  const tokenSplices = [];
  let baseItemIndex = 0;
  let lineDelta = 0;
  for (const encodedSplice of value.splices) {
    assertRecord(encodedSplice, "Language analysis delta splice");
    if (!Array.isArray(encodedSplice.items)) {
      throw new TypeError("Language analysis delta splice must contain items");
    }
    const startItemIndex = decodeNonNegativeSafeInteger(encodedSplice.startItemIndex, "Language analysis delta start item index");
    const deleteItemCount = decodeNonNegativeSafeInteger(encodedSplice.deleteItemCount, "Language analysis delta delete item count");
    if (startItemIndex < baseItemIndex || startItemIndex > baseItems.length || deleteItemCount > baseItems.length - startItemIndex) {
      throw new RangeError("Language analysis delta splices must be ordered, non-overlapping, and inside their base result");
    }
    for (const item of baseItems.slice(baseItemIndex, startItemIndex)) items.push(shiftItem(lane, item, lineDelta));
    const inserted = encodedSplice.items.map(item => decodeItem(lane, item));
    const resultStartItemIndex = items.length;
    items.push(...inserted);
    const nextLineDelta = decodeSafeInteger(encodedSplice.lineDelta, "Language analysis delta line shift");
    tokenSplices.push(Object.freeze({
      baseStartItemIndex: startItemIndex,
      baseDeleteItemCount: deleteItemCount,
      resultStartItemIndex,
      resultInsertItemCount: inserted.length,
      lineDeltaBefore: lineDelta,
      lineDeltaAfter: nextLineDelta,
    }));
    baseItemIndex = startItemIndex + deleteItemCount;
    lineDelta = nextLineDelta;
  }
  if (lineDelta !== snapshot.lineCount - base.snapshot.lineCount) {
    throw new Error("Language analysis delta final line shift does not match its snapshots");
  }
  for (const item of baseItems.slice(baseItemIndex)) items.push(shiftItem(lane, item, lineDelta));
  const result = resultFromItems(lane, items, snapshot);
  if (result.lane === LANGUAGE_TOKEN_LANE) {
    attachLanguageTokenResultDelta(result.value, {
      baseRequestId,
      splices: tokenSplices,
    });
  }
  return result;
}

type AnalysisItem = LanguageAnalysisItem;

function encodeFull(lane: LanguageAnalysisLane, items: readonly AnalysisItem[]): unknown {
  return Object.freeze({
    kind: "full",
    items: Object.freeze(items.map(item => encodeItem(lane, item))),
  });
}

function readBaseItems(lane: LanguageAnalysisLane, base: LanguageWorkerWireResultState<LanguageAnalysisResult> | undefined): readonly AnalysisItem[] | undefined {
  if (!base || base.result.lane !== lane) return undefined;
  return lane === LANGUAGE_TOKEN_LANE
    ? (base.result.value as LanguageTokenResult).tokens
    : (base.result.value as LanguageDiagnosticResult).diagnostics;
}

function resultFromItems(lane: LanguageAnalysisLane, items: readonly AnalysisItem[], snapshot: TextSnapshot): LanguageAnalysisResult {
  return lane === LANGUAGE_TOKEN_LANE
    ? Object.freeze({
      lane: LANGUAGE_TOKEN_LANE,
      value: createLanguageTokenSnapshotNormalizer(snapshot)({ tokens: items as readonly LanguageToken[] }),
    })
    : Object.freeze({
      lane: LANGUAGE_DIAGNOSTIC_LANE,
      value: createLanguageDiagnosticSnapshotNormalizer(snapshot)({ diagnostics: items as readonly LanguageDiagnostic[] }),
    });
}

function encodeItem(lane: LanguageAnalysisLane, item: AnalysisItem): unknown {
  if (lane === LANGUAGE_TOKEN_LANE) {
    const token = item as LanguageToken;
    return Object.freeze({
      range: encodeRange(token.range, "Language token wire range"),
      tokenType: token.tokenType,
      modifiers: Object.freeze([...token.modifiers]),
    });
  }
  const diagnostic = item as LanguageDiagnostic;
  return Object.freeze({
    range: encodeRange(diagnostic.range, "Language diagnostic wire range"),
    severity: diagnostic.severity,
    message: diagnostic.message,
    ...(diagnostic.code === undefined ? {} : { code: diagnostic.code }),
    ...(diagnostic.source === undefined ? {} : { source: diagnostic.source }),
  });
}

function decodeItem(lane: LanguageAnalysisLane, value: unknown): AnalysisItem {
  assertRecord(value, lane === LANGUAGE_TOKEN_LANE ? "Language token wire token" : "Language diagnostic wire diagnostic");
  if (lane === LANGUAGE_TOKEN_LANE) {
    if (!Array.isArray(value.modifiers)) {
      throw new TypeError("Language token wire modifiers must be an array");
    }
    return {
      range: decodeRange(value.range, "Language token wire range"),
      tokenType: decodeString(value.tokenType, "Language token wire type"),
      modifiers: value.modifiers.map(modifier => decodeString(modifier, "Language token wire modifier")),
    };
  }
  const code = decodeDiagnosticCode(value.code);
  const source = value.source === undefined ? undefined : decodeString(value.source, "Language diagnostic wire source");
  return {
    range: decodeRange(value.range, "Language diagnostic wire range"),
    severity: decodeString(value.severity, "Language diagnostic wire severity") as LanguageDiagnostic["severity"],
    message: decodeString(value.message, "Language diagnostic wire message"),
    ...(code === undefined ? {} : { code }),
    ...(source === undefined ? {} : { source }),
  };
}

function shiftItem(lane: LanguageAnalysisLane, item: AnalysisItem, lineDelta: number): AnalysisItem {
  const range = TextRange.from(
    TextPosition.at(item.range.start.lineIndex + lineDelta, item.range.start.columnIndex),
    TextPosition.at(item.range.end.lineIndex + lineDelta, item.range.end.columnIndex),
  );
  return lane === LANGUAGE_TOKEN_LANE
    ? { ...(item as LanguageToken), range }
    : { ...(item as LanguageDiagnostic), range };
}

function encodeRange(range: TextRange, owner: string): unknown {
  if (!(range instanceof TextRange)) throw new TypeError(`${owner} must be a TextRange`);
  return Object.freeze({
    start: Object.freeze({ lineIndex: range.start.lineIndex, columnIndex: range.start.columnIndex }),
    end: Object.freeze({ lineIndex: range.end.lineIndex, columnIndex: range.end.columnIndex }),
  });
}

function decodeRange(value: unknown, owner: string): TextRange {
  assertRecord(value, owner);
  return TextRange.from(decodePosition(value.start, `${owner} start`), decodePosition(value.end, `${owner} end`));
}

function decodePosition(value: unknown, owner: string): TextPosition {
  assertRecord(value, owner);
  return TextPosition.at(decodeNonNegativeSafeInteger(value.lineIndex, `${owner} line index`), decodeNonNegativeSafeInteger(value.columnIndex, `${owner} column index`));
}

function decodeDiagnosticCode(value: unknown): LanguageDiagnosticCode | undefined {
  if (value === undefined || typeof value === "string") return value;
  if (typeof value === "number" && Number.isFinite(value)) return value;
  throw new TypeError("Language diagnostic wire code must be a finite number or string");
}

function assertResultLane(lane: LanguageAnalysisLane, result: LanguageAnalysisResult): void {
  if (!result || result.lane !== lane) throw new TypeError(`Language analysis wire result does not match lane '${lane}'`);
}

function decodeString(value: unknown, owner: string): string {
  if (typeof value !== "string") throw new TypeError(`${owner} must be a string`);
  return value;
}

function decodePositiveSafeInteger(value: unknown, owner: string): number {
  const decoded = decodeNonNegativeSafeInteger(value, owner);
  if (decoded === 0) throw new RangeError(`${owner} must be positive`);
  return decoded;
}

function decodeNonNegativeSafeInteger(value: unknown, owner: string): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) throw new RangeError(`${owner} must be a non-negative safe integer`);
  return value as number;
}

function decodeSafeInteger(value: unknown, owner: string): number {
  if (!Number.isSafeInteger(value)) throw new RangeError(`${owner} must be a safe integer`);
  return value as number;
}

function assertRecord(value: unknown, owner: string): asserts value is Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new TypeError(`${owner} must be an object`);
}

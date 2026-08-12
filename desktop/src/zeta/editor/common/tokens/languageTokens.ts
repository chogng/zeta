import { TextRange, type TextPosition, type TextSnapshot } from "../core/text.js";
import { VersionedLanguageResultStore } from "../languages/languageResultStore.js";
import { type TextModel } from "../model/textModel.js";

export interface LanguageToken {
  readonly range: TextRange;
  readonly tokenType: string;
  readonly modifiers: readonly string[];
}

export interface LanguageTokenResult {
  readonly tokens: readonly LanguageToken[];
}

export interface LanguageTokenResultSplice {
  readonly baseStartItemIndex: number;
  readonly baseDeleteItemCount: number;
  readonly resultStartItemIndex: number;
  readonly resultInsertItemCount: number;
  readonly lineDeltaBefore: number;
  readonly lineDeltaAfter: number;
}

export interface LanguageTokenResultDelta {
  readonly baseRequestId: number;
  readonly splices: readonly LanguageTokenResultSplice[];
}

export function createLanguageTokenStore(model: TextModel): VersionedLanguageResultStore<LanguageTokenResult> {
  return new VersionedLanguageResultStore(model, (value, currentModel) => normalizeLanguageTokenResult(
    value,
    range => assertModelRange(currentModel, range, "Language token"),
    true,
  ));
}

export function createLanguageTokenSnapshotNormalizer(snapshot: TextSnapshot): (value: LanguageTokenResult) => LanguageTokenResult {
  const lines = snapshot.getText().split("\n");
  return value => normalizeLanguageTokenResult(value, range => assertSnapshotRange(lines, range, "Language token"), false);
}

export function attachLanguageTokenResultDelta(result: LanguageTokenResult, delta: LanguageTokenResultDelta): LanguageTokenResult {
  if (!Object.isFrozen(result) || !Object.isFrozen(result.tokens)) {
    throw new TypeError("Language token delta requires an immutable normalized result");
  }
  const normalized = normalizeLanguageTokenResultDelta(delta, result.tokens.length);
  languageTokenResultDeltas.set(result, normalized);
  return result;
}

export function getLanguageTokenResultDelta(result: LanguageTokenResult): LanguageTokenResultDelta | undefined {
  return languageTokenResultDeltas.get(result);
}

const languageTokenResultDeltas = new WeakMap<LanguageTokenResult, LanguageTokenResultDelta>();

function normalizeLanguageTokenResult(value: LanguageTokenResult, validateRange: (range: TextRange) => void, preserveDelta: boolean): LanguageTokenResult {
  if (typeof value !== "object" || value === null || !Array.isArray(value.tokens)) {
    throw new TypeError("Language token result must contain a tokens array");
  }
  const delta = preserveDelta ? getLanguageTokenResultDelta(value) : undefined;
  const tokens: LanguageToken[] = [];
  let previousEnd: TextPosition | undefined;
  for (const token of value.tokens) {
    if (typeof token !== "object" || token === null) {
      throw new TypeError("Language token must be an object");
    }
    validateRange(token.range);
    if (token.range.empty) {
      throw new RangeError("Language token range must not be empty");
    }
    if (token.range.start.lineIndex !== token.range.end.lineIndex) {
      throw new RangeError("Language token range must stay on one line");
    }
    if (previousEnd && previousEnd.compareTo(token.range.start) > 0) {
      throw new RangeError("Language tokens must be sorted and non-overlapping");
    }
    assertIdentifier(token.tokenType, "Language token type");
    if (!Array.isArray(token.modifiers)) {
      throw new TypeError("Language token modifiers must be an array");
    }
    const modifiers = token.modifiers.map((modifier: unknown) => {
      assertIdentifier(modifier, "Language token modifier");
      return modifier;
    });
    if (new Set(modifiers).size !== modifiers.length) {
      throw new RangeError("Language token modifiers must be unique");
    }
    tokens.push(Object.freeze({
      range: token.range,
      tokenType: token.tokenType,
      modifiers: Object.freeze(modifiers),
    }));
    previousEnd = token.range.end;
  }
  const result = Object.freeze({ tokens: Object.freeze(tokens) });
  return delta ? attachLanguageTokenResultDelta(result, delta) : result;
}

function normalizeLanguageTokenResultDelta(delta: LanguageTokenResultDelta, tokenCount: number): LanguageTokenResultDelta {
  if (typeof delta !== "object" || delta === null) {
    throw new TypeError("Language token result delta must be an object");
  }
  assertPositiveSafeInteger(delta.baseRequestId, "Language token delta base request ID");
  if (!Array.isArray(delta.splices)) {
    throw new TypeError("Language token delta splices must be an array");
  }
  let previousBaseEnd = 0;
  let previousResultEnd = 0;
  let previousLineDelta = 0;
  const splices = delta.splices.map((splice, index) => {
    if (typeof splice !== "object" || splice === null) {
      throw new TypeError("Language token delta splice must be an object");
    }
    assertNonNegativeSafeInteger(splice.baseStartItemIndex, "Language token delta base start item index");
    assertNonNegativeSafeInteger(splice.baseDeleteItemCount, "Language token delta base delete item count");
    assertNonNegativeSafeInteger(splice.resultStartItemIndex, "Language token delta result start item index");
    assertNonNegativeSafeInteger(splice.resultInsertItemCount, "Language token delta result insert item count");
    assertSafeInteger(splice.lineDeltaBefore, "Language token delta preceding line shift");
    assertSafeInteger(splice.lineDeltaAfter, "Language token delta following line shift");
    if (splice.baseStartItemIndex < previousBaseEnd || splice.resultStartItemIndex < previousResultEnd) {
      throw new RangeError("Language token delta splices must be ordered and non-overlapping");
    }
    if (splice.baseStartItemIndex - previousBaseEnd !== splice.resultStartItemIndex - previousResultEnd) {
      throw new RangeError("Language token delta unchanged item spans must preserve their length");
    }
    if (splice.lineDeltaBefore !== previousLineDelta) {
      throw new RangeError("Language token delta line shifts must form one continuous mapping");
    }
    if (splice.resultStartItemIndex + splice.resultInsertItemCount > tokenCount) {
      throw new RangeError("Language token delta inserted items exceed the normalized result");
    }
    previousBaseEnd = splice.baseStartItemIndex + splice.baseDeleteItemCount;
    previousResultEnd = splice.resultStartItemIndex + splice.resultInsertItemCount;
    previousLineDelta = splice.lineDeltaAfter;
    return Object.freeze({
      baseStartItemIndex: splice.baseStartItemIndex,
      baseDeleteItemCount: splice.baseDeleteItemCount,
      resultStartItemIndex: splice.resultStartItemIndex,
      resultInsertItemCount: splice.resultInsertItemCount,
      lineDeltaBefore: splice.lineDeltaBefore,
      lineDeltaAfter: splice.lineDeltaAfter,
    });
  });
  if (tokenCount < previousResultEnd) {
    throw new RangeError("Language token delta result item count is inconsistent");
  }
  return Object.freeze({
    baseRequestId: delta.baseRequestId,
    splices: Object.freeze(splices),
  });
}

function assertModelRange(model: TextModel, range: TextRange, owner: string): void {
  if (!(range instanceof TextRange)) {
    throw new TypeError(`${owner} range must be a TextRange`);
  }
  model.offsetAt(range.start);
  model.offsetAt(range.end);
}

function assertSnapshotRange(lines: readonly string[], range: TextRange, owner: string): void {
  if (!(range instanceof TextRange)) {
    throw new TypeError(`${owner} range must be a TextRange`);
  }
  assertSnapshotPosition(lines, range.start, owner);
  assertSnapshotPosition(lines, range.end, owner);
}

function assertSnapshotPosition(lines: readonly string[], position: TextPosition, owner: string): void {
  if (position.lineIndex >= lines.length || position.columnIndex > lines[position.lineIndex]!.length) {
    throw new RangeError(`${owner} range is outside its snapshot`);
  }
}

function assertIdentifier(value: unknown, owner: string): asserts value is string {
  if (typeof value !== "string" || value.length === 0 || value.trim() !== value) {
    throw new TypeError(`${owner} must be a non-empty trimmed string`);
  }
}

function assertPositiveSafeInteger(value: unknown, owner: string): asserts value is number {
  if (!Number.isSafeInteger(value) || (value as number) <= 0) throw new RangeError(`${owner} must be a positive safe integer`);
}

function assertNonNegativeSafeInteger(value: unknown, owner: string): asserts value is number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) throw new RangeError(`${owner} must be a non-negative safe integer`);
}

function assertSafeInteger(value: unknown, owner: string): asserts value is number {
  if (!Number.isSafeInteger(value)) throw new RangeError(`${owner} must be a safe integer`);
}


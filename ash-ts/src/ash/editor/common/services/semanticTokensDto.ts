import { isNonNegativeSafeInteger, isPositiveSafeInteger, isSafeInteger } from '../../../base/common/numbers.js';
import type { LanguageTokenResult } from '../tokens/languageTokens.js';

export interface LanguageTokenResultSplice {
	readonly baseStartItemIndex: number;
	readonly baseDeleteItemCount: number;
	readonly resultStartItemIndex: number;
	readonly resultInsertItemCount: number;
	readonly lineDeltaBefore: number;
	readonly lineDeltaAfter: number;
}

/** Delta metadata accompanying one normalized full token result across a worker boundary. */
export interface LanguageTokenResultDelta {
	readonly baseRequestId: number;
	readonly splices: readonly LanguageTokenResultSplice[];
}

const resultDeltas = new WeakMap<LanguageTokenResult, LanguageTokenResultDelta>();

export function attachLanguageTokenResultDelta(result: LanguageTokenResult, delta: LanguageTokenResultDelta): LanguageTokenResult {
	if (!Object.isFrozen(result) || !Object.isFrozen(result.tokens)) throw new TypeError('Language token delta requires an immutable normalized result');
	resultDeltas.set(result, normalizeLanguageTokenResultDelta(delta, result.tokens.length));
	return result;
}

export function getLanguageTokenResultDelta(result: LanguageTokenResult): LanguageTokenResultDelta | undefined {
	return resultDeltas.get(result);
}

function normalizeLanguageTokenResultDelta(delta: LanguageTokenResultDelta, tokenCount: number): LanguageTokenResultDelta {
	if (typeof delta !== 'object' || delta === null) throw new TypeError('Language token result delta must be an object');
	if (!isPositiveSafeInteger(delta.baseRequestId)) throw new RangeError('Language token delta base request ID must be a positive safe integer');
	if (!Array.isArray(delta.splices)) throw new TypeError('Language token delta splices must be an array');
	let previousBaseEnd = 0;
	let previousResultEnd = 0;
	let previousLineDelta = 0;
	const splices = delta.splices.map(splice => {
		if (typeof splice !== 'object' || splice === null) throw new TypeError('Language token delta splice must be an object');
		assertNonNegativeSafeInteger(splice.baseStartItemIndex, 'Language token delta base start item index');
		assertNonNegativeSafeInteger(splice.baseDeleteItemCount, 'Language token delta base delete item count');
		assertNonNegativeSafeInteger(splice.resultStartItemIndex, 'Language token delta result start item index');
		assertNonNegativeSafeInteger(splice.resultInsertItemCount, 'Language token delta result insert item count');
		assertSafeInteger(splice.lineDeltaBefore, 'Language token delta preceding line shift');
		assertSafeInteger(splice.lineDeltaAfter, 'Language token delta following line shift');
		if (splice.baseStartItemIndex < previousBaseEnd || splice.resultStartItemIndex < previousResultEnd) throw new RangeError('Language token delta splices must be ordered and non-overlapping');
		if (splice.baseStartItemIndex - previousBaseEnd !== splice.resultStartItemIndex - previousResultEnd) throw new RangeError('Language token delta unchanged item spans must preserve their length');
		if (splice.lineDeltaBefore !== previousLineDelta) throw new RangeError('Language token delta line shifts must form one continuous mapping');
		if (splice.resultStartItemIndex + splice.resultInsertItemCount > tokenCount) throw new RangeError('Language token delta inserted items exceed the normalized result');
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
	if (tokenCount < previousResultEnd) throw new RangeError('Language token delta result item count is inconsistent');
	return Object.freeze({ baseRequestId: delta.baseRequestId, splices: Object.freeze(splices) });
}

function assertNonNegativeSafeInteger(value: unknown, owner: string): asserts value is number {
	if (!isNonNegativeSafeInteger(value)) throw new RangeError(`${owner} must be a non-negative safe integer`);
}

function assertSafeInteger(value: unknown, owner: string): asserts value is number {
	if (!isSafeInteger(value)) throw new RangeError(`${owner} must be a safe integer`);
}

import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { type VersionedLanguageResult } from "../languages/languageRequestCoordinator.js";
import { LanguageResultStoreChangeReason, type VersionedLanguageResultStore } from "../languages/languageResultStore.js";
import { getLanguageTokenResultDelta, type LanguageToken, type LanguageTokenResult, type LanguageTokenResultDelta, type LanguageTokenResultSplice } from "./languageTokens.js";
import { TextPosition, TextRange } from "../core/text.js";
import { type TextModel } from "../model/textModel.js";

export interface LanguageTokenLine {
	readonly lineIndex: number;
	readonly tokens: readonly LanguageToken[];
}

export interface LanguageTokenLineIndexChange {
	readonly reason: LanguageResultStoreChangeReason;
	readonly modelVersion: number;
	readonly requestId: number | undefined;
	readonly tokenCount: number;
	readonly rebuiltLineCount: number;
	readonly reusedLineCount: number;
}

interface RelativeLanguageToken {
	readonly startColumn: number;
	readonly endColumn: number;
	readonly tokenType: string;
	readonly modifiers: readonly string[];
	readonly languageId?: LanguageToken["languageId"];
	readonly balancedBrackets?: LanguageToken["balancedBrackets"];
	readonly presentation?: LanguageToken["presentation"];
}

interface LanguageTokenLinePayload {
	readonly tokens: readonly RelativeLanguageToken[];
}

interface LanguageTokenLineState {
	readonly lineIndex: number;
	readonly payload: LanguageTokenLinePayload;
	readonly line: LanguageTokenLine;
}

interface LanguageTokenLineItemRange {
	readonly startItemIndex: number;
	readonly endItemIndex: number;
}

interface LanguageTokenIndexState {
	readonly tokenCount: number;
	readonly lineStates: readonly LanguageTokenLineState[];
	readonly lines: readonly LanguageTokenLine[];
	readonly lineItemRanges: readonly LanguageTokenLineItemRange[];
	readonly statesByLine: ReadonlyMap<number, LanguageTokenLineState>;
}

interface LanguageTokenIndexBase {
	readonly requestId: number;
	readonly state: LanguageTokenIndexState;
}

interface LanguageTokenIndexUpdate {
	readonly state: LanguageTokenIndexState;
	readonly rebuiltLineCount: number;
	readonly reusedLineCount: number;
}

interface UnchangedItemSegment {
	readonly baseStartItemIndex: number;
	readonly baseEndItemIndex: number;
	readonly resultStartItemIndex: number;
	readonly lineDelta: number;
}

interface ReusedLineCandidate {
	readonly state: LanguageTokenLineState;
	readonly range: LanguageTokenLineItemRange;
}

/**
 * Provides sparse, constant-time line queries over one versioned token store.
 *
 * Invalidated lines disappear immediately, while the prior immutable index may
 * remain hidden as a confirmed delta base. Line payloads use line-relative
 * columns, so moved suffixes reuse semantic state and materialize absolute
 * ranges only when queried.
 */
export class LanguageTokenLineIndex extends DisposableOwner {
	private readonly changeEmitter = this.own(new Emitter<LanguageTokenLineIndexChange>());
	private readonly model: TextModel;
	private state: LanguageTokenIndexState = EMPTY_STATE;
	private invalidatedBase: LanguageTokenIndexBase | undefined;
	private indexedModelVersion: number;
	private indexedRequestId: number | undefined;

	readonly onDidChange: Event<LanguageTokenLineIndexChange> = this.changeEmitter.event;

	constructor(private readonly store: VersionedLanguageResultStore<LanguageTokenResult>) {
		super();
		this.model = store.textModel;
		const initialResult = store.result;
		this.indexedModelVersion = initialResult?.modelVersion ?? this.model.version;
		this.indexedRequestId = initialResult?.requestId;
		this.state = initialResult ? buildState(initialResult.value.tokens) : EMPTY_STATE;
		this.own(store.onDidChange(change => this.acceptStoreChange(change.reason, change.modelVersion, change.result)));
		this.defer(() => {
			this.state = EMPTY_STATE;
			this.invalidatedBase = undefined;
		});
	}

	get textModel(): TextModel {
		this.assertNotDisposed();
		return this.model;
	}

	get modelVersion(): number {
		this.assertNotDisposed();
		return this.indexedModelVersion;
	}

	get requestId(): number | undefined {
		this.assertNotDisposed();
		return this.indexedRequestId;
	}

	get tokenCount(): number {
		this.assertNotDisposed();
		return this.state.tokenCount;
	}

	get lines(): readonly LanguageTokenLine[] {
		this.assertNotDisposed();
		return this.state.lines;
	}

	getLineTokens(lineIndex: number): readonly LanguageToken[] {
		this.assertNotDisposed();
		assertLineIndex(lineIndex);
		this.model.getLineContent(lineIndex);
		return this.state.statesByLine.get(lineIndex)?.line.tokens ?? EMPTY_TOKENS;
	}

	private acceptStoreChange(reason: LanguageResultStoreChangeReason, modelVersion: number, result: VersionedLanguageResult<LanguageTokenResult> | undefined): void {
		if (reason === LanguageResultStoreChangeReason.ModelChanged) {
			if (this.indexedRequestId !== undefined) this.invalidatedBase = Object.freeze({ requestId: this.indexedRequestId, state: this.state });
			this.indexedModelVersion = modelVersion;
			this.indexedRequestId = undefined;
			this.state = EMPTY_STATE;
			this.emit(reason, 0, 0);
			return;
		}
		if (!result) {
			this.invalidatedBase = undefined;
			this.indexedModelVersion = modelVersion;
			this.indexedRequestId = undefined;
			this.state = EMPTY_STATE;
			this.emit(reason, 0, 0);
			return;
		}
		const delta = getLanguageTokenResultDelta(result.value);
		const base = delta ? this.findBase(delta.baseRequestId) : undefined;
		const update = delta && base ? applyDelta(base.state, result.value.tokens, delta) : fullUpdate(result.value.tokens);
		this.invalidatedBase = undefined;
		this.indexedModelVersion = result.modelVersion;
		this.indexedRequestId = result.requestId;
		this.state = update.state;
		this.emit(reason, update.rebuiltLineCount, update.reusedLineCount);
	}

	private findBase(requestId: number): LanguageTokenIndexBase | undefined {
		if (this.indexedRequestId === requestId) return Object.freeze({ requestId, state: this.state });
		return this.invalidatedBase?.requestId === requestId ? this.invalidatedBase : undefined;
	}

	private emit(reason: LanguageResultStoreChangeReason, rebuiltLineCount: number, reusedLineCount: number): void {
		this.changeEmitter.fire(Object.freeze({
			reason,
			modelVersion: this.indexedModelVersion,
			requestId: this.indexedRequestId,
			tokenCount: this.state.tokenCount,
			rebuiltLineCount,
			reusedLineCount,
		}));
	}

}

function fullUpdate(tokens: readonly LanguageToken[]): LanguageTokenIndexUpdate {
	const state = buildState(tokens);
	return Object.freeze({ state, rebuiltLineCount: state.lines.length, reusedLineCount: 0 });
}

function applyDelta(base: LanguageTokenIndexState, tokens: readonly LanguageToken[], delta: LanguageTokenResultDelta): LanguageTokenIndexUpdate {
	const segments = createUnchangedSegments(base.tokenCount, tokens.length, delta.splices);
	if (!segments) return fullUpdate(tokens);
	const candidates = createReusedLineCandidates(base, tokens, segments);
	if (candidates.length === 0) return fullUpdate(tokens);
	const lineStates: LanguageTokenLineState[] = [];
	const ranges: LanguageTokenLineItemRange[] = [];
	let tokenIndex = 0;
	let rebuiltLineCount = 0;
	for (const candidate of candidates) {
		const rebuilt = buildLineStates(tokens, tokenIndex, candidate.range.startItemIndex);
		lineStates.push(...rebuilt.states, candidate.state);
		ranges.push(...rebuilt.ranges, candidate.range);
		rebuiltLineCount += rebuilt.states.length;
		tokenIndex = candidate.range.endItemIndex;
	}
	const tail = buildLineStates(tokens, tokenIndex, tokens.length);
	lineStates.push(...tail.states);
	ranges.push(...tail.ranges);
	rebuiltLineCount += tail.states.length;
	return Object.freeze({
		state: createState(tokens.length, lineStates, ranges),
		rebuiltLineCount,
		reusedLineCount: candidates.length,
	});
}

function createUnchangedSegments(baseTokenCount: number, resultTokenCount: number, splices: readonly LanguageTokenResultSplice[]): readonly UnchangedItemSegment[] | undefined {
	const segments: UnchangedItemSegment[] = [];
	let baseItemIndex = 0;
	let resultItemIndex = 0;
	let lineDelta = 0;
	for (const splice of splices) {
		if (splice.baseStartItemIndex < baseItemIndex || splice.baseStartItemIndex > baseTokenCount || splice.baseDeleteItemCount > baseTokenCount - splice.baseStartItemIndex || splice.resultStartItemIndex < resultItemIndex || splice.resultStartItemIndex > resultTokenCount || splice.resultInsertItemCount > resultTokenCount - splice.resultStartItemIndex || splice.lineDeltaBefore !== lineDelta) {
			return undefined;
		}
		const baseUnchangedCount = splice.baseStartItemIndex - baseItemIndex;
		if (splice.resultStartItemIndex - resultItemIndex !== baseUnchangedCount) return undefined;
		if (baseUnchangedCount > 0) {
			segments.push(Object.freeze({
				baseStartItemIndex: baseItemIndex,
				baseEndItemIndex: splice.baseStartItemIndex,
				resultStartItemIndex: resultItemIndex,
				lineDelta,
			}));
		}
		baseItemIndex = splice.baseStartItemIndex + splice.baseDeleteItemCount;
		resultItemIndex = splice.resultStartItemIndex + splice.resultInsertItemCount;
		lineDelta = splice.lineDeltaAfter;
	}
	if (baseTokenCount - baseItemIndex !== resultTokenCount - resultItemIndex) return undefined;
	if (baseItemIndex < baseTokenCount) {
		segments.push(Object.freeze({
			baseStartItemIndex: baseItemIndex,
			baseEndItemIndex: baseTokenCount,
			resultStartItemIndex: resultItemIndex,
			lineDelta,
		}));
	}
	return Object.freeze(segments);
}

function createReusedLineCandidates(base: LanguageTokenIndexState, tokens: readonly LanguageToken[], segments: readonly UnchangedItemSegment[]): readonly ReusedLineCandidate[] {
	const candidates: ReusedLineCandidate[] = [];
	let segmentIndex = 0;
	for (let lineIndex = 0; lineIndex < base.lineStates.length; lineIndex += 1) {
		const line = base.lineStates[lineIndex]!;
		const range = base.lineItemRanges[lineIndex]!;
		while (segmentIndex < segments.length && segments[segmentIndex]!.baseEndItemIndex <= range.startItemIndex) segmentIndex += 1;
		const segment = segments[segmentIndex];
		if (!segment || range.startItemIndex < segment.baseStartItemIndex || range.endItemIndex > segment.baseEndItemIndex) continue;
		const resultStartItemIndex = segment.resultStartItemIndex + range.startItemIndex - segment.baseStartItemIndex;
		const resultEndItemIndex = resultStartItemIndex + range.endItemIndex - range.startItemIndex;
		const shiftedLineIndex = line.lineIndex + segment.lineDelta;
		if (shiftedLineIndex < 0 || !lineMatchesResult(line.payload, shiftedLineIndex, tokens, resultStartItemIndex, resultEndItemIndex)) continue;
		const previousToken = tokens[resultStartItemIndex - 1];
		const nextToken = tokens[resultEndItemIndex];
		if (previousToken?.range.start.lineIndex === shiftedLineIndex || nextToken?.range.start.lineIndex === shiftedLineIndex) continue;
		candidates.push(Object.freeze({
			state: segment.lineDelta === 0 ? line : createLineState(shiftedLineIndex, line.payload),
			range: Object.freeze({ startItemIndex: resultStartItemIndex, endItemIndex: resultEndItemIndex }),
		}));
	}
	return Object.freeze(candidates);
}

function buildState(tokens: readonly LanguageToken[]): LanguageTokenIndexState {
	const built = buildLineStates(tokens, 0, tokens.length);
	return createState(tokens.length, built.states, built.ranges);
}

function buildLineStates(tokens: readonly LanguageToken[], startItemIndex: number, endItemIndex: number): { readonly states: readonly LanguageTokenLineState[]; readonly ranges: readonly LanguageTokenLineItemRange[] } {
	const states: LanguageTokenLineState[] = [];
	const ranges: LanguageTokenLineItemRange[] = [];
	for (let index = startItemIndex; index < endItemIndex;) {
		const lineIndex = tokens[index]!.range.start.lineIndex;
		let end = index + 1;
		while (end < endItemIndex && tokens[end]!.range.start.lineIndex === lineIndex) end += 1;
		const payload = createLinePayload(tokens.slice(index, end));
		states.push(createLineState(lineIndex, payload));
		ranges.push(Object.freeze({ startItemIndex: index, endItemIndex: end }));
		index = end;
	}
	return Object.freeze({ states: Object.freeze(states), ranges: Object.freeze(ranges) });
}

function createLinePayload(tokens: readonly LanguageToken[]): LanguageTokenLinePayload {
	return Object.freeze({
		tokens: Object.freeze(tokens.map(token => Object.freeze({
			startColumn: token.range.start.columnIndex,
			endColumn: token.range.end.columnIndex,
			tokenType: token.tokenType,
			modifiers: token.modifiers,
			...(token.languageId === undefined ? {} : { languageId: token.languageId }),
			...(token.balancedBrackets === undefined ? {} : { balancedBrackets: token.balancedBrackets }),
			...(token.presentation === undefined ? {} : { presentation: token.presentation }),
		}))),
	});
}

function createLineState(lineIndex: number, payload: LanguageTokenLinePayload): LanguageTokenLineState {
	let materializedTokens: readonly LanguageToken[] | undefined;
	const line = Object.freeze({
		lineIndex,
		get tokens(): readonly LanguageToken[] {
			materializedTokens ??= Object.freeze(payload.tokens.map(token => Object.freeze({
				range: TextRange.from(TextPosition.at(lineIndex, token.startColumn), TextPosition.at(lineIndex, token.endColumn)),
				tokenType: token.tokenType,
				modifiers: token.modifiers,
				...(token.languageId === undefined ? {} : { languageId: token.languageId }),
				...(token.balancedBrackets === undefined ? {} : { balancedBrackets: token.balancedBrackets }),
				...(token.presentation === undefined ? {} : { presentation: token.presentation }),
			})));
			return materializedTokens;
		},
	});
	return Object.freeze({ lineIndex, payload, line });
}

function createState(tokenCount: number, lineStates: readonly LanguageTokenLineState[], ranges: readonly LanguageTokenLineItemRange[]): LanguageTokenIndexState {
	const frozenStates = Object.freeze([...lineStates]);
	const statesByLine = new Map<number, LanguageTokenLineState>();
	for (const state of frozenStates) statesByLine.set(state.lineIndex, state);
	return Object.freeze({
		tokenCount,
		lineStates: frozenStates,
		lines: Object.freeze(frozenStates.map(state => state.line)),
		lineItemRanges: Object.freeze([...ranges]),
		statesByLine,
	});
}

function lineMatchesResult(payload: LanguageTokenLinePayload, lineIndex: number, tokens: readonly LanguageToken[], startItemIndex: number, endItemIndex: number): boolean {
	if (endItemIndex - startItemIndex !== payload.tokens.length) return false;
	return payload.tokens.every((relative, index) => {
		const token = tokens[startItemIndex + index]!;
		return token.range.start.lineIndex === lineIndex &&
			token.range.end.lineIndex === lineIndex &&
			token.range.start.columnIndex === relative.startColumn &&
			token.range.end.columnIndex === relative.endColumn &&
			token.tokenType === relative.tokenType &&
			arraysEqual(token.modifiers, relative.modifiers) &&
			token.languageId === relative.languageId &&
			token.balancedBrackets === relative.balancedBrackets &&
			presentationsEqual(token.presentation, relative.presentation);
	});
}

function presentationsEqual(left: LanguageToken["presentation"], right: LanguageToken["presentation"]): boolean {
	return left?.foreground === right?.foreground && left?.background === right?.background && arraysEqual(left?.fontStyle ?? [], right?.fontStyle ?? []);
}

function arraysEqual(left: readonly string[], right: readonly string[]): boolean {
	return left.length === right.length && left.every((value, index) => value === right[index]);
}

const EMPTY_TOKENS: readonly LanguageToken[] = Object.freeze([]);
const EMPTY_STATE: LanguageTokenIndexState = Object.freeze({
	tokenCount: 0,
	lineStates: Object.freeze([]),
	lines: Object.freeze([]),
	lineItemRanges: Object.freeze([]),
	statesByLine: new Map(),
});

function assertLineIndex(lineIndex: number): void {
	if (!Number.isSafeInteger(lineIndex) || lineIndex < 0) throw new RangeError("Language token line index must be a non-negative safe integer");
}

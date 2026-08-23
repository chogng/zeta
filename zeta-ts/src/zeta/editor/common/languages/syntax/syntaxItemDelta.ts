import { SYNTAX_TOKEN_LANE, type SyntaxLane } from "./syntaxService.js";
import { type LanguageDiagnostic, type LanguageToken } from "../languageResults.js";
import { type TextRange, type TextSnapshot } from "../../core/text.js";

export type SyntaxItem = LanguageToken | LanguageDiagnostic;

export interface SyntaxItemSplice {
	readonly startItemIndex: number;
	readonly deleteItemCount: number;
	readonly items: readonly SyntaxItem[];
	readonly lineDeltaBefore: number;
	readonly lineDeltaAfter: number;
}

interface LinePair {
	readonly previousLineIndex: number;
	readonly currentLineIndex: number;
}

interface LineRun extends LinePair {
	readonly lineCount: number;
}

export function createSyntaxItemSplices(lane: SyntaxLane, previous: readonly SyntaxItem[], current: readonly SyntaxItem[], previousSnapshot: TextSnapshot, currentSnapshot: TextSnapshot): readonly SyntaxItemSplice[] {
	const previousLines = previousSnapshot.getText().split("\n");
	const currentLines = currentSnapshot.getText().split("\n");
	const previousBounds = createLineItemBounds(previous, previousLines.length);
	const currentBounds = createLineItemBounds(current, currentLines.length);
	if (!previousBounds || !currentBounds) {
		return Object.freeze([createSingleSplice(lane, previous, current, currentSnapshot.lineCount - previousSnapshot.lineCount)]);
	}
	const runs = createStableLineRuns(lane, previous, current, previousLines, currentLines, previousBounds, currentBounds);
	const splices: SyntaxItemSplice[] = [];
	let previousItemIndex = 0;
	let currentItemIndex = 0;
	let lineDelta = 0;
	for (const run of runs) {
		const previousRunStart = previousBounds[run.previousLineIndex]!;
		const currentRunStart = currentBounds[run.currentLineIndex]!;
		const nextLineDelta = run.currentLineIndex - run.previousLineIndex;
		appendSplice(splices, current, previousItemIndex, previousRunStart, currentItemIndex, currentRunStart, lineDelta, nextLineDelta);
		previousItemIndex = previousBounds[run.previousLineIndex + run.lineCount]!;
		currentItemIndex = currentBounds[run.currentLineIndex + run.lineCount]!;
		lineDelta = nextLineDelta;
	}
	appendSplice(splices, current, previousItemIndex, previous.length, currentItemIndex, current.length, lineDelta, currentSnapshot.lineCount - previousSnapshot.lineCount);
	return Object.freeze(splices);
}

export function syntaxItemsEqual(lane: SyntaxLane, current: SyntaxItem, previous: SyntaxItem, lineDelta: number): boolean {
	if (!rangesEqual(current.range, previous.range, lineDelta)) return false;
	if (lane === SYNTAX_TOKEN_LANE) {
		const currentToken = current as LanguageToken;
		const previousToken = previous as LanguageToken;
		return currentToken.tokenType === previousToken.tokenType && arraysEqual(currentToken.modifiers, previousToken.modifiers);
	}
	const currentDiagnostic = current as LanguageDiagnostic;
	const previousDiagnostic = previous as LanguageDiagnostic;
	return currentDiagnostic.severity === previousDiagnostic.severity &&
		currentDiagnostic.message === previousDiagnostic.message &&
		currentDiagnostic.code === previousDiagnostic.code &&
		currentDiagnostic.source === previousDiagnostic.source;
}

function createStableLineRuns(lane: SyntaxLane, previousItems: readonly SyntaxItem[], currentItems: readonly SyntaxItem[], previousLines: readonly string[], currentLines: readonly string[], previousBounds: readonly number[], currentBounds: readonly number[]): readonly LineRun[] {
	const pairs = createStableLinePairs(previousLines, currentLines).filter(pair => lineItemsEqual(
		lane,
		previousItems.slice(previousBounds[pair.previousLineIndex], previousBounds[pair.previousLineIndex + 1]),
		currentItems.slice(currentBounds[pair.currentLineIndex], currentBounds[pair.currentLineIndex + 1]),
		pair.currentLineIndex - pair.previousLineIndex,
	));
	const runs: LineRun[] = [];
	for (const pair of pairs) {
		const previous = runs.at(-1);
		if (previous && previous.previousLineIndex + previous.lineCount === pair.previousLineIndex && previous.currentLineIndex + previous.lineCount === pair.currentLineIndex) {
			runs[runs.length - 1] = Object.freeze({ ...previous, lineCount: previous.lineCount + 1 });
		} else {
			runs.push(Object.freeze({ ...pair, lineCount: 1 }));
		}
	}
	return runs;
}

function createStableLinePairs(previous: readonly string[], current: readonly string[]): readonly LinePair[] {
	const prefixLength = commonPrefixLength(previous, current);
	const suffixLength = commonSuffixLength(previous, current, prefixLength);
	const pairs = new Map<string, LinePair>();
	for (let index = 0; index < prefixLength; index += 1) addPair(pairs, index, index);
	const previousInteriorEnd = previous.length - suffixLength;
	const currentInteriorEnd = current.length - suffixLength;
	const previousUnique = uniqueLineIndexes(previous, prefixLength, previousInteriorEnd);
	const currentUnique = uniqueLineIndexes(current, prefixLength, currentInteriorEnd);
	const candidates: LinePair[] = [];
	for (const [line, currentLineIndex] of currentUnique) {
		const previousLineIndex = previousUnique.get(line);
		if (previousLineIndex !== undefined) candidates.push({ previousLineIndex, currentLineIndex });
	}
	for (const pair of longestIncreasingPairs(candidates)) addPair(pairs, pair.previousLineIndex, pair.currentLineIndex);
	for (let offset = 0; offset < suffixLength; offset += 1) {
		addPair(pairs, previousInteriorEnd + offset, currentInteriorEnd + offset);
	}
	const anchors = [...pairs.values()].sort(comparePairs);
	const sentinels = [{ previousLineIndex: -1, currentLineIndex: -1 }, ...anchors, {
		previousLineIndex: previous.length,
		currentLineIndex: current.length,
	}];
	for (let index = 1; index < sentinels.length; index += 1) {
		const left = sentinels[index - 1]!;
		const right = sentinels[index]!;
		let previousStart = left.previousLineIndex + 1;
		let currentStart = left.currentLineIndex + 1;
		let previousEnd = right.previousLineIndex;
		let currentEnd = right.currentLineIndex;
		while (previousStart < previousEnd && currentStart < currentEnd && previous[previousStart] === current[currentStart]) {
			addPair(pairs, previousStart++, currentStart++);
		}
		while (previousStart < previousEnd && currentStart < currentEnd && previous[previousEnd - 1] === current[currentEnd - 1]) {
			addPair(pairs, --previousEnd, --currentEnd);
		}
	}
	return Object.freeze([...pairs.values()].sort(comparePairs));
}

function appendSplice(splices: SyntaxItemSplice[], current: readonly SyntaxItem[], previousStart: number, previousEnd: number, currentStart: number, currentEnd: number, lineDeltaBefore: number, lineDeltaAfter: number): void {
	if (previousStart === previousEnd && currentStart === currentEnd && lineDeltaBefore === lineDeltaAfter) return;
	splices.push(Object.freeze({
		startItemIndex: previousStart,
		deleteItemCount: previousEnd - previousStart,
		items: Object.freeze(current.slice(currentStart, currentEnd)),
		lineDeltaBefore,
		lineDeltaAfter,
	}));
}

function createSingleSplice(lane: SyntaxLane, previous: readonly SyntaxItem[], current: readonly SyntaxItem[], lineDelta: number): SyntaxItemSplice {
	const limit = Math.min(previous.length, current.length);
	let prefixLength = 0;
	while (prefixLength < limit && syntaxItemsEqual(lane, current[prefixLength]!, previous[prefixLength]!, 0)) prefixLength += 1;
	let suffixLength = 0;
	while (suffixLength < limit - prefixLength && syntaxItemsEqual(lane, current[current.length - suffixLength - 1]!, previous[previous.length - suffixLength - 1]!, lineDelta)) suffixLength += 1;
	return Object.freeze({
		startItemIndex: prefixLength,
		deleteItemCount: previous.length - prefixLength - suffixLength,
		items: Object.freeze(current.slice(prefixLength, current.length - suffixLength)),
		lineDeltaBefore: 0,
		lineDeltaAfter: lineDelta,
	});
}

function createLineItemBounds(items: readonly SyntaxItem[], lineCount: number): readonly number[] | undefined {
	const bounds = new Array<number>(lineCount + 1);
	let itemIndex = 0;
	for (let lineIndex = 0; lineIndex < lineCount; lineIndex += 1) {
		bounds[lineIndex] = itemIndex;
		while (itemIndex < items.length && items[itemIndex]!.range.start.lineIndex === lineIndex) {
			itemIndex += 1;
		}
		if (itemIndex < items.length && items[itemIndex]!.range.start.lineIndex < lineIndex) return undefined;
	}
	bounds[lineCount] = itemIndex;
	return itemIndex === items.length ? Object.freeze(bounds) : undefined;
}

function lineItemsEqual(lane: SyntaxLane, previous: readonly SyntaxItem[], current: readonly SyntaxItem[], lineDelta: number): boolean {
	return previous.length === current.length && previous.every((item, index) => syntaxItemsEqual(lane, current[index]!, item, lineDelta));
}

function uniqueLineIndexes(lines: readonly string[], start: number, end: number): ReadonlyMap<string, number> {
	const indexes = new Map<string, number>();
	const duplicates = new Set<string>();
	for (let index = start; index < end; index += 1) {
		const line = lines[index]!;
		if (indexes.has(line)) {
			indexes.delete(line);
			duplicates.add(line);
		} else if (!duplicates.has(line)) {
			indexes.set(line, index);
		}
	}
	return indexes;
}

function longestIncreasingPairs(pairs: readonly LinePair[]): readonly LinePair[] {
	if (pairs.length === 0) return [];
	const tails: number[] = [];
	const predecessors = new Array<number>(pairs.length).fill(-1);
	for (let index = 0; index < pairs.length; index += 1) {
		let low = 0;
		let high = tails.length;
		while (low < high) {
			const middle = (low + high) >>> 1;
			if (pairs[tails[middle]!]!.previousLineIndex < pairs[index]!.previousLineIndex) low = middle + 1;
			else high = middle;
		}
		if (low > 0) predecessors[index] = tails[low - 1]!;
		tails[low] = index;
	}
	const result: LinePair[] = [];
	for (let index = tails.at(-1)!; index >= 0; index = predecessors[index]!) result.push(pairs[index]!);
	return result.reverse();
}

function addPair(pairs: Map<string, LinePair>, previousLineIndex: number, currentLineIndex: number): void {
	pairs.set(`${previousLineIndex}:${currentLineIndex}`, Object.freeze({ previousLineIndex, currentLineIndex }));
}

function comparePairs(left: LinePair, right: LinePair): number {
	return left.previousLineIndex - right.previousLineIndex || left.currentLineIndex - right.currentLineIndex;
}

function commonPrefixLength(left: readonly string[], right: readonly string[]): number {
	const limit = Math.min(left.length, right.length);
	let index = 0;
	while (index < limit && left[index] === right[index]) index += 1;
	return index;
}

function commonSuffixLength(left: readonly string[], right: readonly string[], prefixLength: number): number {
	const limit = Math.min(left.length, right.length) - prefixLength;
	let length = 0;
	while (length < limit && left[left.length - length - 1] === right[right.length - length - 1]) length += 1;
	return length;
}

function rangesEqual(current: TextRange, previous: TextRange, lineDelta: number): boolean {
	return current.start.lineIndex === previous.start.lineIndex + lineDelta &&
		current.start.columnIndex === previous.start.columnIndex &&
		current.end.lineIndex === previous.end.lineIndex + lineDelta &&
		current.end.columnIndex === previous.end.columnIndex;
}

function arraysEqual(left: readonly string[], right: readonly string[]): boolean {
	return left.length === right.length && left.every((value, index) => value === right[index]);
}

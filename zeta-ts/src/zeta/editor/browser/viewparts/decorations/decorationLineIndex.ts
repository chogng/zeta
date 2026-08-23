import { type ResolvedDecoration } from "../decorations/decorationPresentation.js";

interface DecorationInterval {
	readonly decoration: ResolvedDecoration;
	readonly startLineIndex: number;
	readonly endLineIndex: number;
	readonly order: number;
}

interface DecorationIntervalNode {
	readonly interval: DecorationInterval;
	readonly maximumEndLineIndex: number;
	readonly left: DecorationIntervalNode | undefined;
	readonly right: DecorationIntervalNode | undefined;
}

/**
 * Immutable interval index owned by the decorations view part.
 *
 * The index stores only the latest presentation snapshot. Decoration sources,
 * source invalidation, and DOM projection remain owned by DecorationsPart.
 */
export class DecorationLineIndex {
	private readonly root: DecorationIntervalNode | undefined;

	constructor(decorations: readonly ResolvedDecoration[]) {
		this.root = buildIntervalTree(decorations.map((decoration, order) => Object.freeze({
			decoration,
			startLineIndex: decoration.range.start.lineIndex,
			endLineIndex: lastCoveredLineIndex(decoration),
			order,
		})).sort(compareIntervals));
	}

	/** Returns decorations that can produce geometry on the inclusive line span. */
	getIntersectingLines(startLineIndex: number, endLineIndex: number): readonly ResolvedDecoration[] {
		if (!Number.isSafeInteger(startLineIndex) || !Number.isSafeInteger(endLineIndex) || startLineIndex < 0 || endLineIndex < startLineIndex) {
			throw new RangeError("Stanza decoration line queries require a non-negative ordered integer span");
		}
		const intervals: DecorationInterval[] = [];
		collectIntersecting(this.root, startLineIndex, endLineIndex, intervals);
		intervals.sort((left, right) => left.order - right.order);
		return Object.freeze(intervals.map(interval => interval.decoration));
	}
}

function buildIntervalTree(intervals: readonly DecorationInterval[]): DecorationIntervalNode | undefined {
	if (intervals.length === 0) return undefined;
	const middle = Math.floor(intervals.length / 2);
	const interval = intervals[middle]!;
	const left = buildIntervalTree(intervals.slice(0, middle));
	const right = buildIntervalTree(intervals.slice(middle + 1));
	return Object.freeze({
		interval,
		maximumEndLineIndex: Math.max(interval.endLineIndex, left?.maximumEndLineIndex ?? -1, right?.maximumEndLineIndex ?? -1),
		left,
		right,
	});
}

function collectIntersecting(node: DecorationIntervalNode | undefined, startLineIndex: number, endLineIndex: number, result: DecorationInterval[]): void {
	if (!node || node.maximumEndLineIndex < startLineIndex) return;
	if (node.interval.startLineIndex > endLineIndex) {
		collectIntersecting(node.left, startLineIndex, endLineIndex, result);
		return;
	}
	collectIntersecting(node.left, startLineIndex, endLineIndex, result);
	if (node.interval.endLineIndex >= startLineIndex) result.push(node.interval);
	collectIntersecting(node.right, startLineIndex, endLineIndex, result);
}

function compareIntervals(left: DecorationInterval, right: DecorationInterval): number {
	return left.startLineIndex - right.startLineIndex || left.endLineIndex - right.endLineIndex || left.order - right.order;
}

function lastCoveredLineIndex(decoration: ResolvedDecoration): number {
	const { start, end } = decoration.range;
	if (decoration.range.empty || end.columnIndex > 0 || end.lineIndex === start.lineIndex) return end.lineIndex;
	return end.lineIndex - 1;
}

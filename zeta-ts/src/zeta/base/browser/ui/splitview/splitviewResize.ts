import { SashState } from "../sash/sash.js";

export interface SplitViewResizeItem {
	readonly size: number;
	readonly cachedVisibleSize: number | undefined;
	readonly minimumSize: number;
	readonly maximumSize: number;
	readonly visible: boolean;
	readonly snap: boolean;
}

export interface SplitViewResizeOptions {
	readonly boundaryIndex: number;
	readonly delta: number;
	readonly input: "pointer" | "keyboard";
	readonly altKey: boolean;
	readonly startSnappingEnabled: boolean;
	readonly endSnappingEnabled: boolean;
}

/** Solves one logical sash movement without depending on DOM or product state. */
export function solveSashResize(items: readonly SplitViewResizeItem[], options: SplitViewResizeOptions): readonly SplitViewResizeItem[] {
	if (options.altKey && options.input === "pointer") {
		return solveSymmetricResize(items, options.boundaryIndex, options.delta);
	}
	const result = items.map((item) => ({ ...item }));
	const originalSizes = items.map((item) => item.size);
	const toggledSnapIndexes = new Set<number>();

	for (;;) {
		const beforeIndexes = indexesBefore(options.boundaryIndex);
		const afterIndexes = indexesAfter(options.boundaryIndex, result.length);
		const { minimumDelta, maximumDelta } = resizeLimits(result, originalSizes, beforeIndexes, afterIndexes);
		const snapBeforeIndex = findFirstSnapIndex(result, beforeIndexes);
		const snapAfterIndex = findFirstSnapIndex(result, afterIndexes);
		const snapBeforeBlocked = snapBeforeIndex !== undefined && (toggledSnapIndexes.has(snapBeforeIndex) || !result[snapBeforeIndex]!.visible && snapBeforeIndex === 0 && !options.startSnappingEnabled);
		const snapAfterBlocked = snapAfterIndex !== undefined && (toggledSnapIndexes.has(snapAfterIndex) || !result[snapAfterIndex]!.visible && snapAfterIndex === result.length - 1 && !options.endSnappingEnabled);
		const snapBefore = snapBeforeBlocked ? undefined : snapBeforeIndex;
		const snapAfter = snapAfterBlocked ? undefined : snapAfterIndex;
		const snappedIndex = findSnapCandidate(result, options, minimumDelta, maximumDelta, snapBefore, snapAfter);
		if (snappedIndex === undefined) {
			applyResize(result, originalSizes, beforeIndexes, afterIndexes, clamp(options.delta, minimumDelta, maximumDelta));
			return result;
		}
		const item = result[snappedIndex]!;
		const visible = !item.visible;
		result[snappedIndex] = {
			...item,
			size: visible ? clamp(item.cachedVisibleSize ?? item.minimumSize, item.minimumSize, item.maximumSize) : 0,
			cachedVisibleSize: visible ? undefined : item.size,
			visible,
		};
		toggledSnapIndexes.add(snappedIndex);
	}
}

function solveSymmetricResize(items: readonly SplitViewResizeItem[], boundaryIndex: number, requestedDelta: number): readonly SplitViewResizeItem[] {
	const oppositeBoundaryIndex = boundaryIndex === items.length - 2 ? boundaryIndex - 1 : boundaryIndex + 1;
	if (oppositeBoundaryIndex < 0 || oppositeBoundaryIndex >= items.length - 1) {
		return solveWithoutSnapping(items, boundaryIndex, requestedDelta);
	}
	const targetIndex = boundaryIndex === items.length - 2 ? boundaryIndex : boundaryIndex + 1;
	const target = items[targetIndex]!;
	if (!target.visible) return solveWithoutSnapping(items, boundaryIndex, requestedDelta);
	const firstRange = sashRange(items, boundaryIndex);
	const oppositeRange = sashRange(items, oppositeBoundaryIndex);
	const growsWithDelta = targetIndex === boundaryIndex;
	const targetMinimumDelta = growsWithDelta
		? (target.minimumSize - target.size) / 2
		: (target.size - target.maximumSize) / 2;
	const targetMaximumDelta = growsWithDelta
		? (target.maximumSize - target.size) / 2
		: (target.size - target.minimumSize) / 2;
	const minimumDelta = Math.max(firstRange.minimumDelta, -oppositeRange.maximumDelta, targetMinimumDelta);
	const maximumDelta = Math.min(firstRange.maximumDelta, -oppositeRange.minimumDelta, targetMaximumDelta);
	if (minimumDelta > maximumDelta) return items.map((item) => ({ ...item }));
	const delta = clamp(requestedDelta, minimumDelta, maximumDelta);
	const result = items.map((item) => ({ ...item }));
	applyResizeAtBoundary(result, boundaryIndex, delta);
	applyResizeAtBoundary(result, oppositeBoundaryIndex, -delta);
	return result;
}

function solveWithoutSnapping(items: readonly SplitViewResizeItem[], boundaryIndex: number, requestedDelta: number): readonly SplitViewResizeItem[] {
	const result = items.map((item) => ({ ...item }));
	const range = sashRange(result, boundaryIndex);
	applyResizeAtBoundary(result, boundaryIndex, clamp(requestedDelta, range.minimumDelta, range.maximumDelta));
	return result;
}

function sashRange(items: readonly SplitViewResizeItem[], boundaryIndex: number): { readonly minimumDelta: number; readonly maximumDelta: number } {
	const sizes = items.map((item) => item.size);
	return resizeLimits(items, sizes, indexesBefore(boundaryIndex), indexesAfter(boundaryIndex, items.length));
}

function applyResizeAtBoundary(items: SplitViewResizeItem[], boundaryIndex: number, delta: number): void {
	const sizes = items.map((item) => item.size);
	applyResize(items, sizes, indexesBefore(boundaryIndex), indexesAfter(boundaryIndex, items.length), delta);
}

export function getSashState(items: readonly SplitViewResizeItem[], boundaryIndex: number, startSnappingEnabled: boolean, endSnappingEnabled: boolean): SashState {
	const adjacentBefore = items[boundaryIndex];
	const adjacentAfter = items[boundaryIndex + 1];
	if (!adjacentBefore || !adjacentAfter || !adjacentBefore.visible && !adjacentAfter.visible) {
		return SashState.Disabled;
	}
	const collapsesBefore = hasCapacity(items, 0, boundaryIndex + 1, "collapse");
	const expandsBefore = hasCapacity(items, 0, boundaryIndex + 1, "expand");
	const collapsesAfter = hasCapacity(items, boundaryIndex + 1, items.length, "collapse");
	const expandsAfter = hasCapacity(items, boundaryIndex + 1, items.length, "expand");
	const atMinimum = !(collapsesBefore && expandsAfter);
	const atMaximum = !(expandsBefore && collapsesAfter);
	if (!atMinimum && !atMaximum) return SashState.Enabled;
	if (atMinimum && !atMaximum) return SashState.AtMinimum;
	if (!atMinimum && atMaximum) return SashState.AtMaximum;

	const beforeIndexes = indexesBefore(boundaryIndex);
	const afterIndexes = indexesAfter(boundaryIndex, items.length);
	const snapBefore = findFirstSnapIndex(items, beforeIndexes);
	const snapAfter = findFirstSnapIndex(items, afterIndexes);
	const position = items.slice(0, boundaryIndex + 1).reduce((total, item) => total + item.size, 0);
	const contentSize = items.reduce((total, item) => total + item.size, 0);
	if (snapBefore !== undefined && !items[snapBefore]!.visible && collapsesAfter && (position > 0 || startSnappingEnabled)) {
		return SashState.AtMinimum;
	}
	if (snapAfter !== undefined && !items[snapAfter]!.visible && collapsesBefore && (position < contentSize || endSnappingEnabled)) {
		return SashState.AtMaximum;
	}
	return SashState.Disabled;
}

export function findFirstSnapIndex(items: readonly SplitViewResizeItem[], indexes: readonly number[]): number | undefined {
	for (const index of indexes) {
		const item = items[index]!;
		if (item.visible && item.snap) return index;
	}
	for (const index of indexes) {
		const item = items[index]!;
		if (item.visible && item.maximumSize > item.minimumSize) return undefined;
		if (!item.visible && item.snap) return index;
	}
	return undefined;
}

function findSnapCandidate(items: readonly SplitViewResizeItem[], options: SplitViewResizeOptions, minimumDelta: number, maximumDelta: number, beforeIndex: number | undefined, afterIndex: number | undefined): number | undefined {
	if (beforeIndex !== undefined) {
		const item = items[beforeIndex]!;
		const threshold = Math.floor(item.minimumSize / 2);
		const crossesThreshold = item.visible
			? options.delta < minimumDelta - threshold
			: options.delta >= minimumDelta + threshold;
		const keyboardCrossesEdge = options.input === "keyboard" && (item.visible ? options.delta < 0 && item.size <= item.minimumSize : options.delta > 0);
		if (crossesThreshold || keyboardCrossesEdge) return beforeIndex;
	}
	if (afterIndex !== undefined) {
		const item = items[afterIndex]!;
		const threshold = Math.floor(item.minimumSize / 2);
		const crossesThreshold = item.visible
			? options.delta >= maximumDelta + threshold
			: options.delta < maximumDelta - threshold;
		const keyboardCrossesEdge = options.input === "keyboard" && (item.visible ? options.delta > 0 && item.size <= item.minimumSize : options.delta < 0);
		if (crossesThreshold || keyboardCrossesEdge) return afterIndex;
	}
	return undefined;
}

function resizeLimits(items: readonly SplitViewResizeItem[], originalSizes: readonly number[], beforeIndexes: readonly number[], afterIndexes: readonly number[]): { readonly minimumDelta: number; readonly maximumDelta: number } {
	const minimumBefore = beforeIndexes.reduce((total, index) => total + effectiveMinimum(items[index]!) - originalSizes[index]!, 0);
	const maximumBefore = beforeIndexes.reduce((total, index) => total + items[index]!.maximumSize - originalSizes[index]!, 0);
	const maximumAfter = afterIndexes.reduce((total, index) => total + originalSizes[index]! - effectiveMinimum(items[index]!), 0);
	const minimumAfter = afterIndexes.reduce((total, index) => total + originalSizes[index]! - items[index]!.maximumSize, 0);
	return {
		minimumDelta: Math.max(minimumBefore, minimumAfter),
		maximumDelta: Math.min(maximumBefore, maximumAfter),
	};
}

function applyResize(items: SplitViewResizeItem[], originalSizes: readonly number[], beforeIndexes: readonly number[], afterIndexes: readonly number[], delta: number): void {
	let remaining = delta;
	for (const index of beforeIndexes) {
		const item = items[index]!;
		const size = clamp(originalSizes[index]! + remaining, effectiveMinimum(item), effectiveMaximum(item));
		remaining -= size - originalSizes[index]!;
		items[index] = { ...item, size };
	}
	remaining = delta;
	for (const index of afterIndexes) {
		const item = items[index]!;
		const size = clamp(originalSizes[index]! - remaining, effectiveMinimum(item), effectiveMaximum(item));
		remaining += size - originalSizes[index]!;
		items[index] = { ...item, size };
	}
}

function hasCapacity(items: readonly SplitViewResizeItem[], start: number, end: number, direction: "collapse" | "expand"): boolean {
	return items.slice(start, end).some((item) => direction === "collapse" ? item.size > effectiveMinimum(item) : effectiveMaximum(item) > item.size);
}

function effectiveMinimum(item: SplitViewResizeItem): number {
	return item.visible ? item.minimumSize : 0;
}

function effectiveMaximum(item: SplitViewResizeItem): number {
	return item.visible ? item.maximumSize : 0;
}

function indexesBefore(boundaryIndex: number): number[] {
	return Array.from({ length: boundaryIndex + 1 }, (_, index) => boundaryIndex - index);
}

function indexesAfter(boundaryIndex: number, itemCount: number): number[] {
	return Array.from({ length: itemCount - boundaryIndex - 1 }, (_, index) => boundaryIndex + index + 1);
}

function clamp(value: number, minimum: number, maximum: number): number {
	return Math.min(Math.max(value, minimum), maximum);
}

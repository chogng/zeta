import { AbstractDisposable, DisposableMap, DisposableOwner, type IDisposable } from "../../../base/common/lifecycle.js";
import { TextPosition, TextRange, type TextModelContentChange } from "../core/text.js";

export enum TrackedRangeStickiness {
	GrowsAtBothEdges = "growsAtBothEdges",
	GrowsOnlyAtStart = "growsOnlyAtStart",
	GrowsOnlyAtEnd = "growsOnlyAtEnd",
	NeverGrowsAtEdges = "neverGrowsAtEdges",
}

export interface TrackedRange extends IDisposable {
	readonly range: TextRange;
	readonly stickiness: TrackedRangeStickiness;
}

interface TrackedRangeRecord {
	startOffset: number;
	endOffset: number;
	readonly stickiness: TrackedRangeStickiness;
}

enum OffsetAffinity {
	Before,
	After,
}

export class TrackedRangeCollection extends DisposableOwner {
	private readonly handles = this.own(new DisposableMap<TrackedRangeRecord, TrackedRangeHandle>());

	constructor(
		private readonly positionAt: (offset: number) => TextPosition,
	) {
		super();
	}

	add(
		startOffset: number,
		endOffset: number,
		stickiness: TrackedRangeStickiness,
	): TrackedRange {
		this.assertNotDisposed();
		if (!isTrackedRangeStickiness(stickiness)) {
			throw new TypeError("Unknown tracked range stickiness");
		}
		const record: TrackedRangeRecord = {
			startOffset,
			endOffset,
			stickiness,
		};
		const trackedRange = new TrackedRangeHandle(
			record,
			this.positionAt,
			() => this.handles.deleteAndLeak(record),
		);
		return this.handles.set(record, trackedRange);
	}

	acceptChanges(
		changes: readonly TextModelContentChange[],
	): void {
		if (changes.length === 0) return;
		for (const record of this.handles.keys()) {
			const mapped = mapTrackedRange(
				record.startOffset,
				record.endOffset,
				record.stickiness,
				changes,
			);
			record.startOffset = mapped.startOffset;
			record.endOffset = mapped.endOffset;
		}
	}

}

class TrackedRangeHandle extends AbstractDisposable implements TrackedRange {
	constructor(
		private readonly record: TrackedRangeRecord,
		private readonly positionAt: (offset: number) => TextPosition,
		private readonly remove: () => void,
	) {
		super();
	}

	get range(): TextRange {
		this.assertNotDisposed();
		return TextRange.from(
			this.positionAt(this.record.startOffset),
			this.positionAt(this.record.endOffset),
		);
	}

	get stickiness(): TrackedRangeStickiness {
		this.assertNotDisposed();
		return this.record.stickiness;
	}

	protected override disposeCore(): void {
		this.remove();
	}

}

function mapTrackedRange(
	startOffset: number,
	endOffset: number,
	stickiness: TrackedRangeStickiness,
	changes: readonly TextModelContentChange[],
): {
	readonly startOffset: number;
	readonly endOffset: number;
} {
	const mappedBefore = mapOffset(
		startOffset,
		OffsetAffinity.Before,
		changes,
	);
	const mappedAfter = mapOffset(
		endOffset,
		OffsetAffinity.After,
		changes,
	);
	const growsAtStart =
		stickiness === TrackedRangeStickiness.GrowsAtBothEdges ||
		stickiness === TrackedRangeStickiness.GrowsOnlyAtStart;
	const growsAtEnd =
		stickiness === TrackedRangeStickiness.GrowsAtBothEdges ||
		stickiness === TrackedRangeStickiness.GrowsOnlyAtEnd;

	if (startOffset === endOffset) {
		const before = mapOffset(
			startOffset,
			OffsetAffinity.Before,
			changes,
		);
		const after = mapOffset(
			startOffset,
			OffsetAffinity.After,
			changes,
		);
		if (growsAtStart && growsAtEnd) {
			return { startOffset: before, endOffset: after };
		}
		const collapsedOffset = growsAtStart ? before : after;
		return {
			startOffset: collapsedOffset,
			endOffset: collapsedOffset,
		};
	}

	const mappedStart = growsAtStart
		? mappedBefore
		: mapOffset(startOffset, OffsetAffinity.After, changes);
	const mappedEnd = growsAtEnd
		? mappedAfter
		: mapOffset(endOffset, OffsetAffinity.Before, changes);
	if (mappedStart <= mappedEnd) {
		return {
			startOffset: mappedStart,
			endOffset: mappedEnd,
		};
	}
	if (growsAtStart && growsAtEnd) {
		return {
			startOffset: mappedEnd,
			endOffset: mappedStart,
		};
	}
	const collapsedOffset = growsAtStart
		? mappedEnd
		: mappedStart;
	return {
		startOffset: collapsedOffset,
		endOffset: collapsedOffset,
	};
}

function isTrackedRangeStickiness(
	value: TrackedRangeStickiness,
): boolean {
	return Object.values(TrackedRangeStickiness).includes(value);
}

function mapOffset(
	offset: number,
	affinity: OffsetAffinity,
	changes: readonly TextModelContentChange[],
): number {
	let cumulativeDelta = 0;
	for (const change of changes) {
		const startOffset = change.rangeOffset;
		const endOffset = startOffset + change.rangeLength;
		if (offset < startOffset) break;
		if (offset > endOffset) {
			cumulativeDelta +=
				change.text.length -
				change.rangeLength;
			continue;
		}
		if (startOffset === endOffset) {
			return startOffset +
				cumulativeDelta +
				(affinity === OffsetAffinity.After
					? change.text.length
					: 0);
		}
		if (offset === endOffset) {
			cumulativeDelta +=
				change.text.length -
				change.rangeLength;
			continue;
		}
		if (offset === startOffset) {
			return startOffset + cumulativeDelta;
		}
		return startOffset +
			cumulativeDelta +
			(affinity === OffsetAffinity.After
				? change.text.length
				: 0);
	}
	return offset + cumulativeDelta;
}

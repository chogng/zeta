export interface TextBufferSnapshot {
	readonly length: number;
	readonly lineCount: number;
	getText(): string;
	getTextBetweenOffsets(startOffset: number, endOffset: number): string;
}

export interface TextBufferSnapshotSegment {
	readonly source: string;
	readonly startOffset: number;
	readonly length: number;
}

export function createTextBufferSnapshot(segments: readonly TextBufferSnapshotSegment[], length: number, lineCount: number): TextBufferSnapshot {
	const capturedSegments = Object.freeze(segments.map(segment => Object.freeze({ ...segment })));
	return Object.freeze({
		length,
		lineCount,
		getText: () => capturedSegments.map(segment => segment.source.slice(segment.startOffset, segment.startOffset + segment.length)).join(''),
		getTextBetweenOffsets: (startOffset: number, endOffset: number) => readTextBetweenOffsets(capturedSegments, length, startOffset, endOffset),
	});
}

function readTextBetweenOffsets(segments: readonly TextBufferSnapshotSegment[], length: number, startOffset: number, endOffset: number): string {
	assertOffsetRange(startOffset, endOffset, length);
	if (startOffset === endOffset) return '';
	const parts: string[] = [];
	let segmentStartOffset = 0;
	for (const segment of segments) {
		const segmentEndOffset = segmentStartOffset + segment.length;
		if (segmentEndOffset > startOffset && segmentStartOffset < endOffset) {
			parts.push(segment.source.slice(
				segment.startOffset + Math.max(startOffset, segmentStartOffset) - segmentStartOffset,
				segment.startOffset + Math.min(endOffset, segmentEndOffset) - segmentStartOffset,
			));
		}
		if (segmentEndOffset >= endOffset) break;
		segmentStartOffset = segmentEndOffset;
	}
	return parts.join('');
}

function assertOffsetRange(startOffset: number, endOffset: number, length: number): void {
	if (!Number.isSafeInteger(startOffset) || !Number.isSafeInteger(endOffset) || startOffset < 0 || endOffset < startOffset || endOffset > length) {
		throw new RangeError(`Offsets must satisfy 0 <= start <= end <= ${length}`);
	}
}

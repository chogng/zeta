import type { TextBufferSnapshot } from './textBufferSnapshot.js';

export interface TextBufferPosition {
	readonly lineIndex: number;
	readonly columnIndex: number;
}

/** Incremental construction contract for large or streamed text sources. */
export interface TextBufferBuilder {
	acceptChunk(chunk: string): void;
	finish(): TextBuffer;
}

/** Internal text and physical-line storage contract owned by TextModel. */
export interface TextBuffer {
	readonly length: number;
	readonly lineCount: number;
	getText(): string;
	getTextInRange(startOffset: number, endOffset: number): string;
	getLineContent(lineIndex: number): string;
	getLineLength(lineIndex: number): number;
	offsetAt(lineIndex: number, columnIndex: number): number;
	positionAt(offset: number): TextBufferPosition;
	replace(startOffset: number, endOffset: number, text: string): void;
	createSnapshot(): TextBufferSnapshot;
	maintainIfNeeded(): boolean;
	needsMaintenance(): boolean;
	maintain(): void;
}

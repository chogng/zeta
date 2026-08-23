import { PieceTreeTextBufferBuilder } from "./pieceTreeTextBuffer/pieceTreeTextBufferBuilder.js";
import type { TextBuffer } from './textBuffer.js';

/** Selects the private TextBuffer implementation used by TextModel and worker mirrors. */
export function createTextBuffer(text: string): TextBuffer {
	const builder = new PieceTreeTextBufferBuilder();
	builder.acceptChunk(text);
	return builder.finish();
}

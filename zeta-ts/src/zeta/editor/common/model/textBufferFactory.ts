import { PieceTreeTextBufferBuilder } from "./pieceTreeTextBuffer/pieceTreeTextBufferBuilder.js";
import { DefaultEndOfLine, type ITextBuffer } from '../model.js';

/** Selects the private TextBuffer implementation used by TextModel and worker mirrors. */
export function createPieceTreeTextBuffer(text: string, defaultEOL: DefaultEndOfLine = DefaultEndOfLine.LF): ITextBuffer {
	const builder = new PieceTreeTextBufferBuilder();
	builder.acceptChunk(text);
	return builder.finish().create(defaultEOL).textBuffer;
}

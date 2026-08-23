import type { TextBufferBuilder } from "../textBuffer.js";
import { PieceTreeTextBuffer } from "./pieceTreeTextBuffer.js";

/** Incrementally collects text before constructing one PieceTreeTextBuffer. */
export class PieceTreeTextBufferBuilder implements TextBufferBuilder {
	private readonly chunks: string[] = [];
	private finished = false;

	acceptChunk(chunk: string): void {
		this.ensureOpen();
		if (chunk.length > 0) this.chunks.push(chunk);
	}

	finish(): PieceTreeTextBuffer {
		this.ensureOpen();
		this.finished = true;
		return new PieceTreeTextBuffer(this.chunks.join(""));
	}

	private ensureOpen(): void {
		if (this.finished) throw new Error("PieceTreeTextBufferBuilder has already finished");
	}
}

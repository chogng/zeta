import { PieceBuffer, PieceNode, canCoalesce, coalescePieces, createPiece, lowerBound, nodeLength, nodeLineFeeds, nodePieces, slicePiece, updateNodeAndAncestors, type Piece } from "./pieceTreeBase.js";
import { NodeColor, deleteNode, insertAfter, insertBefore, nextNode, previousNode, rightmost } from "./rbTreeBase.js";
import type { TextBuffer } from "../textBuffer.js";
import { createTextBufferSnapshot, type TextBufferSnapshot, type TextBufferSnapshotSegment } from "../textBufferSnapshot.js";

export interface PieceTreeTextBufferStatistics {
	readonly liveTextUnits: number;
	readonly retainedTextUnits: number;
	readonly reclaimableTextUnits: number;
	readonly pieceCount: number;
}

const MINIMUM_RECLAIMABLE_TEXT_UNITS = 64 * 1_024;
const MAXIMUM_RECLAIMABLE_TEXT_UNITS = 64 * 1_024 * 1_024;
const MAXIMUM_PIECE_COUNT = 4_096;

/**
 * Private piece-tree storage for `TextModel`.
 *
 * Pieces reference immutable original/add buffers. A red-black tree keeps
 * character length and line-feed counts on every subtree so edits and
 * coordinate queries do not rebuild a document-wide line index.
 */
export class PieceTreeTextBuffer implements TextBuffer {
	private originalBuffer: string;
	private addBuffer = "";
	private root: PieceNode | undefined;

	constructor(text: string) {
		this.originalBuffer = text;
		if (text.length > 0) this.root = this.createRootNode(createPiece(PieceBuffer.Original, 0, text));
	}

	get length(): number {
		return nodeLength(this.root);
	}

	get lineCount(): number {
		return nodeLineFeeds(this.root) + 1;
	}

	get pieceCount(): number {
		return nodePieces(this.root);
	}

	getStatistics(): PieceTreeTextBufferStatistics {
		const retainedTextUnits =
			this.originalBuffer.length +
			this.addBuffer.length;
		return Object.freeze({
			liveTextUnits: this.length,
			retainedTextUnits,
			reclaimableTextUnits: Math.max(
				0,
				retainedTextUnits - this.length,
			),
			pieceCount: this.pieceCount,
		});
	}

	getText(): string {
		const parts: string[] = [];
		this.collectText(this.root, parts);
		return parts.join("");
	}

	createSnapshot(): TextBufferSnapshot {
		const segments: TextBufferSnapshotSegment[] = [];
		this.collectSnapshotSegments(this.root, segments);
		return createTextBufferSnapshot(
			segments,
			this.length,
			this.lineCount,
		);
	}

	getTextInRange(startOffset: number, endOffset: number): string {
		this.assertRange(startOffset, endOffset);
		if (startOffset === endOffset) return "";
		const parts: string[] = [];
		this.collectRange(this.root, 0, startOffset, endOffset, parts);
		return parts.join("");
	}

	getLineContent(lineIndex: number): string {
		this.assertLineIndex(lineIndex);
		const startOffset = this.lineStartOffset(lineIndex);
		return this.getTextInRange(startOffset, this.lineEndOffset(lineIndex));
	}

	getLineLength(lineIndex: number): number {
		this.assertLineIndex(lineIndex);
		return this.lineEndOffset(lineIndex) - this.lineStartOffset(lineIndex);
	}

	offsetAt(lineIndex: number, columnIndex: number): number {
		this.assertLineIndex(lineIndex);
		assertSafeIndex(columnIndex, "columnIndex");
		const startOffset = this.lineStartOffset(lineIndex);
		const lineLength = this.lineEndOffset(lineIndex) - startOffset;
		if (columnIndex > lineLength) {
			throw new RangeError(
				`columnIndex ${columnIndex} exceeds line ${lineIndex} length ${lineLength}`,
			);
		}
		return startOffset + columnIndex;
	}

	positionAt(offset: number): {
		readonly lineIndex: number;
		readonly columnIndex: number;
	} {
		assertSafeIndex(offset, "offset");
		if (offset > this.length) {
			throw new RangeError(
				`offset must be a safe integer between 0 and ${this.length}`,
			);
		}
		const lineIndex = this.countLineFeedsBefore(this.root, offset);
		const startOffset = this.lineStartOffset(lineIndex);
		return {
			lineIndex,
			columnIndex: Math.min(
				offset - startOffset,
				this.lineEndOffset(lineIndex) - startOffset,
			),
		};
	}

	replace(startOffset: number, endOffset: number, text: string): void {
		this.assertRange(startOffset, endOffset);
		const endNode = this.ensureBoundary(endOffset);
		const startNode = this.ensureBoundary(startOffset);
		let current = startNode;
		while (current && current !== endNode) {
			const next = nextNode(current);
			this.root = deleteNode(this.root!, current);
			current = next;
		}

		if (text.length > 0) {
			const addStartOffset = this.addBuffer.length;
			this.addBuffer += text;
			const inserted = new PieceNode(createPiece(PieceBuffer.Add, addStartOffset, text));
			this.root = insertBefore(this.root, endNode, inserted);
			this.coalesceAround(inserted);
		} else if (endNode) {
			this.coalesceAround(endNode);
		}
	}

	compactIfNeeded(): boolean {
		if (!this.needsCompaction()) return false;
		this.compact();
		return true;
	}

	maintainIfNeeded(): boolean {
		return this.compactIfNeeded();
	}

	/** Reports whether retaining obsolete piece buffers exceeds the maintenance budget. */
	needsCompaction(): boolean {
		const statistics = this.getStatistics();
		const fragmented = statistics.pieceCount > MAXIMUM_PIECE_COUNT;
		const disproportionatelyRetained =
			statistics.reclaimableTextUnits >=
				MINIMUM_RECLAIMABLE_TEXT_UNITS &&
			statistics.retainedTextUnits >=
				statistics.liveTextUnits * 2;
		const absolutelyRetained =
			statistics.reclaimableTextUnits >=
				MAXIMUM_RECLAIMABLE_TEXT_UNITS;
		return fragmented || disproportionatelyRetained || absolutelyRetained;
	}

	needsMaintenance(): boolean {
		return this.needsCompaction();
	}

	compact(): void {
		const text = this.getText();
		this.originalBuffer = text;
		this.addBuffer = "";
		this.root = text.length > 0 ? this.createRootNode(createPiece(PieceBuffer.Original, 0, text)) : undefined;
	}

	maintain(): void {
		this.compact();
	}

	private collectText(
		node: PieceNode | undefined,
		parts: string[],
	): void {
		if (!node) return;
		this.collectText(node.left, parts);
		parts.push(this.pieceText(node.piece));
		this.collectText(node.right, parts);
	}

	private collectSnapshotSegments(
		node: PieceNode | undefined,
		segments: TextBufferSnapshotSegment[],
	): void {
		if (!node) return;
		this.collectSnapshotSegments(node.left, segments);
		segments.push({
			source: node.piece.buffer === PieceBuffer.Original
				? this.originalBuffer
				: this.addBuffer,
			startOffset: node.piece.startOffset,
			length: node.piece.length,
		});
		this.collectSnapshotSegments(node.right, segments);
	}

	private collectRange(
		node: PieceNode | undefined,
		baseOffset: number,
		startOffset: number,
		endOffset: number,
		parts: string[],
	): void {
		if (!node || startOffset >= endOffset) return;
		const leftLength = nodeLength(node.left);
		const pieceStartOffset = baseOffset + leftLength;
		const pieceEndOffset = pieceStartOffset + node.piece.length;
		if (startOffset < pieceStartOffset) {
			this.collectRange(
				node.left,
				baseOffset,
				startOffset,
				Math.min(endOffset, pieceStartOffset),
				parts,
			);
		}
		const intersectionStart = Math.max(startOffset, pieceStartOffset);
		const intersectionEnd = Math.min(endOffset, pieceEndOffset);
		if (intersectionStart < intersectionEnd) {
			const buffer = node.piece.buffer === PieceBuffer.Original ? this.originalBuffer : this.addBuffer;
			parts.push(buffer.slice(
				node.piece.startOffset + intersectionStart - pieceStartOffset,
				node.piece.startOffset + intersectionEnd - pieceStartOffset,
			));
		}
		if (endOffset > pieceEndOffset) {
			this.collectRange(
				node.right,
				pieceEndOffset,
				Math.max(startOffset, pieceEndOffset),
				endOffset,
				parts,
			);
		}
	}

	private lineStartOffset(lineIndex: number): number {
		return lineIndex === 0
			? 0
			: this.lineFeedOffset(lineIndex - 1) + 1;
	}

	private lineEndOffset(lineIndex: number): number {
		return lineIndex + 1 < this.lineCount
			? this.lineFeedOffset(lineIndex)
			: this.length;
	}

	private lineFeedOffset(lineFeedIndex: number): number {
		let node = this.root;
		let baseOffset = 0;
		let remaining = lineFeedIndex;
		while (node) {
			const leftLineFeeds = nodeLineFeeds(node.left);
			const leftLength = nodeLength(node.left);
			if (remaining < leftLineFeeds) {
				node = node.left;
				continue;
			}
			remaining -= leftLineFeeds;
			if (remaining < node.piece.lineFeedOffsets.length) {
				return baseOffset +
					leftLength +
					node.piece.lineFeedOffsets[remaining];
			}
			remaining -= node.piece.lineFeedOffsets.length;
			baseOffset += leftLength + node.piece.length;
			node = node.right;
		}
		throw new RangeError(`Unknown line feed index ${lineFeedIndex}`);
	}

	private countLineFeedsBefore(
		node: PieceNode | undefined,
		offset: number,
	): number {
		if (!node || offset <= 0) return 0;
		const leftLength = nodeLength(node.left);
		if (offset <= leftLength) {
			return this.countLineFeedsBefore(node.left, offset);
		}
		let count = nodeLineFeeds(node.left);
		const pieceOffset = Math.min(
			offset - leftLength,
			node.piece.length,
		);
		count += lowerBound(node.piece.lineFeedOffsets, pieceOffset);
		if (offset <= leftLength + node.piece.length) return count;
		return count + this.countLineFeedsBefore(
			node.right,
			offset - leftLength - node.piece.length,
		);
	}

	private ensureBoundary(offset: number): PieceNode | undefined {
		if (offset === this.length) return undefined;
		let node = this.root;
		let baseOffset = 0;
		while (node) {
			const leftLength = nodeLength(node.left);
			const pieceStartOffset = baseOffset + leftLength;
			const pieceEndOffset = pieceStartOffset + node.piece.length;
			if (offset < pieceStartOffset) {
				node = node.left;
				continue;
			}
			if (offset > pieceEndOffset) {
				baseOffset = pieceEndOffset;
				node = node.right;
				continue;
			}
			if (offset === pieceStartOffset) return node;
			if (offset === pieceEndOffset) return nextNode(node);

			const pieceOffset = offset - pieceStartOffset;
			const rightPiece = slicePiece(node.piece, pieceOffset, node.piece.length);
			node.piece = slicePiece(node.piece, 0, pieceOffset);
			updateNodeAndAncestors(node);
			const right = new PieceNode(rightPiece);
			this.root = insertAfter(this.root!, node, right);
			return right;
		}
		throw new Error(`Unable to resolve PieceTree boundary at offset ${offset}`);
	}

	private coalesceAround(node: PieceNode): void {
		let current = node;
		const previous = previousNode(current);
		if (previous && canCoalesce(previous.piece, current.piece)) {
			const combined = coalescePieces(previous.piece, current.piece);
			this.root = deleteNode(this.root!, current)!;
			previous.piece = combined;
			updateNodeAndAncestors(previous);
			current = previous;
		}
		const next = nextNode(current);
		if (next && canCoalesce(current.piece, next.piece)) {
			const combined = coalescePieces(current.piece, next.piece);
			this.root = deleteNode(this.root!, next)!;
			current.piece = combined;
			updateNodeAndAncestors(current);
		}
	}

	private createRootNode(piece: Piece): PieceNode {
		const node = new PieceNode(piece);
		node.color = NodeColor.Black;
		return node;
	}

	private pieceText(piece: Piece): string {
		const buffer = piece.buffer === PieceBuffer.Original
			? this.originalBuffer
			: this.addBuffer;
		return buffer.slice(
			piece.startOffset,
			piece.startOffset + piece.length,
		);
	}

	private assertRange(startOffset: number, endOffset: number): void {
		if (
			!Number.isSafeInteger(startOffset) ||
			!Number.isSafeInteger(endOffset) ||
			startOffset < 0 ||
			endOffset < startOffset ||
			endOffset > this.length
		) {
			throw new RangeError(
				`Offsets must satisfy 0 <= start <= end <= ${this.length}`,
			);
		}
	}

	private assertLineIndex(lineIndex: number): void {
		if (
			!Number.isSafeInteger(lineIndex) ||
			lineIndex < 0 ||
			lineIndex >= this.lineCount
		) {
			throw new RangeError(
				`lineIndex must be a safe integer between 0 and ${this.lineCount - 1}`,
			);
		}
	}
}

function assertSafeIndex(value: number, name: string): void {
	if (!Number.isSafeInteger(value) || value < 0) {
		throw new RangeError(`${name} must be a non-negative safe integer`);
	}
}

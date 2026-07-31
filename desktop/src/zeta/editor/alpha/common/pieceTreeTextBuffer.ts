import { PieceBuffer, PieceNode, canCoalesce, coalescePieces, createPiece, lowerBound, nodeLength, nodeLineFeeds, nodePieces, removeLeftmost, removeRightmost, slicePiece, updateNode, type Piece } from "./pieceTreeNode.js";
import { createTextBufferSnapshot, type TextBufferSnapshot, type TextBufferSnapshotSegment } from "./pieceTreeSnapshot.js";

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
 * Pieces reference immutable original/add buffers. A deterministic treap keeps
 * character length and line-feed counts on every subtree so edits and
 * coordinate queries do not rebuild a document-wide line index.
 */
export class PieceTreeTextBuffer {
  private originalBuffer: string;
  private addBuffer = "";
  private root: PieceNode | undefined;
  private priorityState = 0x6d2b79f5;

  constructor(text: string) {
    this.originalBuffer = text;
    if (text.length > 0) {
      this.root = this.createNode(
        createPiece(PieceBuffer.Original, 0, text),
      );
    }
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
    const [before, remainder] = this.split(this.root, startOffset);
    const [, after] = this.split(
      remainder,
      endOffset - startOffset,
    );
    let inserted: PieceNode | undefined;
    if (text.length > 0) {
      const addStartOffset = this.addBuffer.length;
      this.addBuffer += text;
      inserted = this.createNode(
        createPiece(PieceBuffer.Add, addStartOffset, text),
      );
    }
    this.root = this.mergeCoalescing(
      this.mergeCoalescing(before, inserted),
      after,
    );
  }

  compactIfNeeded(): boolean {
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
    if (
      !fragmented &&
      !disproportionatelyRetained &&
      !absolutelyRetained
    ) {
      return false;
    }
    this.compact();
    return true;
  }

  compact(): void {
    const text = this.getText();
    this.originalBuffer = text;
    this.addBuffer = "";
    this.root = text.length > 0
      ? this.createNode(createPiece(PieceBuffer.Original, 0, text))
      : undefined;
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
      const text = this.pieceText(node.piece);
      parts.push(text.slice(
        intersectionStart - pieceStartOffset,
        intersectionEnd - pieceStartOffset,
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

  private split(
    node: PieceNode | undefined,
    offset: number,
  ): [PieceNode | undefined, PieceNode | undefined] {
    if (!node) return [undefined, undefined];
    const leftLength = nodeLength(node.left);
    if (offset < leftLength) {
      const [before, after] = this.split(node.left, offset);
      node.left = after;
      updateNode(node);
      return [before, node];
    }
    const pieceEndOffset = leftLength + node.piece.length;
    if (offset > pieceEndOffset) {
      const [before, after] = this.split(
        node.right,
        offset - pieceEndOffset,
      );
      node.right = before;
      updateNode(node);
      return [node, after];
    }
    if (offset === leftLength) {
      const before = node.left;
      node.left = undefined;
      updateNode(node);
      return [before, node];
    }
    if (offset === pieceEndOffset) {
      const after = node.right;
      node.right = undefined;
      updateNode(node);
      return [node, after];
    }

    const pieceOffset = offset - leftLength;
    const leftPiece = slicePiece(node.piece, 0, pieceOffset);
    const rightPiece = slicePiece(
      node.piece,
      pieceOffset,
      node.piece.length,
    );
    const before = this.merge(
      node.left,
      this.createNode(leftPiece),
    );
    const after = this.merge(
      this.createNode(rightPiece),
      node.right,
    );
    return [before, after];
  }

  private merge(
    left: PieceNode | undefined,
    right: PieceNode | undefined,
  ): PieceNode | undefined {
    if (!left) return right;
    if (!right) return left;
    if (left.priority <= right.priority) {
      left.right = this.merge(left.right, right);
      updateNode(left);
      return left;
    }
    right.left = this.merge(left, right.left);
    updateNode(right);
    return right;
  }

  private mergeCoalescing(
    left: PieceNode | undefined,
    right: PieceNode | undefined,
  ): PieceNode | undefined {
    if (!left) return right;
    if (!right) return left;

    const [leftRemainder, leftBoundary] = removeRightmost(left);
    const [rightBoundary, rightRemainder] = removeLeftmost(right);
    if (!canCoalesce(leftBoundary.piece, rightBoundary.piece)) {
      return this.merge(
        this.merge(leftRemainder, leftBoundary),
        this.merge(rightBoundary, rightRemainder),
      );
    }

    const combined = this.createNode(
      coalescePieces(leftBoundary.piece, rightBoundary.piece),
    );
    return this.mergeCoalescing(
      this.mergeCoalescing(leftRemainder, combined),
      rightRemainder,
    );
  }

  private createNode(piece: Piece): PieceNode {
    return new PieceNode(piece, this.nextPriority());
  }

  private nextPriority(): number {
    let value = this.priorityState;
    value ^= value << 13;
    value ^= value >>> 17;
    value ^= value << 5;
    this.priorityState = value >>> 0;
    return this.priorityState;
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

export enum PieceBuffer {
	Original,
	Add,
}

export interface Piece {
	readonly buffer: PieceBuffer;
	readonly startOffset: number;
	readonly length: number;
	readonly lineFeedOffsets: readonly number[];
}

export class PieceNode {
	left: PieceNode | undefined;
	right: PieceNode | undefined;
	totalLength: number;
	totalLineFeeds: number;
	totalPieces: number;

	constructor(
		readonly piece: Piece,
		readonly priority: number,
	) {
		this.totalLength = piece.length;
		this.totalLineFeeds = piece.lineFeedOffsets.length;
		this.totalPieces = 1;
	}
}

export function createPiece(
	buffer: PieceBuffer,
	startOffset: number,
	text: string,
): Piece {
	const lineFeedOffsets: number[] = [];
	for (let index = 0; index < text.length; index += 1) {
		if (text.charCodeAt(index) === 10) lineFeedOffsets.push(index);
	}
	return {
		buffer,
		startOffset,
		length: text.length,
		lineFeedOffsets,
	};
}

export function slicePiece(
	piece: Piece,
	startOffset: number,
	endOffset: number,
): Piece {
	const firstLineFeed = lowerBound(piece.lineFeedOffsets, startOffset);
	const lastLineFeed = lowerBound(piece.lineFeedOffsets, endOffset);
	return {
		buffer: piece.buffer,
		startOffset: piece.startOffset + startOffset,
		length: endOffset - startOffset,
		lineFeedOffsets: piece.lineFeedOffsets
			.slice(firstLineFeed, lastLineFeed)
			.map(offset => offset - startOffset),
	};
}

export function canCoalesce(left: Piece, right: Piece): boolean {
	return left.buffer === right.buffer &&
		left.startOffset + left.length === right.startOffset;
}

export function coalescePieces(left: Piece, right: Piece): Piece {
	return {
		buffer: left.buffer,
		startOffset: left.startOffset,
		length: left.length + right.length,
		lineFeedOffsets: [
			...left.lineFeedOffsets,
			...right.lineFeedOffsets.map(offset => left.length + offset),
		],
	};
}

export function removeRightmost(
	node: PieceNode,
): [PieceNode | undefined, PieceNode] {
	if (!node.right) {
		const remainder = node.left;
		node.left = undefined;
		updateNode(node);
		return [remainder, node];
	}
	const [rightRemainder, rightmost] = removeRightmost(node.right);
	node.right = rightRemainder;
	updateNode(node);
	return [node, rightmost];
}

export function removeLeftmost(
	node: PieceNode,
): [PieceNode, PieceNode | undefined] {
	if (!node.left) {
		const remainder = node.right;
		node.right = undefined;
		updateNode(node);
		return [node, remainder];
	}
	const [leftmost, leftRemainder] = removeLeftmost(node.left);
	node.left = leftRemainder;
	updateNode(node);
	return [leftmost, node];
}

export function updateNode(node: PieceNode): void {
	node.totalLength =
		nodeLength(node.left) +
		node.piece.length +
		nodeLength(node.right);
	node.totalLineFeeds =
		nodeLineFeeds(node.left) +
		node.piece.lineFeedOffsets.length +
		nodeLineFeeds(node.right);
	node.totalPieces =
		nodePieces(node.left) +
		1 +
		nodePieces(node.right);
}

export function nodeLength(node: PieceNode | undefined): number {
	return node?.totalLength ?? 0;
}

export function nodeLineFeeds(node: PieceNode | undefined): number {
	return node?.totalLineFeeds ?? 0;
}

export function nodePieces(node: PieceNode | undefined): number {
	return node?.totalPieces ?? 0;
}

export function lowerBound(
	values: readonly number[],
	target: number,
): number {
	let low = 0;
	let high = values.length;
	while (low < high) {
		const middle = Math.floor((low + high) / 2);
		if (values[middle] < target) {
			low = middle + 1;
		} else {
			high = middle;
		}
	}
	return low;
}

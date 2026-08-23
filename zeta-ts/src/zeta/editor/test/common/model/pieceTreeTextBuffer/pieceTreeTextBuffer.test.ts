import assert from "node:assert/strict";
import test from "node:test";
import { CharCode } from "../../../../../base/common/charCode.js";
import { PieceNode } from "../../../../common/model/pieceTreeTextBuffer/pieceTreeBase.js";
import { NodeColor } from "../../../../common/model/pieceTreeTextBuffer/rbTreeBase.js";
import { PieceTreeTextBuffer } from "../../../../common/model/pieceTreeTextBuffer/pieceTreeTextBuffer.js";
import { PieceTreeTextBufferBuilder } from "../../../../common/model/pieceTreeTextBuffer/pieceTreeTextBufferBuilder.js";

test("PieceTreeTextBuffer matches a string oracle and red-black invariants across deterministic edits", () => {
	const random = createRandom(0x71ece);
	const buffer = new PieceTreeTextBuffer("seed\n😀text");
	let oracle = "seed\n😀text";
	const insertions = ["", "a", "XYZ", "\n", "x\ny", "😀"];
	assertTreeInvariants(buffer);

	for (let iteration = 0; iteration < 1_000; iteration += 1) {
		const startOffset = integer(random, oracle.length + 1);
		const endOffset = startOffset +
			integer(random, oracle.length - startOffset + 1);
		const insertedText = insertions[integer(random, insertions.length)];
		buffer.replace(startOffset, endOffset, insertedText);
		assertTreeInvariants(buffer);
		oracle =
			oracle.slice(0, startOffset) +
			insertedText +
			oracle.slice(endOffset);

		const rangeStart = integer(random, oracle.length + 1);
		const rangeEnd = rangeStart +
			integer(random, oracle.length - rangeStart + 1);
		const offset = integer(random, oracle.length + 1);
		const expectedPosition = positionAt(oracle, offset);
		const lineStarts = computeLineStarts(oracle);
		const lineIndex = integer(random, lineStarts.length);
		const lineEndOffset = lineIndex + 1 < lineStarts.length
			? lineStarts[lineIndex + 1] - 1
			: oracle.length;
		const columnIndex = integer(
			random,
			lineEndOffset - lineStarts[lineIndex] + 1,
		);

		assert.deepEqual({
			text: buffer.getText(),
			length: buffer.length,
			lineCount: buffer.lineCount,
			range: buffer.getTextInRange(rangeStart, rangeEnd),
			position: buffer.positionAt(offset),
			offset: buffer.offsetAt(lineIndex, columnIndex),
			line: buffer.getLineContent(lineIndex),
		}, {
			text: oracle,
			length: oracle.length,
			lineCount: lineStarts.length,
			range: oracle.slice(rangeStart, rangeEnd),
			position: expectedPosition,
			offset: lineStarts[lineIndex] + columnIndex,
			line: oracle.slice(lineStarts[lineIndex], lineEndOffset),
		});
	}
});

test("PieceTreeTextBufferBuilder constructs one buffer from ordered chunks", () => {
	const builder = new PieceTreeTextBufferBuilder();
	builder.acceptChunk("first\n");
	builder.acceptChunk("");
	builder.acceptChunk("second😀");
	const buffer = builder.finish();

	assert.deepEqual({
		text: buffer.getText(),
		lineCount: buffer.lineCount,
		position: buffer.positionAt(8),
	}, {
		text: "first\nsecond😀",
		lineCount: 2,
		position: { lineIndex: 1, columnIndex: 2 },
	});
	assert.throws(() => builder.acceptChunk("late"), /already finished/u);
	assert.throws(() => builder.finish(), /already finished/u);
});

test("PieceTreeTextBuffer coalesces contiguous source pieces", () => {
	const typed = new PieceTreeTextBuffer("");
	for (const character of "continuous") {
		typed.replace(typed.length, typed.length, character);
		assertTreeInvariants(typed);
	}
	assert.deepEqual({
		text: typed.getText(),
		pieceCount: typed.pieceCount,
	}, {
		text: "continuous",
		pieceCount: 1,
	});

	const restoredOriginal = new PieceTreeTextBuffer("abcdef");
	restoredOriginal.replace(3, 3, "X");
	assertTreeInvariants(restoredOriginal);
	assert.equal(restoredOriginal.pieceCount, 3);
	restoredOriginal.replace(3, 4, "");
	assertTreeInvariants(restoredOriginal);
	assert.deepEqual({
		text: restoredOriginal.getText(),
		pieceCount: restoredOriginal.pieceCount,
	}, {
		text: "abcdef",
		pieceCount: 1,
	});
});

test("PieceTreeTextBuffer compaction preserves captured sources", () => {
	const insertedText = "line\n".repeat(20_000);
	const retainedText = insertedText.slice(-10_000);
	const buffer = new PieceTreeTextBuffer("");
	buffer.replace(0, 0, insertedText);
	assertTreeInvariants(buffer);
	const snapshot = buffer.createSnapshot();
	buffer.replace(0, insertedText.length - retainedText.length, "");
	assertTreeInvariants(buffer);

	const before = buffer.getStatistics();
	assert.equal(buffer.compactIfNeeded(), true);
	assertTreeInvariants(buffer);
	const after = buffer.getStatistics();

	assert.deepEqual({
		text: buffer.getText(),
		lineCount: buffer.lineCount,
		before,
		after,
		capturedText: snapshot.getText(),
		secondCompaction: buffer.compactIfNeeded(),
	}, {
		text: retainedText,
		lineCount: 2_001,
		before: {
			liveTextUnits: retainedText.length,
			retainedTextUnits: insertedText.length,
			reclaimableTextUnits:
				insertedText.length - retainedText.length,
			pieceCount: 1,
		},
		after: {
			liveTextUnits: retainedText.length,
			retainedTextUnits: retainedText.length,
			reclaimableTextUnits: 0,
			pieceCount: 1,
		},
		capturedText: insertedText,
		secondCompaction: false,
	});
});

function assertTreeInvariants(buffer: PieceTreeTextBuffer): void {
	const root = (buffer as unknown as PieceTreeTextBufferInternals).root;
	if (root) {
		assert.equal(root.color, NodeColor.Black, "red-black root must be black");
		assert.equal(root.parent, undefined, "red-black root must not have a parent");
	}
	const aggregate = assertNode(root, undefined);
	assert.deepEqual({
		length: aggregate.length,
		lineFeeds: aggregate.lineFeeds,
		pieces: aggregate.pieces,
	}, {
		length: buffer.length,
		lineFeeds: buffer.lineCount - 1,
		pieces: buffer.pieceCount,
	});
}

function assertNode(node: PieceNode | undefined, parent: PieceNode | undefined): TreeAggregate {
	if (!node) return { length: 0, lineFeeds: 0, pieces: 0, blackHeight: 1 };
	assert.equal(node.parent, parent, "red-black parent pointer must match its owner");
	const left = assertNode(node.left, node);
	const right = assertNode(node.right, node);
	assert.equal(left.blackHeight, right.blackHeight, "red-black paths must have equal black height");
	if (node.color === NodeColor.Red) {
		assert.equal(node.left?.color ?? NodeColor.Black, NodeColor.Black, "a red node cannot have a red left child");
		assert.equal(node.right?.color ?? NodeColor.Black, NodeColor.Black, "a red node cannot have a red right child");
	}
	assert.ok(node.piece.length > 0, "piece-tree nodes must not retain empty pieces");
	const aggregate = {
		length: left.length + node.piece.length + right.length,
		lineFeeds: left.lineFeeds + node.piece.lineFeedOffsets.length + right.lineFeeds,
		pieces: left.pieces + 1 + right.pieces,
		blackHeight: left.blackHeight + (node.color === NodeColor.Black ? 1 : 0),
	};
	assert.equal(node.totalLength, aggregate.length, "node text-length aggregate must match its subtree");
	assert.equal(node.totalLineFeeds, aggregate.lineFeeds, "node line-feed aggregate must match its subtree");
	assert.equal(node.totalPieces, aggregate.pieces, "node piece-count aggregate must match its subtree");
	return aggregate;
}

interface PieceTreeTextBufferInternals {
	readonly root: PieceNode | undefined;
}

interface TreeAggregate {
	readonly length: number;
	readonly lineFeeds: number;
	readonly pieces: number;
	readonly blackHeight: number;
}

function positionAt(
	text: string,
	offset: number,
): {
	readonly lineIndex: number;
	readonly columnIndex: number;
} {
	const lineStarts = computeLineStarts(text);
	let lineIndex = 0;
	while (
		lineIndex + 1 < lineStarts.length &&
		lineStarts[lineIndex + 1] <= offset
	) {
		lineIndex += 1;
	}
	const lineEndOffset = lineIndex + 1 < lineStarts.length
		? lineStarts[lineIndex + 1] - 1
		: text.length;
	return {
		lineIndex,
		columnIndex: Math.min(
			offset - lineStarts[lineIndex],
			lineEndOffset - lineStarts[lineIndex],
		),
	};
}

function computeLineStarts(text: string): number[] {
	const starts = [0];
	for (let index = 0; index < text.length; index += 1) {
		if (text.charCodeAt(index) === CharCode.LineFeed) starts.push(index + 1);
	}
	return starts;
}

function createRandom(seed: number): () => number {
	let state = seed >>> 0;
	return () => {
		state = (Math.imul(state, 1_664_525) + 1_013_904_223) >>> 0;
		return state / 0x1_0000_0000;
	};
}

function integer(random: () => number, limit: number): number {
	return Math.floor(random() * limit);
}

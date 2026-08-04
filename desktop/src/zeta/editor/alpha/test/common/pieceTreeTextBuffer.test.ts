import assert from "node:assert/strict";
import test from "node:test";
import { PieceTreeTextBuffer } from "../../common/model/pieceTreeTextBuffer/pieceTreeTextBuffer.js";

test("PieceTreeTextBuffer matches a string oracle across random edits", () => {
  const random = createRandom(0x71ece);
  const buffer = new PieceTreeTextBuffer("seed\n😀text");
  let oracle = "seed\n😀text";
  const insertions = ["", "a", "XYZ", "\n", "x\ny", "😀"];

  for (let iteration = 0; iteration < 1_000; iteration += 1) {
    const startOffset = integer(random, oracle.length + 1);
    const endOffset = startOffset +
      integer(random, oracle.length - startOffset + 1);
    const insertedText = insertions[integer(random, insertions.length)];
    buffer.replace(startOffset, endOffset, insertedText);
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

test("PieceTreeTextBuffer coalesces contiguous source pieces", () => {
  const typed = new PieceTreeTextBuffer("");
  for (const character of "continuous") {
    typed.replace(typed.length, typed.length, character);
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
  assert.equal(restoredOriginal.pieceCount, 3);
  restoredOriginal.replace(3, 4, "");
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
  const snapshot = buffer.createSnapshot();
  buffer.replace(0, insertedText.length - retainedText.length, "");

  const before = buffer.getStatistics();
  assert.equal(buffer.compactIfNeeded(), true);
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
    if (text.charCodeAt(index) === 10) starts.push(index + 1);
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

import assert from "node:assert/strict";
import test from "node:test";
import { AlphaDecorationLineIndex } from "../../browser/decorationLineIndex.js";
import { AlphaDecorationPresentation, type AlphaResolvedDecoration } from "../../browser/decorationPresentation.js";
import { type TextDecorationId } from "../../common/decoration.js";
import { TextPosition, TextRange } from "../../common/text.js";

test("Decoration line index resolves visible intervals and preserves source order", () => {
  const index = new AlphaDecorationLineIndex([
    decoration(1, 4, 0, 4, 1),
    decoration(2, 1, 2, 3, 0),
    decoration(3, 0, 0, 0, 1),
    decoration(4, 7, 3, 7, 3),
  ]);

  assert.deepEqual(ids(index.getIntersectingLines(0, 1)), [2, 3]);
  assert.deepEqual(ids(index.getIntersectingLines(2, 2)), [2]);
  assert.deepEqual(ids(index.getIntersectingLines(3, 3)), []);
  assert.deepEqual(ids(index.getIntersectingLines(4, 4)), [1]);
  assert.deepEqual(ids(index.getIntersectingLines(7, 7)), [4]);
});

test("Decoration line index validates line queries", () => {
  const index = new AlphaDecorationLineIndex([]);
  assert.throws(() => index.getIntersectingLines(-1, 0), /non-negative ordered integer span/);
  assert.throws(() => index.getIntersectingLines(2, 1), /non-negative ordered integer span/);
  assert.throws(() => index.getIntersectingLines(0, 0.5), /non-negative ordered integer span/);
});

function decoration(id: number, startLineIndex: number, startColumnIndex: number, endLineIndex: number, endColumnIndex: number): AlphaResolvedDecoration {
  return Object.freeze({
    id: id as TextDecorationId,
    range: TextRange.from(TextPosition.at(startLineIndex, startColumnIndex), TextPosition.at(endLineIndex, endColumnIndex)),
    presentation: AlphaDecorationPresentation.ErrorUnderline,
  });
}

function ids(decorations: readonly AlphaResolvedDecoration[]): readonly number[] {
  return decorations.map(decoration => decoration.id as number);
}

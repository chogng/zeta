import assert from "node:assert/strict";
import test from "node:test";
import { DecorationLineIndex } from "../../browser/viewparts/decorations/decorations.js";
import { DecorationPresentation, type ResolvedDecoration } from "../../browser/viewparts/decorations/decorations.js";
import { type TextDecorationId } from "../../common/model/decorationCollection.js";
import { Position } from "../../common/core/position.js";
import { Range } from "../../common/core/range.js";

test("Decoration line index resolves visible intervals and preserves source order", () => {
	const index = new DecorationLineIndex([
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
	const index = new DecorationLineIndex([]);
	assert.throws(() => index.getIntersectingLines(-1, 0), /non-negative ordered integer span/);
	assert.throws(() => index.getIntersectingLines(2, 1), /non-negative ordered integer span/);
	assert.throws(() => index.getIntersectingLines(0, 0.5), /non-negative ordered integer span/);
});

function decoration(id: number, startLineIndex: number, startColumnIndex: number, endLineIndex: number, endColumnIndex: number): ResolvedDecoration {
	return Object.freeze({
		id: id as TextDecorationId,
		range: Range.fromPositions(new Position((startLineIndex) + 1, (startColumnIndex) + 1), new Position((endLineIndex) + 1, (endColumnIndex) + 1)),
		presentation: DecorationPresentation.ErrorUnderline,
	});
}

function ids(decorations: readonly ResolvedDecoration[]): readonly number[] {
	return decorations.map(decoration => decoration.id as number);
}

import assert from "node:assert/strict";
import test from "node:test";
import { Position } from "../../common/core/position.js";
import { Range } from "../../common/core/range.js";
import { TextModel } from "../../common/model/textModel.js";
import { getWordSelectionRange } from '../../common/cursor/wordSelection.js';

test("Word selection distinguishes words, whitespace, and punctuation", () => {
	using model = new TextModel("alpha  beta.\ntail");

	assert.deepEqual(
		getWordSelectionRange(model, new Position((0) + 1, (2) + 1)),
		range(0, 0, 5),
	);
	assert.deepEqual(
		getWordSelectionRange(model, new Position((0) + 1, (5) + 1)),
		range(0, 5, 7),
	);
	assert.deepEqual(
		getWordSelectionRange(model, new Position((0) + 1, (11) + 1)),
		range(0, 11, 12),
	);
	assert.deepEqual(
		getWordSelectionRange(model, new Position((1) + 1, (4) + 1)),
		range(1, 0, 4),
	);
});

test("Word selection preserves Unicode boundaries and empty lines", () => {
	using model = new TextModel("😀x\ne\u0301lan\n");

	assert.deepEqual(
		getWordSelectionRange(model, new Position((0) + 1, (1) + 1)),
		range(0, 0, 2),
	);
	assert.deepEqual(
		getWordSelectionRange(model, new Position((1) + 1, (1) + 1)),
		range(1, 0, 5),
	);
	assert.deepEqual(
		getWordSelectionRange(model, new Position((2) + 1, (0) + 1)),
		Range.fromPositions(new Position((2) + 1, (0) + 1)),
	);
});

test("Word selection honors a language word pattern before generic segmentation", () => {
	using model = new TextModel("crate::item-name");
	const pattern = /[A-Za-z:]+/;
	assert.deepEqual(getWordSelectionRange(model, new Position((0) + 1, (7) + 1), pattern), range(0, 0, 11));
	assert.deepEqual(getWordSelectionRange(model, new Position((0) + 1, (12) + 1), pattern), range(0, 12, 16));
});

test("Word selection rejects positions outside the model", () => {
	using model = new TextModel("alpha");

	assert.throws(
		() => getWordSelectionRange(model, new Position((0) + 1, (6) + 1)),
		/column/,
	);
	assert.throws(
		() => getWordSelectionRange(model, new Position((1) + 1, (0) + 1)),
		/lineNumber/,
	);
});

function range(lineIndex: number, startColumn: number, endColumn: number): Range {
	return Range.fromPositions(
		new Position((lineIndex) + 1, (startColumn) + 1),
		new Position((lineIndex) + 1, (endColumn) + 1),
	);
}

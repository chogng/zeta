import assert from "node:assert/strict";
import test from "node:test";
import { TextPosition, TextRange } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";
import { WordOperations } from "../../common/cursor/cursorWordOperations.js";

test("Word selection distinguishes words, whitespace, and punctuation", () => {
	using model = new TextModel("alpha  beta.\ntail");

	assert.deepEqual(
		WordOperations.getWordSelectionRange(model, TextPosition.at(0, 2)),
		range(0, 0, 5),
	);
	assert.deepEqual(
		WordOperations.getWordSelectionRange(model, TextPosition.at(0, 5)),
		range(0, 5, 7),
	);
	assert.deepEqual(
		WordOperations.getWordSelectionRange(model, TextPosition.at(0, 11)),
		range(0, 11, 12),
	);
	assert.deepEqual(
		WordOperations.getWordSelectionRange(model, TextPosition.at(1, 4)),
		range(1, 0, 4),
	);
});

test("Word selection preserves Unicode boundaries and empty lines", () => {
	using model = new TextModel("😀x\ne\u0301lan\n");

	assert.deepEqual(
		WordOperations.getWordSelectionRange(model, TextPosition.at(0, 1)),
		range(0, 0, 2),
	);
	assert.deepEqual(
		WordOperations.getWordSelectionRange(model, TextPosition.at(1, 1)),
		range(1, 0, 5),
	);
	assert.deepEqual(
		WordOperations.getWordSelectionRange(model, TextPosition.at(2, 0)),
		TextRange.emptyAt(TextPosition.at(2, 0)),
	);
});

test("Word selection honors a language word pattern before generic segmentation", () => {
	using model = new TextModel("crate::item-name");
	const pattern = /[A-Za-z:]+/;
	assert.deepEqual(WordOperations.getWordSelectionRange(model, TextPosition.at(0, 7), pattern), range(0, 0, 11));
	assert.deepEqual(WordOperations.getWordSelectionRange(model, TextPosition.at(0, 12), pattern), range(0, 12, 16));
});

test("Word selection rejects positions outside the model", () => {
	using model = new TextModel("alpha");

	assert.throws(
		() => WordOperations.getWordSelectionRange(model, TextPosition.at(0, 6)),
		/columnIndex/,
	);
	assert.throws(
		() => WordOperations.getWordSelectionRange(model, TextPosition.at(1, 0)),
		/lineIndex/,
	);
});

function range(lineIndex: number, startColumn: number, endColumn: number): TextRange {
	return TextRange.from(
		TextPosition.at(lineIndex, startColumn),
		TextPosition.at(lineIndex, endColumn),
	);
}

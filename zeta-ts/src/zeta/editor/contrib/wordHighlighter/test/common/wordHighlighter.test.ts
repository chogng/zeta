import assert from "node:assert/strict";
import test from "node:test";
import { getOccurrenceHighlightRanges } from "../../common/wordHighlighter.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition, TextRange } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Occurrence highlights find complete Unicode cursor words without matching substrings", () => {
	using model = new TextModel("café caféine café\nCafé");
	const ranges = getOccurrenceHighlightRanges(model, TextSelectionSet.single(caret(0, 2)));
	assert.deepEqual(ranges, [
		TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 4)),
		TextRange.from(TextPosition.at(0, 13), TextPosition.at(0, 17)),
	]);
});

test("Occurrence highlights use explicit single-line selections and ignore whitespace or multiline ranges", () => {
	using model = new TextModel("item itemized item\nitem");
	const explicit = TextSelectionSet.single(TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 2)));
	assert.deepEqual(getOccurrenceHighlightRanges(model, explicit), [
		TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 2)),
		TextRange.from(TextPosition.at(0, 5), TextPosition.at(0, 7)),
		TextRange.from(TextPosition.at(0, 14), TextPosition.at(0, 16)),
		TextRange.from(TextPosition.at(1, 0), TextPosition.at(1, 2)),
	]);
	assert.deepEqual(getOccurrenceHighlightRanges(model, TextSelectionSet.single(caret(0, 4))), []);
	assert.deepEqual(getOccurrenceHighlightRanges(model, TextSelectionSet.single(TextSelection.from(TextPosition.at(0, 0), TextPosition.at(1, 1)))), []);
});

function caret(lineIndex: number, columnIndex: number): TextSelection {
	return TextSelection.collapsedAt(TextPosition.at(lineIndex, columnIndex));
}

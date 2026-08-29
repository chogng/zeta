import assert from "node:assert/strict";
import test from "node:test";
import { TextPosition } from "../../common/core/text.js";
import { TextRange } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";
import { findNextTextMatch } from "../../common/model/textModelSearch.js";
import { findTextMatches } from "../../common/model/textModelSearch.js";
import { TextSearchPatternKind } from "../../common/model/textModelSearch.js";
import { TextSearchQueryError } from "../../common/model/textModelSearch.js";

test("literal search is case-insensitive by default and preserves UTF-16 ranges", () => {
	using model = new TextModel("Alpha 😀 alpha\nALPHA");

	const matches = findTextMatches(model, { pattern: "alpha" });

	assert.deepEqual(matches.map(match => ({
		start: [match.range.start.lineIndex, match.range.start.columnIndex],
		end: [match.range.end.lineIndex, match.range.end.columnIndex],
		text: match.text,
	})), [
		{ start: [0, 0], end: [0, 5], text: "Alpha" },
		{ start: [0, 9], end: [0, 14], text: "alpha" },
		{ start: [1, 0], end: [1, 5], text: "ALPHA" },
	]);
	assert.equal(findTextMatches(model, { pattern: "alpha", matchCase: true }).length, 1);
});

test("regular-expression search supports multiline captures and bounded ranges", () => {
	using model = new TextModel("before\nname: zeta\nafter\nname: alpha");
	const range = TextRange.from(TextPosition.at(1, 0), TextPosition.at(3, 11));

	const matches = findTextMatches(model, {
		pattern: "name: (zeta|alpha)",
		patternKind: TextSearchPatternKind.RegularExpression,
	}, { range, resultLimit: 1 });

	assert.equal(matches.length, 1);
	assert.equal(matches[0].text, "name: zeta");
	assert.deepEqual(matches[0].captures, ["zeta"]);
	assert.deepEqual(matches[0].range, TextRange.from(TextPosition.at(1, 0), TextPosition.at(1, 10)));
});

test("whole-word search uses Unicode word boundaries", () => {
	using model = new TextModel("cat scatter cat_ 猫 猫咪 猫");

	assert.deepEqual(
		findTextMatches(model, { pattern: "cat", wholeWord: true }).map(match => match.range.start.columnIndex),
		[0],
	);
	assert.deepEqual(
		findTextMatches(model, { pattern: "猫", wholeWord: true }).map(match => match.range.start.columnIndex),
		[17, 22],
	);
});

test('whole-word search obeys editor word separators', () => {
	using model = new TextModel('foo.bar foo-bar foobar');
	assert.deepEqual(
		findTextMatches(model, { pattern: 'foo', wholeWord: true, wordSeparators: '.-' }).map(match => match.range.start.columnIndex),
		[0, 8],
	);
	assert.deepEqual(
		findTextMatches(model, { pattern: 'foo', wholeWord: true, wordSeparators: '-' }).map(match => match.range.start.columnIndex),
		[8],
	);
});

test("zero-length regular expressions advance and next search wraps", () => {
	using model = new TextModel("a\nb");
	const lineStarts = findTextMatches(model, {
		pattern: "^",
		patternKind: TextSearchPatternKind.RegularExpression,
	});
	assert.deepEqual(lineStarts.map(match => match.range.start), [
		TextPosition.at(0, 0),
		TextPosition.at(1, 0),
	]);

	const wrapped = findNextTextMatch(model, { pattern: "a", matchCase: true }, TextPosition.at(1, 1));
	assert.deepEqual(wrapped?.range, TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 1)));
});

test("invalid expressions and limits fail before scanning", () => {
	using model = new TextModel("text");
	assert.throws(() => findTextMatches(model, {
		pattern: "(",
		patternKind: TextSearchPatternKind.RegularExpression,
	}), TextSearchQueryError);
	assert.throws(() => findTextMatches(model, { pattern: "text" }, { resultLimit: -1 }), RangeError);
	assert.deepEqual(findTextMatches(model, { pattern: "text" }, { resultLimit: 0 }), []);
});

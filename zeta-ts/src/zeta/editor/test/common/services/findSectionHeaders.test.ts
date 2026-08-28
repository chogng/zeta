import assert from "node:assert/strict";
import test from "node:test";
import { TextPosition, TextRange } from "../../../common/core/text.js";
import { TextModel } from "../../../common/model/textModel.js";
import { findSectionHeaders } from "../../../common/services/findSectionHeaders.js";

test("section headers include named regions but not ordinary foldable lines", () => {
	using model = new TextModel("// #region -- Runtime --\nfunction run() {\n}\n// #endregion");
	const headers = findSectionHeaders(model, {
		foldingMarkers: { start: /^\s*\/\/\s*#?region\b/iu, end: /^\s*\/\/\s*#?endregion\b/iu },
		findRegionSectionHeaders: true,
		findMarkSectionHeaders: false,
		markSectionHeaderRegex: "",
	});

	assert.deepEqual(headers, [{
		range: TextRange.from(TextPosition.at(0, 10), TextPosition.at(0, 24)),
		text: " Runtime ",
		hasSeparatorLine: true,
		shouldBeInComments: false,
	}]);
});

test("section headers collect single-line and multiline MARK matches", () => {
	using model = new TextModel("const value = 1; // MARK: API\n/* MARK:\n * Details */\nconst other = 2;");
	const headers = findSectionHeaders(model, {
		findRegionSectionHeaders: false,
		findMarkSectionHeaders: true,
		markSectionHeaderRegex: "MARK:[ \\t]*(?<separator>-?)[ \\t]*(?<label>[^\\n*]+|\\n\\s*\\*\\s*[^*]+)",
	});

	assert.equal(headers.length, 2);
	assert.equal(headers[0]?.text, "API");
	assert.equal(headers[0]?.range.start.lineIndex, 0);
	assert.equal(headers[1]?.range.start.lineIndex, 1);
	assert.equal(headers[1]?.range.end.lineIndex, 2);
});

test("section headers reject invalid MARK expressions", () => {
	using model = new TextModel("// MARK: Value");
	assert.throws(() => findSectionHeaders(model, {
		findRegionSectionHeaders: false,
		findMarkSectionHeaders: true,
		markSectionHeaderRegex: "(",
	}), /valid regular expression/);
});

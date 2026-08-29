import assert from "node:assert/strict";
import test from "node:test";
import { LanguageBracketPairs } from "../../../common/languages/languageBracketPairs.js";
import { LanguageConfigurationRegistry } from "../../../common/languages/languageConfiguration.js";
import { LanguageLexicalContextIndex } from "../../../common/languages/languageLexicalContext.js";
import { TextPosition, TextRange } from "../../../common/core/text.js";
import { TextModel } from "../../../common/model/textModel.js";

test("Language bracket pairs resolve nested cross-line configured pairs", () => {
	using model = new TextModel("function value() {\n  return [call(1)];\n}");
	using configurations = bracketConfigurations();
	using lexical = new LanguageLexicalContextIndex(model, "typescript", configurations);
	using bracketPairs = new LanguageBracketPairs(model, lexical);

	assert.deepEqual(bracketPairs.matchBracket(TextPosition.at(0, 17)), {
		opening: TextRange.from(TextPosition.at(0, 17), TextPosition.at(0, 18)),
		closing: TextRange.from(TextPosition.at(2, 0), TextPosition.at(2, 1)),
	});
	assert.deepEqual(bracketPairs.matchBracket(TextPosition.at(1, 16)), {
		opening: TextRange.from(TextPosition.at(1, 14), TextPosition.at(1, 15)),
		closing: TextRange.from(TextPosition.at(1, 16), TextPosition.at(1, 17)),
	});
	assert.deepEqual(bracketPairs.getBracketPairsInLineRange(1, 1).map(bracket => bracket.token), ['{', '[', '(']);
});

test("Language bracket pairs ignore strings and comments, and invalidate on edits", () => {
	using model = new TextModel("const text = \"{\"; // [\n{");
	using configurations = bracketConfigurations();
	using lexical = new LanguageLexicalContextIndex(model, "typescript", configurations);
	using bracketPairs = new LanguageBracketPairs(model, lexical);
	assert.equal(bracketPairs.matchBracket(TextPosition.at(0, 14)), undefined);
	assert.equal(bracketPairs.matchBracket(TextPosition.at(0, 21)), undefined);
	assert.equal(bracketPairs.matchBracket(TextPosition.at(1, 0)), undefined);

	model.applyEdits([{
		range: TextRange.emptyAt(TextPosition.at(1, 1)),
		text: "\n}",
	}]);
	assert.deepEqual(bracketPairs.matchBracket(TextPosition.at(1, 0)), {
		opening: TextRange.from(TextPosition.at(1, 0), TextPosition.at(1, 1)),
		closing: TextRange.from(TextPosition.at(2, 0), TextPosition.at(2, 1)),
	});
});

test("Language bracket pairs expose enclosing, next, and invalid bracket structure", () => {
	using model = new TextModel("{ value ([)] }");
	using configurations = bracketConfigurations();
	using lexical = new LanguageLexicalContextIndex(model, "typescript", configurations);
	using bracketPairs = new LanguageBracketPairs(model, lexical);
	assert.deepEqual(bracketPairs.findEnclosingBrackets(TextPosition.at(0, 3)), {
		opening: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 1)),
		closing: TextRange.from(TextPosition.at(0, 13), TextPosition.at(0, 14)),
	});
	assert.equal(bracketPairs.findNextBracket(TextPosition.at(0, 8))?.token, "(");
	assert.deepEqual(bracketPairs.getLineBrackets(0).filter(bracket => bracket.isInvalid).map(bracket => bracket.token), ["[", "]"]);
});

test("Language bracket pairs invalidate when the language configuration changes", () => {
	using model = new TextModel("{}");
	using configurations = new LanguageConfigurationRegistry();
	using registration = configurations.registerMany([{
		languageId: "typescript",
		configuration: { brackets: [{ open: "{", close: "}" }] },
	}]);
	using lexical = new LanguageLexicalContextIndex(model, "typescript", configurations);
	using bracketPairs = new LanguageBracketPairs(model, lexical);
	assert.equal(bracketPairs.getLineBrackets(0).length, 2);

	registration.replace([{ languageId: "typescript", configuration: { brackets: [] } }]);
	assert.deepEqual(bracketPairs.getLineBrackets(0), []);
});

function bracketConfigurations(): LanguageConfigurationRegistry {
	const configurations = new LanguageConfigurationRegistry();
	configurations.register("typescript", {
		comments: { lineComment: "//", blockComment: { open: "/*", close: "*/" } },
		brackets: [
			{ open: "(", close: ")" },
			{ open: "[", close: "]" },
			{ open: "{", close: "}" },
		],
	});
	return configurations;
}

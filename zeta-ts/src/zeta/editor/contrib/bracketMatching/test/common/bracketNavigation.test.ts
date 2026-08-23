import assert from "node:assert/strict";
import test from "node:test";
import { jumpToMatchingBrackets, selectToMatchingBrackets } from "../../common/bracketNavigation.js";
import { LanguageConfigurationRegistry } from "../../../../common/languages/languageConfiguration.js";
import { LanguageBracketMatcher } from "../../common/bracketMatching.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Bracket navigation jumps and selects lexical configured pairs without changing text", () => {
	using model = new TextModel("{\n  (value)\n}");
	using configurations = bracketConfigurations();
	using matcher = new LanguageBracketMatcher(model, "typescript", configurations);
	const selections = TextSelectionSet.withPrimary([
		TextSelection.collapsedAt(TextPosition.at(0, 0)),
		TextSelection.collapsedAt(TextPosition.at(1, 8)),
	], 1);

	const jumped = jumpToMatchingBrackets(matcher, selections);
	assert.deepEqual(jumped, TextSelectionSet.withPrimary([
		TextSelection.collapsedAt(TextPosition.at(2, 0)),
		TextSelection.collapsedAt(TextPosition.at(1, 2)),
	], 1));
	assert.deepEqual(selectToMatchingBrackets(matcher, jumped), TextSelectionSet.withPrimary([
		TextSelection.from(TextPosition.at(0, 0), TextPosition.at(2, 1)),
		TextSelection.from(TextPosition.at(1, 2), TextPosition.at(1, 9)),
	], 1));
	assert.equal(model.getText(), "{\n  (value)\n}");
});

test("Bracket navigation leaves selections without a lexical match unchanged", () => {
	using model = new TextModel("// ( text");
	using configurations = bracketConfigurations();
	using matcher = new LanguageBracketMatcher(model, "typescript", configurations);
	const selections = TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 3)));
	assert.equal(jumpToMatchingBrackets(matcher, selections), selections);
	assert.equal(selectToMatchingBrackets(matcher, selections), selections);
});

function bracketConfigurations(): LanguageConfigurationRegistry {
	const configurations = new LanguageConfigurationRegistry();
	configurations.register("typescript", {
		comments: { lineComment: "//", blockComment: { open: "/*", close: "*/" } },
		brackets: [{ open: "(", close: ")" }, { open: "{", close: "}" }],
	});
	return configurations;
}

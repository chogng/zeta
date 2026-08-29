import assert from "node:assert/strict";
import test from "node:test";
import { createRemoveMatchingBracketsCommand } from "../../common/bracketEditing.js";
import { EditorSelectionController } from "../../../../common/cursor/cursor.js";
import { LanguageConfigurationRegistry } from "../../../../common/languages/languageConfiguration.js";
import { LanguageBracketPairs } from "../../../../common/languages/languageBracketPairs.js";
import { LanguageLexicalContextIndex } from "../../../../common/languages/languageLexicalContext.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Remove matching brackets deletes distinct lexical pairs atomically and restores undo", () => {
	using model = new TextModel("{(value)}");
	using configurations = configurationsForBrackets();
	using lexical = new LanguageLexicalContextIndex(model, "typescript", configurations);
	using bracketPairs = new LanguageBracketPairs(model, lexical);
	using selections = new EditorSelectionController(model, TextSelectionSet.withPrimary([
		TextSelection.collapsedAt(TextPosition.at(0, 0)),
		TextSelection.collapsedAt(TextPosition.at(0, 1)),
	], 1));
	const command = createRemoveMatchingBracketsCommand(bracketPairs, selections.selections);
	assert.ok(command);
	selections.execute(command);
	assert.equal(model.getText(), "value");
	assert.deepEqual(selections.selections, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
	selections.undo();
	assert.equal(model.getText(), "{(value)}");
});

test("Remove matching brackets leaves non-bracket or range selections alone", () => {
	using model = new TextModel("// ()");
	using configurations = configurationsForBrackets();
	using lexical = new LanguageLexicalContextIndex(model, "typescript", configurations);
	using bracketPairs = new LanguageBracketPairs(model, lexical);
	const cursor = TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 3)));
	assert.equal(createRemoveMatchingBracketsCommand(bracketPairs, cursor), undefined);
	const range = TextSelectionSet.single(TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 2)));
	assert.equal(createRemoveMatchingBracketsCommand(bracketPairs, range), undefined);
});

function configurationsForBrackets(): LanguageConfigurationRegistry {
	const configurations = new LanguageConfigurationRegistry();
	configurations.register("typescript", {
		comments: { lineComment: "//", blockComment: { open: "/*", close: "*/" } },
		brackets: [{ open: "(", close: ")" }, { open: "{", close: "}" }],
	});
	return configurations;
}

import assert from "node:assert/strict";
import test from "node:test";
import { createRemoveMatchingBracketsCommand } from "../../common/bracketEditing.js";
import { CursorsController } from "../../../../common/cursor/cursor.js";
import { OwnedLanguageConfigurationContributions } from "../../../../common/languages/ownedLanguageConfigurationContributions.js";
import { LanguageBracketPairs } from "../../../../common/languages/languageBracketPairs.js";
import { LanguageLexicalContextIndex } from "../../../../common/languages/languageLexicalContext.js";
import { Selection } from "../../../../common/core/selection.js";
import { SelectionSet } from "../../../../common/cursor/selectionSet.js";
import { Position } from "../../../../common/core/position.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Remove matching brackets deletes distinct lexical pairs atomically and restores undo", () => {
	using model = new TextModel("{(value)}");
	using configurations = configurationsForBrackets();
	using lexical = new LanguageLexicalContextIndex(model, "typescript", configurations);
	using bracketPairs = new LanguageBracketPairs(model, lexical);
	using selections = new CursorsController(model, SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (0) + 1)),
		Selection.fromPositions(new Position((0) + 1, (1) + 1)),
	], 1));
	const command = createRemoveMatchingBracketsCommand(bracketPairs, selections.selections);
	assert.ok(command);
	selections.execute(command);
	assert.equal(model.getText(), "value");
	assert.deepEqual(selections.selections, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1))));
	selections.undo();
	assert.equal(model.getText(), "{(value)}");
});

test("Remove matching brackets leaves non-bracket or range selections alone", () => {
	using model = new TextModel("// ()");
	using configurations = configurationsForBrackets();
	using lexical = new LanguageLexicalContextIndex(model, "typescript", configurations);
	using bracketPairs = new LanguageBracketPairs(model, lexical);
	const cursor = SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (3) + 1)));
	assert.equal(createRemoveMatchingBracketsCommand(bracketPairs, cursor), undefined);
	const range = SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (2) + 1)));
	assert.equal(createRemoveMatchingBracketsCommand(bracketPairs, range), undefined);
});

function configurationsForBrackets(): OwnedLanguageConfigurationContributions {
	const configurations = new OwnedLanguageConfigurationContributions();
	configurations.register("typescript", {
		comments: { lineComment: "//", blockComment: { open: "/*", close: "*/" } },
		brackets: [{ open: "(", close: ")" }, { open: "{", close: "}" }],
	});
	return configurations;
}

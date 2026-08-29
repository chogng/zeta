import assert from "node:assert/strict";
import test from "node:test";
import { BracketColorizationSource } from "../../browser/bracketColorizationPresentation.js";
import { LanguageBracketPairs } from "../../../../common/languages/languageBracketPairs.js";
import { LanguageConfigurationRegistry } from "../../../../common/languages/languageConfiguration.js";
import { LanguageLexicalContextIndex } from "../../../../common/languages/languageLexicalContext.js";
import { TextRange } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Bracket colorization follows lexical nesting and excludes brackets in strings", () => {
	using model = new TextModel("{\n  (\"}\")\n}");
	using configurations = new LanguageConfigurationRegistry();
	using registration = configurations.register("typescript", {
		brackets: [{ open: "{", close: "}" }, { open: "(", close: ")" }],
	});
	using lexical = new LanguageLexicalContextIndex(model, "typescript", configurations);
	using bracketPairs = new LanguageBracketPairs(model, lexical);
	const colors = new BracketColorizationSource(bracketPairs);

	assert.deepEqual(colors.getLineBrackets(0), [{ startColumn: 0, endColumn: 1, level: 1 }]);
	assert.deepEqual(colors.getLineBrackets(1), [
		{ startColumn: 2, endColumn: 3, level: 2 },
		{ startColumn: 6, endColumn: 7, level: 2 },
	]);
	assert.deepEqual(colors.getLineBrackets(2), [{ startColumn: 0, endColumn: 1, level: 1 }]);
});

test("Bracket colorization invalidates its cached nesting after model edits", () => {
	using model = new TextModel("{\n}");
	using configurations = new LanguageConfigurationRegistry();
	using registration = configurations.register("typescript", { brackets: [{ open: "{", close: "}" }] });
	using lexical = new LanguageLexicalContextIndex(model, "typescript", configurations);
	using bracketPairs = new LanguageBracketPairs(model, lexical);
	const colors = new BracketColorizationSource(bracketPairs);
	assert.deepEqual(colors.getLineBrackets(1), [{ startColumn: 0, endColumn: 1, level: 1 }]);
	model.applyEdits([{ range: TextRange.emptyAt(model.positionAt(0)), text: "{\n" }]);
	assert.deepEqual(colors.getLineBrackets(2), [{ startColumn: 0, endColumn: 1, level: 2 }]);
});

test('Bracket guide projection remains available when bracket colors are disabled', () => {
	using model = new TextModel('{\n  value\n}');
	using configurations = new LanguageConfigurationRegistry();
	using registration = configurations.register('typescript', { brackets: [{ open: '{', close: '}' }] });
	using lexical = new LanguageLexicalContextIndex(model, 'typescript', configurations);
	using bracketPairs = new LanguageBracketPairs(model, lexical);
	const guides = new BracketColorizationSource(bracketPairs, false);

	assert.deepEqual(guides.getLineBrackets(0), []);
	assert.deepEqual(guides.getBracketGuides(1, 1), [{
		opening: TextRange.from(model.positionAt(0), model.positionAt(1)),
		closing: TextRange.from(model.positionAt(model.getText().length - 1), model.positionAt(model.getText().length)),
		level: 1,
	}]);
});

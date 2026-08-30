import { strict as assert } from "node:assert";
import test from "node:test";
import { CursorsController } from "../../../common/cursor/cursor.js";
import { LanguageAutoClosingTracker } from "../../../common/cursor/languageAutoClosingTracker.js";
import { registerBuiltinLanguageConfigurations } from "../../../common/languages/languageBuiltinConfigurations.js";
import { OwnedLanguageConfigurationContributions } from "../../../common/languages/ownedLanguageConfigurationContributions.js";
import { LanguageLexicalContextIndex } from "../../../common/languages/languageLexicalContext.js";
import { createLanguagePairBackspaceCommand, createLanguagePairTypeCommand } from "../../../common/cursor/languagePairEditing.js";
import { Selection } from "../../../common/core/selection.js";
import { SelectionSet } from "../../../common/cursor/selectionSet.js";
import { Position } from "../../../common/core/position.js";
import { TextModel } from "../../../common/model/textModel.js";

test("Language pair typing auto-closes and overtypes one existing closer", () => {
	using model = new TextModel("call");
	using selections = new CursorsController(model, SelectionSet.single(caret(4)));
	using configurations = new OwnedLanguageConfigurationContributions();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	using tracker = new LanguageAutoClosingTracker(model, selections);
	const configuration = configurations.getLanguageConfiguration("typescript");

	const opening = createLanguagePairTypeCommand(model, selections.selections, "(", configuration)!;
	assert.equal(opening.didInsertText, true);
	const openingChange = selections.execute(opening.command);
	assert.ok(openingChange);
	tracker.record(opening.autoClosingActions, openingChange.version);
	assert.equal(model.getText(), "call()");
	assert.deepEqual(selections.selections.primary, caret(5));
	const version = model.version;

	const closing = createLanguagePairTypeCommand(model, selections.selections, ")", configuration, { autoClosingTrust: tracker })!;
	assert.equal(closing.didInsertText, false);
	selections.execute(closing.command);
	assert.equal(model.getText(), "call()");
	assert.equal(model.version, version);
	assert.deepEqual(selections.selections.primary, caret(6));

	selections.undo();
	assert.equal(model.getText(), "call");
	assert.deepEqual(selections.selections.primary, caret(4));
});

test("Language pair backspace removes both empty sides and remains one undo step", () => {
	using model = new TextModel("");
	using selections = new CursorsController(model, SelectionSet.single(caret(0)));
	using configurations = new OwnedLanguageConfigurationContributions();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	using tracker = new LanguageAutoClosingTracker(model, selections);
	const configuration = configurations.getLanguageConfiguration("typescript");
	const opening = createLanguagePairTypeCommand(model, selections.selections, "[", configuration)!;
	const openingChange = selections.execute(opening.command);
	assert.ok(openingChange);
	tracker.record(opening.autoClosingActions, openingChange.version);

	const deletion = createLanguagePairBackspaceCommand(model, selections.selections, configuration, tracker);
	assert.ok(deletion);
	selections.execute(deletion);
	assert.equal(model.getText(), "");
	assert.deepEqual(selections.selections.primary, caret(0));

	selections.undo();
	assert.equal(model.getText(), "[]");
	assert.deepEqual(selections.selections.primary, caret(1));
});

test("Language pair typing surrounds directional selections and auto-closes collapsed cursors", () => {
	using model = new TextModel("alpha beta");
	const backward = Selection.fromPositions(new Position((0) + 1, (5) + 1), new Position((0) + 1, (0) + 1));
	using selections = new CursorsController(model, SelectionSet.withPrimary([backward, caret(10)], 1));
	using configurations = new OwnedLanguageConfigurationContributions();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	const result = createLanguagePairTypeCommand(model, selections.selections, "\"", configurations.getLanguageConfiguration("typescript"));

	assert.ok(result);
	selections.execute(result.command);
	assert.equal(model.getText(), "\"alpha\" beta\"\"");
	assert.deepEqual(selections.selections, SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (6) + 1), new Position((0) + 1, (1) + 1)),
		caret(13),
	], 1));

	selections.undo();
	assert.equal(model.getText(), "alpha beta");
	assert.deepEqual(selections.selections, SelectionSet.withPrimary([backward, caret(10)], 1));
});

test("Auto-closing respects following text and supports multi-token pairs", () => {
	using model = new TextModel("name");
	using selections = new CursorsController(model, SelectionSet.single(caret(0)));
	using configurations = new OwnedLanguageConfigurationContributions();
	using custom = configurations.register("template", {
		autoClosingPairs: [{ open: "<%", close: "%>" }],
		surroundingPairs: [{ open: "<%", close: "%>" }],
		autoCloseBefore: " ",
	});
	using tracker = new LanguageAutoClosingTracker(model, selections);
	const configuration = configurations.getLanguageConfiguration("template");

	const beforeText = createLanguagePairTypeCommand(model, selections.selections, "<%", configuration)!;
	selections.execute(beforeText.command);
	assert.equal(model.getText(), "<%name");
	assert.deepEqual(selections.selections.primary, caret(2));

	selections.setSelections(SelectionSet.single(caret(6)));
	const atEnd = createLanguagePairTypeCommand(model, selections.selections, "<%", configuration)!;
	const change = selections.execute(atEnd.command);
	assert.ok(change);
	tracker.record(atEnd.autoClosingActions, change.version);
	assert.equal(model.getText(), "<%name<%%>");
	assert.deepEqual(selections.selections.primary, caret(8));
	selections.execute(createLanguagePairBackspaceCommand(model, selections.selections, configuration, tracker)!);
	assert.equal(model.getText(), "<%name");
});

test("Auto-closing notIn keeps string and comment input single while code still pairs", () => {
	using configurations = new OwnedLanguageConfigurationContributions();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	const configuration = configurations.getLanguageConfiguration("typescript");

	using stringModel = new TextModel("\"value \"");
	using stringSelections = new CursorsController(stringModel, SelectionSet.single(caret(7)));
	using stringContext = new LanguageLexicalContextIndex(stringModel, "typescript", configurations);
	const stringQuote = createLanguagePairTypeCommand(stringModel, stringSelections.selections, "'", configuration, {
		lexicalContext: stringContext,
	})!;
	stringSelections.execute(stringQuote.command);
	assert.equal(stringModel.getText(), "\"value '\"");
	assert.deepEqual(stringQuote.autoClosingActions, []);

	using commentModel = new TextModel("// note ");
	using commentSelections = new CursorsController(commentModel, SelectionSet.single(caret(8)));
	using commentContext = new LanguageLexicalContextIndex(commentModel, "typescript", configurations);
	const commentQuote = createLanguagePairTypeCommand(commentModel, commentSelections.selections, "'", configuration, {
		lexicalContext: commentContext,
	})!;
	commentSelections.execute(commentQuote.command);
	assert.equal(commentModel.getText(), "// note '");

	using codeModel = new TextModel("");
	using codeSelections = new CursorsController(codeModel, SelectionSet.single(caret(0)));
	using codeContext = new LanguageLexicalContextIndex(codeModel, "typescript", configurations);
	const codeQuote = createLanguagePairTypeCommand(codeModel, codeSelections.selections, "'", configuration, {
		lexicalContext: codeContext,
	})!;
	codeSelections.execute(codeQuote.command);
	assert.equal(codeModel.getText(), "''");
	assert.equal(codeQuote.autoClosingActions.length, 1);
});

function caret(columnIndex: number): Selection {
	return Selection.fromPositions(new Position((0) + 1, (columnIndex) + 1));
}

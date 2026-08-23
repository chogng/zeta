import { strict as assert } from "node:assert";
import test from "node:test";
import { EditorSelectionController } from "../../../../common/cursor/editorSelectionController.js";
import { LanguageAutoClosingTracker } from "../../common/autoClosingTracker.js";
import { registerBuiltinLanguageConfigurations } from "../../../../common/languages/languageBuiltinConfigurations.js";
import { LanguageConfigurationRegistry } from "../../../../common/languages/languageConfiguration.js";
import { createLanguagePairBackspaceCommand, createLanguagePairTypeCommand, type LanguagePairTypeCommand } from "../../common/pairEditing.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition, TextRange, type TextModelChange } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Auto-closing trust follows external edits and rejects a changed closer", () => {
	using model = new TextModel("x");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(1)));
	using configurations = new LanguageConfigurationRegistry();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	using tracker = new LanguageAutoClosingTracker(model, selections);
	const configuration = configurations.getLanguageConfiguration("typescript");
	executeAndRecord(selections, tracker, createLanguagePairTypeCommand(model, selections.selections, "(", configuration)!);

	model.applyEdits([{ range: TextRange.emptyAt(TextPosition.at(0, 0)), text: "pre" }]);
	assert.deepEqual(selections.selections.primary, caret(5));
	assert.equal(tracker.canOvertype(TextPosition.at(0, 5), ")"), true);

	model.applyEdits([{ range: TextRange.from(TextPosition.at(0, 5), TextPosition.at(0, 6)), text: "]" }]);
	assert.equal(tracker.canOvertype(selections.selections.primary.active, ")"), false);
});

test("Leaving an auto-closed pair invalidates its trust permanently", () => {
	using model = new TextModel("");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(0)));
	using configurations = new LanguageConfigurationRegistry();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	using tracker = new LanguageAutoClosingTracker(model, selections);
	const configuration = configurations.getLanguageConfiguration("typescript");
	executeAndRecord(selections, tracker, createLanguagePairTypeCommand(model, selections.selections, "(", configuration)!);
	assert.equal(tracker.canOvertype(TextPosition.at(0, 1), ")"), true);

	selections.setSelections(TextSelectionSet.single(caret(0)));
	selections.setSelections(TextSelectionSet.single(caret(1)));
	assert.equal(tracker.canOvertype(TextPosition.at(0, 1), ")"), false);
});

test("User-authored pairs cannot be overtyped or pair-deleted", () => {
	using model = new TextModel("()");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(1)));
	using configurations = new LanguageConfigurationRegistry();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	using tracker = new LanguageAutoClosingTracker(model, selections);
	const configuration = configurations.getLanguageConfiguration("typescript");

	const closing = createLanguagePairTypeCommand(model, selections.selections, ")", configuration, { autoClosingTrust: tracker })!;
	assert.equal(closing.didInsertText, true);
	selections.execute(closing.command);
	assert.equal(model.getText(), "())");

	selections.setSelections(TextSelectionSet.single(caret(1)));
	assert.equal(createLanguagePairBackspaceCommand(model, selections.selections, configuration, tracker), undefined);
});

test("Multi-selection auto-closing entries retain independent ownership", () => {
	using model = new TextModel("a b");
	using selections = new EditorSelectionController(model, TextSelectionSet.withPrimary([caret(1), caret(3)], 0));
	using configurations = new LanguageConfigurationRegistry();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	using tracker = new LanguageAutoClosingTracker(model, selections);
	const configuration = configurations.getLanguageConfiguration("typescript");
	executeAndRecord(selections, tracker, createLanguagePairTypeCommand(model, selections.selections, "(", configuration)!);
	assert.equal(model.getText(), "a() b()");
	assert.equal(tracker.canOvertype(TextPosition.at(0, 2), ")"), true);
	assert.equal(tracker.canOvertype(TextPosition.at(0, 6), ")"), true);

	selections.setSelections(TextSelectionSet.single(caret(2)));
	assert.equal(tracker.canOvertype(TextPosition.at(0, 2), ")"), true);
	assert.equal(tracker.canOvertype(TextPosition.at(0, 6), ")"), false);
});

test("Undo removes provenance and redo does not invent it again", () => {
	using model = new TextModel("");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(0)));
	using configurations = new LanguageConfigurationRegistry();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	using tracker = new LanguageAutoClosingTracker(model, selections);
	const configuration = configurations.getLanguageConfiguration("typescript");
	executeAndRecord(selections, tracker, createLanguagePairTypeCommand(model, selections.selections, "(", configuration)!);

	selections.undo();
	assert.equal(model.getText(), "");
	selections.redo();
	assert.equal(model.getText(), "()");
	assert.equal(tracker.canOvertype(TextPosition.at(0, 1), ")"), false);
	assert.equal(createLanguagePairBackspaceCommand(model, selections.selections, configuration, tracker), undefined);
});

test("Stale recording is ignored and disposal leaves borrowed dependencies alive", () => {
	using model = new TextModel("");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(0)));
	using configurations = new LanguageConfigurationRegistry();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	const tracker = new LanguageAutoClosingTracker(model, selections);
	const configuration = configurations.getLanguageConfiguration("typescript");
	const command = createLanguagePairTypeCommand(model, selections.selections, "(", configuration)!;
	const change = selections.execute(command.command);
	assert.ok(change);
	model.applyEdits([{ range: TextRange.emptyAt(TextPosition.at(0, 0)), text: "x" }]);
	tracker.record(command.autoClosingActions, change.version);
	assert.equal(tracker.canOvertype(selections.selections.primary.active, ")"), false);

	tracker.dispose();
	assert.throws(() => tracker.canOvertype(TextPosition.at(0, 2), ")"), ReferenceError);
	selections.setSelections(TextSelectionSet.single(caret(0)));
	assert.equal(model.getText(), "x()");
});

function executeAndRecord(selections: EditorSelectionController, tracker: LanguageAutoClosingTracker, command: LanguagePairTypeCommand): TextModelChange {
	const change = selections.execute(command.command);
	assert.ok(change);
	tracker.record(command.autoClosingActions, change.version);
	return change;
}

function caret(columnIndex: number): TextSelection {
	return TextSelection.collapsedAt(TextPosition.at(0, columnIndex));
}

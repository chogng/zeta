import { strict as assert } from "node:assert";
import test from "node:test";
import { CursorsController } from "../../../common/cursor/cursor.js";
import { LanguageAutoClosingTracker } from "../../../common/cursor/languageAutoClosingTracker.js";
import { registerBuiltinLanguageConfigurations } from "../../../common/languages/languageBuiltinConfigurations.js";
import { TestLanguageConfigurationService } from '../modes/testLanguageConfigurationService.js';
import { createLanguagePairBackspaceCommand, createLanguagePairTypeCommand, type LanguagePairTypeCommand } from "../../../common/cursor/languagePairEditing.js";
import { Selection } from "../../../common/core/selection.js";
import { SelectionSet } from "../../../common/cursor/selectionSet.js";
import { Position } from "../../../common/core/position.js";
import { Range } from "../../../common/core/range.js";
import { type TextModelChange } from "../../../common/core/textChange.js";
import { TextModel } from "../../../common/model/textModel.js";

test("Auto-closing trust follows external edits and rejects a changed closer", () => {
	using model = new TextModel("x");
	using selections = new CursorsController(model, SelectionSet.single(caret(1)));
	using configurations = new TestLanguageConfigurationService();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	using tracker = new LanguageAutoClosingTracker(model, selections);
	const configuration = configurations.getLanguageConfiguration("typescript");
	executeAndRecord(selections, tracker, createLanguagePairTypeCommand(model, selections.selections, "(", configuration)!);

	model.applyEdits([{ range: Range.fromPositions(new Position((0) + 1, (0) + 1)), text: "pre" }]);
	assert.deepEqual(selections.selections.primary, caret(5));
	assert.equal(tracker.canOvertype(new Position((0) + 1, (5) + 1), ")"), true);

	model.applyEdits([{ range: Range.fromPositions(new Position((0) + 1, (5) + 1), new Position((0) + 1, (6) + 1)), text: "]" }]);
	assert.equal(tracker.canOvertype(selections.selections.primary.getPosition(), ")"), false);
});

test("Leaving an auto-closed pair invalidates its trust permanently", () => {
	using model = new TextModel("");
	using selections = new CursorsController(model, SelectionSet.single(caret(0)));
	using configurations = new TestLanguageConfigurationService();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	using tracker = new LanguageAutoClosingTracker(model, selections);
	const configuration = configurations.getLanguageConfiguration("typescript");
	executeAndRecord(selections, tracker, createLanguagePairTypeCommand(model, selections.selections, "(", configuration)!);
	assert.equal(tracker.canOvertype(new Position((0) + 1, (1) + 1), ")"), true);

	selections.setSelections(SelectionSet.single(caret(0)));
	selections.setSelections(SelectionSet.single(caret(1)));
	assert.equal(tracker.canOvertype(new Position((0) + 1, (1) + 1), ")"), false);
});

test("User-authored pairs cannot be overtyped or pair-deleted", () => {
	using model = new TextModel("()");
	using selections = new CursorsController(model, SelectionSet.single(caret(1)));
	using configurations = new TestLanguageConfigurationService();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	using tracker = new LanguageAutoClosingTracker(model, selections);
	const configuration = configurations.getLanguageConfiguration("typescript");

	const closing = createLanguagePairTypeCommand(model, selections.selections, ")", configuration, { autoClosingTrust: tracker })!;
	assert.equal(closing.didInsertText, true);
	selections.execute(closing.command);
	assert.equal(model.getText(), "())");

	selections.setSelections(SelectionSet.single(caret(1)));
	assert.equal(createLanguagePairBackspaceCommand(model, selections.selections, configuration, tracker), undefined);
});

test("Multi-selection auto-closing entries retain independent ownership", () => {
	using model = new TextModel("a b");
	using selections = new CursorsController(model, SelectionSet.withPrimary([caret(1), caret(3)], 0));
	using configurations = new TestLanguageConfigurationService();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	using tracker = new LanguageAutoClosingTracker(model, selections);
	const configuration = configurations.getLanguageConfiguration("typescript");
	executeAndRecord(selections, tracker, createLanguagePairTypeCommand(model, selections.selections, "(", configuration)!);
	assert.equal(model.getText(), "a() b()");
	assert.equal(tracker.canOvertype(new Position((0) + 1, (2) + 1), ")"), true);
	assert.equal(tracker.canOvertype(new Position((0) + 1, (6) + 1), ")"), true);

	selections.setSelections(SelectionSet.single(caret(2)));
	assert.equal(tracker.canOvertype(new Position((0) + 1, (2) + 1), ")"), true);
	assert.equal(tracker.canOvertype(new Position((0) + 1, (6) + 1), ")"), false);
});

test("Undo removes provenance and redo does not invent it again", () => {
	using model = new TextModel("");
	using selections = new CursorsController(model, SelectionSet.single(caret(0)));
	using configurations = new TestLanguageConfigurationService();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	using tracker = new LanguageAutoClosingTracker(model, selections);
	const configuration = configurations.getLanguageConfiguration("typescript");
	executeAndRecord(selections, tracker, createLanguagePairTypeCommand(model, selections.selections, "(", configuration)!);

	selections.undo();
	assert.equal(model.getText(), "");
	selections.redo();
	assert.equal(model.getText(), "()");
	assert.equal(tracker.canOvertype(new Position((0) + 1, (1) + 1), ")"), false);
	assert.equal(createLanguagePairBackspaceCommand(model, selections.selections, configuration, tracker), undefined);
});

test("Stale recording is ignored and disposal leaves borrowed dependencies alive", () => {
	using model = new TextModel("");
	using selections = new CursorsController(model, SelectionSet.single(caret(0)));
	using configurations = new TestLanguageConfigurationService();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	const tracker = new LanguageAutoClosingTracker(model, selections);
	const configuration = configurations.getLanguageConfiguration("typescript");
	const command = createLanguagePairTypeCommand(model, selections.selections, "(", configuration)!;
	const change = selections.execute(command.command);
	assert.ok(change);
	model.applyEdits([{ range: Range.fromPositions(new Position((0) + 1, (0) + 1)), text: "x" }]);
	tracker.record(command.autoClosingActions, change.version);
	assert.equal(tracker.canOvertype(selections.selections.primary.getPosition(), ")"), false);

	tracker.dispose();
	assert.throws(() => tracker.canOvertype(new Position((0) + 1, (2) + 1), ")"), ReferenceError);
	selections.setSelections(SelectionSet.single(caret(0)));
	assert.equal(model.getText(), "x()");
});

function executeAndRecord(selections: CursorsController, tracker: LanguageAutoClosingTracker, command: LanguagePairTypeCommand): TextModelChange {
	const change = selections.execute(command.command);
	assert.ok(change);
	tracker.record(command.autoClosingActions, change.version);
	return change;
}

function caret(columnIndex: number): Selection {
	return Selection.fromPositions(new Position((0) + 1, (columnIndex) + 1));
}

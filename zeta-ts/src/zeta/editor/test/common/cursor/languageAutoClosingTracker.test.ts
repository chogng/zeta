import { strict as assert } from "node:assert";
import test from "node:test";
import { CursorsController } from "../../../common/cursor/cursor.js";
import { DeleteOperations } from "../../../common/cursor/cursorDeleteOperations.js";
import { TypeOperations } from "../../../common/cursor/cursorTypeOperations.js";
import { registerBuiltinLanguageConfigurations } from "../../../common/languages/languageBuiltinConfigurations.js";
import { TestLanguageConfigurationService } from '../modes/testLanguageConfigurationService.js';
import { Selection } from "../../../common/core/selection.js";
import { Position } from "../../../common/core/position.js";
import { Range } from "../../../common/core/range.js";
import { TextModel } from "../../../common/model/textModel.js";
import { createTestCursorConfiguration } from '../testCursorConfiguration.js';
import { createTestCursorsController } from '../testCursorConfiguration.js';

test("Auto-closing trust follows external edits and rejects a changed closer", () => {
	using model = new TextModel("x");
	using selections = createTestCursorsController(model, [caret(1)]);
	using configurations = new TestLanguageConfigurationService();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	const configuration = configurations.getLanguageConfiguration("typescript");
	executeAndRecord(model, selections, typeCommand(model, selections, "(", configuration)!);

	model.applyEdits([{ range: Range.fromPositions(new Position((0) + 1, (0) + 1)), text: "pre" }]);
	assert.deepEqual(selections.getSelections()[0]!, caret(5));
	assert.equal(canOvertype(model, selections, new Position((0) + 1, (5) + 1), ")"), true);

	model.applyEdits([{ range: Range.fromPositions(new Position((0) + 1, (5) + 1), new Position((0) + 1, (6) + 1)), text: "]" }]);
	assert.equal(canOvertype(model, selections, selections.getSelections()[0]!.getPosition(), ")"), false);
});

test("Leaving an auto-closed pair invalidates its trust permanently", () => {
	using model = new TextModel("");
	using selections = createTestCursorsController(model, [caret(0)]);
	using configurations = new TestLanguageConfigurationService();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	const configuration = configurations.getLanguageConfiguration("typescript");
	executeAndRecord(model, selections, typeCommand(model, selections, "(", configuration)!);
	assert.equal(canOvertype(model, selections, new Position((0) + 1, (1) + 1), ")"), true);

	selections.setSelections([caret(0)]);
	selections.setSelections([caret(1)]);
	assert.equal(canOvertype(model, selections, new Position((0) + 1, (1) + 1), ")"), false);
});

test("User-authored pairs cannot be overtyped or pair-deleted", () => {
	using model = new TextModel("()", { languageId: 'typescript' });
	using selections = createTestCursorsController(model, [caret(1)]);
	using configurations = new TestLanguageConfigurationService();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	const configuration = configurations.getLanguageConfiguration("typescript");

	const closing = typeCommand(model, selections, ")", configuration)!;
	assert.equal(closing.insertedText, true);
	selections.execute(closing.command);
	assert.equal(model.getText(), "())");

	selections.setSelections([caret(1)]);
	executeBackspace(model, selections, configurations);
	assert.equal(model.getText(), "))");
});

test("Multi-selection auto-closing entries retain independent ownership", () => {
	using model = new TextModel("a b");
	using selections = createTestCursorsController(model, primaryFirst([caret(1), caret(3)], 0));
	using configurations = new TestLanguageConfigurationService();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	const configuration = configurations.getLanguageConfiguration("typescript");
	executeAndRecord(model, selections, typeCommand(model, selections, "(", configuration)!);
	assert.equal(model.getText(), "a() b()");
	assert.equal(canOvertype(model, selections, new Position((0) + 1, (2) + 1), ")"), true);
	assert.equal(canOvertype(model, selections, new Position((0) + 1, (6) + 1), ")"), true);

	selections.setSelections([caret(2)]);
	assert.equal(canOvertype(model, selections, new Position((0) + 1, (2) + 1), ")"), true);
	assert.equal(canOvertype(model, selections, new Position((0) + 1, (6) + 1), ")"), false);
});

test("Undo removes provenance and redo does not invent it again", () => {
	using model = new TextModel("", { languageId: 'typescript' });
	using selections = createTestCursorsController(model, [caret(0)]);
	using configurations = new TestLanguageConfigurationService();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	const configuration = configurations.getLanguageConfiguration("typescript");
	executeAndRecord(model, selections, typeCommand(model, selections, "(", configuration)!);

	selections.undo();
	assert.equal(model.getText(), "");
	selections.redo();
	assert.equal(model.getText(), "()");
	assert.equal(canOvertype(model, selections, new Position((0) + 1, (1) + 1), ")"), false);
	executeBackspace(model, selections, configurations);
	assert.equal(model.getText(), ")");
});

test("Stale recording is ignored and cursor disposal leaves the model alive", () => {
	using model = new TextModel("");
	using selections = createTestCursorsController(model, [caret(0)]);
	using configurations = new TestLanguageConfigurationService();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	const configuration = configurations.getLanguageConfiguration("typescript");
	const command = typeCommand(model, selections, "(", configuration)!;
	const change = selections.execute(command.command);
	assert.ok(change);
	model.applyEdits([{ range: Range.fromPositions(new Position((0) + 1, (0) + 1)), text: "x" }]);
	recordAutoClosed(model, selections, command, change.version);
	assert.equal(canOvertype(model, selections, selections.getSelections()[0]!.getPosition(), ")"), false);

	selections.dispose();
	assert.throws(() => selections.getAutoClosedCharacters(), ReferenceError);
	assert.equal(model.getText(), "x()");
});

function typeCommand(model: TextModel, selections: CursorsController, text: string, configuration: Parameters<typeof TypeOperations.typeWithInterceptors>[3]) {
	return TypeOperations.typeWithInterceptors(model, selections.getSelections(), text, configuration, selections.getAutoClosedCharacters());
}

function executeAndRecord(model: TextModel, selections: CursorsController, command: NonNullable<ReturnType<typeof TypeOperations.typeWithInterceptors>>): void {
	const change = selections.execute(command.command);
	assert.ok(change);
	recordAutoClosed(model, selections, command, change.version);
}

function executeBackspace(model: TextModel, selections: CursorsController, configurations: TestLanguageConfigurationService): void {
	const operation = DeleteOperations.deleteLeft(
		selections.getPrevEditOperationType(),
		createTestCursorConfiguration(model, configurations),
		model,
		[...selections.getSelections()],
		[...selections.getAutoClosedCharacters()],
	);
	if (operation[0]) selections.pushUndoStop();
	selections.executeCommands(operation[1]);
}

function recordAutoClosed(model: TextModel, selections: CursorsController, command: NonNullable<ReturnType<typeof TypeOperations.typeWithInterceptors>>, version: number): void {
	selections.recordAutoClosedCharacters(
		command.autoClosedCharacters.map(range => Range.fromPositions(model.positionAt(range.startOffset), model.positionAt(range.endOffset))),
		command.autoClosedEnclosing.map(range => Range.fromPositions(model.positionAt(range.startOffset), model.positionAt(range.endOffset))),
		version,
	);
}

function canOvertype(model: TextModel, selections: CursorsController, position: Position, close: string): boolean {
	return selections.getAutoClosedCharacters().some(range => Position.equals(range.getStartPosition(), position) && model.getTextInRange(range) === close);
}

function caret(columnIndex: number): Selection {
	return Selection.fromPositions(new Position((0) + 1, (columnIndex) + 1));
}

function primaryFirst<T>(items: readonly T[], primaryIndex: number): readonly T[] {
	if (primaryIndex === 0) return Object.freeze([...items]);
	return Object.freeze([items[primaryIndex]!, ...items.slice(0, primaryIndex), ...items.slice(primaryIndex + 1)]);
}

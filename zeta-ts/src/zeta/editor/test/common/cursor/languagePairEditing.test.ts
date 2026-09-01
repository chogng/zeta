import { strict as assert } from "node:assert";
import test from "node:test";
import { CursorsController } from "../../../common/cursor/cursor.js";
import { DeleteOperations } from "../../../common/cursor/cursorDeleteOperations.js";
import { TypeOperations } from "../../../common/cursor/cursorTypeOperations.js";
import { registerBuiltinLanguageConfigurations } from "../../../common/languages/languageBuiltinConfigurations.js";
import { TestLanguageConfigurationService } from '../modes/testLanguageConfigurationService.js';
import { LanguageLexicalContextIndex } from "../../../common/languages/languageLexicalContext.js";
import { Selection } from "../../../common/core/selection.js";
import { Position } from "../../../common/core/position.js";
import { Range } from "../../../common/core/range.js";
import { TextModel } from "../../../common/model/textModel.js";
import { createTestCursorConfiguration } from '../testCursorConfiguration.js';
import { createTestCursorsController } from '../testCursorConfiguration.js';

test("Language pair typing auto-closes and overtypes one existing closer", () => {
	using model = new TextModel("call");
	using selections = createTestCursorsController(model, [caret(4)]);
	using configurations = new TestLanguageConfigurationService();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	const configuration = configurations.getLanguageConfiguration("typescript");

	const opening = typeCommand(model, selections, "(", configuration)!;
	assert.equal(opening.insertedText, true);
	executeAndRecord(model, selections, opening);
	assert.equal(model.getText(), "call()");
	assert.deepEqual(selections.getSelections()[0]!, caret(5));
	const version = model.version;

	const closing = typeCommand(model, selections, ")", configuration)!;
	assert.equal(closing.insertedText, false);
	selections.execute(closing.command);
	assert.equal(model.getText(), "call()");
	assert.equal(model.version, version);
	assert.deepEqual(selections.getSelections()[0]!, caret(6));

	selections.undo();
	assert.equal(model.getText(), "call");
	assert.deepEqual(selections.getSelections()[0]!, caret(4));
});

test("Language pair backspace removes both empty sides and remains one undo step", () => {
	using model = new TextModel("", { languageId: 'typescript' });
	using selections = createTestCursorsController(model, [caret(0)]);
	using configurations = new TestLanguageConfigurationService();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	const configuration = configurations.getLanguageConfiguration("typescript");
	const opening = typeCommand(model, selections, "[", configuration)!;
	executeAndRecord(model, selections, opening);

	executeBackspace(model, selections, configurations);
	assert.equal(model.getText(), "");
	assert.deepEqual(selections.getSelections()[0]!, caret(0));

	selections.undo();
	assert.equal(model.getText(), "[]");
	assert.deepEqual(selections.getSelections()[0]!, caret(1));
});

test("Language pair typing surrounds directional selections and auto-closes collapsed cursors", () => {
	using model = new TextModel("alpha beta");
	const backward = Selection.fromPositions(new Position((0) + 1, (5) + 1), new Position((0) + 1, (0) + 1));
	using selections = createTestCursorsController(model, primaryFirst([backward, caret(10)], 1));
	using configurations = new TestLanguageConfigurationService();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	const result = typeCommand(model, selections, "\"", configurations.getLanguageConfiguration("typescript"));

	assert.ok(result);
	selections.execute(result.command);
	assert.equal(model.getText(), "\"alpha\" beta\"\"");
	assert.deepEqual(selections.getSelections(), primaryFirst([
		Selection.fromPositions(new Position((0) + 1, (6) + 1), new Position((0) + 1, (1) + 1)),
		caret(13),
	], 1));

	selections.undo();
	assert.equal(model.getText(), "alpha beta");
	assert.deepEqual(selections.getSelections(), primaryFirst([backward, caret(10)], 1));
});

test("Auto-closing respects following text and supports multi-token pairs", () => {
	using model = new TextModel("name", { languageId: 'template' });
	using selections = createTestCursorsController(model, [caret(0)]);
	using configurations = new TestLanguageConfigurationService();
	using custom = configurations.register("template", {
		autoClosingPairs: [{ open: "<%", close: "%>" }],
		surroundingPairs: [{ open: "<%", close: "%>" }],
		autoCloseBefore: " ",
	});
	const configuration = configurations.getLanguageConfiguration("template");

	const beforeText = typeCommand(model, selections, "<%", configuration)!;
	selections.execute(beforeText.command);
	assert.equal(model.getText(), "<%name");
	assert.deepEqual(selections.getSelections()[0]!, caret(2));

	selections.setSelections([caret(6)]);
	const atEnd = typeCommand(model, selections, "<%", configuration)!;
	executeAndRecord(model, selections, atEnd);
	assert.equal(model.getText(), "<%name<%%>");
	assert.deepEqual(selections.getSelections()[0]!, caret(8));
	executeBackspace(model, selections, configurations);
	assert.equal(model.getText(), "<%name");
});

test("Auto-closing notIn keeps string and comment input single while code still pairs", () => {
	using configurations = new TestLanguageConfigurationService();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	const configuration = configurations.getLanguageConfiguration("typescript");

	using stringModel = new TextModel("\"value \"");
	using stringSelections = createTestCursorsController(stringModel, [caret(7)]);
	using stringContext = new LanguageLexicalContextIndex(stringModel, "typescript", configurations);
	const stringQuote = typeCommand(stringModel, stringSelections, "'", configuration, stringContext)!;
	stringSelections.execute(stringQuote.command);
	assert.equal(stringModel.getText(), "\"value '\"");
	assert.deepEqual(stringQuote.autoClosedCharacters, []);

	using commentModel = new TextModel("// note ");
	using commentSelections = createTestCursorsController(commentModel, [caret(8)]);
	using commentContext = new LanguageLexicalContextIndex(commentModel, "typescript", configurations);
	const commentQuote = typeCommand(commentModel, commentSelections, "'", configuration, commentContext)!;
	commentSelections.execute(commentQuote.command);
	assert.equal(commentModel.getText(), "// note '");

	using codeModel = new TextModel("");
	using codeSelections = createTestCursorsController(codeModel, [caret(0)]);
	using codeContext = new LanguageLexicalContextIndex(codeModel, "typescript", configurations);
	const codeQuote = typeCommand(codeModel, codeSelections, "'", configuration, codeContext)!;
	codeSelections.execute(codeQuote.command);
	assert.equal(codeModel.getText(), "''");
	assert.equal(codeQuote.autoClosedCharacters.length, 1);
});

function typeCommand(
	model: TextModel,
	selections: CursorsController,
	text: string,
	configuration: Parameters<typeof TypeOperations.typeWithInterceptors>[3],
	lexicalContext?: Parameters<typeof TypeOperations.typeWithInterceptors>[5],
) {
	return TypeOperations.typeWithInterceptors(model, selections.getSelections(), text, configuration, selections.getAutoClosedCharacters(), lexicalContext);
}

function executeAndRecord(
	model: TextModel,
	selections: CursorsController,
	result: NonNullable<ReturnType<typeof TypeOperations.typeWithInterceptors>>,
): void {
	const change = selections.execute(result.command);
	assert.ok(change);
	selections.recordAutoClosedCharacters(
		result.autoClosedCharacters.map(range => Range.fromPositions(model.positionAt(range.startOffset), model.positionAt(range.endOffset))),
		result.autoClosedEnclosing.map(range => Range.fromPositions(model.positionAt(range.startOffset), model.positionAt(range.endOffset))),
		change.version,
	);
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

function caret(columnIndex: number): Selection {
	return Selection.fromPositions(new Position((0) + 1, (columnIndex) + 1));
}

function primaryFirst<T>(items: readonly T[], primaryIndex: number): readonly T[] {
	if (primaryIndex === 0) return Object.freeze([...items]);
	return Object.freeze([items[primaryIndex]!, ...items.slice(0, primaryIndex), ...items.slice(primaryIndex + 1)]);
}

import { strict as assert } from "node:assert";
import test from "node:test";
import { TypeOperations } from "../../../common/cursor/cursorTypeOperations.js";
import { EditorIndentationKind } from "../../../common/core/misc/indentation.js";
import { CursorsController } from "../../../common/cursor/cursor.js";
import { registerBuiltinLanguageConfigurations } from "../../../common/languages/languageBuiltinConfigurations.js";
import { LanguageConfigurationRegistry, LanguageIndentAction } from "../../../common/languages/languageConfiguration.js";
import { createLanguageEnterCommand } from "../../../common/cursor/languageEnter.js";
import { LanguageLexicalContextIndex } from "../../../common/languages/languageLexicalContext.js";
import { TextSelection, TextSelectionSet } from "../../../common/core/selection.js";
import { TextPosition } from "../../../common/core/text.js";
import { TextModel } from "../../../common/model/textModel.js";

test("Language Enter creates an indented line between configured brackets", () => {
	using model = new TextModel("if (ok) {}");
	using selections = new CursorsController(model, TextSelectionSet.single(caret(9)));
	using configurations = new LanguageConfigurationRegistry();
	using builtins = registerBuiltinLanguageConfigurations(configurations);

	selections.execute(createLanguageEnterCommand(model, selections.selections, configurations.getLanguageConfiguration("typescript"), {
		indentation: {
			kind: EditorIndentationKind.Spaces,
			tabSize: 2,
		},
	}));

	assert.equal(model.getText(), "if (ok) {\n  \n}");
	assert.deepEqual(selections.selections.primary, TextSelection.collapsedAt(TextPosition.at(1, 2)));
});

test("Rust Enter continues line comments and applies Rust bracket indentation", () => {
	using commentModel = new TextModel("  // explain");
	using commentSelections = new CursorsController(commentModel, TextSelectionSet.single(caret(12)));
	using configurations = new LanguageConfigurationRegistry();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	const indentation = { kind: EditorIndentationKind.Spaces, tabSize: 2 } as const;

	commentSelections.execute(createLanguageEnterCommand(commentModel, commentSelections.selections, configurations.getLanguageConfiguration("rust"), { indentation }));
	assert.equal(commentModel.getText(), "  // explain\n  // ");

	using blockModel = new TextModel("fn main() {}");
	using blockSelections = new CursorsController(blockModel, TextSelectionSet.single(caret(11)));
	blockSelections.execute(createLanguageEnterCommand(blockModel, blockSelections.selections, configurations.getLanguageConfiguration("rust"), { indentation }));
	assert.equal(blockModel.getText(), "fn main() {\n  \n}");
});

test("Explicit on-enter rules precede bracket fallback and continue documentation comments", () => {
	using model = new TextModel("/** */");
	using selections = new CursorsController(model, TextSelectionSet.single(caret(3)));
	using configurations = new LanguageConfigurationRegistry();
	using builtins = registerBuiltinLanguageConfigurations(configurations);

	selections.execute(createLanguageEnterCommand(model, selections.selections, configurations.getLanguageConfiguration("typescript"), {
		indentation: {
			kind: EditorIndentationKind.Spaces,
			tabSize: 2,
		},
	}));

	assert.equal(model.getText(), "/**\n * \n */");
	assert.deepEqual(selections.selections.primary, TextSelection.collapsedAt(TextPosition.at(1, 3)));
});

test("On-enter rules observe previous, before, and after text in registration order", () => {
	using model = new TextModel("header\n  beginEND");
	using selections = new CursorsController(model, TextSelectionSet.single(TextSelection.from(TextPosition.at(1, 7), TextPosition.at(1, 10))));
	using configurations = new LanguageConfigurationRegistry();
	using rules = configurations.register("demo", {
		onEnterRules: [
			{
				previousLineText: /^header$/,
				beforeText: /begin$/,
				afterText: /^$/,
				action: { indentAction: LanguageIndentAction.Indent, appendText: "first" },
			},
			{
				beforeText: /begin$/,
				action: { indentAction: LanguageIndentAction.None, appendText: "second" },
			},
		],
	});

	selections.execute(createLanguageEnterCommand(model, selections.selections, configurations.getLanguageConfiguration("demo"), {
		indentation: {
			kind: EditorIndentationKind.Spaces,
			tabSize: 2,
		},
	}));

	assert.equal(model.getText(), "header\n  begin\n    first");
	assert.deepEqual(selections.selections.primary, TextSelection.collapsedAt(TextPosition.at(2, 9)));
});

test("Indentation rules increase, decrease, and ignore matching lines", () => {
	using configurations = new LanguageConfigurationRegistry();
	using rules = configurations.register("demo", {
		indentationRules: {
			increaseIndentPattern: /:\s*$/,
			decreaseIndentPattern: /^end\b/,
			unIndentedLinePattern: /^\s*\*/,
		},
	});
	const configuration = configurations.getLanguageConfiguration("demo");
	const indentation = { kind: EditorIndentationKind.Spaces, tabSize: 2 } as const;

	using increaseModel = new TextModel("  block:");
	using increaseSelections = new CursorsController(increaseModel, TextSelectionSet.single(caret(8)));
	increaseSelections.execute(createLanguageEnterCommand(increaseModel, increaseSelections.selections, configuration, { indentation }));
	assert.equal(increaseModel.getText(), "  block:\n    ");

	using decreaseModel = new TextModel("    valueend");
	using decreaseSelections = new CursorsController(decreaseModel, TextSelectionSet.single(caret(9)));
	decreaseSelections.execute(createLanguageEnterCommand(decreaseModel, decreaseSelections.selections, configuration, { indentation }));
	assert.equal(decreaseModel.getText(), "    value\n  end");

	using ignoredModel = new TextModel("  *:");
	using ignoredSelections = new CursorsController(ignoredModel, TextSelectionSet.single(caret(4)));
	ignoredSelections.execute(createLanguageEnterCommand(ignoredModel, ignoredSelections.selections, configuration, { indentation }));
	assert.equal(ignoredModel.getText(), "  *:\n  ");
});

test("Language Enter maps multiple cursors through one pre-change transaction", () => {
	using model = new TextModel("{} []");
	using selections = new CursorsController(model, TextSelectionSet.withPrimary([caret(1), caret(4)], 1));
	using configurations = new LanguageConfigurationRegistry();
	using builtins = registerBuiltinLanguageConfigurations(configurations);

	selections.execute(createLanguageEnterCommand(model, selections.selections, configurations.getLanguageConfiguration("typescript"), {
		indentation: {
			kind: EditorIndentationKind.Tabs,
			tabSize: 4,
		},
	}));

	assert.equal(model.getText(), "{\n\t\n} [\n\t\n]");
	assert.deepEqual(selections.selections, TextSelectionSet.withPrimary([
		TextSelection.collapsedAt(TextPosition.at(1, 1)),
		TextSelection.collapsedAt(TextPosition.at(3, 1)),
	], 1));
});

test("Language Enter starts a new typing history group that following text may join", () => {
	using model = new TextModel("{");
	using selections = new CursorsController(model, TextSelectionSet.single(caret(1)));
	using configurations = new LanguageConfigurationRegistry();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	selections.execute(TypeOperations.typeWithoutInterceptors(model, selections.selections, " "));
	selections.execute(createLanguageEnterCommand(model, selections.selections, configurations.getLanguageConfiguration("typescript"), {
		indentation: {
			kind: EditorIndentationKind.Spaces,
			tabSize: 2,
		},
	}));
	selections.execute(TypeOperations.typeWithoutInterceptors(model, selections.selections, "x"));
	assert.equal(model.getText(), "{ \n  x");

	selections.undo();
	assert.equal(model.getText(), "{ ");
	assert.deepEqual(selections.selections.primary, caret(2));
	selections.undo();
	assert.equal(model.getText(), "{");
});

test("Language Enter normalizes removeText and validates editor indentation before mutation", () => {
	using model = new TextModel("    stop");
	using selections = new CursorsController(model, TextSelectionSet.single(caret(8)));
	using configurations = new LanguageConfigurationRegistry();
	using rule = configurations.register("demo", {
		onEnterRules: [{
			beforeText: /stop$/,
			action: { indentAction: LanguageIndentAction.None, removeText: 2 },
		}],
	});
	const command = createLanguageEnterCommand(model, selections.selections, configurations.getLanguageConfiguration("demo"), {
		indentation: {
			kind: EditorIndentationKind.Spaces,
			tabSize: 2,
		},
	});
	selections.execute(command);
	assert.equal(model.getText(), "    stop\n  ");

	assert.throws(() => createLanguageEnterCommand(model, selections.selections, configurations.getLanguageConfiguration("demo"), {
		indentation: { tabSize: 0 },
	}), /tab size/);
	assert.equal(model.getText(), "    stop\n  ");
});

test("Language Enter ignores bracket-looking text in strings and comments", () => {
	assert.deepEqual(enterWithLexicalContext("const text = \"{\"", TextPosition.at(0, 16)), {
		text: "const text = \"{\"\n",
		position: TextPosition.at(1, 0),
	});
	assert.deepEqual(enterWithLexicalContext("// {", TextPosition.at(0, 4)), {
		text: "// {\n",
		position: TextPosition.at(1, 0),
	});
	assert.deepEqual(enterWithLexicalContext("/*\n{", TextPosition.at(1, 1)), {
		text: "/*\n{\n",
		position: TextPosition.at(2, 0),
	});
	assert.deepEqual(enterWithLexicalContext("if (ok) {", TextPosition.at(0, 9)), {
		text: "if (ok) {\n  ",
		position: TextPosition.at(1, 2),
	});
});

test("Language Enter rejects lexical context from another model or language", () => {
	using model = new TextModel("{}");
	using otherModel = new TextModel("{}");
	using configurations = new LanguageConfigurationRegistry();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	using otherModelContext = new LanguageLexicalContextIndex(otherModel, "typescript", configurations);
	using otherLanguageContext = new LanguageLexicalContextIndex(model, "json", configurations);
	const configuration = configurations.getLanguageConfiguration("typescript");

	assert.throws(() => createLanguageEnterCommand(model, TextSelectionSet.single(caret(1)), configuration, {
		lexicalContext: otherModelContext,
	}), /match its model and language/);
	assert.throws(() => createLanguageEnterCommand(model, TextSelectionSet.single(caret(1)), configuration, {
		lexicalContext: otherLanguageContext,
	}), /match its model and language/);
});

function enterWithLexicalContext(initialText: string, position: TextPosition): { readonly text: string; readonly position: TextPosition } {
	using model = new TextModel(initialText);
	using selections = new CursorsController(model, TextSelectionSet.single(TextSelection.collapsedAt(position)));
	using configurations = new LanguageConfigurationRegistry();
	using builtins = registerBuiltinLanguageConfigurations(configurations);
	using lexicalContext = new LanguageLexicalContextIndex(model, "typescript", configurations);
	selections.execute(createLanguageEnterCommand(model, selections.selections, configurations.getLanguageConfiguration("typescript"), {
		indentation: {
			kind: EditorIndentationKind.Spaces,
			tabSize: 2,
		},
		lexicalContext,
	}));
	return {
		text: model.getText(),
		position: selections.selections.primary.active,
	};
}

function caret(columnIndex: number): TextSelection {
	return TextSelection.collapsedAt(TextPosition.at(0, columnIndex));
}

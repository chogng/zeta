import assert from "node:assert/strict";
import test from "node:test";
import { createBackspaceCommand, createCutCommand, createDeleteForwardCommand, createDeleteToLineEndCommand, createDeleteToLineStartCommand } from "../../common/cursor/cursorDeleteOperations.js";
import { createDeleteWordBackwardCommand, createDeleteWordForwardCommand } from "../../common/cursor/cursorWordOperations.js";
import { createDistributedPasteTextCommand, createPasteTextCommand, createTypeTextCommand } from "../../common/cursor/cursorTypeOperations.js";
import { EditorSelectionController } from "../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../common/core/selection.js";
import { TextPosition } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";

test("Typing replaces multiple selections and coalesces with following text", () => {
	using model = new TextModel("abcd efgh");
	const initial = TextSelectionSet.withPrimary([
		TextSelection.from(TextPosition.at(0, 3), TextPosition.at(0, 1)),
		caret(0, 8),
	], 1);
	using controller = new EditorSelectionController(model, initial);

	controller.execute(createTypeTextCommand(
		model,
		controller.selections,
		"X",
	));
	assert.deepEqual({
		text: model.getText(),
		selections: controller.selections,
	}, {
		text: "aXd efgXh",
		selections: TextSelectionSet.withPrimary([
			caret(0, 2),
			caret(0, 8),
		], 1),
	});

	controller.execute(createTypeTextCommand(
		model,
		controller.selections,
		"Y",
	));
	assert.equal(model.getText(), "aXYd efgXYh");
	controller.undo();
	assert.deepEqual({
		text: model.getText(),
		selections: controller.selections,
	}, {
		text: "abcd efgh",
		selections: initial,
	});
});

test("Backspace deletes graphemes and joins lines", () => {
	using model = new TextModel("a😀b\ncd");
	using controller = new EditorSelectionController(
		model,
		TextSelectionSet.single(caret(0, 3)),
	);

	controller.execute(createBackspaceCommand(model, controller.selections));
	assert.deepEqual({
		text: model.getText(),
		selection: controller.selections.primary,
	}, {
		text: "ab\ncd",
		selection: caret(0, 1),
	});

	controller.setSelections(TextSelectionSet.single(caret(1, 0)));
	controller.execute(createBackspaceCommand(model, controller.selections));
	assert.deepEqual({
		text: model.getText(),
		selection: controller.selections.primary,
	}, {
		text: "abcd",
		selection: caret(0, 2),
	});
});

test("Forward Delete removes graphemes and line breaks", () => {
	using model = new TextModel("a😀b\ncd");
	using controller = new EditorSelectionController(
		model,
		TextSelectionSet.single(caret(0, 1)),
	);

	controller.execute(createDeleteForwardCommand(
		model,
		controller.selections,
	));
	assert.deepEqual({
		text: model.getText(),
		selection: controller.selections.primary,
	}, {
		text: "ab\ncd",
		selection: caret(0, 1),
	});

	controller.setSelections(TextSelectionSet.single(caret(0, 2)));
	controller.execute(createDeleteForwardCommand(
		model,
		controller.selections,
	));
	assert.deepEqual({
		text: model.getText(),
		selection: controller.selections.primary,
	}, {
		text: "abcd",
		selection: caret(0, 2),
	});
});

test("Word deletion uses shared editor word boundaries and coalesces by direction", () => {
	using model = new TextModel("alpha beta gamma");
	using controller = new EditorSelectionController(model, TextSelectionSet.single(caret(0, 10)));

	controller.execute(createDeleteWordBackwardCommand(model, controller.selections));
	controller.execute(createDeleteWordBackwardCommand(model, controller.selections));
	assert.equal(model.getText(), " gamma");
	assert.deepEqual(controller.selections.primary, caret(0, 0));
	controller.undo();
	assert.equal(model.getText(), "alpha beta gamma");

	controller.setSelections(TextSelectionSet.single(caret(0, 0)));
	controller.execute(createDeleteWordForwardCommand(model, controller.selections));
	assert.equal(model.getText(), "beta gamma");
});

test("Line-boundary deletion is isolated, multi-cursor aware, and preserves selected ranges", () => {
	using model = new TextModel("alpha\nbeta");
	using controller = new EditorSelectionController(model, TextSelectionSet.withPrimary([
		caret(0, 3),
		caret(1, 1),
	], 1));

	controller.execute(createDeleteToLineStartCommand(model, controller.selections));
	assert.deepEqual({ text: model.getText(), selections: controller.selections }, {
		text: "ha\neta",
		selections: TextSelectionSet.withPrimary([caret(0, 0), caret(1, 0)], 1),
	});
	controller.undo();
	assert.equal(model.getText(), "alpha\nbeta");

	controller.setSelections(TextSelectionSet.single(TextSelection.from(TextPosition.at(0, 1), TextPosition.at(1, 2))));
	controller.execute(createDeleteToLineEndCommand(model, controller.selections));
	assert.deepEqual({ text: model.getText(), selection: controller.selections.primary }, {
		text: "ata",
		selection: caret(0, 1),
	});
});

test("Typing normalizes line endings before calculating carets", () => {
	using model = new TextModel("ab");
	using controller = new EditorSelectionController(
		model,
		TextSelectionSet.single(caret(0, 1)),
	);

	controller.execute(createTypeTextCommand(
		model,
		controller.selections,
		"\r\n",
	));
	assert.deepEqual({
		text: model.getText(),
		selection: controller.selections.primary,
	}, {
		text: "a\nb",
		selection: caret(1, 0),
	});
});

test("Delete boundaries are no-ops and overlapping selections fail early", () => {
	using model = new TextModel("abc");
	using controller = new EditorSelectionController(
		model,
		TextSelectionSet.single(caret(0, 0)),
	);

	const version = model.version;
	controller.execute(createBackspaceCommand(model, controller.selections));
	assert.equal(model.version, version);
	controller.setSelections(TextSelectionSet.single(caret(0, 3)));
	controller.execute(createDeleteForwardCommand(
		model,
		controller.selections,
	));
	assert.equal(model.version, version);

	const overlapping = TextSelectionSet.withPrimary([
		TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 2)),
		TextSelection.from(TextPosition.at(0, 1), TextPosition.at(0, 3)),
	], 0);
	assert.throws(
		() => createTypeTextCommand(model, overlapping, "X"),
		/must not overlap/,
	);
	assert.equal(model.getText(), "abc");
});

test("Adjacent deletions merge converged carets while history restores sources", () => {
	using model = new TextModel("abc");
	const initial = TextSelectionSet.withPrimary([
		caret(0, 1),
		caret(0, 2),
	], 1);
	using controller = new EditorSelectionController(model, initial);

	controller.execute(createBackspaceCommand(model, controller.selections));
	assert.deepEqual({
		text: model.getText(),
		selections: controller.selections,
	}, {
		text: "c",
		selections: TextSelectionSet.single(caret(0, 0)),
	});

	controller.undo();
	assert.deepEqual({
		text: model.getText(),
		selections: controller.selections,
	}, {
		text: "abc",
		selections: initial,
	});
	controller.redo();
	assert.deepEqual(controller.selections, TextSelectionSet.single(caret(0, 0)));
});

test("Paste commands support shared and distributed isolated text", () => {
	using model = new TextModel("ab cd");
	using controller = new EditorSelectionController(
		model,
		TextSelectionSet.withPrimary([
			TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 2)),
			TextSelection.from(TextPosition.at(0, 3), TextPosition.at(0, 5)),
		], 1),
	);

	controller.execute(createDistributedPasteTextCommand(
		model,
		controller.selections,
		["A\r\nB", "C"],
	));
	assert.deepEqual({
		text: model.getText(),
		selections: controller.selections,
	}, {
		text: "A\nB C",
		selections: TextSelectionSet.withPrimary([
			caret(1, 1),
			caret(1, 3),
		], 1),
	});

	controller.execute(createPasteTextCommand(model, controller.selections, "!"));
	assert.equal(model.getText(), "A\nB! C!");
	controller.undo();
	assert.equal(model.getText(), "A\nB C");
	controller.undo();
	assert.equal(model.getText(), "ab cd");

	assert.throws(
		() => createDistributedPasteTextCommand(model, controller.selections, ["only one"]),
		/match the selection count/,
	);
});

test("Cut preserves collapsed cursors and restores history", () => {
	using model = new TextModel("abc def");
	const initial = TextSelectionSet.withPrimary([
		TextSelection.from(TextPosition.at(0, 2), TextPosition.at(0, 0)),
		caret(0, 7),
	], 0);
	using controller = new EditorSelectionController(model, initial);

	controller.execute(createCutCommand(model, controller.selections, controller.selections.selections.map(selection => selection.range)));
	assert.deepEqual({
		text: model.getText(),
		selections: controller.selections,
	}, {
		text: "c def",
		selections: TextSelectionSet.withPrimary([
			caret(0, 0),
			caret(0, 5),
		], 0),
	});

	controller.undo();
	assert.deepEqual({
		text: model.getText(),
		selections: controller.selections,
	}, {
		text: "abc def",
		selections: initial,
	});
});

function caret(lineIndex: number, columnIndex: number): TextSelection {
	return TextSelection.collapsedAt(TextPosition.at(lineIndex, columnIndex));
}

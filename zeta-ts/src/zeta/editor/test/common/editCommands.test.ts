import assert from "node:assert/strict";
import test from "node:test";
import { DeleteOperations } from "../../common/cursor/cursorDeleteOperations.js";
import { WordOperations } from "../../common/cursor/cursorWordOperations.js";
import { TypeOperations } from "../../common/cursor/cursorTypeOperations.js";
import { CursorsController } from "../../common/cursor/cursor.js";
import { Selection } from "../../common/core/selection.js";
import { SelectionSet } from "../../common/cursor/selectionSet.js";
import { Position } from "../../common/core/position.js";
import { TextModel } from "../../common/model/textModel.js";

test("Typing replaces multiple selections and coalesces with following text", () => {
	using model = new TextModel("abcd efgh");
	const initial = SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (3) + 1), new Position((0) + 1, (1) + 1)),
		caret(0, 8),
	], 1);
	using controller = new CursorsController(model, initial);

	controller.execute(TypeOperations.typeWithoutInterceptors(
		model,
		controller.selections,
		"X",
	));
	assert.deepEqual({
		text: model.getText(),
		selections: controller.selections,
	}, {
		text: "aXd efgXh",
		selections: SelectionSet.withPrimary([
			caret(0, 2),
			caret(0, 8),
		], 1),
	});

	controller.execute(TypeOperations.typeWithoutInterceptors(
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
	using controller = new CursorsController(
		model,
		SelectionSet.single(caret(0, 3)),
	);

	controller.execute(DeleteOperations.deleteLeft(model, controller.selections));
	assert.deepEqual({
		text: model.getText(),
		selection: controller.selections.primary,
	}, {
		text: "ab\ncd",
		selection: caret(0, 1),
	});

	controller.setSelections(SelectionSet.single(caret(1, 0)));
	controller.execute(DeleteOperations.deleteLeft(model, controller.selections));
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
	using controller = new CursorsController(
		model,
		SelectionSet.single(caret(0, 1)),
	);

	controller.execute(DeleteOperations.deleteRight(
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

	controller.setSelections(SelectionSet.single(caret(0, 2)));
	controller.execute(DeleteOperations.deleteRight(
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
	using controller = new CursorsController(model, SelectionSet.single(caret(0, 10)));

	controller.execute(WordOperations.deleteWordLeft(model, controller.selections));
	controller.execute(WordOperations.deleteWordLeft(model, controller.selections));
	assert.equal(model.getText(), " gamma");
	assert.deepEqual(controller.selections.primary, caret(0, 0));
	controller.undo();
	assert.equal(model.getText(), "alpha beta gamma");

	controller.setSelections(SelectionSet.single(caret(0, 0)));
	controller.execute(WordOperations.deleteWordRight(model, controller.selections));
	assert.equal(model.getText(), "beta gamma");
});

test("Line-boundary deletion is isolated, multi-cursor aware, and preserves selected ranges", () => {
	using model = new TextModel("alpha\nbeta");
	using controller = new CursorsController(model, SelectionSet.withPrimary([
		caret(0, 3),
		caret(1, 1),
	], 1));

	controller.execute(DeleteOperations.deleteToBeginningOfLine(model, controller.selections));
	assert.deepEqual({ text: model.getText(), selections: controller.selections }, {
		text: "ha\neta",
		selections: SelectionSet.withPrimary([caret(0, 0), caret(1, 0)], 1),
	});
	controller.undo();
	assert.equal(model.getText(), "alpha\nbeta");

	controller.setSelections(SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (1) + 1), new Position((1) + 1, (2) + 1))));
	controller.execute(DeleteOperations.deleteToEndOfLine(model, controller.selections));
	assert.deepEqual({ text: model.getText(), selection: controller.selections.primary }, {
		text: "ata",
		selection: caret(0, 1),
	});
});

test("Typing normalizes line endings before calculating carets", () => {
	using model = new TextModel("ab");
	using controller = new CursorsController(
		model,
		SelectionSet.single(caret(0, 1)),
	);

	controller.execute(TypeOperations.typeWithoutInterceptors(
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
	using controller = new CursorsController(
		model,
		SelectionSet.single(caret(0, 0)),
	);

	const version = model.version;
	controller.execute(DeleteOperations.deleteLeft(model, controller.selections));
	assert.equal(model.version, version);
	controller.setSelections(SelectionSet.single(caret(0, 3)));
	controller.execute(DeleteOperations.deleteRight(
		model,
		controller.selections,
	));
	assert.equal(model.version, version);

	const overlapping = SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (2) + 1)),
		Selection.fromPositions(new Position((0) + 1, (1) + 1), new Position((0) + 1, (3) + 1)),
	], 0);
	assert.throws(
		() => TypeOperations.typeWithoutInterceptors(model, overlapping, "X"),
		/must not overlap/,
	);
	assert.equal(model.getText(), "abc");
});

test("Adjacent deletions merge converged carets while history restores sources", () => {
	using model = new TextModel("abc");
	const initial = SelectionSet.withPrimary([
		caret(0, 1),
		caret(0, 2),
	], 1);
	using controller = new CursorsController(model, initial);

	controller.execute(DeleteOperations.deleteLeft(model, controller.selections));
	assert.deepEqual({
		text: model.getText(),
		selections: controller.selections,
	}, {
		text: "c",
		selections: SelectionSet.single(caret(0, 0)),
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
	assert.deepEqual(controller.selections, SelectionSet.single(caret(0, 0)));
});

test("Paste commands support shared and distributed isolated text", () => {
	using model = new TextModel("ab cd");
	using controller = new CursorsController(
		model,
		SelectionSet.withPrimary([
			Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (2) + 1)),
			Selection.fromPositions(new Position((0) + 1, (3) + 1), new Position((0) + 1, (5) + 1)),
		], 1),
	);

	controller.execute(TypeOperations.distributedPaste(
		model,
		controller.selections,
		["A\r\nB", "C"],
	));
	assert.deepEqual({
		text: model.getText(),
		selections: controller.selections,
	}, {
		text: "A\nB C",
		selections: SelectionSet.withPrimary([
			caret(1, 1),
			caret(1, 3),
		], 1),
	});

	controller.execute(TypeOperations.paste(model, controller.selections, "!"));
	assert.equal(model.getText(), "A\nB! C!");
	controller.undo();
	assert.equal(model.getText(), "A\nB C");
	controller.undo();
	assert.equal(model.getText(), "ab cd");

	assert.throws(
		() => TypeOperations.distributedPaste(model, controller.selections, ["only one"]),
		/match the selection count/,
	);
});

test("Cut preserves collapsed cursors and restores history", () => {
	using model = new TextModel("abc def");
	const initial = SelectionSet.withPrimary([
		Selection.fromPositions(new Position((0) + 1, (2) + 1), new Position((0) + 1, (0) + 1)),
		caret(0, 7),
	], 0);
	using controller = new CursorsController(model, initial);

	controller.execute(DeleteOperations.cut(model, controller.selections, controller.selections.selections.map(selection => selection)));
	assert.deepEqual({
		text: model.getText(),
		selections: controller.selections,
	}, {
		text: "c def",
		selections: SelectionSet.withPrimary([
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

function caret(lineIndex: number, columnIndex: number): Selection {
	return Selection.fromPositions(new Position((lineIndex) + 1, (columnIndex) + 1));
}

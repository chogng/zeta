import assert from "node:assert/strict";
import test from "node:test";
import { EditorCursorNavigationCommand, EditorCursorNavigationMode, MoveOperations, type EditorCursorNavigationResult } from "../../common/cursor/cursorMoveOperations.js";
import { TextSelection, TextSelectionSet } from "../../common/core/selection.js";
import { TextPosition } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";

test("Character navigation preserves graphemes and crosses line boundaries", () => {
	using model = new TextModel("a😀b\nx");

	assert.deepEqual(
		navigate(model, caret(0, 1), EditorCursorNavigationCommand.CharacterRight)
			.selections.primary,
		caret(0, 3),
	);
	assert.deepEqual(
		navigate(model, caret(0, 3), EditorCursorNavigationCommand.CharacterLeft)
			.selections.primary,
		caret(0, 1),
	);
	assert.deepEqual(
		navigate(model, caret(0, 4), EditorCursorNavigationCommand.CharacterRight)
			.selections.primary,
		caret(1, 0),
	);
	assert.deepEqual(
		navigate(model, caret(1, 0), EditorCursorNavigationCommand.CharacterLeft)
			.selections.primary,
		caret(0, 4),
	);
});

test("Horizontal movement collapses ranges and Shift preserves the anchor", () => {
	using model = new TextModel("abcdef");
	const selection = TextSelection.from(
		TextPosition.at(0, 5),
		TextPosition.at(0, 2),
	);

	assert.deepEqual(
		navigate(model, selection, EditorCursorNavigationCommand.CharacterLeft)
			.selections.primary,
		caret(0, 2),
	);
	assert.deepEqual(
		navigate(model, selection, EditorCursorNavigationCommand.CharacterRight)
			.selections.primary,
		caret(0, 5),
	);
	assert.deepEqual(
		navigate(
			model,
			caret(0, 1),
			EditorCursorNavigationCommand.CharacterRight,
			EditorCursorNavigationMode.Extend,
		).selections.primary,
		TextSelection.from(TextPosition.at(0, 1), TextPosition.at(0, 2)),
	);
});

test("Vertical navigation retains preferred columns across short lines", () => {
	using model = new TextModel("abcdef\nx\n😀\nabcdef");
	const first = navigate(
		model,
		caret(0, 5),
		EditorCursorNavigationCommand.LineDown,
	);
	const second = navigate(
		model,
		first.selections.primary,
		EditorCursorNavigationCommand.LineDown,
		EditorCursorNavigationMode.Move,
		first.preferredColumns,
	);
	const third = navigate(
		model,
		second.selections.primary,
		EditorCursorNavigationCommand.LineDown,
		EditorCursorNavigationMode.Move,
		second.preferredColumns,
	);

	assert.deepEqual({
		first: first.selections.primary,
		second: second.selections.primary,
		third: third.selections.primary,
		preferred: third.preferredColumns,
	}, {
		first: caret(1, 1),
		second: caret(2, 2),
		third: caret(3, 5),
		preferred: [5],
	});
	assert.deepEqual(
		navigate(
			model,
			caret(0, 1),
			EditorCursorNavigationCommand.LineUp,
			EditorCursorNavigationMode.Move,
			[5],
		).selections.primary,
		caret(0, 1),
	);
});

test("Word navigation uses word-like segments across lines", () => {
	using model = new TextModel("alpha  beta\n😀 gamma\nlast");

	assert.deepEqual(
		navigate(model, caret(0, 2), EditorCursorNavigationCommand.WordRight)
			.selections.primary,
		caret(0, 7),
	);
	assert.deepEqual(
		navigate(model, caret(0, 9), EditorCursorNavigationCommand.WordLeft)
			.selections.primary,
		caret(0, 7),
	);
	assert.deepEqual(
		navigate(model, caret(0, 7), EditorCursorNavigationCommand.WordLeft)
			.selections.primary,
		caret(0, 0),
	);
	assert.deepEqual(
		navigate(model, caret(0, 7), EditorCursorNavigationCommand.WordRight)
			.selections.primary,
		caret(1, 3),
	);
});

test("Line, page, document, and multi-selection navigation remain explicit", () => {
	using model = new TextModel("abcd\nx\nabcdef\nlast");
	const selections = TextSelectionSet.withPrimary([
		caret(0, 3),
		caret(1, 1),
	], 1);
	const page = MoveOperations.navigate(model, selections, {
		command: EditorCursorNavigationCommand.PageDown,
		mode: EditorCursorNavigationMode.Extend,
		pageLineCount: 2,
	});

	assert.deepEqual(page.selections, TextSelectionSet.withPrimary([
		TextSelection.from(TextPosition.at(0, 3), TextPosition.at(2, 3)),
		TextSelection.from(TextPosition.at(1, 1), TextPosition.at(3, 1)),
	], 1));
	assert.deepEqual(
		navigate(model, caret(2, 4), EditorCursorNavigationCommand.LineStart)
			.selections.primary,
		caret(2, 0),
	);
	assert.deepEqual(
		navigate(model, caret(2, 4), EditorCursorNavigationCommand.LineEnd)
			.selections.primary,
		caret(2, 6),
	);
	assert.deepEqual(
		navigate(model, caret(2, 4), EditorCursorNavigationCommand.DocumentStart)
			.selections.primary,
		caret(0, 0),
	);
	assert.deepEqual(
		navigate(model, caret(2, 4), EditorCursorNavigationCommand.DocumentEnd)
			.selections.primary,
		caret(3, 4),
	);
});

test("Navigation coalesces exact duplicate results and validates requests", () => {
	using model = new TextModel("abc");
	const selections = TextSelectionSet.withPrimary([
		caret(0, 1),
		caret(0, 2),
	], 1);
	const result = MoveOperations.navigate(model, selections, {
		command: EditorCursorNavigationCommand.DocumentStart,
		mode: EditorCursorNavigationMode.Move,
	});

	assert.deepEqual(result.selections, TextSelectionSet.single(caret(0, 0)));
	assert.throws(() => MoveOperations.navigate(model, selections, {
		command: EditorCursorNavigationCommand.PageDown,
		mode: EditorCursorNavigationMode.Move,
		pageLineCount: 0,
	}), /pageLineCount/);
	assert.throws(() => MoveOperations.navigate(model, selections, {
		command: EditorCursorNavigationCommand.LineDown,
		mode: EditorCursorNavigationMode.Move,
		preferredColumns: [1],
	}), /preferredColumns/);
	assert.throws(() => MoveOperations.navigate(model, selections, {
		command: "unknown" as EditorCursorNavigationCommand,
		mode: EditorCursorNavigationMode.Move,
	}), /Unknown editor cursor navigation command/);
});

test('Character navigation honors configured atomic tab stops in indentation', () => {
	using model = new TextModel('        value');
	const left = MoveOperations.navigate(model, TextSelectionSet.single(caret(0, 8)), {
		command: EditorCursorNavigationCommand.CharacterLeft,
		mode: EditorCursorNavigationMode.Move,
		atomicTabSize: 4,
	});
	const right = MoveOperations.navigate(model, TextSelectionSet.single(caret(0, 0)), {
		command: EditorCursorNavigationCommand.CharacterRight,
		mode: EditorCursorNavigationMode.Move,
		atomicTabSize: 4,
	});

	assert.deepEqual(left.selections.primary, caret(0, 4));
	assert.deepEqual(right.selections.primary, caret(0, 4));
	assert.throws(() => MoveOperations.navigate(model, TextSelectionSet.single(caret(0, 0)), {
		command: EditorCursorNavigationCommand.CharacterRight,
		mode: EditorCursorNavigationMode.Move,
		atomicTabSize: 0,
	}), /atomicTabSize/);
});

function navigate(
	model: TextModel,
	selection: TextSelection,
	command: EditorCursorNavigationCommand,
	mode = EditorCursorNavigationMode.Move,
	preferredColumns?: readonly number[],
): EditorCursorNavigationResult {
	return MoveOperations.navigate(
		model,
		TextSelectionSet.single(selection),
		{ command, mode, preferredColumns },
	);
}

function caret(lineIndex: number, columnIndex: number): TextSelection {
	return TextSelection.collapsedAt(TextPosition.at(lineIndex, columnIndex));
}

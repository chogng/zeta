import assert from "node:assert/strict";
import test from "node:test";
import { EditorCursorNavigationCommand, EditorCursorNavigationMode, MoveOperations, type EditorCursorNavigationResult } from "../../common/cursor/cursorMoveOperations.js";
import { Selection } from "../../common/core/selection.js";
import { Position } from "../../common/core/position.js";
import { TextModel } from "../../common/model/textModel.js";

test("Character navigation preserves graphemes and crosses line boundaries", () => {
	using model = new TextModel("a😀b\nx");

	assert.deepEqual(
		navigate(model, caret(0, 1), EditorCursorNavigationCommand.CharacterRight)
			.selections[0]!,
		caret(0, 3),
	);
	assert.deepEqual(
		navigate(model, caret(0, 3), EditorCursorNavigationCommand.CharacterLeft)
			.selections[0]!,
		caret(0, 1),
	);
	assert.deepEqual(
		navigate(model, caret(0, 4), EditorCursorNavigationCommand.CharacterRight)
			.selections[0]!,
		caret(1, 0),
	);
	assert.deepEqual(
		navigate(model, caret(1, 0), EditorCursorNavigationCommand.CharacterLeft)
			.selections[0]!,
		caret(0, 4),
	);
});

test("Horizontal movement collapses ranges and Shift preserves the anchor", () => {
	using model = new TextModel("abcdef");
	const selection = Selection.fromPositions(
		new Position((0) + 1, (5) + 1),
		new Position((0) + 1, (2) + 1),
	);

	assert.deepEqual(
		navigate(model, selection, EditorCursorNavigationCommand.CharacterLeft)
			.selections[0]!,
		caret(0, 2),
	);
	assert.deepEqual(
		navigate(model, selection, EditorCursorNavigationCommand.CharacterRight)
			.selections[0]!,
		caret(0, 5),
	);
	assert.deepEqual(
		navigate(
			model,
			caret(0, 1),
			EditorCursorNavigationCommand.CharacterRight,
			EditorCursorNavigationMode.Extend,
		).selections[0]!,
		Selection.fromPositions(new Position((0) + 1, (1) + 1), new Position((0) + 1, (2) + 1)),
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
		first.selections[0]!,
		EditorCursorNavigationCommand.LineDown,
		EditorCursorNavigationMode.Move,
		first.preferredColumns,
	);
	const third = navigate(
		model,
		second.selections[0]!,
		EditorCursorNavigationCommand.LineDown,
		EditorCursorNavigationMode.Move,
		second.preferredColumns,
	);

	assert.deepEqual({
		first: first.selections[0]!,
		second: second.selections[0]!,
		third: third.selections[0]!,
		preferred: third.preferredColumns,
	}, {
		first: caret(1, 1),
		second: caret(2, 2),
		third: caret(3, 5),
		preferred: [6],
	});
	assert.deepEqual(
		navigate(
			model,
			caret(0, 1),
			EditorCursorNavigationCommand.LineUp,
			EditorCursorNavigationMode.Move,
			[5],
		).selections[0]!,
		caret(0, 1),
	);
});

test("Word navigation uses word-like segments across lines", () => {
	using model = new TextModel("alpha  beta\n😀 gamma\nlast");

	assert.deepEqual(
		navigate(model, caret(0, 2), EditorCursorNavigationCommand.WordRight)
			.selections[0]!,
		caret(0, 7),
	);
	assert.deepEqual(
		navigate(model, caret(0, 9), EditorCursorNavigationCommand.WordLeft)
			.selections[0]!,
		caret(0, 7),
	);
	assert.deepEqual(
		navigate(model, caret(0, 7), EditorCursorNavigationCommand.WordLeft)
			.selections[0]!,
		caret(0, 0),
	);
	assert.deepEqual(
		navigate(model, caret(0, 7), EditorCursorNavigationCommand.WordRight)
			.selections[0]!,
		caret(1, 3),
	);
});

test("Line, page, document, and multi-selection navigation remain explicit", () => {
	using model = new TextModel("abcd\nx\nabcdef\nlast");
	const selections = primaryFirst([
		caret(0, 3),
		caret(1, 1),
	], 1);
	const page = MoveOperations.navigate(model, selections, {
		command: EditorCursorNavigationCommand.PageDown,
		mode: EditorCursorNavigationMode.Extend,
		pageLineCount: 2,
	});

	assert.deepEqual(page.selections, primaryFirst([
		Selection.fromPositions(new Position((0) + 1, (3) + 1), new Position((2) + 1, (3) + 1)),
		Selection.fromPositions(new Position((1) + 1, (1) + 1), new Position((3) + 1, (1) + 1)),
	], 1));
	assert.deepEqual(
		navigate(model, caret(2, 4), EditorCursorNavigationCommand.LineStart)
			.selections[0]!,
		caret(2, 0),
	);
	assert.deepEqual(
		navigate(model, caret(2, 4), EditorCursorNavigationCommand.LineEnd)
			.selections[0]!,
		caret(2, 6),
	);
	assert.deepEqual(
		navigate(model, caret(2, 4), EditorCursorNavigationCommand.DocumentStart)
			.selections[0]!,
		caret(0, 0),
	);
	assert.deepEqual(
		navigate(model, caret(2, 4), EditorCursorNavigationCommand.DocumentEnd)
			.selections[0]!,
		caret(3, 4),
	);
});

test("Navigation coalesces exact duplicate results and validates requests", () => {
	using model = new TextModel("abc");
	const selections = primaryFirst([
		caret(0, 1),
		caret(0, 2),
	], 1);
	const result = MoveOperations.navigate(model, selections, {
		command: EditorCursorNavigationCommand.DocumentStart,
		mode: EditorCursorNavigationMode.Move,
	});

	assert.deepEqual(result.selections, [caret(0, 0)]);
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
	const left = MoveOperations.navigate(model, [caret(0, 8)], {
		command: EditorCursorNavigationCommand.CharacterLeft,
		mode: EditorCursorNavigationMode.Move,
		atomicTabSize: 4,
	});
	const right = MoveOperations.navigate(model, [caret(0, 0)], {
		command: EditorCursorNavigationCommand.CharacterRight,
		mode: EditorCursorNavigationMode.Move,
		atomicTabSize: 4,
	});

	assert.deepEqual(left.selections[0]!, caret(0, 4));
	assert.deepEqual(right.selections[0]!, caret(0, 4));
	assert.throws(() => MoveOperations.navigate(model, [caret(0, 0)], {
		command: EditorCursorNavigationCommand.CharacterRight,
		mode: EditorCursorNavigationMode.Move,
		atomicTabSize: 0,
	}), /atomicTabSize/);
});

function navigate(
	model: TextModel,
	selection: Selection,
	command: EditorCursorNavigationCommand,
	mode = EditorCursorNavigationMode.Move,
	preferredColumns?: readonly number[],
): EditorCursorNavigationResult {
	return MoveOperations.navigate(
		model,
		[selection],
		{ command, mode, preferredColumns },
	);
}

function caret(lineIndex: number, columnIndex: number): Selection {
	return Selection.fromPositions(new Position((lineIndex) + 1, (columnIndex) + 1));
}

function primaryFirst<T>(items: readonly T[], primaryIndex: number): readonly T[] {
	if (primaryIndex === 0) return Object.freeze([...items]);
	return Object.freeze([items[primaryIndex]!, ...items.slice(0, primaryIndex), ...items.slice(primaryIndex + 1)]);
}

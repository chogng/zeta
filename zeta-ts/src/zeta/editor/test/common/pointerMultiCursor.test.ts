import assert from "node:assert/strict";
import test from "node:test";
import { CursorMoveCommands, PointerMultiCursorModifier, type PointerModifierState } from "../../common/cursor/cursorMoveCommands.js";
import { TextSelection, TextSelectionSet } from "../../common/core/selection.js";
import { TextPosition } from "../../common/core/text.js";

test("Pointer multi-cursor modifiers require their exact configured chord", () => {
	const state = (
		altKey: boolean,
		ctrlKey: boolean,
		metaKey: boolean,
		shiftKey = false,
	) => ({ altKey, ctrlKey, metaKey, shiftKey });

	assert.equal(CursorMoveCommands.isPointerMultiCursorGesture(
		state(true, false, false),
		PointerMultiCursorModifier.Alt,
	), true);
	assert.equal(CursorMoveCommands.isPointerMultiCursorGesture(
		state(true, false, false, true),
		PointerMultiCursorModifier.Alt,
	), false);
	assert.equal(CursorMoveCommands.isPointerMultiCursorGesture(
		state(true, true, false),
		PointerMultiCursorModifier.Alt,
	), false);
	assert.equal(CursorMoveCommands.isPointerMultiCursorGesture(
		state(false, true, false),
		PointerMultiCursorModifier.ControlOrMeta,
	), true);
	assert.equal(CursorMoveCommands.isPointerMultiCursorGesture(
		state(false, false, true),
		PointerMultiCursorModifier.ControlOrMeta,
	), true);
	assert.equal(CursorMoveCommands.readPointerMultiCursorModifier(undefined), PointerMultiCursorModifier.Alt);
	assert.throws(
		() => CursorMoveCommands.readPointerMultiCursorModifier("shift" as PointerMultiCursorModifier),
		/Unknown Stanza pointer multi-cursor modifier/,
	);
});

test("Pointer multi-cursor combination adds, toggles, and deduplicates", () => {
	const first = caret(0, 1);
	const second = caret(1, 2);
	const third = caret(2, 3);
	const base = TextSelectionSet.withPrimary([first, second], 1);

	assert.deepEqual(
		CursorMoveCommands.combinePointerSelection(base, third, undefined),
		TextSelectionSet.withPrimary([first, second, third], 2),
	);
	assert.deepEqual(
		CursorMoveCommands.combinePointerSelection(base, first, 0),
		TextSelectionSet.single(second),
	);
	assert.equal(
		CursorMoveCommands.combinePointerSelection(TextSelectionSet.single(first), first, 0)
			.selections.length,
		1,
	);
	assert.deepEqual(
		CursorMoveCommands.combinePointerSelection(base, second, 0),
		TextSelectionSet.single(second),
	);
});

test("Pointer multi-cursor combination replaces overlapping ranges", () => {
	const first = caret(0, 1);
	const inside = caret(1, 2);
	const outside = caret(2, 3);
	const active = TextSelection.from(
		TextPosition.at(0, 0),
		TextPosition.at(1, 4),
	);
	const base = TextSelectionSet.withPrimary([first, inside, outside], 2);

	assert.deepEqual(
		CursorMoveCommands.combinePointerSelection(base, active, undefined),
		TextSelectionSet.withPrimary([outside, active], 1),
	);
	assert.throws(
		() => CursorMoveCommands.combinePointerSelection(base, active, 3),
		/outside the selection set/,
	);
});

function caret(lineIndex: number, columnIndex: number): TextSelection {
	return TextSelection.collapsedAt(TextPosition.at(lineIndex, columnIndex));
}

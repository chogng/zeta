import assert from "node:assert/strict";
import test from "node:test";
import { AlphaPointerMultiCursorModifier, combineAlphaPointerSelection, isAlphaPointerMultiCursorGesture, readAlphaPointerMultiCursorModifier } from "../../browser/pointerMultiCursor.js";
import { TextSelection, TextSelectionSet } from "../../common/selection.js";
import { TextPosition } from "../../common/text.js";

test("Pointer multi-cursor modifiers require their exact configured chord", () => {
  const state = (
    altKey: boolean,
    ctrlKey: boolean,
    metaKey: boolean,
    shiftKey = false,
  ) => ({ altKey, ctrlKey, metaKey, shiftKey });

  assert.equal(isAlphaPointerMultiCursorGesture(
    state(true, false, false),
    AlphaPointerMultiCursorModifier.Alt,
  ), true);
  assert.equal(isAlphaPointerMultiCursorGesture(
    state(true, false, false, true),
    AlphaPointerMultiCursorModifier.Alt,
  ), false);
  assert.equal(isAlphaPointerMultiCursorGesture(
    state(true, true, false),
    AlphaPointerMultiCursorModifier.Alt,
  ), false);
  assert.equal(isAlphaPointerMultiCursorGesture(
    state(false, true, false),
    AlphaPointerMultiCursorModifier.ControlOrMeta,
  ), true);
  assert.equal(isAlphaPointerMultiCursorGesture(
    state(false, false, true),
    AlphaPointerMultiCursorModifier.ControlOrMeta,
  ), true);
  assert.equal(readAlphaPointerMultiCursorModifier(undefined), AlphaPointerMultiCursorModifier.Alt);
  assert.throws(
    () => readAlphaPointerMultiCursorModifier("shift" as AlphaPointerMultiCursorModifier),
    /Unknown Alpha pointer multi-cursor modifier/,
  );
});

test("Pointer multi-cursor combination adds, toggles, and deduplicates", () => {
  const first = caret(0, 1);
  const second = caret(1, 2);
  const third = caret(2, 3);
  const base = TextSelectionSet.withPrimary([first, second], 1);

  assert.deepEqual(
    combineAlphaPointerSelection(base, third, undefined),
    TextSelectionSet.withPrimary([first, second, third], 2),
  );
  assert.deepEqual(
    combineAlphaPointerSelection(base, first, 0),
    TextSelectionSet.single(second),
  );
  assert.equal(
    combineAlphaPointerSelection(TextSelectionSet.single(first), first, 0)
      .selections.length,
    1,
  );
  assert.deepEqual(
    combineAlphaPointerSelection(base, second, 0),
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
    combineAlphaPointerSelection(base, active, undefined),
    TextSelectionSet.withPrimary([outside, active], 1),
  );
  assert.throws(
    () => combineAlphaPointerSelection(base, active, 3),
    /outside the selection set/,
  );
});

function caret(lineIndex: number, columnIndex: number): TextSelection {
  return TextSelection.collapsedAt(TextPosition.at(lineIndex, columnIndex));
}

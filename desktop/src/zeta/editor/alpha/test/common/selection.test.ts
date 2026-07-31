import assert from "node:assert/strict";
import test from "node:test";
import { SelectionDirection, TextSelection, TextSelectionSet } from "../../common/selection.js";
import { TextPosition, TextRange } from "../../common/text.js";

test("TextSelection preserves anchor direction and ordered range", () => {
  const start = TextPosition.at(1, 2);
  const end = TextPosition.at(3, 4);
  const forward = TextSelection.from(start, end);
  const backward = TextSelection.from(end, start);
  const collapsed = TextSelection.collapsedAt(start);

  assert.deepEqual({
    forward: {
      direction: forward.direction,
      range: forward.range,
      collapsed: forward.collapsed,
    },
    backward: {
      direction: backward.direction,
      range: backward.range,
      collapsed: backward.collapsed,
    },
    collapsed: {
      direction: collapsed.direction,
      range: collapsed.range,
      collapsed: collapsed.collapsed,
    },
  }, {
    forward: {
      direction: SelectionDirection.Forward,
      range: TextRange.from(start, end),
      collapsed: false,
    },
    backward: {
      direction: SelectionDirection.Backward,
      range: TextRange.from(start, end),
      collapsed: false,
    },
    collapsed: {
      direction: SelectionDirection.Forward,
      range: TextRange.emptyAt(start),
      collapsed: true,
    },
  });
});

test("TextSelectionSet owns immutable multi-cursor order and primary", () => {
  const first = TextSelection.collapsedAt(TextPosition.at(0, 1));
  const second = TextSelection.collapsedAt(TextPosition.at(2, 3));
  const selections = [first, second];
  const set = TextSelectionSet.withPrimary(selections, 1);
  selections.reverse();

  assert.deepEqual({
    frozen: Object.isFrozen(set),
    selectionsFrozen: Object.isFrozen(set.selections),
    selections: set.selections,
    primary: set.primary,
  }, {
    frozen: true,
    selectionsFrozen: true,
    selections: [first, second],
    primary: second,
  });
  assert.equal(TextSelectionSet.single(first).primary, first);
  assert.throws(
    () => TextSelectionSet.withPrimary([], 0),
    /must not be empty/,
  );
  assert.throws(
    () => TextSelectionSet.withPrimary([first], 1),
    /primaryIndex/,
  );
});

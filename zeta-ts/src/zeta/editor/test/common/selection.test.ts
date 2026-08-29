import assert from "node:assert/strict";
import test from "node:test";
import { SelectionDirection, Selection } from "../../common/core/selection.js";
import { SelectionSet } from "../../common/cursor/selectionSet.js";
import { Position } from "../../common/core/position.js";
import { Range } from "../../common/core/range.js";

test("Selection preserves anchor direction and ordered range", () => {
	const start = new Position((1) + 1, (2) + 1);
	const end = new Position((3) + 1, (4) + 1);
	const forward = Selection.fromPositions(start, end);
	const backward = Selection.fromPositions(end, start);
	const collapsed = Selection.fromPositions(start);

	assert.deepEqual({
		forward: {
			direction: forward.getDirection(),
			range: new Range(forward.startLineNumber, forward.startColumn, forward.endLineNumber, forward.endColumn),
			collapsed: forward.isEmpty(),
		},
		backward: {
			direction: backward.getDirection(),
			range: new Range(backward.startLineNumber, backward.startColumn, backward.endLineNumber, backward.endColumn),
			collapsed: backward.isEmpty(),
		},
		collapsed: {
			direction: collapsed.getDirection(),
			range: new Range(collapsed.startLineNumber, collapsed.startColumn, collapsed.endLineNumber, collapsed.endColumn),
			collapsed: collapsed.isEmpty(),
		},
	}, {
		forward: {
			direction: SelectionDirection.LTR,
			range: Range.fromPositions(start, end),
			collapsed: false,
		},
		backward: {
			direction: SelectionDirection.RTL,
			range: Range.fromPositions(start, end),
			collapsed: false,
		},
		collapsed: {
			direction: SelectionDirection.LTR,
			range: Range.fromPositions(start),
			collapsed: true,
		},
	});
});

test("SelectionSet owns immutable multi-cursor order and primary", () => {
	const first = Selection.fromPositions(new Position((0) + 1, (1) + 1));
	const second = Selection.fromPositions(new Position((2) + 1, (3) + 1));
	const selections = [first, second];
	const set = SelectionSet.withPrimary(selections, 1);
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
	assert.equal(SelectionSet.single(first).primary, first);
	assert.throws(
		() => SelectionSet.withPrimary([], 0),
		/must not be empty/,
	);
	assert.throws(
		() => SelectionSet.withPrimary([first], 1),
		/primaryIndex/,
	);
});

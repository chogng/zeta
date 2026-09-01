import assert from "node:assert/strict";
import test from "node:test";
import { UndoRedoGroup } from '../../../platform/undoRedo/common/undoRedo.js';
import { EditorCommandHistoryMode } from '../../common/commands/editorEditCommand.js';
import { CursorsController } from '../../common/cursor/cursor.js';
import { Selection } from "../../common/core/selection.js";
import { Position } from "../../common/core/position.js";
import { Range } from "../../common/core/range.js";
import type { TextModelChange } from '../../common/core/textChange.js';
import type { IIdentifiedSingleEditOperation } from '../../common/model.js';
import { TextModel } from "../../common/model/textModel.js";
import { createTestCursorsController } from './testCursorConfiguration.js';

const position = (lineIndex: number, columnIndex: number): Position => new Position(lineIndex + 1, columnIndex + 1);
const range = (
	startColumn: number,
	endColumn: number,
): Range => Range.fromPositions(
	position(0, startColumn),
	position(0, endColumn),
);
const cursors = (
	offsets: readonly number[],
	primaryIndex = 0,
): readonly Selection[] => primaryFirst(offsets.map(offset => Selection.fromPositions(
		position(0, offset),
	)), primaryIndex);
const selections = (
	offsets: readonly (readonly [number, number])[],
	primaryIndex = 0,
): readonly Selection[] => primaryFirst(offsets.map(([anchorOffset, activeOffset]) => Selection.fromPositions(
		position(0, anchorOffset),
		position(0, activeOffset),
	)), primaryIndex);

function pushEdits(model: TextModel, edits: IIdentifiedSingleEditOperation[], group?: UndoRedoGroup): TextModelChange | undefined {
	let change: TextModelChange | undefined;
	const listener = model.onDidChangeContent(event => {
		change = event;
	});
	try {
		model.pushEditOperations(null, edits, () => null, group);
	} finally {
		listener.dispose();
	}
	return change;
}

test("TextModel coalesces consecutive adjacent insertions", () => {
	using model = new TextModel("");
	const historyGroup = new UndoRedoGroup();
	const transactionIds: number[] = [];

	for (const character of "abc") {
		const end = model.positionAt(model.getText().length);
		const change = pushEdits(
			model,
			[{ range: Range.fromPositions(end), text: character }],
			historyGroup,
		);
		transactionIds.push(change?.transactionId ?? -1);
	}
	const undo = model.undo();
	const afterUndo = model.getText();
	const redo = model.redo();

	assert.deepEqual({
		transactionIds,
		undoTransactionId: undo?.transactionId,
		redoTransactionId: redo?.transactionId,
		afterUndo,
		afterRedo: model.getText(),
		version: model.version,
	}, {
		transactionIds: [1, 1, 1],
		undoTransactionId: 1,
		redoTransactionId: 1,
		afterUndo: "",
		afterRedo: "abc",
		version: 6,
	});
});

test("TextModel starts a new step for non-adjacent group edits", () => {
	using model = new TextModel("");
	const historyGroup = new UndoRedoGroup();
	const first = pushEdits(
		model,
		[{ range: range(0, 0), text: "A" }],
		historyGroup,
	);
	const second = pushEdits(
		model,
		[{ range: range(0, 0), text: "B" }],
		historyGroup,
	);

	model.undo();
	const afterFirstUndo = model.getText();
	model.undo();

	assert.deepEqual({
		first: first?.transactionId,
		second: second?.transactionId,
		afterFirstUndo,
		afterSecondUndo: model.getText(),
	}, {
		first: 1,
		second: 2,
		afterFirstUndo: "A",
		afterSecondUndo: "",
	});
});

test("CursorsController coalesces multi-cursor typing", () => {
	using model = new TextModel("ab");
	using controller = createTestCursorsController(
		model,
		cursors([0, 2]),
	);

	const first = controller.execute({
		edits: [
			{ range: range(0, 0), text: "X" },
			{ range: range(2, 2), text: "X" },
		],
		selectionsAfter: [
			{ anchorOffset: 1, activeOffset: 1 },
			{ anchorOffset: 4, activeOffset: 4 },
		],
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.CoalesceTyping,
	});
	const second = controller.execute({
		edits: [
			{ range: range(1, 1), text: "Y" },
			{ range: range(4, 4), text: "Y" },
		],
		selectionsAfter: [
			{ anchorOffset: 2, activeOffset: 2 },
			{ anchorOffset: 6, activeOffset: 6 },
		],
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.CoalesceTyping,
	});

	const afterTyping = controller.getSelections();
	controller.undo();
	const afterUndo = controller.getSelections();
	controller.redo();

	assert.deepEqual({
		transactionIds: [
			first?.transactionId,
			second?.transactionId,
		],
		text: model.getText(),
		afterTyping,
		afterUndo,
		afterRedo: controller.getSelections(),
	}, {
		transactionIds: [1, 1],
		text: "XYabXY",
		afterTyping: cursors([2, 6]),
		afterUndo: cursors([0, 2]),
		afterRedo: cursors([2, 6]),
	});
});

test("Explicit selection changes break typing coalescing", () => {
	using model = new TextModel("");
	using controller = createTestCursorsController(
		model,
		cursors([0]),
	);
	const first = controller.execute({
		edits: [{ range: range(0, 0), text: "A" }],
		selectionsAfter: [{ anchorOffset: 1, activeOffset: 1 }],
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.CoalesceTyping,
	});
	controller.setSelections(cursors([1]));
	const second = controller.execute({
		edits: [{ range: range(1, 1), text: "B" }],
		selectionsAfter: [{ anchorOffset: 2, activeOffset: 2 }],
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.CoalesceTyping,
	});

	controller.undo();

	assert.deepEqual({
		first: first?.transactionId,
		second: second?.transactionId,
		text: model.getText(),
	}, {
		first: 1,
		second: 2,
		text: "A",
	});
});

test("Explicit undo stops break coalescing without adding a step", () => {
	using model = new TextModel("");
	using controller = createTestCursorsController(
		model,
		cursors([0]),
	);
	const first = controller.execute({
		edits: [{ range: range(0, 0), text: "A" }],
		selectionsAfter: [{ anchorOffset: 1, activeOffset: 1 }],
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.CoalesceTyping,
	});
	controller.pushUndoStop();
	controller.pushUndoStop();
	const second = controller.execute({
		edits: [{ range: range(1, 1), text: "B" }],
		selectionsAfter: [{ anchorOffset: 2, activeOffset: 2 }],
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.CoalesceTyping,
	});

	assert.notEqual(first?.transactionId, second?.transactionId);
	controller.undo();
	assert.equal(model.getText(), "A");
	controller.undo();
	assert.equal(model.getText(), "");
});

test("Typing coalesces an initial replacement and following overwrite", () => {
	using model = new TextModel("hello");
	using controller = createTestCursorsController(
		model,
		selections([[1, 4]]),
	);
	const first = controller.execute({
		edits: [{ range: range(1, 4), text: "X" }],
		selectionsAfter: [{ anchorOffset: 2, activeOffset: 2 }],
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.CoalesceTyping,
	});
	const second = controller.execute({
		edits: [{ range: range(2, 2), text: "Y" }],
		selectionsAfter: [{ anchorOffset: 3, activeOffset: 3 }],
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.CoalesceTyping,
	});
	const third = controller.execute({
		edits: [{ range: range(3, 4), text: "Z" }],
		selectionsAfter: [{ anchorOffset: 4, activeOffset: 4 }],
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.CoalesceTyping,
	});

	const afterTyping = model.getText();
	controller.undo();
	const afterUndo = {
		text: model.getText(),
		selections: controller.getSelections(),
	};
	controller.redo();

	assert.deepEqual({
		transactionIds: [
			first?.transactionId,
			second?.transactionId,
			third?.transactionId,
		],
		afterTyping,
		afterUndo,
		afterRedo: {
			text: model.getText(),
			selections: controller.getSelections(),
		},
	}, {
		transactionIds: [1, 1, 1],
		afterTyping: "hXYZ",
		afterUndo: {
			text: "hello",
			selections: selections([[1, 4]]),
		},
		afterRedo: {
			text: "hXYZ",
			selections: cursors([4]),
		},
	});
});

test("Typing coalesces replacements independently at multiple selections", () => {
	using model = new TextModel("ab__CD");
	using controller = createTestCursorsController(
		model,
		selections([[0, 2], [4, 6]], 1),
	);
	controller.execute({
		edits: [
			{ range: range(0, 2), text: "X" },
			{ range: range(4, 6), text: "X" },
		],
		selectionsAfter: [
			{ anchorOffset: 1, activeOffset: 1 },
			{ anchorOffset: 4, activeOffset: 4 },
		],
		primarySelectionIndex: 1,
		historyMode: EditorCommandHistoryMode.CoalesceTyping,
	});
	controller.execute({
		edits: [
			{ range: range(1, 1), text: "Y" },
			{ range: range(4, 4), text: "Y" },
		],
		selectionsAfter: [
			{ anchorOffset: 2, activeOffset: 2 },
			{ anchorOffset: 6, activeOffset: 6 },
		],
		primarySelectionIndex: 1,
		historyMode: EditorCommandHistoryMode.CoalesceTyping,
	});

	controller.undo();
	assert.deepEqual({
		text: model.getText(),
		selections: controller.getSelections(),
	}, {
		text: "ab__CD",
		selections: selections([[0, 2], [4, 6]], 1),
	});
	controller.redo();
	assert.deepEqual({
		text: model.getText(),
		selections: controller.getSelections(),
	}, {
		text: "XY__XY",
		selections: cursors([2, 6], 1),
	});
});

test("TextModel normalizes converged inverse insertions", () => {
	using model = new TextModel("abc");
	model.pushEditOperations(null, [
		{ range: range(0, 1), text: "" },
		{ range: range(1, 2), text: "" },
	], () => null);
	assert.equal(model.getText(), "c");

	model.undo();

	assert.equal(model.getText(), "abc");
});

test("Coalesced history obeys the text-unit budget", () => {
	using model = new TextModel("ab", {
		historyLimit: {
			transactions: 10,
			textUnits: 1,
		},
	});
	using controller = createTestCursorsController(
		model,
		cursors([2]),
	);
	const first = controller.execute({
		edits: [{ range: range(1, 2), text: "" }],
		selectionsAfter: [{ anchorOffset: 1, activeOffset: 1 }],
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.CoalesceBackspace,
	});
	const second = controller.execute({
		edits: [{ range: range(0, 1), text: "" }],
		selectionsAfter: [{ anchorOffset: 0, activeOffset: 0 }],
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.CoalesceBackspace,
	});

	assert.deepEqual({
		transactionIds: [first?.transactionId, second?.transactionId],
		text: model.getText(),
		undo: controller.undo(),
	}, {
		transactionIds: [1, 1],
		text: "",
		undo: undefined,
	});
});

test("CursorsController coalesces Backspace commands", () => {
	using model = new TextModel("abcd");
	using controller = createTestCursorsController(
		model,
		cursors([4]),
	);
	const first = controller.execute({
		edits: [{ range: range(3, 4), text: "" }],
		selectionsAfter: [{ anchorOffset: 3, activeOffset: 3 }],
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.CoalesceBackspace,
	});
	const second = controller.execute({
		edits: [{ range: range(2, 3), text: "" }],
		selectionsAfter: [{ anchorOffset: 2, activeOffset: 2 }],
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.CoalesceBackspace,
	});

	controller.undo();
	const afterUndo = controller.getSelections();
	controller.redo();

	assert.deepEqual({
		transactionIds: [
			first?.transactionId,
			second?.transactionId,
		],
		text: model.getText(),
		afterUndo,
		afterRedo: controller.getSelections(),
	}, {
		transactionIds: [1, 1],
		text: "ab",
		afterUndo: cursors([4]),
		afterRedo: cursors([2]),
	});
});

test("CursorsController coalesces forward Delete commands", () => {
	using model = new TextModel("abcd");
	using controller = createTestCursorsController(
		model,
		cursors([1]),
	);
	const first = controller.execute({
		edits: [{ range: range(1, 2), text: "" }],
		selectionsAfter: [{ anchorOffset: 1, activeOffset: 1 }],
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.CoalesceDelete,
	});
	const second = controller.execute({
		edits: [{ range: range(1, 2), text: "" }],
		selectionsAfter: [{ anchorOffset: 1, activeOffset: 1 }],
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.CoalesceDelete,
	});

	controller.undo();
	const afterUndo = model.getText();
	controller.redo();

	assert.deepEqual({
		transactionIds: [
			first?.transactionId,
			second?.transactionId,
		],
		afterUndo,
		afterRedo: model.getText(),
	}, {
		transactionIds: [1, 1],
		afterUndo: "abcd",
		afterRedo: "ad",
	});
});

test("Backspace coalescing adjusts separated multi-cursor offsets", () => {
	using model = new TextModel("ab__CD");
	using controller = createTestCursorsController(
		model,
		cursors([2, 6]),
	);
	controller.execute({
		edits: [
			{ range: range(1, 2), text: "" },
			{ range: range(5, 6), text: "" },
		],
		selectionsAfter: [
			{ anchorOffset: 1, activeOffset: 1 },
			{ anchorOffset: 4, activeOffset: 4 },
		],
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.CoalesceBackspace,
	});
	controller.execute({
		edits: [
			{ range: range(0, 1), text: "" },
			{ range: range(3, 4), text: "" },
		],
		selectionsAfter: [
			{ anchorOffset: 0, activeOffset: 0 },
			{ anchorOffset: 2, activeOffset: 2 },
		],
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.CoalesceBackspace,
	});

	controller.undo();

	assert.deepEqual({
		text: model.getText(),
		selections: controller.getSelections(),
	}, {
		text: "ab__CD",
		selections: cursors([2, 6]),
	});
});

function primaryFirst<T>(items: readonly T[], primaryIndex: number): readonly T[] {
	if (primaryIndex === 0) return Object.freeze([...items]);
	return Object.freeze([items[primaryIndex]!, ...items.slice(0, primaryIndex), ...items.slice(primaryIndex + 1)]);
}

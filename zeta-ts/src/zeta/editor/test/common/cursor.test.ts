import assert from "node:assert/strict";
import test from "node:test";
import { DisposableTracker, installDisposableTracker } from "../../../base/common/lifecycle.js";
import { CursorsController } from "../../common/cursor/cursor.js";
import { CursorChangeReason } from "../../common/cursorEvents.js";
import { Selection } from "../../common/core/selection.js";
import { SelectionSet } from "../../common/cursor/selectionSet.js";
import { Position } from "../../common/core/position.js";
import { Range } from "../../common/core/range.js";
import { TextModel } from "../../common/model/textModel.js";

const position = (lineIndex: number, columnIndex: number): Position => new Position(lineIndex + 1, columnIndex + 1);
const range = (
	startColumn: number,
	endColumn: number,
): Range => Range.fromPositions(
	position(0, startColumn),
	position(0, endColumn),
);
const single = (
	anchorColumn: number,
	activeColumn: number,
): SelectionSet => SelectionSet.single(
	Selection.fromPositions(
		position(0, anchorColumn),
		position(0, activeColumn),
	),
);

test("CursorsController restores command selections", () => {
	using model = new TextModel("hello");
	using controller = new CursorsController(
		model,
		single(4, 1),
	);
	const reasons: CursorChangeReason[] = [];
	using listener = controller.onDidChange(
		event => reasons.push(event.reason),
	);

	const change = controller.execute({
		edits: [{ range: range(1, 4), text: "i" }],
		selectionsAfter: [{ anchorOffset: 2, activeOffset: 2 }],
		primarySelectionIndex: 0,
	});
	const afterCommand = controller.selections;
	const undoChange = controller.undo();
	const afterUndo = controller.selections;
	const redoChange = controller.redo();

	assert.deepEqual({
		text: model.getText(),
		transactionIds: [
			change?.transactionId,
			undoChange?.transactionId,
			redoChange?.transactionId,
		],
		afterCommand,
		afterUndo,
		afterRedo: controller.selections,
		reasons,
	}, {
		text: "hio",
		transactionIds: [1, 1, 1],
		afterCommand: single(2, 2),
		afterUndo: single(4, 1),
		afterRedo: single(2, 2),
		reasons: [
			CursorChangeReason.NotSet,
			CursorChangeReason.Undo,
			CursorChangeReason.Redo,
		],
	});
});

test("Read-only editor instances preserve selection while rejecting document commands", () => {
	using model = new TextModel("abc");
	using controller = new CursorsController(model, single(0, 0), { readOnly: true });

	const command = {
		edits: [{ range: range(0, 0), text: "X" }],
		selectionsAfter: [{ anchorOffset: 1, activeOffset: 1 }],
		primarySelectionIndex: 0,
	};
	assert.equal(controller.readOnly, true);
	assert.equal(controller.execute(command), undefined);
	assert.equal(model.getText(), "abc");
	assert.deepEqual(controller.selections, single(0, 0));
	controller.setSelections(single(2, 2));
	assert.deepEqual(controller.selections, single(2, 2));
	assert.equal(controller.undo(), undefined);
	assert.equal(controller.redo(), undefined);
	assert.throws(() => controller.beginComposition(), /read-only/);
});

test("Cursor-only selection history restores multi-cursor operations without changing document undo", () => {
	using model = new TextModel("abc");
	using controller = new CursorsController(model, single(0, 0), { cursorHistoryLimit: 1 });
	const reasons: CursorChangeReason[] = [];
	using listener = controller.onDidChange(event => reasons.push(event.reason));
	const first = SelectionSet.withPrimary([
		Selection.fromPositions(position(0, 0)),
		Selection.fromPositions(position(0, 1)),
	], 1);
	const second = SelectionSet.withPrimary([
		Selection.fromPositions(position(0, 0)),
		Selection.fromPositions(position(0, 1)),
		Selection.fromPositions(position(0, 2)),
	], 2);

	controller.setCursorSelections(first);
	controller.setCursorSelections(second);
	assert.equal(controller.undoCursorOperation(), true);
	assert.deepEqual(controller.selections, first);
	assert.equal(controller.undoCursorOperation(), false);
	controller.setCursorSelections(second);
	controller.setSelections(single(2, 2));
	assert.equal(controller.undoCursorOperation(), false);
	assert.equal(model.version, 1);
	assert.deepEqual(reasons, [
		CursorChangeReason.Explicit,
		CursorChangeReason.Explicit,
		CursorChangeReason.Explicit,
		CursorChangeReason.Explicit,
		CursorChangeReason.NotSet,
	]);
});

test("CursorsController maps external model edits", () => {
	using model = new TextModel("abc");
	using controller = new CursorsController(
		model,
		single(2, 1),
	);
	const events: unknown[] = [];
	using listener = controller.onDidChange(event => events.push(event));

	model.applyEdits([{ range: range(0, 0), text: "X" }]);

	assert.deepEqual({
		text: model.getText(),
		selections: controller.selections,
		events,
	}, {
		text: "Xabc",
		selections: single(3, 2),
		events: [{
			selections: single(3, 2),
			reason: CursorChangeReason.RecoverFromMarkers,
			modelVersion: 2,
		}],
	});
});

test("CursorsController projects tracked selections before downstream command listeners", () => {
	using model = new TextModel("const value = 1;\n");
	using controller = new CursorsController(
		model,
		SelectionSet.single(Selection.fromPositions(
			new Position((0) + 1, (0) + 1),
			model.positionAt(model.length),
		)),
	);
	const observed: Selection[] = [];
	using listener = model.onDidChange(() => {
		const selection = controller.selections.primary;
		assert.doesNotThrow(() => {
			model.offsetAt(selection.getSelectionStart());
			model.offsetAt(selection.getPosition());
		});
		observed.push(selection);
	});

	controller.execute({
		edits: [{ range: Range.fromPositions(new Position((0) + 1, (0) + 1), model.positionAt(model.length)), text: "x" }],
		selectionsAfter: [{ anchorOffset: 1, activeOffset: 1 }],
		primarySelectionIndex: 0,
	});

	assert.deepEqual(observed, [Selection.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (1) + 1))]);
	assert.deepEqual(controller.selections, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (1) + 1))));
});

test("CursorsController releases tracked ranges without taking their model ownership", () => {
	const tracker = new DisposableTracker();
	{
		using installation = installDisposableTracker(tracker);
		using model = new TextModel("abc");
		using controller = new CursorsController(model, single(0, 0));

		controller.setSelections(single(2, 2));
	}

	tracker.assertNoLeaks();
});

test("Shared editors retain independent selection ownership", () => {
	using model = new TextModel("abc");
	using first = new CursorsController(model, single(1, 1));
	using second = new CursorsController(model, single(3, 3));

	first.execute({
		edits: [{ range: range(1, 1), text: "X" }],
		selectionsAfter: [{ anchorOffset: 2, activeOffset: 2 }],
		primarySelectionIndex: 0,
	});
	assert.deepEqual({
		first: first.selections,
		second: second.selections,
	}, {
		first: single(2, 2),
		second: single(4, 4),
	});

	second.undo();
	assert.deepEqual({
		text: model.getText(),
		first: first.selections,
		second: second.selections,
	}, {
		text: "abc",
		first: single(1, 1),
		second: single(3, 3),
	});

	first.redo();
	assert.deepEqual({
		text: model.getText(),
		first: first.selections,
		second: second.selections,
	}, {
		text: "aXbc",
		first: single(2, 2),
		second: single(4, 4),
	});
});

test("CursorsController validates commands before mutation", () => {
	using model = new TextModel("abc");
	using controller = new CursorsController(
		model,
		single(0, 0),
	);

	assert.throws(() => controller.execute({
		edits: [{ range: range(1, 2), text: "" }],
		selectionsAfter: [{
			anchorOffset: 3,
			activeOffset: 3,
		}],
		primarySelectionIndex: 0,
	}), /anchorOffset/);
	assert.deepEqual({
		text: model.getText(),
		version: model.version,
		selections: controller.selections,
	}, {
		text: "abc",
		version: 1,
		selections: single(0, 0),
	});
});

test("CursorsController disposal does not own the model", () => {
	using model = new TextModel("abc");
	const controller = new CursorsController(
		model,
		single(0, 0),
	);
	controller.dispose();

	assert.throws(
		() => controller.selections,
		/already disposed/,
	);
	model.applyEdits([{ range: range(0, 1), text: "A" }]);
	assert.equal(model.getText(), "Abc");
});

test("CursorsController rejects stale post-command selections", () => {
	using model = new TextModel("abc");
	using controller = new CursorsController(
		model,
		single(0, 0),
	);
	const reasons: CursorChangeReason[] = [];
	using controllerListener = controller.onDidChange(
		event => reasons.push(event.reason),
	);
	using reentrantListener = model.onDidChange(event => {
		if (event.version === 2) {
			model.applyEdits([{
				range: Range.fromPositions(model.positionAt(model.getText().length)),
				text: "Y",
			}]);
		}
	});

	controller.execute({
		edits: [{ range: range(0, 0), text: "X" }],
		selectionsAfter: [{ anchorOffset: 1, activeOffset: 1 }],
		primarySelectionIndex: 0,
	});

	assert.deepEqual({
		text: model.getText(),
		version: model.version,
		selections: controller.selections,
		reasons,
	}, {
		text: "XabcY",
		version: 3,
		selections: single(1, 1),
		reasons: [CursorChangeReason.RecoverFromMarkers],
	});
});

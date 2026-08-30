import assert from "node:assert/strict";
import test from "node:test";
import { IME } from "../../../base/common/ime.js";
import { CursorsController } from "../../common/cursor/cursor.js";
import { Selection } from "../../common/core/selection.js";
import { SelectionSet } from "../../common/cursor/selectionSet.js";
import { Position } from "../../common/core/position.js";
import { Range } from "../../common/core/range.js";
import { TextModelChangeReason } from "../../common/core/textChange.js";
import { TextModel } from "../../common/model/textModel.js";

const position = (lineIndex: number, columnIndex: number): Position => new Position(lineIndex + 1, columnIndex + 1);
const range = (
	startColumn: number,
	endColumn: number,
): Range => Range.fromPositions(
	position(0, startColumn),
	position(0, endColumn),
);
const selection = (
	anchorOffset: number,
	activeOffset: number,
): SelectionSet => SelectionSet.single(Selection.fromPositions(
	position(0, anchorOffset),
	position(0, activeOffset),
));

test("Composition revisions commit as one selection-aware undo step", () => {
	using model = new TextModel("hello");
	using controller = new CursorsController(
		model,
		selection(1, 4),
	);
	const composition = controller.beginComposition();
	const first = composition.update({
		text: "n",
		selection: { anchorOffset: 1, activeOffset: 1 },
	});
	const second = composition.update({
		text: "ni",
		selection: { anchorOffset: 2, activeOffset: 2 },
	});
	const third = composition.update({
		text: "你",
		selection: { anchorOffset: 1, activeOffset: 1 },
	});

	composition.commit();
	const afterCommit = {
		text: model.getText(),
		selections: controller.selections,
		active: composition.active,
	};
	controller.undo();
	const afterUndo = {
		text: model.getText(),
		selections: controller.selections,
	};
	controller.redo();

	assert.deepEqual({
		transactionIds: [
			first?.transactionId,
			second?.transactionId,
			third?.transactionId,
		],
		afterCommit,
		afterUndo,
		afterRedo: {
			text: model.getText(),
			selections: controller.selections,
		},
	}, {
		transactionIds: [1, 1, 1],
		afterCommit: {
			text: "h你o",
			selections: selection(2, 2),
			active: false,
		},
		afterUndo: {
			text: "hello",
			selections: selection(1, 4),
		},
		afterRedo: {
			text: "h你o",
			selections: selection(2, 2),
		},
	});
});

test("Composition exposes only its active provisional model range", () => {
	using model = new TextModel("a\nbc");
	using controller = new CursorsController(
		model,
		SelectionSet.single(Selection.fromPositions(
			position(1, 1),
			position(1, 2),
		)),
	);
	const composition = controller.beginComposition();
	assert.deepEqual(
		composition.currentRange,
		Range.fromPositions(position(1, 1), position(1, 2)),
	);

	composition.update({
		text: "x\r\ny",
		selection: { anchorOffset: 3, activeOffset: 3 },
	});
	assert.deepEqual({
		text: model.getText(),
		range: composition.currentRange,
	}, {
		text: "a\nbx\ny",
		range: Range.fromPositions(position(1, 1), position(2, 1)),
	});

	composition.commit();
	assert.throws(() => composition.currentRange, /already closed/);
});

test("Composition cancellation restores text without redo history", () => {
	using model = new TextModel("hello");
	using controller = new CursorsController(
		model,
		selection(1, 4),
	);
	const composition = controller.beginComposition();
	composition.update({
		text: "ni",
		selection: { anchorOffset: 2, activeOffset: 2 },
	});
	composition.update({
		text: "你",
		selection: { anchorOffset: 1, activeOffset: 1 },
	});

	const cancellation = composition.cancel();

	assert.deepEqual({
		text: model.getText(),
		selections: controller.selections,
		reason: cancellation?.reason,
		active: composition.active,
		canUndo: model.canUndo(),
		canRedo: model.canRedo(),
		undo: controller.undo(),
	}, {
		text: "hello",
		selections: selection(1, 4),
		reason: TextModelChangeReason.HistoryCancellation,
		active: false,
		canUndo: false,
		canRedo: false,
		undo: undefined,
	});
});

test("Active composition survives zero history budgets until resolution", () => {
	using cancelModel = new TextModel("hello", {
		historyLimit: { transactions: 0, textUnits: 0 },
	});
	using cancelController = new CursorsController(
		cancelModel,
		selection(1, 4),
	);
	const cancelled = cancelController.beginComposition();
	cancelled.update({
		text: "n",
		selection: { anchorOffset: 1, activeOffset: 1 },
	});
	cancelled.update({
		text: "你",
		selection: { anchorOffset: 1, activeOffset: 1 },
	});
	assert.equal(cancelModel.canUndo(), true);
	cancelled.cancel();

	using commitModel = new TextModel("hello", {
		historyLimit: { transactions: 0, textUnits: 0 },
	});
	using commitController = new CursorsController(
		commitModel,
		selection(1, 4),
	);
	const committed = commitController.beginComposition();
	committed.update({
		text: "你",
		selection: { anchorOffset: 1, activeOffset: 1 },
	});
	assert.equal(commitModel.canUndo(), true);
	committed.commit();

	assert.deepEqual({
		cancelledText: cancelModel.getText(),
		cancelledCanUndo: cancelModel.canUndo(),
		committedText: commitModel.getText(),
		committedCanUndo: commitModel.canUndo(),
	}, {
		cancelledText: "hello",
		cancelledCanUndo: false,
		committedText: "h你o",
		committedCanUndo: false,
	});
});

test("No-op composition updates may move the caret without history", () => {
	using model = new TextModel("a");
	using controller = new CursorsController(
		model,
		selection(0, 1),
	);
	const composition = controller.beginComposition();
	const change = composition.update({
		text: "a",
		selection: { anchorOffset: 0, activeOffset: 0 },
	});

	composition.commit();

	assert.deepEqual({
		change,
		text: model.getText(),
		selections: controller.selections,
		canUndo: model.canUndo(),
	}, {
		change: undefined,
		text: "a",
		selections: selection(0, 0),
		canUndo: false,
	});
});

test("Composition returning to its original text leaves no undo step", () => {
	using model = new TextModel("a");
	using controller = new CursorsController(
		model,
		selection(0, 1),
	);
	const committed = controller.beginComposition();
	committed.update({
		text: "X",
		selection: { anchorOffset: 1, activeOffset: 1 },
	});
	committed.update({
		text: "a",
		selection: { anchorOffset: 1, activeOffset: 1 },
	});
	committed.commit();

	assert.deepEqual({
		text: model.getText(),
		canUndo: model.canUndo(),
		undo: controller.undo(),
	}, {
		text: "a",
		canUndo: false,
		undo: undefined,
	});

	controller.setSelections(selection(0, 1));
	const cancelled = controller.beginComposition();
	cancelled.update({
		text: "X",
		selection: { anchorOffset: 1, activeOffset: 1 },
	});
	cancelled.update({
		text: "a",
		selection: { anchorOffset: 1, activeOffset: 1 },
	});
	assert.equal(cancelled.cancel(), undefined);
	assert.equal(model.canUndo(), false);
});

test("External model edits invalidate an active composition", () => {
	using model = new TextModel("ab");
	using controller = new CursorsController(
		model,
		selection(0, 1),
	);
	const composition = controller.beginComposition();
	composition.update({
		text: "X",
		selection: { anchorOffset: 1, activeOffset: 1 },
	});

	model.applyEdits([{ range: range(2, 2), text: "!" }]);

	assert.equal(composition.active, false);
	assert.throws(
		() => composition.update({
			text: "Y",
			selection: { anchorOffset: 1, activeOffset: 1 },
		}),
		/no longer active/,
	);
	assert.throws(() => composition.commit(), /no longer active/);
	assert.throws(() => composition.cancel(), /no longer active/);
	assert.equal(model.getText(), "Xb!");
});

test("Reentrant model edits invalidate composition before update returns", () => {
	using model = new TextModel("ab");
	using controller = new CursorsController(
		model,
		selection(0, 1),
	);
	let nestedEditApplied = false;
	using listener = model.onDidChangeContent(() => {
		if (nestedEditApplied) return;
		nestedEditApplied = true;
		const end = model.positionAt(model.getText().length);
		model.applyEdits([{
			range: Range.fromPositions(end),
			text: "!",
		}]);
	});
	const composition = controller.beginComposition();

	assert.throws(
		() => composition.update({
			text: "X",
			selection: { anchorOffset: 1, activeOffset: 1 },
		}),
		/no longer active/,
	);
	assert.deepEqual({
		active: composition.active,
		text: model.getText(),
	}, {
		active: false,
		text: "Xb!",
	});
});

test("Composition rejects ambiguous ownership and invalid relative offsets", () => {
	using model = new TextModel("ab");
	using controller = new CursorsController(
		model,
		selection(0, 1),
	);
	const composition = controller.beginComposition();

	assert.throws(
		() => controller.execute({
			edits: [{ range: range(0, 1), text: "X" }],
			selectionsAfter: [{ anchorOffset: 1, activeOffset: 1 }],
			primarySelectionIndex: 0,
		}),
		/active composition/,
	);
	assert.throws(
		() => composition.update({
			text: "X",
			selection: { anchorOffset: 2, activeOffset: 2 },
		}),
		/must be between/,
	);
	assert.equal(model.getText(), "ab");
	composition.cancel();

	using multiController = new CursorsController(
		model,
		SelectionSet.withPrimary([
			Selection.fromPositions(position(0, 0)),
			Selection.fromPositions(position(0, 2)),
		], 0),
	);
	assert.throws(
		() => multiController.beginComposition(),
		/exactly one selection/,
	);
});

test("Composition observes the shared base IME coordination state", () => {
	using model = new TextModel("a");
	using controller = new CursorsController(
		model,
		selection(0, 1),
	);

	IME.disable();
	try {
		assert.throws(
			() => controller.beginComposition(),
			/currently disabled/,
		);
		assert.equal(model.getText(), "a");
		assert.equal(model.canUndo(), false);
	} finally {
		IME.enable();
	}
});

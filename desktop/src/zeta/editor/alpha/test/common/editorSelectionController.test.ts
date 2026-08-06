import assert from "node:assert/strict";
import test from "node:test";
import { DisposableTracker, installDisposableTracker } from "../../../../base/common/disposableTracker.js";
import { EditorSelectionChangeReason, EditorSelectionController } from "../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../common/core/selection.js";
import { TextPosition, TextRange } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";

const position = TextPosition.at;
const range = (
  startColumn: number,
  endColumn: number,
): TextRange => TextRange.from(
  position(0, startColumn),
  position(0, endColumn),
);
const single = (
  anchorColumn: number,
  activeColumn: number,
): TextSelectionSet => TextSelectionSet.single(
  TextSelection.from(
    position(0, anchorColumn),
    position(0, activeColumn),
  ),
);

test("EditorSelectionController restores command selections", () => {
  using model = new TextModel("hello");
  using controller = new EditorSelectionController(
    model,
    single(4, 1),
  );
  const reasons: EditorSelectionChangeReason[] = [];
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
      EditorSelectionChangeReason.Command,
      EditorSelectionChangeReason.Undo,
      EditorSelectionChangeReason.Redo,
    ],
  });
});

test("Read-only editor instances preserve selection while rejecting document commands", () => {
  using model = new TextModel("abc");
  using controller = new EditorSelectionController(model, single(0, 0), { readOnly: true });

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
  using controller = new EditorSelectionController(model, single(0, 0), { cursorHistoryLimit: 1 });
  const reasons: EditorSelectionChangeReason[] = [];
  using listener = controller.onDidChange(event => reasons.push(event.reason));
  const first = TextSelectionSet.withPrimary([
    TextSelection.collapsedAt(position(0, 0)),
    TextSelection.collapsedAt(position(0, 1)),
  ], 1);
  const second = TextSelectionSet.withPrimary([
    TextSelection.collapsedAt(position(0, 0)),
    TextSelection.collapsedAt(position(0, 1)),
    TextSelection.collapsedAt(position(0, 2)),
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
    EditorSelectionChangeReason.CursorOperation,
    EditorSelectionChangeReason.CursorOperation,
    EditorSelectionChangeReason.CursorUndo,
    EditorSelectionChangeReason.CursorOperation,
    EditorSelectionChangeReason.Explicit,
  ]);
});

test("EditorSelectionController maps external model edits", () => {
  using model = new TextModel("abc");
  using controller = new EditorSelectionController(
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
      reason: EditorSelectionChangeReason.ModelChange,
      modelVersion: 2,
    }],
  });
});

test("EditorSelectionController projects tracked selections before downstream command listeners", () => {
  using model = new TextModel("const value = 1;\n");
  using controller = new EditorSelectionController(
    model,
    TextSelectionSet.single(TextSelection.from(
      TextPosition.at(0, 0),
      model.positionAt(model.length),
    )),
  );
  const observed: TextSelection[] = [];
  using listener = model.onDidChange(() => {
    const selection = controller.selections.primary;
    assert.doesNotThrow(() => {
      model.offsetAt(selection.anchor);
      model.offsetAt(selection.active);
    });
    observed.push(selection);
  });

  controller.execute({
    edits: [{ range: TextRange.from(TextPosition.at(0, 0), model.positionAt(model.length)), text: "x" }],
    selectionsAfter: [{ anchorOffset: 1, activeOffset: 1 }],
    primarySelectionIndex: 0,
  });

  assert.deepEqual(observed, [TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 1))]);
  assert.deepEqual(controller.selections, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 1))));
});

test("EditorSelectionController releases tracked ranges without taking their model ownership", () => {
  const tracker = new DisposableTracker();
  {
    using installation = installDisposableTracker(tracker);
    using model = new TextModel("abc");
    using controller = new EditorSelectionController(model, single(0, 0));

    controller.setSelections(single(2, 2));
  }

  tracker.assertNoLeaks();
});

test("Shared editors retain independent selection ownership", () => {
  using model = new TextModel("abc");
  using first = new EditorSelectionController(model, single(1, 1));
  using second = new EditorSelectionController(model, single(3, 3));

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

test("EditorSelectionController validates commands before mutation", () => {
  using model = new TextModel("abc");
  using controller = new EditorSelectionController(
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

test("EditorSelectionController disposal does not own the model", () => {
  using model = new TextModel("abc");
  const controller = new EditorSelectionController(
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

test("EditorSelectionController rejects stale post-command selections", () => {
  using model = new TextModel("abc");
  using controller = new EditorSelectionController(
    model,
    single(0, 0),
  );
  const reasons: EditorSelectionChangeReason[] = [];
  using controllerListener = controller.onDidChange(
    event => reasons.push(event.reason),
  );
  using reentrantListener = model.onDidChange(event => {
    if (event.version === 2) {
      model.applyEdits([{
        range: TextRange.emptyAt(model.positionAt(model.getText().length)),
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
    reasons: [EditorSelectionChangeReason.ModelChange],
  });
});

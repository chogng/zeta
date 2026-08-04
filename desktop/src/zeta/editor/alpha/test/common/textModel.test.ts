import assert from "node:assert/strict";
import test from "node:test";
import { TextModelChangeReason, TextPosition, TextRange } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";

const position = TextPosition.at;
const range = (
  startLine: number,
  startColumn: number,
  endLine: number,
  endColumn: number,
): TextRange => TextRange.from(
  position(startLine, startColumn),
  position(endLine, endColumn),
);

test("TextModel normalizes line endings and maps UTF-16 positions", () => {
  using model = new TextModel("A😀\r\nbeta\rgamma\u2028");

  assert.deepEqual({
    text: model.getText(),
    lineCount: model.lineCount,
    lines: [0, 1, 2, 3].map(index => model.getLineContent(index)),
    emojiEndOffset: model.offsetAt(position(0, 3)),
    emojiInterior: model.positionAt(2),
    end: model.positionAt(model.getText().length),
  }, {
    text: "A😀\nbeta\ngamma\n",
    lineCount: 4,
    lines: ["A😀", "beta", "gamma", ""],
    emojiEndOffset: 3,
    emojiInterior: position(0, 2),
    end: position(3, 0),
  });
});

test("TextModel exposes allocation-free document and line lengths", () => {
  using model = new TextModel("alpha\n😀");

  assert.equal(model.length, 8);
  assert.equal(model.getLineLength(0), 5);
  assert.equal(model.getLineLength(1), 2);
  assert.throws(() => model.getLineLength(2), /lineIndex/);
});

test("TextModel reset replaces content and clears undo and redo history", () => {
  using model = new TextModel("initial");
  const snapshot = model.createSnapshot();
  const events: unknown[] = [];
  using listener = model.onDidChange(change => events.push(change));
  model.applyEdits([{ range: range(0, 0, 0, 7), text: "edited" }]);
  model.undo();

  const reset = model.reset("next\r\nline");

  assert.ok(reset);
  assert.equal(model.getText(), "next\nline");
  assert.equal(model.length, 9);
  assert.equal(model.canUndo, false);
  assert.equal(model.canRedo, false);
  assert.equal(model.undo(), undefined);
  assert.equal(model.redo(), undefined);
  assert.equal(snapshot.getText(), "initial");
  assert.equal(reset.reason, TextModelChangeReason.Reset);
  assert.equal(reset.changes.length, 1);
  assert.equal(events.length, 3);

  const sameTextReset = model.reset("next\nline");
  assert.equal(sameTextReset, undefined);
  assert.equal(model.version, reset.version);
  assert.equal(events.length, 3);
});

test("TextModel applies unordered edits against one atomic snapshot", () => {
  using model = new TextModel("alpha\nbeta\ngamma");
  const events: unknown[] = [];
  using listener = model.onDidChange(event => events.push(event));

  const change = model.applyEdits([
    { range: range(2, 0, 2, 5), text: "G" },
    { range: range(0, 5, 0, 5), text: "!" },
    { range: range(1, 0, 1, 4), text: "B\r\nB2" },
  ]);

  assert.deepEqual({
    text: model.getText(),
    version: model.version,
    change,
    eventCount: events.length,
  }, {
    text: "alpha!\nB\nB2\nG",
    version: 2,
    change: {
      version: 2,
      transactionId: 1,
      reason: TextModelChangeReason.Edit,
      changes: [
        {
          range: range(0, 5, 0, 5),
          rangeOffset: 5,
          rangeLength: 0,
          text: "!",
        },
        {
          range: range(1, 0, 1, 4),
          rangeOffset: 6,
          rangeLength: 4,
          text: "B\nB2",
        },
        {
          range: range(2, 0, 2, 5),
          rangeOffset: 11,
          rangeLength: 5,
          text: "G",
        },
      ],
    },
    eventCount: 1,
  });
});

test("TextModel rejects overlapping edits without mutating", () => {
  using model = new TextModel("abcdef");

  assert.throws(() => model.applyEdits([
    { range: range(0, 1, 0, 4), text: "x" },
    { range: range(0, 3, 0, 5), text: "y" },
  ]), /must not overlap/);
  assert.throws(() => model.applyEdits([
    { range: TextRange.emptyAt(position(0, 2)), text: "x" },
    { range: TextRange.emptyAt(position(0, 2)), text: "y" },
  ]), /must not overlap/);
  assert.deepEqual({
    text: model.getText(),
    version: model.version,
    canUndo: model.canUndo,
  }, {
    text: "abcdef",
    version: 1,
    canUndo: false,
  });
});

test("TextModel undo and redo preserve transaction boundaries", () => {
  using model = new TextModel("abcdef");
  const reasons: TextModelChangeReason[] = [];
  using listener = model.onDidChange(event => reasons.push(event.reason));

  model.applyEdits([
    { range: range(0, 1, 0, 3), text: "LONG" },
    { range: range(0, 5, 0, 6), text: "" },
  ]);
  const edited = model.getText();
  model.undo();
  const undone = model.getText();
  model.redo();

  assert.deepEqual({
    edited,
    undone,
    redone: model.getText(),
    version: model.version,
    canUndo: model.canUndo,
    canRedo: model.canRedo,
    reasons,
  }, {
    edited: "aLONGde",
    undone: "abcdef",
    redone: "aLONGde",
    version: 4,
    canUndo: true,
    canRedo: false,
    reasons: [
      TextModelChangeReason.Edit,
      TextModelChangeReason.Undo,
      TextModelChangeReason.Redo,
    ],
  });
});

test("TextModel clears redo on a new edit and ignores exact no-ops", () => {
  using model = new TextModel("abc");
  let eventCount = 0;
  using listener = model.onDidChange(() => eventCount += 1);

  assert.equal(model.applyEdits([
    { range: range(0, 0, 0, 3), text: "abc" },
  ]), undefined);
  model.applyEdits([
    { range: range(0, 0, 0, 1), text: "A" },
  ]);
  model.undo();
  model.applyEdits([
    { range: range(0, 1, 0, 2), text: "B" },
  ]);

  assert.deepEqual({
    text: model.getText(),
    version: model.version,
    canRedo: model.canRedo,
    eventCount,
  }, {
    text: "aBc",
    version: 4,
    canRedo: false,
    eventCount: 3,
  });
});

test("TextModel validates positions and rejects access after disposal", () => {
  const model = new TextModel("abc");

  assert.throws(() => model.offsetAt(position(1, 0)), /lineIndex/);
  assert.throws(() => model.offsetAt(position(0, 4)), /columnIndex/);
  assert.throws(() => model.positionAt(4), /offset/);
  model.dispose();
  assert.throws(() => model.getText(), /already disposed/);
});

test("TextModel commits history before reentrant change listeners run", () => {
  using model = new TextModel("abc");
  const versions: number[] = [];
  using listener = model.onDidChange(event => {
    versions.push(event.version);
    if (event.version === 2) {
      model.applyEdits([
        { range: range(0, 1, 0, 2), text: "B" },
      ]);
    }
  });

  model.applyEdits([
    { range: range(0, 0, 0, 1), text: "A" },
  ]);
  model.undo();
  const afterFirstUndo = model.getText();
  model.undo();

  assert.deepEqual({
    afterFirstUndo,
    afterSecondUndo: model.getText(),
    versions,
  }, {
    afterFirstUndo: "Abc",
    afterSecondUndo: "abc",
    versions: [2, 3, 4, 5],
  });
});

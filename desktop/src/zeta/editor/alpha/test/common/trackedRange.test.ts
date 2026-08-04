import assert from "node:assert/strict";
import test from "node:test";
import { TextPosition, TextRange } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";
import { TrackedRangeStickiness, type TrackedRange } from "../../common/model/trackedRange.js";

const position = TextPosition.at;
const range = (
  startColumn: number,
  endColumn: number,
): TextRange => TextRange.from(
  position(0, startColumn),
  position(0, endColumn),
);

test("TrackedRange applies explicit insertion-edge stickiness", () => {
  using model = new TextModel("abcd");
  using both = model.trackRange(
    range(1, 3),
    TrackedRangeStickiness.GrowsAtBothEdges,
  );
  using start = model.trackRange(
    range(1, 3),
    TrackedRangeStickiness.GrowsOnlyAtStart,
  );
  using end = model.trackRange(
    range(1, 3),
    TrackedRangeStickiness.GrowsOnlyAtEnd,
  );
  using neither = model.trackRange(
    range(1, 3),
    TrackedRangeStickiness.NeverGrowsAtEdges,
  );

  model.applyEdits([
    { range: range(3, 3), text: "Y" },
    { range: range(1, 1), text: "X" },
  ]);

  assert.deepEqual({
    text: model.getText(),
    both: model.getTextInRange(both.range),
    start: model.getTextInRange(start.range),
    end: model.getTextInRange(end.range),
    neither: model.getTextInRange(neither.range),
  }, {
    text: "aXbcYd",
    both: "XbcY",
    start: "Xbc",
    end: "bcY",
    neither: "bc",
  });
});

test("Collapsed tracked ranges stay ordered at inserted text", () => {
  using model = new TextModel("abc");
  const tracked = new Map<TrackedRangeStickiness, TrackedRange>();
  for (const stickiness of Object.values(TrackedRangeStickiness)) {
    tracked.set(
      stickiness,
      model.trackRange(range(1, 1), stickiness),
    );
  }

  model.applyEdits([{ range: range(1, 1), text: "X" }]);

  assert.deepEqual(
    Object.fromEntries(
      [...tracked].map(([stickiness, trackedRange]) => [
        stickiness,
        {
          range: trackedRange.range,
          text: model.getTextInRange(trackedRange.range),
        },
      ]),
    ),
    {
      [TrackedRangeStickiness.GrowsAtBothEdges]: {
        range: range(1, 2),
        text: "X",
      },
      [TrackedRangeStickiness.GrowsOnlyAtStart]: {
        range: range(1, 1),
        text: "",
      },
      [TrackedRangeStickiness.GrowsOnlyAtEnd]: {
        range: range(2, 2),
        text: "",
      },
      [TrackedRangeStickiness.NeverGrowsAtEdges]: {
        range: range(2, 2),
        text: "",
      },
    },
  );

  for (const trackedRange of tracked.values()) {
    trackedRange.dispose();
  }
});

test("TrackedRange resolves replacement containment deterministically", () => {
  using model = new TextModel("abcdef");
  using both = model.trackRange(
    range(2, 4),
    TrackedRangeStickiness.GrowsAtBothEdges,
  );
  using start = model.trackRange(
    range(2, 4),
    TrackedRangeStickiness.GrowsOnlyAtStart,
  );
  using end = model.trackRange(
    range(2, 4),
    TrackedRangeStickiness.GrowsOnlyAtEnd,
  );
  using neither = model.trackRange(
    range(2, 4),
    TrackedRangeStickiness.NeverGrowsAtEdges,
  );

  model.applyEdits([{ range: range(1, 5), text: "Z" }]);

  assert.deepEqual({
    text: model.getText(),
    both: both.range,
    start: start.range,
    end: end.range,
    neither: neither.range,
  }, {
    text: "aZf",
    both: range(1, 2),
    start: range(1, 1),
    end: range(2, 2),
    neither: range(2, 2),
  });
});

test("TrackedRange updates before events and follows undo and redo", () => {
  using model = new TextModel("abcdef");
  using tracked = model.trackRange(
    range(2, 4),
    TrackedRangeStickiness.NeverGrowsAtEdges,
  );
  const observedText: string[] = [];
  using listener = model.onDidChange(() => {
    observedText.push(model.getTextInRange(tracked.range));
  });

  model.applyEdits([{ range: range(2, 4), text: "XYZ" }]);
  model.undo();
  model.redo();

  assert.deepEqual(observedText, ["XYZ", "cd", "XYZ"]);
});

test("TrackedRange validates model positions and disposal", () => {
  const model = new TextModel("abc");
  assert.throws(
    () => model.trackRange(
      TextRange.emptyAt(position(1, 0)),
      TrackedRangeStickiness.NeverGrowsAtEdges,
    ),
    /lineIndex/,
  );

  const explicitlyDisposed = model.trackRange(
    range(0, 1),
    TrackedRangeStickiness.NeverGrowsAtEdges,
  );
  explicitlyDisposed.dispose();
  assert.throws(
    () => explicitlyDisposed.range,
    /already disposed/,
  );

  const modelOwned = model.trackRange(
    range(1, 2),
    TrackedRangeStickiness.NeverGrowsAtEdges,
  );
  model.dispose();
  assert.throws(() => modelOwned.range, /already disposed/);
});

test("TrackedRange rejects unknown stickiness", () => {
  using model = new TextModel("abc");
  assert.throws(
    () => model.trackRange(
      range(0, 1),
      "invalid" as TrackedRangeStickiness,
    ),
    /Unknown tracked range stickiness/,
  );
});

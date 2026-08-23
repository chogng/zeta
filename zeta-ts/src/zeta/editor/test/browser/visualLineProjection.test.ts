import assert from "node:assert/strict";
import test from "node:test";
import { toDisposable } from "../../../base/common/lifecycle.js";
import { EditorLineWrapping, VisualLineProjection } from "../../browser/viewModel/visualLineProjection.js";
import { type TextMeasurer } from "../../browser/measurement/fontMetrics.js";
import { TextModel } from "../../common/model/textModel.js";
import { TextPosition, TextRange } from "../../common/core/text.js";

test("browser visual-line projection wraps at grapheme boundaries and rebuilds after edits", () => {
  using model = new TextModel("ab😀cd\nxyz");
  using projection = new VisualLineProjection(model, new FixedTextMeasurer(), {
    wrapping: EditorLineWrapping.On,
    wrapWidth: 20,
  });
  let changes = 0;
  using listener = projection.onDidChange(() => changes += 1);

  assert.deepEqual(projection.projection.lines.map(line => ({
    logical: line.logicalLineIndex,
    start: line.startColumn,
    end: line.endColumn,
  })), [
    { logical: 0, start: 0, end: 2 },
    { logical: 0, start: 2, end: 5 },
    { logical: 0, start: 5, end: 6 },
    { logical: 1, start: 0, end: 2 },
    { logical: 1, start: 2, end: 3 },
  ]);

  model.applyEdits([{
    range: TextRange.from(TextPosition.at(1, 0), TextPosition.at(1, 0)),
    text: "qq",
  }]);
  assert.equal(changes, 1);
  assert.equal(projection.projection.visualLineCount, 6);

  projection.setWrapping(EditorLineWrapping.Off);
  assert.equal(projection.projection.visualLineCount, 2);
});

test("browser visual-line projection validates its public wrapping inputs", () => {
  using model = new TextModel("text");
  assert.throws(() => new VisualLineProjection(model, new FixedTextMeasurer(), {
    wrapping: "invalid" as EditorLineWrapping,
  }), /wrapping mode/);
  assert.throws(() => new VisualLineProjection(model, new FixedTextMeasurer(), {
    wrapWidth: -1,
  }), /wrap width/);
  assert.throws(() => new VisualLineProjection(model, new FixedTextMeasurer(), {
    initialWrappingMeasurement: { schedule: undefined as never },
  }), /requires a scheduler/);
  assert.throws(() => new VisualLineProjection(model, new FixedTextMeasurer(), {
    initialWrappingMeasurement: { initialLineCount: 0, schedule: () => toDisposable(() => {}) },
  }), /measurement count/);
});

test("browser visual-line projection measures initial wrapped rows in cancellable idle slices", () => {
  using model = new TextModel("abc\ndefg\nhij");
  const scheduled: (() => void)[] = [];
  const measurer = new CountingTextMeasurer();
  using projection = new VisualLineProjection(model, measurer, {
    wrapping: EditorLineWrapping.On,
    wrapWidth: 20,
    initialWrappingMeasurement: {
      initialLineCount: 1,
      linesPerSlice: 1,
      schedule: callback => {
        scheduled.push(callback);
        return toDisposable(() => {
          const index = scheduled.indexOf(callback);
          if (index >= 0) scheduled.splice(index, 1);
        });
      },
    },
  });

  assert.equal(projection.complete, false);
  assert.equal(measurer.calls, 3);
  assert.deepEqual(projection.projection.lines.map(line => line.endColumn), [2, 3, 4, 3]);
  const first = scheduled.shift();
  assert.ok(first);
  first();
  assert.equal(projection.complete, false);
  assert.equal(measurer.calls, 7);
  assert.deepEqual(projection.projection.lines.map(line => line.endColumn), [2, 3, 4, 3]);
  const second = scheduled.shift();
  assert.ok(second);
  second();
  assert.equal(projection.complete, true);
  assert.equal(measurer.calls, 10);
  assert.deepEqual(projection.projection.lines.map(line => line.endColumn), [2, 3, 2, 4, 2, 3]);
});

test("browser visual-line projection restarts an incomplete wrapped scan after an edit", () => {
  using model = new TextModel("abc\ndef");
  const scheduled: (() => void)[] = [];
  using projection = new VisualLineProjection(model, new FixedTextMeasurer(), {
    wrapping: EditorLineWrapping.On,
    wrapWidth: 20,
    initialWrappingMeasurement: {
      initialLineCount: 1,
      linesPerSlice: 1,
      schedule: callback => {
        scheduled.push(callback);
        return toDisposable(() => {
          const index = scheduled.indexOf(callback);
          if (index >= 0) scheduled.splice(index, 1);
        });
      },
    },
  });

  model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(0, 3)),
    text: "d",
  }]);
  assert.equal(projection.complete, false);
  assert.deepEqual(projection.projection.lines.map(line => ({ logical: line.logicalLineIndex, end: line.endColumn })), [
    { logical: 0, end: 2 },
    { logical: 0, end: 4 },
    { logical: 1, end: 3 },
  ]);
  const complete = scheduled.shift();
  assert.ok(complete);
  complete();
  assert.equal(projection.complete, true);
  assert.equal(projection.projection.visualLineCount, 4);
});

class FixedTextMeasurer implements TextMeasurer {
  readonly horizontalPadding = 0;
  readonly contentLeftPadding = 0;

  refresh(): boolean {
    return false;
  }

  measureLineWidth(text: string): number {
    return [...text].length * 10;
  }
}

class CountingTextMeasurer extends FixedTextMeasurer {
  calls = 0;

  override measureLineWidth(text: string): number {
    this.calls += 1;
    return super.measureLineWidth(text);
  }
}

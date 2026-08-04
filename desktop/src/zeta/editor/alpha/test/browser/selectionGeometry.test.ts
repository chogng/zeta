import assert from "node:assert/strict";
import test from "node:test";
import { type AlphaTextMeasurer } from "../../browser/view/fontMetrics.js";
import { createAlphaSelectionGeometry } from "../../browser/view/selectionGeometry.js";
import { TextSelection, TextSelectionSet } from "../../common/core/selection.js";
import { TextPosition } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";

test("Selection geometry preserves direction, multiple carets, and newlines", () => {
  using model = new TextModel("abcd\nefgh\nij");
  const measurer = new FixedTextMeasurer();
  const selections = TextSelectionSet.withPrimary([
    TextSelection.from(
      TextPosition.at(1, 3),
      TextPosition.at(0, 1),
    ),
    TextSelection.collapsedAt(TextPosition.at(2, 1)),
  ], 0);

  const geometry = createAlphaSelectionGeometry(
    model,
    selections,
    { startLineIndex: 0, endLineIndexExclusive: 3 },
    38,
    measurer,
  );

  assert.deepEqual(geometry, {
    selections: [{
      selectionIndex: 0,
      lineIndex: 0,
      left: 48,
      width: 40,
    }, {
      selectionIndex: 0,
      lineIndex: 1,
      left: 38,
      width: 30,
    }],
    carets: [{
      selectionIndex: 0,
      lineIndex: 0,
      left: 48,
      primary: true,
    }, {
      selectionIndex: 1,
      lineIndex: 2,
      left: 48,
      primary: false,
    }],
  });
});

test("Selection ending at column zero renders only the selected newline", () => {
  using model = new TextModel("abcd\nefgh");
  const geometry = createAlphaSelectionGeometry(
    model,
    TextSelectionSet.single(TextSelection.from(
      TextPosition.at(0, 2),
      TextPosition.at(1, 0),
    )),
    { startLineIndex: 0, endLineIndexExclusive: 2 },
    38,
    new FixedTextMeasurer(),
  );

  assert.deepEqual(geometry.selections, [{
    selectionIndex: 0,
    lineIndex: 0,
    left: 58,
    width: 30,
  }]);
  assert.deepEqual(geometry.carets, [{
    selectionIndex: 0,
    lineIndex: 1,
    left: 38,
    primary: true,
  }]);
});

class FixedTextMeasurer implements AlphaTextMeasurer {
  readonly horizontalPadding = 24;
  readonly contentLeftPadding = 12;

  refresh(): boolean {
    return false;
  }

  measureLineWidth(text: string): number {
    return [...text].length * 10;
  }
}

import assert from "node:assert/strict";
import test from "node:test";
import { AlphaVisibleLineProjection } from "../../browser/view/visibleLineProjection.js";
import { AlphaEditorLineWrapping, AlphaVisualLineProjection } from "../../browser/view/visualLineProjection.js";
import { type AlphaTextMeasurer } from "../../browser/view/fontMetrics.js";
import { EditorFoldingModel } from "../../contrib/folding/browser/foldingModel.js";
import { EditorHiddenRangeModel } from "../../contrib/folding/browser/hiddenRangeModel.js";
import { TextModel } from "../../common/model/textModel.js";
import { TextPosition, TextRange } from "../../common/core/text.js";

test("Visible visual-line projection removes hidden bodies while preserving wrapped header rows", () => {
  using model = new TextModel("header\ninside\nend\nlast");
  using wrapping = new AlphaVisualLineProjection(model, new FixedTextMeasurer(), {
    wrapping: AlphaEditorLineWrapping.On,
    wrapWidth: 20,
  });
  using folding = new EditorFoldingModel(model);
  using hiddenRanges = new EditorHiddenRangeModel(model, folding);
  using projection = new AlphaVisibleLineProjection(wrapping, hiddenRanges);

  assert.deepEqual(projection.projection.lines.map(line => ({ logical: line.logicalLineIndex, start: line.startColumn, end: line.endColumn })), [
    { logical: 0, start: 0, end: 2 },
    { logical: 0, start: 2, end: 4 },
    { logical: 0, start: 4, end: 6 },
    { logical: 1, start: 0, end: 2 },
    { logical: 1, start: 2, end: 4 },
    { logical: 1, start: 4, end: 6 },
    { logical: 2, start: 0, end: 2 },
    { logical: 2, start: 2, end: 3 },
    { logical: 3, start: 0, end: 2 },
    { logical: 3, start: 2, end: 4 },
  ]);

  folding.setRanges([{ startLineIndex: 0, endLineIndex: 2, collapsed: true }]);
  assert.deepEqual(projection.projection.lines.map(line => line.logicalLineIndex), [0, 0, 0, 3, 3]);
  assert.equal(projection.lineSource.lineCount, 5);
  assert.equal(projection.projection.visualLineIndexAt(TextPosition.at(1, 3)), 2);
  assert.equal(projection.projection.lineAt(2)?.logicalLineIndex, 0);
});

test("Visible visual-line projection refreshes the source before collapsed ranges observe a shrinking model", () => {
  using model = new TextModel("header\ninside\nend");
  using folding = new EditorFoldingModel(model);
  using hiddenRanges = new EditorHiddenRangeModel(model, folding);
  using wrapping = new AlphaVisualLineProjection(model, new FixedTextMeasurer());
  using projection = new AlphaVisibleLineProjection(wrapping, hiddenRanges);
  folding.setRanges([{ startLineIndex: 0, endLineIndex: 2, collapsed: true }]);

  assert.doesNotThrow(() => model.applyEdits([{
    range: TextRange.from(TextPosition.at(0, 0), model.positionAt(model.length)),
    text: "x",
  }]));
  assert.equal(projection.projection.logicalLineCount, 1);
  assert.equal(projection.projection.lines.length, 1);
  assert.equal(projection.projection.lineAt(0)?.logicalLineIndex, 0);
});

class FixedTextMeasurer implements AlphaTextMeasurer {
  readonly horizontalPadding = 0;
  readonly contentLeftPadding = 0;

  refresh(): boolean {
    return false;
  }

  measureLineWidth(text: string): number {
    return text.length * 10;
  }
}

import assert from "node:assert/strict";
import test from "node:test";
import { AlphaFoldedVisualLineProjection } from "../../browser/foldedVisualLineProjection.js";
import { AlphaEditorLineWrapping, AlphaVisualLineProjection } from "../../browser/visualLineProjection.js";
import { type AlphaTextMeasurer } from "../../browser/fontMetrics.js";
import { EditorFoldingModel } from "../../language/common/folding.js";
import { TextModel } from "../../common/textModel.js";
import { TextPosition } from "../../common/text.js";

test("Folded visual-line projection removes folded bodies while preserving wrapped header rows", () => {
  using model = new TextModel("header\ninside\nend\nlast");
  using wrapping = new AlphaVisualLineProjection(model, new FixedTextMeasurer(), {
    wrapping: AlphaEditorLineWrapping.On,
    wrapWidth: 20,
  });
  using folding = new EditorFoldingModel(model);
  using projection = new AlphaFoldedVisualLineProjection(wrapping, folding);

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

import assert from "node:assert/strict";
import test from "node:test";
import { TextPosition } from "../../common/text.js";
import { TextModel } from "../../common/textModel.js";
import { EditorVisualLineProjection } from "../../common/visualLineProjection.js";
import { hitTestAlphaVisualEditorPoint, AlphaEditorHitTargetKind } from "../../browser/pointerHitTest.js";
import { type AlphaTextMeasurer } from "../../browser/fontMetrics.js";

test("visual hit testing maps wrapped visual coordinates back to logical UTF-16 positions", () => {
  using model = new TextModel("abcdef\ngh");
  const projection = EditorVisualLineProjection.fromBreakColumns(model, [[2, 4, 6], [2]]);
  const layout = {
    lineHeight: 20,
    viewportSize: { width: 200, height: 80 },
    scrollPosition: { left: 0, top: 0 },
  };
  const metrics = { gutterWidth: 30, textLeft: 40 };
  const measurer = new FixedTextMeasurer();

  assert.deepEqual(hitTestAlphaVisualEditorPoint(model, projection, layout, { left: 52, top: 25 }, metrics, measurer), {
    kind: AlphaEditorHitTargetKind.Text,
    position: TextPosition.at(0, 3),
  });
  assert.deepEqual(hitTestAlphaVisualEditorPoint(model, projection, layout, { left: 100, top: 45 }, metrics, measurer), {
    kind: AlphaEditorHitTargetKind.EmptyContent,
    position: TextPosition.at(0, 6),
  });
  assert.deepEqual(hitTestAlphaVisualEditorPoint(model, projection, layout, { left: 10, top: 65 }, metrics, measurer), {
    kind: AlphaEditorHitTargetKind.Gutter,
    position: TextPosition.at(1, 0),
  });
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

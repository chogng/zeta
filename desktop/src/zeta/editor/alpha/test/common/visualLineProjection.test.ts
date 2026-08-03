import assert from "node:assert/strict";
import test from "node:test";
import { TextPosition } from "../../common/text.js";
import { TextModel } from "../../common/textModel.js";
import { EditorVisualLineProjection } from "../../common/visualLineProjection.js";

test("visual line projection maps logical rows, UTF-16 columns, and empty lines", () => {
  using model = new TextModel("ab😀cd\n\nxyz");
  const projection = EditorVisualLineProjection.fromBreakColumns(model, [
    [2, 4, 6],
    [0],
    [1, 3],
  ]);

  assert.deepEqual(projection.lines, [
    {
      visualLineIndex: 0,
      logicalLineIndex: 0,
      startColumn: 0,
      endColumn: 2,
      firstForLogicalLine: true,
      lastForLogicalLine: false,
    },
    {
      visualLineIndex: 1,
      logicalLineIndex: 0,
      startColumn: 2,
      endColumn: 4,
      firstForLogicalLine: false,
      lastForLogicalLine: false,
    },
    {
      visualLineIndex: 2,
      logicalLineIndex: 0,
      startColumn: 4,
      endColumn: 6,
      firstForLogicalLine: false,
      lastForLogicalLine: true,
    },
    {
      visualLineIndex: 3,
      logicalLineIndex: 1,
      startColumn: 0,
      endColumn: 0,
      firstForLogicalLine: true,
      lastForLogicalLine: true,
    },
    {
      visualLineIndex: 4,
      logicalLineIndex: 2,
      startColumn: 0,
      endColumn: 1,
      firstForLogicalLine: true,
      lastForLogicalLine: false,
    },
    {
      visualLineIndex: 5,
      logicalLineIndex: 2,
      startColumn: 1,
      endColumn: 3,
      firstForLogicalLine: false,
      lastForLogicalLine: true,
    },
  ]);
  assert.equal(projection.visualLineIndexAt(TextPosition.at(0, 2)), 1);
  assert.equal(projection.visualLineIndexAt(TextPosition.at(0, 6)), 2);
  assert.equal(projection.visualLineIndexAt(TextPosition.at(1, 0)), 3);
  assert.equal(projection.firstVisualLineIndex(2), 4);
});

test("visual line projection rejects stale or invalid break coordinates", () => {
  using model = new TextModel("a😀");
  assert.throws(() => EditorVisualLineProjection.fromBreakColumns(model, [[1]]), /final visual line break/);
  assert.throws(() => EditorVisualLineProjection.fromBreakColumns(model, [[2, 3]]), /grapheme boundaries/);
  assert.throws(() => EditorVisualLineProjection.fromBreakColumns(model, [[0, 3]]), /empty visual segment/);
  assert.throws(() => EditorVisualLineProjection.fromBreakColumns(model, []), /one entry/);
});

import assert from "node:assert/strict";
import test from "node:test";
import { navigateAsterVisualCursors } from "../../browser/view/visualCursorNavigation.js";
import { EditorCursorNavigationCommand, EditorCursorNavigationMode } from "../../common/cursor/cursorNavigation.js";
import { TextSelection, TextSelectionSet } from "../../common/core/selection.js";
import { TextPosition, TextRange } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";
import { EditorVisualLineProjection } from "../../common/viewModel/modelLineProjection.js";

test("Visual cursor navigation preserves measured horizontal intent across wrapped rows", () => {
  using model = new TextModel("abcdef\na😀bc");
  const projection = EditorVisualLineProjection.fromBreakColumns(model, [
    [2, 4, 6],
    [3, 5],
  ]);
  const first = navigateAsterVisualCursors(
    model,
    projection,
    TextSelectionSet.single(caret(0, 1)),
    {
      command: EditorCursorNavigationCommand.LineDown,
      mode: EditorCursorNavigationMode.Move,
      pageLineCount: 1,
    },
    text => [...text].length * 10,
  );
  assert.deepEqual(first.selections.primary, caret(0, 3));
  assert.deepEqual(first.preferredHorizontalOffsets, [10]);

  const second = navigateAsterVisualCursors(
    model,
    projection,
    first.selections,
    {
      command: EditorCursorNavigationCommand.PageDown,
      mode: EditorCursorNavigationMode.Move,
      pageLineCount: 2,
      preferredHorizontalOffsets: first.preferredHorizontalOffsets,
    },
    text => [...text].length * 10,
  );
  assert.deepEqual(second.selections.primary, caret(1, 1));
  assert.deepEqual(second.preferredHorizontalOffsets, [10]);

  const extended = navigateAsterVisualCursors(
    model,
    projection,
    TextSelectionSet.single(caret(0, 1)),
    {
      command: EditorCursorNavigationCommand.LineDown,
      mode: EditorCursorNavigationMode.Extend,
      pageLineCount: 1,
    },
    text => [...text].length * 10,
  );
  assert.deepEqual(
    extended.selections.primary,
    TextSelection.from(TextPosition.at(0, 1), TextPosition.at(0, 3)),
  );
});

test("Visual cursor navigation uses browser geometry when bidirectional layout provides it", () => {
  using model = new TextModel("abc אבג");
  const projection = EditorVisualLineProjection.fromBreakColumns(model, [[3, 7]]);
  const result = navigateAsterVisualCursors(
    model,
    projection,
    TextSelectionSet.single(caret(0, 1)),
    {
      command: EditorCursorNavigationCommand.LineDown,
      mode: EditorCursorNavigationMode.Move,
      pageLineCount: 1,
    },
    () => 0,
    {
      getHorizontalOffset: position => position.columnIndex === 1 ? 91 : undefined,
      getNearestPosition: (visualLineIndex, horizontalOffset) => {
        assert.equal(visualLineIndex, 1);
        assert.equal(horizontalOffset, 91);
        return TextPosition.at(0, 5);
      },
    },
  );
  assert.deepEqual(result.selections.primary, caret(0, 5));
  assert.deepEqual(result.preferredHorizontalOffsets, [91]);
});

test("Visual cursor navigation rejects stale projections and invalid preferred offsets", () => {
  using model = new TextModel("abc");
  const projection = EditorVisualLineProjection.fromBreakColumns(model, [[3]]);
  model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(0, 3)),
    text: "d",
  }]);

  assert.throws(() => navigateAsterVisualCursors(
    model,
    projection,
    TextSelectionSet.single(caret(0, 0)),
    {
      command: EditorCursorNavigationCommand.LineDown,
      mode: EditorCursorNavigationMode.Move,
      pageLineCount: 1,
    },
    text => text.length,
  ), /current text model projection/);

  const current = EditorVisualLineProjection.fromBreakColumns(model, [[4]]);
  assert.throws(() => navigateAsterVisualCursors(
    model,
    current,
    TextSelectionSet.single(caret(0, 0)),
    {
      command: EditorCursorNavigationCommand.LineDown,
      mode: EditorCursorNavigationMode.Move,
      pageLineCount: 1,
      preferredHorizontalOffsets: [-1],
    },
    text => text.length,
  ), /preferred horizontal offsets/);
});

function caret(lineIndex: number, columnIndex: number): TextSelection {
  return TextSelection.collapsedAt(TextPosition.at(lineIndex, columnIndex));
}

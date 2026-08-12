import assert from "node:assert/strict";
import test from "node:test";
import { EditorClipboardPasteMode, EditorEmptySelectionClipboardPolicy, getEditorClipboardEntries } from "../../common/clipboard.js";
import { createClipboardCutCommand } from "../../../../common/cursor/cursorDeleteOperations.js";
import { createLinePasteCommand } from "../../../../common/cursor/cursorTypeOperations.js";
import { EditorSelectionController } from "../../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition, TextRange } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Clipboard entries resolve selected text and complete-line ranges", () => {
  using model = new TextModel("one\ntwo\nthree");
  const selections = TextSelectionSet.withPrimary([
    caret(0, 2),
    selection(1, 1, 1, 3),
    caret(2, 1),
  ], 2);

  assert.deepEqual(
    getEditorClipboardEntries(
      model,
      selections,
      EditorEmptySelectionClipboardPolicy.Line,
    ),
    [
      {
        text: "one\n",
        sourceRange: range(0, 0, 1, 0),
        pasteMode: EditorClipboardPasteMode.Line,
      },
      {
        text: "wo",
        sourceRange: range(1, 1, 1, 3),
        pasteMode: EditorClipboardPasteMode.Selection,
      },
      {
        text: "three\n",
        sourceRange: range(1, 3, 2, 5),
        pasteMode: EditorClipboardPasteMode.Line,
      },
    ],
  );
});

test("Whole-line cut merges duplicate and overlapping source ranges", () => {
  using model = new TextModel("one\ntwo\nthree");
  const initial = TextSelectionSet.withPrimary([
    caret(0, 2),
    selection(1, 1, 1, 3),
    caret(2, 1),
  ], 2);
  using controller = new EditorSelectionController(model, initial);

  controller.execute(createClipboardCutCommand(
    model,
    controller.selections,
    EditorEmptySelectionClipboardPolicy.Line,
  ));
  assert.deepEqual({
    text: model.getText(),
    selections: controller.selections,
  }, {
    text: "t",
    selections: TextSelectionSet.withPrimary([
      caret(0, 0),
      caret(0, 1),
    ], 1),
  });

  controller.undo();
  assert.deepEqual({
    text: model.getText(),
    selections: controller.selections,
  }, {
    text: "one\ntwo\nthree",
    selections: initial,
  });

  controller.setSelections(TextSelectionSet.withPrimary([
    caret(1, 0),
    caret(1, 2),
  ], 1));
  controller.execute(createClipboardCutCommand(
    model,
    controller.selections,
    EditorEmptySelectionClipboardPolicy.Line,
  ));
  assert.deepEqual({
    text: model.getText(),
    selections: controller.selections,
  }, {
    text: "one\nthree",
    selections: TextSelectionSet.single(caret(1, 0)),
  });
});

test("Whole-line cut handles final and only lines", () => {
  using finalModel = new TextModel("a\nb");
  using finalController = new EditorSelectionController(
    finalModel,
    TextSelectionSet.single(caret(1, 1)),
  );
  finalController.execute(createClipboardCutCommand(
    finalModel,
    finalController.selections,
    EditorEmptySelectionClipboardPolicy.Line,
  ));
  assert.deepEqual({
    text: finalModel.getText(),
    selection: finalController.selections.primary,
  }, {
    text: "a",
    selection: caret(0, 1),
  });

  using onlyModel = new TextModel("abc");
  using onlyController = new EditorSelectionController(
    onlyModel,
    TextSelectionSet.single(caret(0, 2)),
  );
  onlyController.execute(createClipboardCutCommand(
    onlyModel,
    onlyController.selections,
    EditorEmptySelectionClipboardPolicy.Line,
  ));
  assert.deepEqual({
    text: onlyModel.getText(),
    selection: onlyController.selections.primary,
  }, {
    text: "",
    selection: caret(0, 0),
  });
});

test("Line paste inserts before target lines and preserves caret columns", () => {
  using model = new TextModel("a\nbc");
  const initial = TextSelectionSet.withPrimary([
    caret(0, 1),
    caret(1, 1),
  ], 1);
  using controller = new EditorSelectionController(model, initial);

  controller.execute(createLinePasteCommand(
    model,
    controller.selections,
    ["one\r\n", "two\n"],
  ));
  assert.deepEqual({
    text: model.getText(),
    selections: controller.selections,
  }, {
    text: "one\na\ntwo\nbc",
    selections: TextSelectionSet.withPrimary([
      caret(1, 1),
      caret(3, 1),
    ], 1),
  });

  controller.undo();
  assert.deepEqual({
    text: model.getText(),
    selections: controller.selections,
  }, {
    text: "a\nbc",
    selections: initial,
  });
  assert.throws(
    () => createLinePasteCommand(model, TextSelectionSet.single(
      selection(0, 0, 0, 1),
    ), ["x\n"]),
    /requires collapsed selections/,
  );
});

test("Line paste groups multiple target carets on the same line", () => {
  using model = new TextModel("abc");
  const initial = TextSelectionSet.withPrimary([
    caret(0, 1),
    caret(0, 2),
  ], 1);
  using controller = new EditorSelectionController(model, initial);

  controller.execute(createLinePasteCommand(
    model,
    controller.selections,
    ["first\n", "second\n"],
  ));
  assert.deepEqual({
    text: model.getText(),
    selections: controller.selections,
  }, {
    text: "first\nsecond\nabc",
    selections: TextSelectionSet.withPrimary([
      caret(2, 1),
      caret(2, 2),
    ], 1),
  });

  controller.undo();
  assert.deepEqual({
    text: model.getText(),
    selections: controller.selections,
  }, {
    text: "abc",
    selections: initial,
  });
});

test("Clipboard policy and line-paste inputs validate before mutation", () => {
  using model = new TextModel("abc");
  const selections = TextSelectionSet.single(caret(0, 1));
  assert.throws(
    () => getEditorClipboardEntries(
      model,
      selections,
      "unknown" as EditorEmptySelectionClipboardPolicy,
    ),
    /Unknown editor empty-selection clipboard policy/,
  );
  assert.throws(
    () => createLinePasteCommand(model, selections, ["missing newline"]),
    /must end with a line break/,
  );
  assert.equal(model.getText(), "abc");
});

function range(startLine: number, startColumn: number, endLine: number, endColumn: number): TextRange {
  return TextRange.from(
    TextPosition.at(startLine, startColumn),
    TextPosition.at(endLine, endColumn),
  );
}

function selection(startLine: number, startColumn: number, endLine: number, endColumn: number): TextSelection {
  return TextSelection.from(
    TextPosition.at(startLine, startColumn),
    TextPosition.at(endLine, endColumn),
  );
}

function caret(lineIndex: number, columnIndex: number): TextSelection {
  return TextSelection.collapsedAt(TextPosition.at(lineIndex, columnIndex));
}

import assert from "node:assert/strict";
import test from "node:test";
import { createBackspaceCommand, createCutCommand, createDeleteForwardCommand, createDistributedPasteTextCommand, createPasteTextCommand, createTypeTextCommand } from "../../common/editCommands.js";
import { EditorSelectionController } from "../../common/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../common/selection.js";
import { getSelectionTexts } from "../../common/selectionText.js";
import { TextPosition } from "../../common/text.js";
import { TextModel } from "../../common/textModel.js";

test("Typing replaces multiple selections and coalesces with following text", () => {
  using model = new TextModel("abcd efgh");
  const initial = TextSelectionSet.withPrimary([
    TextSelection.from(TextPosition.at(0, 3), TextPosition.at(0, 1)),
    caret(0, 8),
  ], 1);
  using controller = new EditorSelectionController(model, initial);

  controller.execute(createTypeTextCommand(
    model,
    controller.selections,
    "X",
  ));
  assert.deepEqual({
    text: model.getText(),
    selections: controller.selections,
  }, {
    text: "aXd efgXh",
    selections: TextSelectionSet.withPrimary([
      caret(0, 2),
      caret(0, 8),
    ], 1),
  });

  controller.execute(createTypeTextCommand(
    model,
    controller.selections,
    "Y",
  ));
  assert.equal(model.getText(), "aXYd efgXYh");
  controller.undo();
  assert.deepEqual({
    text: model.getText(),
    selections: controller.selections,
  }, {
    text: "abcd efgh",
    selections: initial,
  });
});

test("Backspace deletes graphemes and joins lines", () => {
  using model = new TextModel("a😀b\ncd");
  using controller = new EditorSelectionController(
    model,
    TextSelectionSet.single(caret(0, 3)),
  );

  controller.execute(createBackspaceCommand(model, controller.selections));
  assert.deepEqual({
    text: model.getText(),
    selection: controller.selections.primary,
  }, {
    text: "ab\ncd",
    selection: caret(0, 1),
  });

  controller.setSelections(TextSelectionSet.single(caret(1, 0)));
  controller.execute(createBackspaceCommand(model, controller.selections));
  assert.deepEqual({
    text: model.getText(),
    selection: controller.selections.primary,
  }, {
    text: "abcd",
    selection: caret(0, 2),
  });
});

test("Forward Delete removes graphemes and line breaks", () => {
  using model = new TextModel("a😀b\ncd");
  using controller = new EditorSelectionController(
    model,
    TextSelectionSet.single(caret(0, 1)),
  );

  controller.execute(createDeleteForwardCommand(
    model,
    controller.selections,
  ));
  assert.deepEqual({
    text: model.getText(),
    selection: controller.selections.primary,
  }, {
    text: "ab\ncd",
    selection: caret(0, 1),
  });

  controller.setSelections(TextSelectionSet.single(caret(0, 2)));
  controller.execute(createDeleteForwardCommand(
    model,
    controller.selections,
  ));
  assert.deepEqual({
    text: model.getText(),
    selection: controller.selections.primary,
  }, {
    text: "abcd",
    selection: caret(0, 2),
  });
});

test("Typing normalizes line endings before calculating carets", () => {
  using model = new TextModel("ab");
  using controller = new EditorSelectionController(
    model,
    TextSelectionSet.single(caret(0, 1)),
  );

  controller.execute(createTypeTextCommand(
    model,
    controller.selections,
    "\r\n",
  ));
  assert.deepEqual({
    text: model.getText(),
    selection: controller.selections.primary,
  }, {
    text: "a\nb",
    selection: caret(1, 0),
  });
});

test("Delete boundaries are no-ops and overlapping selections fail early", () => {
  using model = new TextModel("abc");
  using controller = new EditorSelectionController(
    model,
    TextSelectionSet.single(caret(0, 0)),
  );

  const version = model.version;
  controller.execute(createBackspaceCommand(model, controller.selections));
  assert.equal(model.version, version);
  controller.setSelections(TextSelectionSet.single(caret(0, 3)));
  controller.execute(createDeleteForwardCommand(
    model,
    controller.selections,
  ));
  assert.equal(model.version, version);

  const overlapping = TextSelectionSet.withPrimary([
    TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 2)),
    TextSelection.from(TextPosition.at(0, 1), TextPosition.at(0, 3)),
  ], 0);
  assert.throws(
    () => createTypeTextCommand(model, overlapping, "X"),
    /must not overlap/,
  );
  assert.equal(model.getText(), "abc");
});

test("Adjacent deletions merge converged carets while history restores sources", () => {
  using model = new TextModel("abc");
  const initial = TextSelectionSet.withPrimary([
    caret(0, 1),
    caret(0, 2),
  ], 1);
  using controller = new EditorSelectionController(model, initial);

  controller.execute(createBackspaceCommand(model, controller.selections));
  assert.deepEqual({
    text: model.getText(),
    selections: controller.selections,
  }, {
    text: "c",
    selections: TextSelectionSet.single(caret(0, 0)),
  });

  controller.undo();
  assert.deepEqual({
    text: model.getText(),
    selections: controller.selections,
  }, {
    text: "abc",
    selections: initial,
  });
  controller.redo();
  assert.deepEqual(controller.selections, TextSelectionSet.single(caret(0, 0)));
});

test("Paste commands support shared and distributed isolated text", () => {
  using model = new TextModel("ab cd");
  using controller = new EditorSelectionController(
    model,
    TextSelectionSet.withPrimary([
      TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 2)),
      TextSelection.from(TextPosition.at(0, 3), TextPosition.at(0, 5)),
    ], 1),
  );

  controller.execute(createDistributedPasteTextCommand(
    model,
    controller.selections,
    ["A\r\nB", "C"],
  ));
  assert.deepEqual({
    text: model.getText(),
    selections: controller.selections,
  }, {
    text: "A\nB C",
    selections: TextSelectionSet.withPrimary([
      caret(1, 1),
      caret(1, 3),
    ], 1),
  });

  controller.execute(createPasteTextCommand(model, controller.selections, "!"));
  assert.equal(model.getText(), "A\nB! C!");
  controller.undo();
  assert.equal(model.getText(), "A\nB C");
  controller.undo();
  assert.equal(model.getText(), "ab cd");

  assert.throws(
    () => createDistributedPasteTextCommand(model, controller.selections, ["only one"]),
    /match the selection count/,
  );
});

test("Selection text and cut preserve collapsed cursors and restore history", () => {
  using model = new TextModel("abc def");
  const initial = TextSelectionSet.withPrimary([
    TextSelection.from(TextPosition.at(0, 2), TextPosition.at(0, 0)),
    caret(0, 7),
  ], 0);
  using controller = new EditorSelectionController(model, initial);

  assert.deepEqual(getSelectionTexts(model, initial), ["ab", ""]);
  controller.execute(createCutCommand(model, controller.selections));
  assert.deepEqual({
    text: model.getText(),
    selections: controller.selections,
  }, {
    text: "c def",
    selections: TextSelectionSet.withPrimary([
      caret(0, 0),
      caret(0, 5),
    ], 0),
  });

  controller.undo();
  assert.deepEqual({
    text: model.getText(),
    selections: controller.selections,
  }, {
    text: "abc def",
    selections: initial,
  });
});

function caret(lineIndex: number, columnIndex: number): TextSelection {
  return TextSelection.collapsedAt(TextPosition.at(lineIndex, columnIndex));
}

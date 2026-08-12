import assert from "node:assert/strict";
import test from "node:test";
import { createToggleBlockCommentCommand } from "../../common/blockCommentCommands.js";
import { EditorSelectionController } from "../../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Block comments wrap and unwrap directional selections in isolated undo steps", () => {
  using model = new TextModel("alpha beta");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(
    TextSelection.from(TextPosition.at(0, 10), TextPosition.at(0, 6)),
  ));
  const options = { open: "/*", close: "*/" };

  selections.execute(createToggleBlockCommentCommand(model, selections.selections, options));
  assert.equal(model.getText(), "alpha /* beta */");
  assert.deepEqual(selections.selections.primary, TextSelection.from(
    TextPosition.at(0, 13),
    TextPosition.at(0, 9),
  ));
  selections.execute(createToggleBlockCommentCommand(model, selections.selections, options));
  assert.equal(model.getText(), "alpha beta");
  assert.deepEqual(selections.selections.primary, TextSelection.from(
    TextPosition.at(0, 10),
    TextPosition.at(0, 6),
  ));
  selections.undo();
  assert.equal(model.getText(), "alpha /* beta */");
});

test("Block comments place collapsed carets inside the generated pair and support independent cursors", () => {
  using model = new TextModel("one two");
  using selections = new EditorSelectionController(model, TextSelectionSet.withPrimary([
    TextSelection.collapsedAt(TextPosition.at(0, 0)),
    TextSelection.collapsedAt(TextPosition.at(0, 4)),
  ], 1));
  selections.execute(createToggleBlockCommentCommand(model, selections.selections, {
    open: "/*",
    close: "*/",
  }));
  assert.equal(model.getText(), "/* */one /* */two");
  assert.deepEqual(selections.selections, TextSelectionSet.withPrimary([
    TextSelection.collapsedAt(TextPosition.at(0, 3)),
    TextSelection.collapsedAt(TextPosition.at(0, 12)),
  ], 1));
});

test("Block comments reject overlapping selections and invalid tokens before mutation", () => {
  using model = new TextModel("alpha");
  const options = { open: "/*", close: "*/" };
  const overlap = TextSelectionSet.withPrimary([
    TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 3)),
    TextSelection.from(TextPosition.at(0, 2), TextPosition.at(0, 5)),
  ], 0);
  assert.throws(() => createToggleBlockCommentCommand(model, overlap, options), /must not overlap/);
  assert.throws(() => createToggleBlockCommentCommand(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))), {
    open: "",
    close: "*/",
  }), /non-empty/);
  assert.equal(model.getText(), "alpha");
});

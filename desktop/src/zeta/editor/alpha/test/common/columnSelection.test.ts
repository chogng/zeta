import assert from "node:assert/strict";
import test from "node:test";
import { createEditorColumnSelectionSet } from "../../common/cursor/columnSelection.js";
import { TextSelection } from "../../common/core/selection.js";
import { TextPosition } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";

test("Column selection creates directional same-column selections for every physical line", () => {
  using model = new TextModel("abcdef\nab\n12345\nxy");
  const selections = createEditorColumnSelectionSet(
    model,
    TextPosition.at(3, 2),
    TextPosition.at(0, 5),
  );

  assert.deepEqual(selections.selections, [
    TextSelection.from(TextPosition.at(0, 2), TextPosition.at(0, 5)),
    TextSelection.from(TextPosition.at(1, 2), TextPosition.at(1, 2)),
    TextSelection.from(TextPosition.at(2, 2), TextPosition.at(2, 5)),
    TextSelection.from(TextPosition.at(3, 2), TextPosition.at(3, 2)),
  ]);
  assert.equal(selections.primaryIndex, 0);
});

test("Column selection validates both positions against its text model", () => {
  using model = new TextModel("one");
  assert.throws(() => createEditorColumnSelectionSet(model, TextPosition.at(0, 0), TextPosition.at(1, 0)), /lineIndex/);
});

import assert from "node:assert/strict";
import test from "node:test";
import { TextSelection } from "../../../../common/core/selection.js";
import { TextPosition } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";
import { expandSmartSelection } from "../../common/smartSelect.js";

test("Smart select expands a caret through word, pair, line, and document scopes", () => {
  using model = new TextModel("const value = (one + two);\nnext");
  const caret = TextSelection.collapsedAt(TextPosition.at(0, 16));
  const word = expandSmartSelection(model, caret);
  assert.equal(model.getTextInRange(word.range), "one");
  const pair = expandSmartSelection(model, word);
  assert.equal(model.getTextInRange(pair.range), "(one + two)");
  const line = expandSmartSelection(model, pair);
  assert.equal(model.getTextInRange(line.range), "const value = (one + two);");
});

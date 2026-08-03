import assert from "node:assert/strict";
import test from "node:test";
import { expandLineSelections } from "../../common/lineSelection.js";
import { TextSelection, TextSelectionSet } from "../../common/selection.js";
import { TextPosition } from "../../common/text.js";
import { TextModel } from "../../common/textModel.js";

test("Line selection expands through successive physical lines and includes their line breaks", () => {
  using model = new TextModel("zero\none\ntwo");
  let selections = TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 2)));

  selections = expandLineSelections(model, selections);
  assert.deepEqual(selections.primary, TextSelection.from(TextPosition.at(0, 0), TextPosition.at(1, 0)));
  selections = expandLineSelections(model, selections);
  assert.deepEqual(selections.primary, TextSelection.from(TextPosition.at(0, 0), TextPosition.at(2, 0)));
  selections = expandLineSelections(model, selections);
  assert.deepEqual(selections.primary, TextSelection.from(TextPosition.at(0, 0), TextPosition.at(2, 3)));
  assert.deepEqual(expandLineSelections(model, selections), selections);
});

test("Line selection normalizes reverse multi-selections while retaining the primary item", () => {
  using model = new TextModel("zero\none\ntwo\nthree");
  const selections = TextSelectionSet.withPrimary([
    TextSelection.from(TextPosition.at(2, 2), TextPosition.at(1, 1)),
    TextSelection.collapsedAt(TextPosition.at(3, 4)),
  ], 1);
  assert.deepEqual(expandLineSelections(model, selections), TextSelectionSet.withPrimary([
    TextSelection.from(TextPosition.at(1, 0), TextPosition.at(3, 0)),
    TextSelection.from(TextPosition.at(3, 0), TextPosition.at(3, 5)),
  ], 1));
});

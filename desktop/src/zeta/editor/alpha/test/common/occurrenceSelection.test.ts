import assert from "node:assert/strict";
import test from "node:test";
import { addOccurrenceSelection, EditorOccurrenceDirection, selectAllOccurrences } from "../../common/occurrenceSelection.js";
import { TextSelection, TextSelectionSet } from "../../common/selection.js";
import { TextPosition } from "../../common/text.js";
import { TextModel } from "../../common/textModel.js";

test("Occurrence selection starts from a caret word and adds unselected matches with wraparound", () => {
  using model = new TextModel("echo echo\necho");
  let selections = TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 1)));

  selections = addOccurrenceSelection(model, selections, EditorOccurrenceDirection.Next);
  assert.deepEqual(selections, TextSelectionSet.single(TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 4))));
  selections = addOccurrenceSelection(model, selections, EditorOccurrenceDirection.Next);
  assert.deepEqual(selections, TextSelectionSet.withPrimary([
    TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 4)),
    TextSelection.from(TextPosition.at(0, 5), TextPosition.at(0, 9)),
  ], 1));
  selections = addOccurrenceSelection(model, selections, EditorOccurrenceDirection.Next);
  assert.deepEqual(selections, TextSelectionSet.withPrimary([
    TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 4)),
    TextSelection.from(TextPosition.at(0, 5), TextPosition.at(0, 9)),
    TextSelection.from(TextPosition.at(1, 0), TextPosition.at(1, 4)),
  ], 2));
  assert.equal(addOccurrenceSelection(model, selections, EditorOccurrenceDirection.Next), selections);
});

test("Occurrence selection preserves other cursors when its primary cursor becomes a word selection", () => {
  using model = new TextModel("echo echo");
  const selections = TextSelectionSet.withPrimary([
    TextSelection.collapsedAt(TextPosition.at(0, 1)),
    TextSelection.collapsedAt(TextPosition.at(0, 6)),
  ], 0);
  assert.deepEqual(addOccurrenceSelection(model, selections, EditorOccurrenceDirection.Next), TextSelectionSet.withPrimary([
    TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 4)),
    TextSelection.collapsedAt(TextPosition.at(0, 6)),
  ], 0));
});

test("Occurrence selection supports previous direction, Unicode text, select-all, and input validation", () => {
  using model = new TextModel("猫 猫\n犬 猫");
  const source = TextSelectionSet.single(TextSelection.from(TextPosition.at(1, 2), TextPosition.at(1, 3)));
  const previous = addOccurrenceSelection(model, source, EditorOccurrenceDirection.Previous);
  assert.deepEqual(previous, TextSelectionSet.withPrimary([
    TextSelection.from(TextPosition.at(1, 2), TextPosition.at(1, 3)),
    TextSelection.from(TextPosition.at(0, 2), TextPosition.at(0, 3)),
  ], 1));
  const all = selectAllOccurrences(model, source);
  assert.deepEqual(all, TextSelectionSet.withPrimary([
    TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 1)),
    TextSelection.from(TextPosition.at(0, 2), TextPosition.at(0, 3)),
    TextSelection.from(TextPosition.at(1, 2), TextPosition.at(1, 3)),
  ], 2));
  assert.throws(() => addOccurrenceSelection(model, source, "elsewhere" as EditorOccurrenceDirection), /Unknown editor occurrence direction/);
});

test("Occurrence selection leaves an empty cursor unchanged", () => {
  using model = new TextModel("");
  const selections = TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0)));
  assert.equal(addOccurrenceSelection(model, selections, EditorOccurrenceDirection.Next), selections);
  assert.equal(selectAllOccurrences(model, selections), selections);
});

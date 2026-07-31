import { type TextSelectionSet } from "./selection.js";
import { type TextModel } from "./textModel.js";

/** Reads selection text in the stable order owned by the selection set. */
export function getSelectionTexts(model: TextModel, selections: TextSelectionSet): readonly string[] {
  return Object.freeze(
    selections.selections.map(selection => model.getTextInRange(selection.range)),
  );
}

import { type TextSelectionSet } from "../core/selection.js";
import { type TextModel } from "../model/textModel.js";

/** Reads selection text in the stable order owned by the selection set. */
export function getSelectionTexts(model: TextModel, selections: TextSelectionSet): readonly string[] {
  return Object.freeze(
    selections.selections.map(selection => model.getTextInRange(selection.range)),
  );
}

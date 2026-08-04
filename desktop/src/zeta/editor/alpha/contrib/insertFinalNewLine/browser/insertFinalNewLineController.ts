import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { createInsertFinalNewLineCommand } from "../common/insertFinalNewLine.js";

/** Applies the final-newline policy immediately before a save operation. */
export class AlphaInsertFinalNewLineController extends DisposableOwner {
  constructor(private readonly selections: EditorSelectionController) {
    super();
  }

  prepareSave(): void {
    const command = createInsertFinalNewLineCommand(this.selections.textModel, this.selections.selections);
    if (command) this.selections.execute(command);
  }
}

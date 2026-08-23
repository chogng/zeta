import { addDisposableListener, stopEvent } from "../../base/browser/dom.js";
import { DisposableOwner } from "../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../common/core/selection.js";
import { type EditorViewport } from "./view/editorViewport.js";

/** Routes synchronous document-wide editing shortcuts into Aster commands. */
export class EditingCommandController extends DisposableOwner {
  constructor(
    input: HTMLTextAreaElement,
    private readonly viewport: EditorViewport,
    private readonly selections: EditorSelectionController,
  ) {
    super();
    if (viewport.textModel !== selections.textModel) {
      this.dispose();
      throw new TypeError("Aster editing command dependencies must share one text model");
    }
    this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
    if ((event.ctrlKey || event.metaKey) && !event.shiftKey && !event.altKey && event.key.toLowerCase() === "a") {
      stopEvent(event);
      const end = this.viewport.textModel.positionAt(this.viewport.textModel.createSnapshot().length);
      this.selections.setSelections(TextSelectionSet.single(TextSelection.from(this.viewport.textModel.positionAt(0), end)));
      this.viewport.revealPosition(end);
      return;
    }
  }
}

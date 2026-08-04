import { addDisposableListener, stopEvent } from "../../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type TextSelectionSet } from "../../../common/core/selection.js";
import { type AlphaEditorViewport } from "../../../browser/view/editorViewport.js";
import { expandSmartSelection } from "../common/smartSelect.js";

/** Routes the editor smart-select shortcut into the DOM-free range expansion policy. */
export class AlphaSmartSelectController extends DisposableOwner {
  private readonly history: TextSelectionSet[] = [];

  constructor(private readonly input: HTMLTextAreaElement, private readonly viewport: AlphaEditorViewport, private readonly selections: EditorSelectionController, private readonly wordPattern?: () => RegExp | undefined) {
    super();
    if (viewport.textModel !== selections.textModel) throw new TypeError("Alpha smart select dependencies must share a text model");
    this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event), true));
    this.own(selections.onDidChange(change => {
      if (change.reason !== "explicit" && change.reason !== "cursorOperation") this.history.length = 0;
    }));
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing || event.altKey || (!event.ctrlKey && !event.metaKey) || !event.shiftKey) return;
    if (event.key === "ArrowRight") {
      stopEvent(event, { immediate: true });
      this.history.push(this.selections.selections);
      this.selections.setSelections(this.selections.selections.map(selection => expandSmartSelection(this.viewport.textModel, selection, this.wordPattern?.())));
      this.viewport.revealPosition(this.selections.selections.primary.active);
    } else if (event.key === "ArrowLeft") {
      stopEvent(event, { immediate: true });
      const previous = this.history.pop();
      if (previous) this.selections.setSelections(previous);
      this.viewport.revealPosition(this.selections.selections.primary.active);
    }
  }
}

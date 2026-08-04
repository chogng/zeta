import { addDisposableListener } from "../../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type AlphaEditorViewport } from "../../../browser/view/editorViewport.js";
import { type EditorStateModel } from "../common/editorState.js";

/** Binds browser focus, selection, and scroll events into the common editor-state model. */
export class AlphaEditorStateController extends DisposableOwner {
  constructor(private readonly input: HTMLTextAreaElement, private readonly viewport: AlphaEditorViewport, private readonly selections: EditorSelectionController, private readonly state: EditorStateModel) {
    super();
    this.own(addDisposableListener(input, "focus", () => state.setFocused(true)));
    this.own(addDisposableListener(input, "blur", () => state.setFocused(false)));
    this.own(selections.onDidChange(change => state.setSelections(change.selections)));
    this.own(viewport.onDidChangeLayout(layout => state.setScrollPosition(layout.layout.scrollPosition.left, layout.layout.scrollPosition.top)));
  }
}

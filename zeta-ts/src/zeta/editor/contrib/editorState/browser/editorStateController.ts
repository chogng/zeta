import { addDisposableListener } from "../../../../base/browser/dom.js";
import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { EditorStateModel } from "../common/editorState.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

/** Binds browser focus, selection, and scroll events into the common editor-state model. */
export class EditorStateController extends DisposableOwner {
	constructor(private readonly input: HTMLElement, private readonly viewport: EditorViewport, private readonly selections: EditorSelectionController, private readonly state: EditorStateModel) {
		super();
		this.own(addDisposableListener(input, "focus", () => state.setFocused(true)));
		this.own(addDisposableListener(input, "blur", () => state.setFocused(false)));
		this.own(selections.onDidChange(change => state.setSelections(change.selections)));
		this.own(viewport.onDidChangeLayout(layout => state.setScrollPosition(layout.layout.scrollPosition.left, layout.layout.scrollPosition.top)));
	}
}

registerEditorContribution({ id: "editor.contrib.editorState", install: context => {
	if (context.kind !== "text") return;
	const state = context.own(new EditorStateModel(context.model, context.selections.selections));
	context.own(new EditorStateController(context.view.element, context.viewport, context.selections, state));
} });

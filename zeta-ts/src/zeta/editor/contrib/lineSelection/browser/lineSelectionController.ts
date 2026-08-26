import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";
import { expandLineSelections } from "./lineSelection.js";

/** Routes the optional Ctrl/Cmd+L line-expansion command. */
export class LineSelectionController extends DisposableOwner {
	constructor(input: HTMLElement, private readonly viewport: EditorViewport, private readonly selections: EditorSelectionController) {
		super();
		if (viewport.textModel !== selections.textModel) throw new TypeError("Stanza line selection dependencies must share one text model");
		this.own(addDisposableListener(input, "keydown", event => {
			if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph") || (!event.ctrlKey && !event.metaKey) || event.shiftKey || event.altKey || event.key.toLowerCase() !== "l") return;
			stopEvent(event);
			const next = expandLineSelections(viewport.textModel, selections.selections);
			selections.setSelections(next);
			viewport.revealPosition(next.primary.active);
		}));
	}
}

registerEditorContribution({
	id: "editor.contrib.lineSelection",
	install: context => {
		if (context.kind !== "text") return;
		context.own(new LineSelectionController(context.input.element, context.viewport, context.selections));
	},
});

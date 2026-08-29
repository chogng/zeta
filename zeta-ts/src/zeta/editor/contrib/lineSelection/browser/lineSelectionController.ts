import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { type CursorsController } from "../../../common/cursor/cursor.js";
import { type EditorViewport } from "../../../browser/view.js";
import { expandLineSelections } from "./lineSelection.js";

/** Routes the optional Ctrl/Cmd+L line-expansion command. */
export class LineSelectionController extends Disposable {
	constructor(input: HTMLElement, private readonly viewport: EditorViewport, private readonly selections: CursorsController) {
		super();
		if (viewport.textModel !== selections.textModel) throw new TypeError("Stanza line selection dependencies must share one text model");
		this._register(addDisposableListener(input, "keydown", event => {
			if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph") || (!event.ctrlKey && !event.metaKey) || event.shiftKey || event.altKey || event.key.toLowerCase() !== "l") return;
			stopEvent(event);
			const next = expandLineSelections(viewport.textModel, selections.selections);
			selections.setSelections(next);
			viewport.revealPosition(next.primary.getPosition());
		}));
	}
}

registerEditorContribution({
	id: "editor.contrib.lineSelection",
	install: context => {
		if (context.kind !== "text") return;
		context.register(new LineSelectionController(context.view.element, context.viewport, context.selections));
	},
});

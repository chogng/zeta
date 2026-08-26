import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { EditorLineWrapping } from "../../../browser/viewModel/visualLineProjection.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

/** Owns the transient Alt+Z word-wrap toggle for one Stanza viewport. */
export class WordWrapController extends DisposableOwner {
	constructor(
		input: HTMLElement,
		private readonly viewport: EditorViewport,
	) {
		super();
		this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
	}

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
		if (!event.altKey || event.ctrlKey || event.metaKey || event.shiftKey || event.key.toLowerCase() !== "z") return;
		stopEvent(event);
		this.viewport.setLineWrapping(this.viewport.lineWrapping === EditorLineWrapping.On
			? EditorLineWrapping.Off
			: EditorLineWrapping.On);
	}
}

registerEditorContribution({ id: "editor.contrib.wordWrap", install: context => {
	if (context.kind !== "text") return;
	context.own(new WordWrapController(context.input.element, context.viewport));
} });

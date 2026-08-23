import { addDisposableListener } from "../../../../base/browser/dom.js";
import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type TextPosition } from "../../../common/core/text.js";
import { type EditorHitTarget } from "../../../common/viewModel/pointerHitTest.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

export interface ContextMenuRequest { readonly position: TextPosition; readonly target: EditorHitTarget | undefined; readonly clientX: number; readonly clientY: number; }

/** Delegates context-menu composition to the host while keeping editor hit testing local. */
export class ContextMenuController extends DisposableOwner {
	constructor(private readonly viewport: EditorViewport, private readonly showContextMenu: (request: ContextMenuRequest) => void | Promise<void>, private readonly onError: (error: unknown) => void = error => console.error("Stanza context menu failed", error)) {
		super();
		this.own(addDisposableListener<MouseEvent>(viewport.element, "contextmenu", event => {
			event.preventDefault();
			const target = viewport.getNearestTargetAtClientPoint({ clientX: event.clientX, clientY: event.clientY });
			const position = target?.kind === "text" ? target.position : viewport.textModel.positionAt(viewport.textModel.length);
			try {
				const result = showContextMenu({ position, target, clientX: event.clientX, clientY: event.clientY });
				if (result && typeof (result as Promise<void>).then === "function") void (result as Promise<void>).catch(onError);
			} catch (error) {
				onError(error);
			}
		}));
	}
}

registerEditorContribution({
	id: "editor.contrib.contextMenu",
	install: context => {
		if (context.kind !== "text" || !context.options.onShowContextMenu) return;
		context.own(new ContextMenuController(context.viewport, context.options.onShowContextMenu, context.options.onLanguageError));
	},
});

import { addDisposableListener } from "../../../../base/browser/dom.js";
import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { type Position } from "../../../common/core/position.js";
import { type EditorHitTarget } from "../../../common/viewModel/pointerHitTest.js";
import { type EditorViewport } from "../../../browser/view.js";

export interface ContextMenuRequest { readonly position: Position; readonly target: EditorHitTarget | undefined; readonly clientX: number; readonly clientY: number; }

/** Delegates context-menu composition to the host while keeping editor hit testing local. */
export class ContextMenuController extends Disposable {
	constructor(private readonly viewport: EditorViewport, private readonly showContextMenu: (request: ContextMenuRequest) => void | Promise<void>, private readonly onError: (error: unknown) => void = error => console.error("Stanza context menu failed", error)) {
		super();
		this._register(addDisposableListener<MouseEvent>(viewport.element, "contextmenu", event => {
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
		context.register(new ContextMenuController(context.viewport, context.options.onShowContextMenu, context.options.onLanguageError));
	},
});

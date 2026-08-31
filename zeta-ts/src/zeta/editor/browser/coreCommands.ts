import { addDisposableListener, stopEvent } from "../../base/browser/dom.js";
import { type IDisposable } from "../../base/common/lifecycle.js";
import { Selection } from "../common/core/selection.js";
import { type TextModel } from "../common/model/textModel.js";
import { type CursorsController } from '../common/cursor/cursor.js';
import { registerTextEditorCapabilityContribution } from "./editorExtensions.js";
import { type View } from "./view.js";

export const EditorCoreCommandId = Object.freeze({
	selectAll: "editor.action.selectAll",
});

export const enum NavigationCommandRevealType {
	Regular = 0,
	Minimal = 1,
	None = 2,
}

export interface CoreTextEditorCommandContext {
	readonly model: TextModel;
	readonly viewport: View;
	readonly viewModel: CursorsController;
}

/** Executes the built-in text-editor Select All command. */
export function selectAll(context: CoreTextEditorCommandContext): void {
	if (context.model !== context.viewport.textModel || context.model !== context.viewModel.textModel) {
		throw new TypeError("Editor core command dependencies must share one text model");
	}
	const end = context.model.positionAt(context.model.createVersionedSnapshot().length);
	context.viewModel.setSelections([Selection.fromPositions(context.model.positionAt(0), end)]);
	context.viewport.revealPosition(end);
}

/** Installs text-editor core keybindings for one editor lifetime. */
export function installCoreTextEditorCommands(
	input: HTMLElement,
	viewport: View,
	viewModel: CursorsController,
): IDisposable {
	if (viewport.textModel !== viewModel.textModel) {
		throw new TypeError("Editor core command dependencies must share one text model");
	}
	return addDisposableListener(input, "keydown", event => {
		if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
		if ((event.ctrlKey || event.metaKey) && !event.shiftKey && !event.altKey && event.key.toLowerCase() === "a") {
			stopEvent(event);
			selectAll({ model: viewport.textModel, viewport, viewModel });
		}
	});
}

registerTextEditorCapabilityContribution({
	id: EditorCoreCommandId.selectAll,
	install: context => {
		if (context.kind !== "text") return;
		context.register(installCoreTextEditorCommands(context.view.element, context.viewport, context.viewModel));
	},
});

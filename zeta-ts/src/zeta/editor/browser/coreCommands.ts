import { addDisposableListener, stopEvent } from "../../base/browser/dom.js";
import { type IDisposable } from "../../base/common/lifecycle.js";
import { type CursorsController } from "../common/cursor/cursor.js";
import { Selection } from "../common/core/selection.js";
import { SelectionSet } from "../common/cursor/selectionSet.js";
import { type TextModel } from "../common/model/textModel.js";
import { registerTextEditorCapabilityContribution } from "./editorExtensions.js";
import { type View } from "./view.js";

export const EditorCoreCommandId = Object.freeze({
	selectAll: "editor.action.selectAll",
});

export interface CoreTextEditorCommandContext {
	readonly model: TextModel;
	readonly viewport: View;
	readonly selections: CursorsController;
}

/** Executes the built-in text-editor Select All command. */
export function selectAll(context: CoreTextEditorCommandContext): void {
	if (context.model !== context.viewport.textModel || context.model !== context.viewModel.textModel) {
		throw new TypeError("Editor core command dependencies must share one text model");
	}
	const end = context.model.positionAt(context.model.createVersionedSnapshot().length);
	context.viewModel.setSelections(SelectionSet.single(Selection.fromPositions(context.model.positionAt(0), end)));
	context.viewport.revealPosition(end);
}

/** Installs text-editor core keybindings for one editor lifetime. */
export function installCoreTextEditorCommands(
	input: HTMLElement,
	viewport: View,
	selections: CursorsController,
): IDisposable {
	if (viewport.textModel !== selections.textModel) {
		throw new TypeError("Editor core command dependencies must share one text model");
	}
	return addDisposableListener(input, "keydown", event => {
		if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
		if ((event.ctrlKey || event.metaKey) && !event.shiftKey && !event.altKey && event.key.toLowerCase() === "a") {
			stopEvent(event);
			selectAll({ model: viewport.textModel, viewport, selections });
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

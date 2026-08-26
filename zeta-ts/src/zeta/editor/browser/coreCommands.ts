import { addDisposableListener, stopEvent } from "../../base/browser/dom.js";
import { type IDisposable } from "../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../common/core/selection.js";
import { type TextModel } from "../common/model/textModel.js";
import { registerEditorContribution } from "./editorExtensions.js";
import { type EditorViewport } from "./view/editorViewport.js";

export const EditorCoreCommandId = Object.freeze({
	selectAll: "editor.action.selectAll",
});

export interface CoreTextEditorCommandContext {
	readonly model: TextModel;
	readonly viewport: EditorViewport;
	readonly selections: EditorSelectionController;
}

/** Executes the built-in text-editor Select All command. */
export function selectAll(context: CoreTextEditorCommandContext): void {
	if (context.model !== context.viewport.textModel || context.model !== context.selections.textModel) {
		throw new TypeError("Editor core command dependencies must share one text model");
	}
	const end = context.model.positionAt(context.model.createSnapshot().length);
	context.selections.setSelections(TextSelectionSet.single(TextSelection.from(context.model.positionAt(0), end)));
	context.viewport.revealPosition(end);
}

/** Installs text-editor core keybindings for one editor lifetime. */
export function installCoreTextEditorCommands(
	input: HTMLElement,
	viewport: EditorViewport,
	selections: EditorSelectionController,
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

registerEditorContribution({
	id: EditorCoreCommandId.selectAll,
	install: context => {
		if (context.kind !== "text") return;
		context.own(installCoreTextEditorCommands(context.input.element, context.viewport, context.selections));
	},
});

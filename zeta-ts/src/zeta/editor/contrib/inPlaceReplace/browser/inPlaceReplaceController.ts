import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { createEditorEditCommand } from "../../../common/commands/editorCommand.js";
import { type CursorsController } from "../../../common/cursor/cursor.js";
import { type EditorViewport } from "../../../browser/view.js";
import { type IEditorWorkerClient } from "../../../common/services/editorWorker.js";
import { DEFAULT_WORD_REGEXP } from "../../../common/core/wordHelper.js";

/** Replaces the current number or well-known value with its neighbor. */
export class InPlaceReplaceController extends Disposable {
	constructor(private readonly input: HTMLElement, private readonly viewport: EditorViewport, private readonly selections: CursorsController, private readonly editorWorker: IEditorWorkerClient, private readonly wordDefinition: () => RegExp, private readonly onError: (error: unknown) => void) {
		super();
		if (viewport.textModel !== selections.textModel) throw new TypeError("Stanza in-place replace dependencies must share a text model");
		this._register(addDisposableListener(input, "keydown", event => {
			if (event.defaultPrevented || event.isComposing || !event.shiftKey || event.key !== "Enter" || (!event.ctrlKey && !event.metaKey)) return;
			stopEvent(event);
			void this.replace(event.altKey ? -1 : 1).catch(this.onError);
		}, true));
	}

	async replace(direction: 1 | -1): Promise<boolean> {
		const model = this.viewport.textModel;
		const selectionState = this.selections.selections;
		const selection = selectionState.primary;
		if (selection.range.start.lineIndex !== selection.range.end.lineIndex) return false;
		const result = await this.editorWorker.navigateValueSet(selection.range, direction > 0, this.wordDefinition());
		if (!result || !this.selections.selections.equals(selectionState)) return false;
		const command = createEditorEditCommand(model, this.selections.selections, [{ range: result.range, text: result.value }]);
		if (!command) return false;
		this.selections.execute(command);
		this.viewport.revealPosition(result.range.start);
		return true;
	}
}

registerEditorContribution({
	id: "editor.contrib.inPlaceReplace",
	install: context => {
		if (context.kind !== "text") return;
		context.register(new InPlaceReplaceController(
			context.view.element,
			context.viewport,
			context.selections,
			context.editorWorker,
			() => context.configurations.getLanguageConfiguration(context.languageId).wordPattern ?? DEFAULT_WORD_REGEXP,
			context.onLanguageError,
		));
	},
});

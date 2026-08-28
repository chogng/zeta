import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { createRemoveMatchingBracketsCommand } from "../common/bracketEditing.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type LanguageBracketPairs } from "../../../common/languages/languageBracketPairs.js";
import { type EditorViewport } from "../../../browser/view.js";

/** Routes the VS Code remove-brackets chord through the shared structural bracket index. */
export class BracketEditingController extends Disposable {
	constructor(
		input: HTMLElement,
		private readonly viewport: EditorViewport,
		private readonly selections: EditorSelectionController,
		private readonly bracketPairs: LanguageBracketPairs,
	) {
		super();
		try {
			if (viewport.textModel !== selections.textModel || viewport.textModel !== bracketPairs.textModel) {
				throw new TypeError("Stanza bracket editing dependencies must share one text model");
			}
			this._register(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
		if ((!event.ctrlKey && !event.metaKey) || !event.altKey || event.shiftKey || event.key !== "Backspace") return;
		const command = createRemoveMatchingBracketsCommand(this.bracketPairs, this.selections.selections);
		if (!command) return;
		stopEvent(event);
		this.selections.execute(command);
		this.viewport.revealPosition(this.selections.selections.primary.active);
	}
}

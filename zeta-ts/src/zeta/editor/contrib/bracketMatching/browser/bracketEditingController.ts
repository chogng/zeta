import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { createRemoveMatchingBracketsCommand } from "../common/bracketEditing.js";
import { type CursorsController } from "../../../common/cursor/cursor.js";
import { type LanguageBracketPairs } from "../../../common/languages/languageBracketPairs.js";
import { type View } from "../../../browser/view.js";
import { type EditorCommandExecutor } from '../../../browser/editorExtensions.js';

export const RemoveBracketsCommandId = 'editor.action.removeBrackets';

/** Routes the VS Code remove-brackets chord through the shared structural bracket index. */
export class BracketEditingController extends Disposable {
	constructor(
		input: HTMLElement,
		private readonly viewport: View,
		private readonly selections: CursorsController,
		private readonly bracketPairs: LanguageBracketPairs,
		private readonly executeCommand: EditorCommandExecutor = (_commandId, operation) => operation(),
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
		this.executeCommand(RemoveBracketsCommandId, () => this.selections.execute(command));
		this.viewport.revealPosition(this.selections.selections.primary.getPosition());
	}
}

import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { jumpToMatchingBrackets } from "../common/bracketNavigation.js";
import { type CursorsController } from "../../../common/cursor/cursor.js";
import { type LanguageBracketPairs } from "../../../common/languages/languageBracketPairs.js";
import { type EditorViewport } from "../../../browser/view.js";

/** Routes the VS Code go-to-bracket shortcut through the shared structural bracket index. */
export class BracketNavigationController extends Disposable {
	constructor(
		input: HTMLElement,
		private readonly viewport: EditorViewport,
		private readonly selections: CursorsController,
		private readonly bracketPairs: LanguageBracketPairs,
	) {
		super();
		try {
			if (viewport.textModel !== selections.textModel || viewport.textModel !== bracketPairs.textModel) {
				throw new TypeError("Stanza bracket navigation dependencies must share one text model");
			}
			this._register(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
		if ((!event.ctrlKey && !event.metaKey) || !event.shiftKey || event.altKey || event.key !== "\\") return;
		stopEvent(event);
		const next = jumpToMatchingBrackets(this.bracketPairs, this.selections.selections);
		this.selections.setSelections(next);
		this.viewport.revealPosition(next.primary.active);
	}
}

import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { jumpToMatchingBrackets } from "../common/bracketNavigation.js";
import { type LanguageBracketPairs } from "../../../common/languages/languageBracketPairs.js";
import { type View } from "../../../browser/view.js";
import { type IViewModel } from '../../../common/viewModel.js';

/** Routes the VS Code go-to-bracket shortcut through the shared structural bracket index. */
export class BracketNavigationController extends Disposable {
	constructor(
		input: HTMLElement,
		private readonly viewport: View,
		private readonly viewModel: IViewModel,
		private readonly bracketPairs: LanguageBracketPairs,
	) {
		super();
		try {
			if (viewport.textModel !== viewModel.model || viewport.textModel !== bracketPairs.textModel) {
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
		const next = jumpToMatchingBrackets(this.bracketPairs, this.viewModel.getSelections());
		this.viewModel.setSelections('editor.action.jumpToBracket', next);
		this.viewport.revealPosition(next[0]!.getPosition());
	}
}

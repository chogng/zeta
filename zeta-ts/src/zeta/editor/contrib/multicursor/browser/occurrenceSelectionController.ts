import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { addOccurrenceSelection, EditorOccurrenceDirection, selectAllOccurrences } from "../common/occurrenceSelection.js";
import { type View } from "../../../browser/view.js";
import { type IViewModel } from '../../../common/viewModel.js';
import { CursorChangeReason } from '../../../common/cursorEvents.js';

/** Routes VS Code-compatible occurrence-selection shortcuts through Stanza's common model. */
export class OccurrenceSelectionController extends Disposable {
	constructor(
		input: HTMLElement,
		private readonly viewport: View,
		private readonly viewModel: IViewModel,
	) {
		super();
		try {
			if (viewport.textModel !== viewModel.model) {
				throw new TypeError("Stanza occurrence selection dependencies must share one text model");
			}
			this._register(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
		if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
		if (!event.shiftKey && event.key.toLowerCase() === "d") {
			stopEvent(event);
			this.setSelections(addOccurrenceSelection(
				this.viewport.textModel,
				this.viewModel.getSelections(),
				EditorOccurrenceDirection.Next,
			));
			return;
		}
		if (event.shiftKey && event.key.toLowerCase() === "l") {
			stopEvent(event);
			this.setSelections(selectAllOccurrences(this.viewport.textModel, this.viewModel.getSelections()));
		}
	}

	private setSelections(next: ReturnType<typeof selectAllOccurrences>): void {
		this.viewModel.setSelections('editor.action.selectHighlights', next, CursorChangeReason.Explicit);
		this.viewport.revealPosition(next[0]!.getPosition());
	}
}

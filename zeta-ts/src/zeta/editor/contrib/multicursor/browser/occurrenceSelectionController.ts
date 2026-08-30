import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { addOccurrenceSelection, EditorOccurrenceDirection, selectAllOccurrences } from "../common/occurrenceSelection.js";
import { type CursorsController } from "../../../common/cursor/cursor.js";
import { type View } from "../../../browser/view.js";

/** Routes VS Code-compatible occurrence-selection shortcuts through Stanza's common model. */
export class OccurrenceSelectionController extends Disposable {
	constructor(
		input: HTMLElement,
		private readonly viewport: View,
		private readonly selections: CursorsController,
	) {
		super();
		try {
			if (viewport.textModel !== selections.textModel) {
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
				this.selections.selections,
				EditorOccurrenceDirection.Next,
			));
			return;
		}
		if (event.shiftKey && event.key.toLowerCase() === "l") {
			stopEvent(event);
			this.setSelections(selectAllOccurrences(this.viewport.textModel, this.selections.selections));
		}
	}

	private setSelections(next: ReturnType<typeof selectAllOccurrences>): void {
		this.selections.setCursorSelections(next);
		this.viewport.revealPosition(next.primary.getPosition());
	}
}

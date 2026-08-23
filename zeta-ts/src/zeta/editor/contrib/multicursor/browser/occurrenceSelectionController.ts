import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { addOccurrenceSelection, EditorOccurrenceDirection, selectAllOccurrences } from "../common/occurrenceSelection.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

export interface OccurrenceSelectionControllerOptions {
	readonly wordPattern?: () => RegExp | undefined;
}

/** Routes VS Code-compatible occurrence-selection shortcuts through Aster's common model. */
export class OccurrenceSelectionController extends DisposableOwner {
	constructor(
		input: HTMLTextAreaElement,
		private readonly viewport: EditorViewport,
		private readonly selections: EditorSelectionController,
		options: OccurrenceSelectionControllerOptions = {},
	) {
		super();
		try {
			if (viewport.textModel !== selections.textModel) {
				throw new TypeError("Aster occurrence selection dependencies must share one text model");
			}
			if (options.wordPattern !== undefined && typeof options.wordPattern !== "function") {
				throw new TypeError("Aster occurrence word pattern resolver must be a function");
			}
			this.wordPattern = options.wordPattern;
			this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	private readonly wordPattern: (() => RegExp | undefined) | undefined;

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
		if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
		if (!event.shiftKey && event.key.toLowerCase() === "d") {
			stopEvent(event);
			this.setSelections(addOccurrenceSelection(
				this.viewport.textModel,
				this.selections.selections,
				EditorOccurrenceDirection.Next,
				this.wordPattern?.(),
			));
			return;
		}
		if (event.shiftKey && event.key.toLowerCase() === "l") {
			stopEvent(event);
			this.setSelections(selectAllOccurrences(this.viewport.textModel, this.selections.selections, this.wordPattern?.()));
		}
	}

	private setSelections(next: ReturnType<typeof selectAllOccurrences>): void {
		this.selections.setCursorSelections(next);
		this.viewport.revealPosition(next.primary.active);
	}
}

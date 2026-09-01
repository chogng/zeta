import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { isCancellationError } from "../../../../base/common/errors.js";
import { registerTextEditorCapabilityContribution } from "../../../browser/editorExtensions.js";
import { Disposable, toDisposable } from "../../../../base/common/lifecycle.js";
import { type CursorsController } from "../../../common/cursor/cursor.js";
import { CursorChangeReason } from "../../../common/cursorEvents.js";
import { Selection } from "../../../common/core/selection.js";
import { type Range } from "../../../common/core/range.js";
import { type TextSnapshot } from "../../../common/core/textChange.js";
import { type View } from "../../../browser/view.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { expandSmartSelection } from "../common/smartSelectionExpansion.js";
import { SelectionRangeService } from "../common/selectionRanges.js";

/** Routes the editor smart-select shortcut into the DOM-free range expansion policy. */
export class SmartSelectController extends Disposable {
	private readonly history: Array<readonly Selection[]> = [];
	private request: AbortController | undefined;

	constructor(private readonly input: HTMLElement, private readonly viewport: View, private readonly selections: CursorsController, private readonly languageId: string, private readonly selectionRanges: SelectionRangeService, private readonly onError: (error: unknown) => void) {
		super();
		if (viewport.textModel !== selections.textModel) throw new TypeError("Stanza smart select dependencies must share a text model");
		this._register(addDisposableListener(input, "keydown", event => this.handleKeydown(event), true));
		this._register(selections.onDidChange(change => {
			if (change.reason !== CursorChangeReason.Explicit) this.history.length = 0;
		}));
		this._register(toDisposable(() => this.request?.abort()));
	}

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.altKey || (!event.ctrlKey && !event.metaKey) || !event.shiftKey) return;
		if (event.key === "ArrowRight") {
			stopEvent(event, { immediate: true });
			const before = this.selections.selections;
			this.request?.abort();
			this.request = undefined;
			if (before.every(selection => selection.isEmpty())) {
				this.commitExpansion(before, this.viewport.textModel.createVersionedSnapshot(), []);
				return;
			}
			const request = this.request = new AbortController();
			const snapshot = this.viewport.textModel.createVersionedSnapshot();
			void this.expand(request, before, snapshot);
		} else if (event.key === "ArrowLeft") {
			stopEvent(event, { immediate: true });
			this.request?.abort();
			this.request = undefined;
			const previous = this.history.pop();
			if (previous) this.selections.setSelections(previous);
			this.viewport.revealPosition(this.selections.selections[0]!.getPosition());
		}
	}

	private async expand(request: AbortController, before: readonly Selection[], snapshot: TextSnapshot): Promise<void> {
		try {
			const syntaxRanges = await this.selectionRanges.provideSelectionRanges(this.languageId, before.map(selection => selection), request.signal);
			if (request.signal.aborted || this.request !== request) return;
			this.commitExpansion(before, snapshot, syntaxRanges);
		} catch (error) {
			if (!isCancellationError(error)) {
				this.onError(error);
				if (this.request === request) this.commitExpansion(before, snapshot, []);
			}
		} finally {
			if (this.request === request) this.request = undefined;
		}
	}

	private commitExpansion(before: readonly Selection[], snapshot: TextSnapshot, syntaxRanges: readonly Range[]): void {
		if (this.viewport.textModel.version !== snapshot.version || !Selection.selectionsArrEqual([...this.selections.selections], [...before])) return;
		this.history.push(before);
		this.selections.setSelections(before.map(selection => expandSmartSelection(this.viewport.textModel, selection, syntaxRanges)));
		this.viewport.revealPosition(this.selections.selections[0]!.getPosition());
	}
}

registerTextEditorCapabilityContribution({
	id: "editor.contrib.smartSelect",
	install: context => {
		if (context.kind !== "text") return;
		const selectionRanges = context.register(new SelectionRangeService(context.model, context.languageFeaturesService.selectionRangeProvider, context.options.input.resource));
		context.register(new SmartSelectController(context.view.element, context.viewport, context.selectionController, context.languageId, selectionRanges, context.onLanguageError));
	},
});

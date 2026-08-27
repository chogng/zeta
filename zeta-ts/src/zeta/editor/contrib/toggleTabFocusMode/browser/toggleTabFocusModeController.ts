import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { type TabFocus } from "../../../browser/config/tabFocus.js";
import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { type EditorViewport } from "../../../browser/view.js";

/** Controls whether Tab is routed to editor text insertion or browser focus traversal. */
export class ToggleTabFocusModeController extends Disposable {
	constructor(private readonly input: HTMLElement, private readonly viewport: EditorViewport, private readonly tabFocus: TabFocus) {
		super();
		this._register(this.tabFocus.onDidChange(() => this.updateState()));
		this._register(addDisposableListener(input, "keydown", event => this.handleToggle(event), true));
		this._register(addDisposableListener(input, "keydown", event => {
			if (this.tabFocus.isEnabled && !event.defaultPrevented && !event.isComposing && event.key === "Tab" && !event.ctrlKey && !event.altKey && !event.metaKey) {
				event.stopImmediatePropagation();
			}
		}, true));
		this.updateState();
	}

	get isEnabled(): boolean { return this.tabFocus.isEnabled; }

	setEnabled(enabled: boolean): void { this.tabFocus.setEnabled(enabled); }

	private handleToggle(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.key.toLowerCase() !== "m" || !event.ctrlKey || event.metaKey || event.altKey || event.shiftKey) return;
		stopEvent(event, { immediate: true });
		const enabled = this.tabFocus.toggle();
		this.viewport.announceAccessibilityStatus(enabled ? "Tab moves focus out of the editor" : "Tab inserts indentation");
	}

	private updateState(): void {
		this.viewport.element.dataset.tabFocusMode = String(this.tabFocus.isEnabled);
	}
}

registerEditorContribution({
	id: "editor.contrib.toggleTabFocusMode",
	install: context => {
		if (context.kind !== "text") return;
		context.register(new ToggleTabFocusModeController(context.view.element, context.viewport, context.tabFocus));
	},
});

import "./media/message.css";
import { h } from "../../../../base/browser/dom.js";
import { disposableWindowTimeout } from "../../../../base/browser/scheduler.js";
import { DisposableOwner, DisposableSlot, type IDisposable } from "../../../../base/common/lifecycle.js";
import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

/** Owns transient editor-local messages without replacing host notifications. */
export class MessageController extends DisposableOwner {
	private readonly element: HTMLDivElement;
	private readonly timer = this.own(new DisposableSlot<IDisposable>());

	constructor(private readonly viewport: EditorViewport) {
		super();
		this.element = h(viewport.element.ownerDocument, "div");
		this.element.className = "stanza-editor-message";
		this.element.hidden = true;
		this.element.setAttribute("role", "status");
		this.element.setAttribute("aria-live", "polite");
		viewport.element.append(this.element);
		this.defer(() => this.element.remove());
	}

	show(message: string, durationMs = 3000): void {
		if (typeof message !== "string" || message.trim().length === 0) throw new TypeError("Stanza editor message must be non-empty");
		if (!Number.isSafeInteger(durationMs) || durationMs < 0) throw new RangeError("Stanza editor message duration must be non-negative");
		this.timer.clear();
		this.element.textContent = message.trim();
		this.element.hidden = false;
		const targetWindow = this.element.ownerDocument.defaultView;
		if (durationMs > 0 && targetWindow) this.timer.replace(disposableWindowTimeout(targetWindow, () => { this.timer.clear(); this.element.hidden = true; }, durationMs));
	}

	hide(): void { this.timer.clear(); this.element.hidden = true; }
}

registerEditorContribution({
	id: "editor.contrib.message",
	install: context => {
		if (context.kind !== "text") return;
		context.own(new MessageController(context.viewport));
	},
});

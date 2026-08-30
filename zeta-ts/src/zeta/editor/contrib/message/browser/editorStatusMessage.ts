import "./messageController.css";
import { h } from "../../../../base/browser/dom.js";
import { disposableWindowTimeout } from "../../../../base/browser/scheduler.js";
import { Disposable, MutableDisposable, type IDisposable, toDisposable } from "../../../../base/common/lifecycle.js";
import { registerTextEditorCapabilityContribution } from "../../../browser/editorExtensions.js";
import { type View } from "../../../browser/view.js";

/** Owns transient editor-local messages without replacing host notifications. */
export class EditorStatusMessage extends Disposable {
	private readonly element: HTMLDivElement;
	private readonly timer = this._register(new MutableDisposable<IDisposable>());

	constructor(private readonly viewport: View) {
		super();
		this.element = h(viewport.element.ownerDocument, "div");
		this.element.className = "stanza-editor-message";
		this.element.hidden = true;
		this.element.setAttribute("role", "status");
		this.element.setAttribute("aria-live", "polite");
		viewport.element.append(this.element);
		this._register(toDisposable(() => this.element.remove()));
	}

	show(message: string, durationMs = 3000): void {
		if (typeof message !== "string" || message.trim().length === 0) throw new TypeError("Stanza editor message must be non-empty");
		if (!Number.isSafeInteger(durationMs) || durationMs < 0) throw new RangeError("Stanza editor message duration must be non-negative");
		this.timer.clear();
		this.element.textContent = message.trim();
		this.element.hidden = false;
		const targetWindow = this.element.ownerDocument.defaultView;
		if (durationMs > 0 && targetWindow) this.timer.value = disposableWindowTimeout(targetWindow, () => { this.timer.clear(); this.element.hidden = true; }, durationMs);
	}

	hide(): void { this.timer.clear(); this.element.hidden = true; }
}

registerTextEditorCapabilityContribution({
	id: "editor.contrib.message",
	install: context => {
		if (context.kind !== "text") return;
		context.register(new EditorStatusMessage(context.viewport));
	},
});

import "./media/readOnlyMessage.css";
import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { addDisposableListener, stopEvent, h } from "../../../../base/browser/dom.js";
import { disposableWindowTimeout } from "../../../../base/browser/scheduler.js";
import { DisposableOwner, DisposableSlot, type IDisposable } from "../../../../base/common/lifecycle.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

export interface ReadOnlyMessageControllerOptions {
	readonly message?: string;
	readonly durationMs?: number;
}

/** Explains blocked mutations without making read-only state part of model policy. */
export class ReadOnlyMessageController extends DisposableOwner {
	readonly element: HTMLDivElement;
	private readonly durationMs: number;
	private readonly hideTimer = this.own(new DisposableSlot<IDisposable>());

	constructor(
		input: HTMLElement,
		private readonly viewport: EditorViewport,
		options: ReadOnlyMessageControllerOptions = {},
	) {
		super();
		const message = options.message ?? "This editor is read-only";
		this.durationMs = options.durationMs ?? 2_400;
		if (typeof message !== "string" || message.trim().length === 0) {
			this.dispose();
			throw new TypeError("Stanza read-only message must not be empty");
		}
		if (!Number.isSafeInteger(this.durationMs) || this.durationMs < 0) {
			this.dispose();
			throw new RangeError("Stanza read-only message duration must be a non-negative safe integer");
		}
		const ownerDocument = viewport.element.ownerDocument;
		this.element = h(ownerDocument, "div");
		this.element.className = "stanza-editor-read-only-message";
		this.element.hidden = true;
		this.element.textContent = message;
		this.element.setAttribute("role", "status");
		this.element.setAttribute("aria-live", "polite");
		viewport.element.append(this.element);
		this.defer(() => this.element.remove());
		this.own(addDisposableListener(input, "keydown", event => {
			if (event.defaultPrevented || event.isComposing || !isMutationKey(event)) return;
			stopEvent(event);
			this.show();
		}));
		this.own(addDisposableListener(input, "beforeinput", event => {
			if (event.defaultPrevented || !isMutationInput(event)) return;
			stopEvent(event);
			this.show();
		}));
		this.own(addDisposableListener(input, "paste", event => {
			stopEvent(event);
			this.show();
		}));
		this.own(addDisposableListener(input, "cut", event => {
			stopEvent(event);
			this.show();
		}));
	}

	show(): void {
		this.element.hidden = false;
		this.element.classList.add("visible");
		this.hideTimer.clear();
		if (this.durationMs === 0) {
			this.hide();
			return;
		}
		const targetWindow = this.element.ownerDocument.defaultView;
		if (!targetWindow) return;
		this.hideTimer.replace(disposableWindowTimeout(targetWindow, () => {
			this.hideTimer.clear();
			this.hide();
		}, this.durationMs));
	}

	hide(): void {
		this.hideTimer.clear();
		this.element.hidden = true;
		this.element.classList.remove("visible");
	}
}

function isMutationKey(event: KeyboardEvent): boolean {
	if (event.ctrlKey || event.metaKey || event.altKey) {
		return (event.ctrlKey || event.metaKey) && !event.shiftKey && (event.key.toLowerCase() === "x" || event.key.toLowerCase() === "v");
	}
	return event.key.length === 1 || event.key === "Backspace" || event.key === "Delete" || event.key === "Enter";
}

function isMutationInput(event: InputEvent): boolean {
	return event.inputType.startsWith("insert") || event.inputType.startsWith("delete") || event.inputType === "historyUndo" || event.inputType === "historyRedo";
}

registerEditorContribution({ id: "editor.contrib.readOnlyMessage", install: context => {
	if (context.kind !== "text" || !context.options.input.readOnly) return;
	context.own(new ReadOnlyMessageController(context.input.element, context.viewport));
} });
